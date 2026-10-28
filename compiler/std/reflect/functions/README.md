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
