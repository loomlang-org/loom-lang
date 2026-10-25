# HTTP/1.1 connection serving

`std.net.http.server` serves one caller-owned connection using ordinary Streams,
Tasks and the source HTTP codec. It adds no compiler or runtime operations.

```loom
let sink = async fn(bytes Bytes) Result[Int, TcpError] {
    write_bytes(connection, bytes).await
}
let result = serve(read(connection, 16384), packet, sink, handle, Options {
        secure = false
        authorities = ["example.test:8080"]
        max_requests = 100
        limits = limits()
    }).await
```

See the complete [TCP example](../../../../examples/http_server/main.loom).
Run `target/loom run compiler/examples/http_server` or
`target/loom test compiler/std/net/http/server`.

## Policy and ordering

`serve(source, bytes, sink, handler, options)` returns
`Task[Result[Int, Error[E, F, H]]]`. The converter maps `S.Item` to
`Result[Bytes, E]`; the sink returns `Task[Result[Int, F]]`; the handler accepts
an ordinary `Request` and returns `Task[Result[Response, H]]`.
Success counts fully written responses. Input, Output and Handler errors retain
their typed causes; Count reports an incorrect sink byte count.

Options are validated and privately snapshotted synchronously, before any Task
can pull input or invoke application code. Authorities must be a nonempty explicit
allowlist; `max_requests` must be positive. The codec's nonnegative line, metadata
and body limits apply to each message. Producers should supply bounded chunks.
Bodies are collected within those limits, not streamed into the handler.

Authority syntax and normalization reuse the HTTP(S) URL parser: ASCII host
case and default ports normalize, but IDNA and general IPv6 spelling equivalence
are not supplied. `secure` selects the scheme/default port of the caller-provided
connection; it does not create TLS. DNS names are not resolved for authorization.

Origin-form uses Host; absolute-form uses the target URI's authority, not a
conflicting Host. The handler receives a canonical Host and the origin-form
path/query without decoding escapes or removing dot segments. `OPTIONS *` stays
explicit. Invalid or unapproved targets reject before collecting a body or
calling the handler. CONNECT, Upgrade and unsupported expectations reject at
that same boundary. An authorized `Expect: 100-continue` request receives its
interim response before body collection; interim responses do not count toward
the request limit. Library-generated policy errors contain no received authority
or body data.

Requests and responses remain sequential. Connection options are case-insensitive
token lists. A request or response containing `close`, or the configured request
limit, ends the loop after the current response. No later request is processed,
even if its bytes were already prefetched. An intentional close is advertised
with `Connection: close` on its final response. Response headers are copied
before close-policy checks and encoding, so retained mutable values cannot
change that decision afterward.

The handler supplies a final response, not a 1xx response or protocol switch.
Ordinary codec framing still applies: HEAD/bodyless responses require an empty
body; HEAD can carry explicit representation-length metadata.
The sink must consume each complete buffer or return an error. Partial failures
are not retried or rolled back; a failure never advances the pipeline.

## Lifetimes and boundaries

The library scopes each incoming message, uses `std.net.http.collect` after
header policy succeeds, and retires its reader on completion, error, fault or
cancellation. Cancellation drains pending children before cleanup; already
completed application effects or output are not undone.

The source and sink borrow a caller-owned transport. Close that transport after
`serve` returns on every outcome; graceful TCP half-close/drain policy, TLS,
accept-loop concurrency and deadlines remain explicit caller policy. This is not
a listening framework, proxy, router, connection pool, or unbounded server loop.
Use a caller-managed producer, as with codec readers; this does not add general
borrow-retaining adapters for scoped producers. Use the lower-level HTTP
body/writer APIs for streaming applications.

The policy follows [RFC 9112 target forms and persistence](https://www.rfc-editor.org/rfc/rfc9112.html#section-3.2)
and [RFC 9110 authority handling](https://www.rfc-editor.org/rfc/rfc9110.html#section-7.2).
