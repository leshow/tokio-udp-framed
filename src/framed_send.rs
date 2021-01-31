use tokio_util::codec::Encoder;

use crate::framed_impl::{UdpFramedImpl, WriteFrame};

use pin_project_lite::pin_project;
use tokio::net::UdpSocket;

use bytes::BytesMut;
use futures_sink::Sink;
use std::{
    borrow::Borrow,
    fmt, io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

pin_project! {
    /// A [`Sink`] of frames encoded for udp.
    ///
    /// [`Sink`]: futures_sink::Sink
    pub struct UdpFramedSend<T, C> {
        #[pin]
        inner: UdpFramedImpl<T, C, WriteFrame>,
    }
}

impl<T, C> UdpFramedSend<T, C>
where
    T: Borrow<UdpSocket>,
{
    /// Create a new `UdpFramed` backed by the given socket and codec.
    ///
    /// See struct level documentation for more details.
    pub fn new(socket: T, codec: C) -> UdpFramedSend<T, C> {
        Self {
            inner: UdpFramedImpl {
                codec,
                state: WriteFrame {
                    buffer: BytesMut::with_capacity(crate::framed_impl::INITIAL_WR_CAPACITY),
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
    pub fn encoder(&self) -> &C {
        &self.inner.codec
    }

    /// Returns a mutable reference to the underlying codec wrapped by
    /// `UdpFramed`.
    ///
    /// Note that care should be taken to not tamper with the underlying codec
    /// as it may corrupt the stream of frames otherwise being worked with.
    pub fn encoder_mut(&mut self) -> &mut C {
        &mut self.inner.codec
    }

    /// Consumes the `Framed`, returning its underlying I/O stream.
    pub fn into_inner(self) -> T {
        self.inner.inner
    }
}

// This impl just defers to the underlying FramedImpl
impl<T, I, U> Sink<(I, SocketAddr)> for UdpFramedSend<T, U>
where
    T: Borrow<UdpSocket>,
    U: Encoder<I>,
    U::Error: From<io::Error>,
{
    type Error = U::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: (I, SocketAddr)) -> Result<(), Self::Error> {
        self.project().inner.start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx)
    }
}

impl<T, C> fmt::Debug for UdpFramedSend<T, C>
where
    T: Borrow<UdpSocket>,
    C: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UdpFramedSend")
            .field("io", self.get_ref())
            .field("codec", self.encoder())
            .field("buffer", &self.inner.state.buffer)
            .finish()
    }
}
