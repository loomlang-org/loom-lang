# Bounded datagram application

This native application combines directory packages, an immutable message with
a checked byte bound, colocated tests, asynchronous UDP and scoped cleanup.

```sh
loom check compiler/examples/bounded_datagrams
loom test compiler/examples/bounded_datagrams --recursive
loom run compiler/examples/bounded_datagrams
loom build compiler/examples/bounded_datagrams --output bounded-datagrams
./bounded-datagrams 'hello'
```

The protocol package admits at most 64 UTF-8 bytes. `message` checks external
Text; `decode` rejects malformed UTF-8 and copies the datagram into immutable
Text. Mutating the original Bytes cannot change a published message. The
representation lives in a private `Payload` record. Other packages can pass a
`Message` and call its public functions, but cannot inspect that private record.
Both `text` and `encode` have mandatory length postconditions. The integration
test changes `encode` to append a byte and requires the ordinary checker to
reject the unproved postcondition.

The transport binds an ephemeral IPv4 loopback port, starts a single echo
receive, and sends from a connected scoped client. The server explicitly
detaches its scoped socket to share it between the protocol Tasks and registers
`defer close` before starting them. The client acquires its owner inside its
Task and directly awaits borrowed I/O. Cancellation drains pending I/O before
the server's deferred close. A ten-second deadline requests cancellation; it
does not promise a hard wall-clock bound. UDP delivery and ordering remain
properties of the network, so failures produce a nonzero application exit.

Root tests cover the real exchange; protocol tests cover the private
representation, mutable-input isolation and malformed/oversized input. Running
the root tests alone does not include dependency-package tests; `--recursive`
runs both package scopes. Editor queries use the same ordinary checked source
without running the application. See the [compiler guide](../../README.md) and
[scoped UDP API](../../std/net/udp/README.md).
