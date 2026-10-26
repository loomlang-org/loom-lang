# Source HTTP server

```sh
target/loom check compiler/examples/http_server
target/loom test compiler/examples/http_server
target/loom run compiler/examples/http_server
```

The application opens a real loopback TCP listener and sends POST/HEAD requests
over one persistent connection. `std.net.http.server` authorizes the effective
authority, rewrites a conflicting Host for an absolute-form target, invokes the
source handler in order, and marks its second response as the last response.
The caller retains and closes both transport identities.

The native integration test also connects this same serving function to an
independent Rust peer, checking binary content, HEAD framing and connection
closure, including a client waiting for 100 Continue before sending its body,
at O0/O2 with forced GC. An optional address argument selects that peer
mode; it is not an HTTP URL or a request to an external server.
