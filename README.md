# tokio-udp-framed (Update!)

**Update** Was able to merge code into `tokio-util` that provides this functionality without breaking the API (thanks Darksonn!). So this crate is effectively dead and you should use `tokio-util` ([link to pr](https://github.com/tokio-rs/tokio/pull/3451))

This started from a copy of `UdpFramed` from `tokio-util` with a few modifications that provides a somewhat different API:

- All `UdpFramed` types take a `Borrow<UdpSocket>` so you can pass an `Arc<UdpSocket>` or `&UdpSocket`
- There are `UpdFramedRecv` and `UdpFramedSend` types for specifically `send` and `recv` in `Sink`/`Stream`
- Because of `Borrow<UdpSocket>` you can't use `get_mut` anymore

The main benefit can be easily explained in an example:

```rust
let a_soc = Arc::new(UdpSocket::bind("127.0.0.1:0").await?);
let b_soc = a_soc.clone();

let a_addr = a_soc.local_addr()?;
let b_addr = b_soc.local_addr()?;

let mut a = UdpFramed::new(a_soc, ByteCodec);
let mut b = UdpFramed::new(b_soc, LinesCodec::new());

let msg = b"1\r\n2\r\n3\r\n".to_vec();
a.send((&msg, b_addr)).await?;

let msg = b"4\r\n5\r\n6\r\n".to_vec();
a.send((&msg, b_addr)).await?;

assert_eq!(b.next().await.unwrap().unwrap(), ("1".to_string(), a_addr));
assert_eq!(b.next().await.unwrap().unwrap(), ("2".to_string(), a_addr));
assert_eq!(b.next().await.unwrap().unwrap(), ("3".to_string(), a_addr));

assert_eq!(b.next().await.unwrap().unwrap(), ("4".to_string(), a_addr));
assert_eq!(b.next().await.unwrap().unwrap(), ("5".to_string(), a_addr));
assert_eq!(b.next().await.unwrap().unwrap(), ("6".to_string(), a_addr));
```
