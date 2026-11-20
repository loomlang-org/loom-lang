# Scoped UDP sockets

`bind_owned(numeric_address).await` returns `Result[BoundSocket, UdpError]`.
`connect_owned(numeric_peer).await` and `connect_owned(host, port).await` return
`Result[ConnectedSocket, UdpError]`. Extract their successful results with
`scoped`:

```loom
scoped server = bind_owned("127.0.0.1:0").await?
scoped client = connect_owned(local_address(server)?).await?
discard send(client, to_bytes("hello")).await?
let packet = receive(server).await?
discard send(server, packet.data, packet.sender).await?
let reply = receive(client).await?
```

Scope exit closes the socket after its borrowed operations drain, including
faults and cancellation. The acquiring Task constructs the resource itself;
completed but unclaimed acquisition results retain their cleanup too. A private
resource payload prevents copying or reboxing another owner. Calls borrowing a
scoped socket must be awaited directly. See the runnable
[UDP example](../../../examples/udp_echo/README.md) and its
[scoped exchange](../../../examples/udp_echo/owned.loom).

`BoundSocket` supports addressed sends and receives. `ConnectedSocket` supports
sends to its fixed peer and kernel-filtered receives. Both use the existing
packet, limit and truncation rules: empty packets are valid, a receive returns
one fresh `Datagram`, and oversized packets report `UdpError.Truncated`.
Numeric connection success establishes a local peer, without a remote
handshake or delivery guarantee. The hostname overload retains resolver order
and drains a running lookup when cancelled.

Both types expose local addresses, buffer-size queries/setters and idempotent
`close`. Connected sockets also expose their peer address; bound sockets support
broadcast and the existing IPv4/IPv6 multicast membership and outbound options.
OS rounding, interface requirements and partial option-failure semantics still
apply. Operations on closed or detached owners return `UdpError.Close`.

`detach(owner)?` explicitly transfers cleanup to the returned copyable `Socket`
or `Connection`, preserving the bound/fixed-peer mode. The owner becomes
inactive and its later disposal leaves the transferred socket open. Register
manual cleanup before another fallible operation or suspension:

```loom
let raw = detach(server)?
defer { discard close(raw) }
```

The `bind`/`connect` APIs return these manually managed identities directly.
They remain useful for protocols that explicitly share pending Tasks and drain
them before close. An ordinary wrapper around `bind().await` or `connect().await`
does not give their unclaimed copyable results resource cleanup.
