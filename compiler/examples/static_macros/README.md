# Static macro parameters

```sh
target/loom check compiler/examples/static_macros
target/loom test compiler/examples/static_macros
target/loom run compiler/examples/static_macros
```

A named source macro generator may declare `comptime` parameters alongside its
single `List[std.reflect.Schema]` parameter. An invocation supplies static
operands first, in their declaration order, followed by the ordinary input
expressions:

```loom
fn field(comptime name Text, types List[Schema]) Text {
    assert length(types) == 1
    concat("$0.", name)
}

field!("lower", value)
```

The descriptor list contains only the ordinary inputs; `$0` names `value`.
Static operands use the ordinary rules for `comptime` parameters, including
immutable aggregates and function values. Available captured environments may
be evaluated during generation; a runtime local or live runtime capture cannot
be read at compile time. The generator must remain pure and satisfy its
preconditions. Its descriptor parameter may appear anywhere in its declaration.

The example renames and reorders fields across two nominal record types. The
generated projections retain the source record's whole-record constraint, so
the ordinary checker proves the destination constructor and the adapter's
postcondition. It also exercises a generic forwarding function, an input with
an effect that runs once, and a callback with a compile-time available capture.

Generated names resolve in the generator's package. Private fields and required
proofs still use the ordinary access and checking rules. Static parameters
configure generation; they are not generated lexical bindings. Macro invocation
still requires a named source generator, and generator overloads remain subject
to the existing ambiguity check. This feature does not certify an arbitrary
database migration or give an unavailable runtime capture a compile-time value.
