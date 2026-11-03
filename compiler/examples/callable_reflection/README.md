# Inspecting compile-time callbacks

```sh
target/loom check compiler/examples/callable_reflection
target/loom test compiler/examples/callable_reflection
target/loom run compiler/examples/callable_reflection
```

The example classifies source functions, a partial function, a nonterminating
function and a stateful closure without running them. It checks a public callback
requirement and a generic `comptime if` fallback. The stateful callback runs only
at its explicit calls and keeps its live state across inspection.

CTFE eligibility permits mutable arguments; it does not establish a call's
domain, result property, termination or interference safety. The effectful
callback and unknown runtime dispatch are not admitted. See the
[API boundary](../../std/reflect/functions/README.md).

The same program uses `read_only_callable` to select a read-only callback through
an ordinary generic wrapper and to transform a row into a constrained result.
Shared-input writes and a stateful counter are unproved as read-only. An admitted
captured List stays shared: explicit writes after inspection remain visible to
the next call. Both queries preserve required-contract failures and leave
definedness, result properties and termination as separate obligations.
