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
