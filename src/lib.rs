//! # tokio-udp-framed
//!
//! This started from a copy of `UdpFramed` from `tokio-util` with a few modifications that provides a somewhat different API:
//!
//! - All `UdpFramed` types take a `Borrow<UdpSocket>` so you can pass an `Arc<UdpSocket>` or `&UdpSocket`
//! - There are `UpdFramedRecv` and `UdpFramedSend` types for specifically `send` and `recv` in `Sink`/`Stream`
//! - Because of `Borrow<UdpSocket>` you can't use `get_mut` anymore
//!
//! The main benefit can be easily explained in an example:
//!
//! ```rust
//! # use std::sync::Arc;
//! # use tokio::net::UdpSocket;
//! use tokio_util::codec::{Decoder, Encoder, LinesCodec};
//! use tokio_udp_framed::UdpFramed;
//!
//! let a_soc = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
//! let b_soc = a_soc.clone();
//!
//! let a_addr = a_soc.local_addr()?;
//! let b_addr = b_soc.local_addr()?;
//! // `UdpFramed` is created from an `Arc<UdpSocket>` here!!
//! let mut a = UdpFramed::new(a_soc, ByteCodec);
//! // we can make another from a cloned on here!!
//! let mut b = UdpFramed::new(b_soc, LinesCodec::new());
//!
//! let msg = b"1\r\n2\r\n3\r\n".to_vec();
//! a.send((&msg, b_addr)).await?;
//!
//! let msg = b"4\r\n5\r\n6\r\n".to_vec();
//! a.send((&msg, b_addr)).await?;
//!
//! assert_eq!(b.next().await.unwrap().unwrap(), ("1".to_string(), a_addr));
//! assert_eq!(b.next().await.unwrap().unwrap(), ("2".to_string(), a_addr));
//! assert_eq!(b.next().await.unwrap().unwrap(), ("3".to_string(), a_addr));
//!
//! assert_eq!(b.next().await.unwrap().unwrap(), ("4".to_string(), a_addr));
//! assert_eq!(b.next().await.unwrap().unwrap(), ("5".to_string(), a_addr));
//! assert_eq!(b.next().await.unwrap().unwrap(), ("6".to_string(), a_addr));
//!
//! pub struct ByteCodec;
//!
//! impl Decoder for ByteCodec {
//!     type Item = Vec<u8>;
//!     type Error = io::Error;
//!
//!     fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Vec<u8>>, io::Error> {
//!         let len = buf.len();
//!         Ok(Some(buf.split_to(len).to_vec()))
//!     }
//! }
//!
//! impl Encoder<&[u8]> for ByteCodec {
//!     type Error = io::Error;
//!
//!     fn encode(&mut self, data: &[u8], buf: &mut BytesMut) -> Result<(), io::Error> {
//!         buf.reserve(data.len());
//!         buf.put_slice(data);
//!         Ok(())
//!     }
//! }
//! ```
mod frame;
mod framed_impl;
mod framed_recv;
mod framed_send;

pub use frame::UdpFramed;
pub use framed_recv::UdpFramedRecv;
pub use framed_send::UdpFramedSend;
