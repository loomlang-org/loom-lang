# Compile-time callable inspection

`comptime_callable(callback)` inspects a known function without invoking it.
It returns true when the bounded closed execution check and CTFE operation
validator admit its selected body and entry conditions. Ordinary `comptime`
argument rules apply; non-function stored values return false.

```loom
import std.reflect.functions.comptime_callable

pub fn selected[T](value T, comptime action fn(T) T) T {
    comptime if comptime_callable(action) {
        action(value)
    } else {
        value
    }
}
```

True grants no result, successful-evaluation or termination guarantee. Arguments
must still be available in the phase where a call runs. CTFE can mutate supplied
Lists/Bytes and live capture cells; this is not a read-only or isolation promise.
Indirect/dynamic calls and implicit resource traversals return false,
including cases that another CTFE execution may resolve from actual values.

Selected bodies must type-check and prove their required contracts. Those errors
remain errors; inspection cannot catch them. Cyclic capability inspection also
rejects, so a callback cannot choose its effects from its own classification.
Preparing a body follows ordinary tracked compile-time inputs, while the callback
itself does not execute. Abstract queries provide independent Boolean choices,
not purity axioms. Concrete instances recheck their actual target.

Queries create no runtime registry and retain no native callback root or capture
copy. See the [executable example](../../../examples/callable_reflection).

`read_only_callable(callback)` further checks the compiler's existing bounded
read-only graph rules. A true result admits reads of inputs and captured state
through a closed checked call graph, including entry conditions, while excluding
shared writes, I/O, tasks and implicit cleanup. Required contracts are checked
before classification. False includes unsupported effects, erased dispatch and
exhausted reasoning; it is not evidence that a function necessarily writes.

```loom
import std.reflect.functions.read_only_callable

pub fn read_only_apply[Ts..., R](comptime action fn(Ts...) R, values Ts...) R
requires read_only_callable(action) {
    action(values...)
}
```

The argument follows ordinary compile-time identity rules. A known callback's
live environment is inspected by type, without reading or copying its values.
Its result may share mutable data with an input or capture; later alias writes
remain visible. The query grants no frozen snapshot, invocation domain, result
property or termination guarantee. The example exercises a constrained row
conversion, a shared captured List, an excluded mutating callback, generic packs
and explicit invocations after inspection. These facts do not establish a
database mapping or authorize a migration.
