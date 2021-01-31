use tokio_util::codec::{Decoder, Encoder};

use pin_project_lite::pin_project;
use tokio::{io::ReadBuf, net::UdpSocket};
use tokio_stream::Stream;

use bytes::{BufMut, BytesMut};

use futures_core::ready;
use futures_sink::Sink;

use std::{
    borrow::{Borrow, BorrowMut},
    io,
    mem::MaybeUninit,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    #[derive(Debug)]
    pub(crate) struct FramedImpl<T, U, State> {
        #[pin]
        pub(crate) inner: T,
        pub(crate) state: State,
        pub(crate) codec: U,
    }
}

const INITIAL_CAPACITY: usize = 8 * 1024;

pub(crate) struct ReadFrame {
    pub(crate) eof: bool,
    pub(crate) is_readable: bool,
    pub(crate) buffer: BytesMut,
}

pub(crate) struct WriteFrame {
    pub(crate) buffer: BytesMut,
}

#[derive(Default)]
pub(crate) struct RWFrames {
    pub(crate) read: ReadFrame,
    pub(crate) write: WriteFrame,
}

impl Default for ReadFrame {
    fn default() -> Self {
        Self {
            eof: false,
            is_readable: false,
            buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
        }
    }
}

impl Default for WriteFrame {
    fn default() -> Self {
        Self {
            buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
        }
    }
}

impl From<BytesMut> for ReadFrame {
    fn from(mut buffer: BytesMut) -> Self {
        let size = buffer.capacity();
        if size < INITIAL_CAPACITY {
            buffer.reserve(INITIAL_CAPACITY - size);
        }

        Self {
            buffer,
            is_readable: size > 0,
            eof: false,
        }
    }
}

impl From<BytesMut> for WriteFrame {
    fn from(mut buffer: BytesMut) -> Self {
        let size = buffer.capacity();
        if size < INITIAL_CAPACITY {
            buffer.reserve(INITIAL_CAPACITY - size);
        }

        Self { buffer }
    }
}

impl Borrow<ReadFrame> for RWFrames {
    fn borrow(&self) -> &ReadFrame {
        &self.read
    }
}
impl BorrowMut<ReadFrame> for RWFrames {
    fn borrow_mut(&mut self) -> &mut ReadFrame {
        &mut self.read
    }
}
impl Borrow<WriteFrame> for RWFrames {
    fn borrow(&self) -> &WriteFrame {
        &self.write
    }
}
impl BorrowMut<WriteFrame> for RWFrames {
    fn borrow_mut(&mut self) -> &mut WriteFrame {
        &mut self.write
    }
}

pin_project! {
    #[derive(Debug)]
    pub(crate) struct UdpFramedImpl<T, U, State> {
        #[pin]
        pub(crate) inner: T,
        pub(crate) state: State,
        pub(crate) codec: U,
        pub(crate) current_addr: Option<SocketAddr>,
        pub(crate) out_addr: SocketAddr,
        pub(crate) flushed: bool,
    }
}

pub(crate) const INITIAL_RD_CAPACITY: usize = 64 * 1024;
pub(crate) const INITIAL_WR_CAPACITY: usize = 8 * 1024;

impl<T, C, R> Stream for UdpFramedImpl<T, C, R>
where
    T: Borrow<UdpSocket>,
    C: Decoder,
    R: BorrowMut<ReadFrame>,
{
    type Item = Result<(C::Item, SocketAddr), C::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let pin = self.project();

        let read_state: &mut ReadFrame = pin.state.borrow_mut();
        read_state.buffer.reserve(INITIAL_RD_CAPACITY);

        loop {
            // Are there are still bytes left in the read buffer to decode?
            if read_state.is_readable {
                if let Some(frame) = pin.codec.decode_eof(&mut read_state.buffer)? {
                    let current_addr = pin
                        .current_addr
                        .expect("will always be set before this line is called");

                    return Poll::Ready(Some(Ok((frame, current_addr))));
                }

                // if this line has been reached then decode has returned `None`.
                read_state.is_readable = false;
                read_state.buffer.clear();
            }

            // We're out of data. Try and fetch more data to decode
            let addr = unsafe {
                // Convert `&mut [MaybeUnit<u8>]` to `&mut [u8]` because we will be
                // writing to it via `poll_recv_from` and therefore initializing the memory.
                let buf =
                    &mut *(read_state.buffer.chunk_mut() as *mut _ as *mut [MaybeUninit<u8>]);
                let mut read = ReadBuf::uninit(buf);
                let ptr = read.filled().as_ptr();
                let res = ready!((*pin.inner).borrow().poll_recv_from(cx, &mut read));

                assert_eq!(ptr, read.filled().as_ptr());
                let addr = res?;
                read_state.buffer.advance_mut(read.filled().len());
                addr
            };

            *pin.current_addr = Some(addr);
            read_state.is_readable = true;
        }
    }
}

impl<T, I, C, W> Sink<(I, SocketAddr)> for UdpFramedImpl<T, C, W>
where
    T: Borrow<UdpSocket>,
    C: Encoder<I>,
    C::Error: From<io::Error>,
    W: BorrowMut<WriteFrame>,
{
    type Error = C::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if !self.flushed {
            match self.poll_flush(cx)? {
                Poll::Ready(()) => {}
                Poll::Pending => return Poll::Pending,
            }
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: (I, SocketAddr)) -> Result<(), Self::Error> {
        let (frame, out_addr) = item;

        let pin = self.project();
        let write_state: &mut WriteFrame = pin.state.borrow_mut();

        pin.codec.encode(frame, &mut write_state.buffer)?;
        *pin.out_addr = out_addr;
        *pin.flushed = false;

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let pin = self.project();
        if *pin.flushed {
            return Poll::Ready(Ok(()));
        }

        let write_state: &mut WriteFrame = pin.state.borrow_mut();
        let n = ready!((*pin.inner)
            .borrow()
            .poll_send_to(cx, &write_state.buffer, *pin.out_addr))?;

        let wrote_all = n == write_state.buffer.len();
        write_state.buffer.clear();
        *pin.flushed = true;

        let res = if wrote_all {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to write entire datagram to socket",
            )
            .into())
        };

        Poll::Ready(res)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }
}
