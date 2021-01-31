use tokio_util::codec::Decoder;

use crate::framed_impl::{ReadFrame, UdpFramedImpl};

use pin_project_lite::pin_project;
use tokio::net::UdpSocket;
use tokio_stream::Stream;

use bytes::BytesMut;
use std::{
    borrow::Borrow,
    fmt,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    /// A [`Stream`] of messages decoded from a [`UdpSocket`].
    ///
    /// [`Stream`]: tokio::stream::Stream
    /// [`AsyncRead`]: tokio::udp::UdpSocket
    pub struct UdpFramedRecv<T, C> {
        #[pin]
        inner: UdpFramedImpl<T, C, ReadFrame>,
    }
}

impl<T, C> UdpFramedRecv<T, C>
where
    T: Borrow<UdpSocket>,
{
    /// Create a new `UdpFramed` backed by the given socket and codec.
    ///
    /// See struct level documentation for more details.
    pub fn new(socket: T, codec: C) -> UdpFramedRecv<T, C> {
        Self {
            inner: UdpFramedImpl {
                codec,
                state: ReadFrame {
                    buffer: BytesMut::with_capacity(crate::framed_impl::INITIAL_RD_CAPACITY),
                    ..ReadFrame::default()
                },
                inner: socket,
                current_addr: None,
                out_addr: ([0, 0, 0, 0], 0).into(),
                flushed: true,
            },
        }
    }

    /// Returns a reference to the underlying I/O stream wrapped by `Framed`.
    ///
    /// # Note
    ///
    /// Care should be taken to not tamper with the underlying stream of data
    /// coming in as it may corrupt the stream of frames otherwise being worked
    /// with.
    pub fn get_ref(&self) -> &UdpSocket {
        self.inner.inner.borrow()
    }

    /// Returns a reference to the underlying codec wrapped by
    /// `Framed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec(&self) -> &C {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying codec wrapped by
    /// `UdpFramed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn codec_mut(&mut self) -> &mut C {
        &mut self.inner.codec
    }

    /// Returns a reference to the read buffer.
    pub fn read_buffer(&self) -> &BytesMut {
        &self.inner.state.buffer
    }

    /// Returns a mutable reference to the read buffer.
    pub fn read_buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.inner.state.buffer
    }

    /// Consumes the `Framed`, returning its underlying I/O stream.
    pub fn into_inner(self) -> T {
        self.inner.inner
    }
}

impl<T, C> Stream for UdpFramedRecv<T, C>
where
    T: Borrow<UdpSocket>,
    C: Decoder,
{
    type Item = Result<(C::Item, SocketAddr), C::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

impl<T, C> fmt::Debug for UdpFramedRecv<T, C>
where
    T: Borrow<UdpSocket>,
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpFramedRecv")
            .field("io", self.get_ref())
            .field("codec", self.codec())
            .field("current_addr", &self.inner.current_addr)
            .field("is_readable", &self.inner.state.is_readable)
            .field("eof", &self.inner.state.eof)
            .field("read_buffer", &self.read_buffer())
            .finish()
    }
}
