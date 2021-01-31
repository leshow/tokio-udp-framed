# tokio-udp-framed

This is a copy of `UdpFramed` from `tokio-util` with a few modifications that provides a somewhat different API:

- There are `UpdFramedRecv` and `UdpFramedSend` types for specifically `send` and `recv` in `Sink`/`Stream`
- All `UdpFramed` types take a `Borrow<UdpSocket>` so you can pass an `Arc<UdpSocket>` or `&UdpSocket`
- Because of the above, you can no longer consume `UdpFramed` to get your `UdpSocket` back
