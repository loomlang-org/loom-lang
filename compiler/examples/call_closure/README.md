# Inspect checked calls

This native public-library client loads and checks a source package, locates its
unique root `main`, and inspects direct calls with `std.loom.analysis.call_closure`.

```sh
loom run compiler/examples/call_closure -- /path/to/app /path/to/std boundary
```

`boundary` names one unique function in the application's root package. Pass an
empty final argument to inspect without cutting a callee. The client prints the
number of call sites, runtime/intrinsic operations and unresolved issues. It does
not invoke the application's runtime functions. Loading and checking still uses
ordinary source visibility, contracts and tracked compile-time inputs.

A cut records an opaque function call; argument construction remains visible.
Zero exposed operations does not establish purity or justify the cut. Nonzero
issues retain unresolved calls or inspection limits. This is a library example,
not a deployment check or an artifact-compatibility certificate.
