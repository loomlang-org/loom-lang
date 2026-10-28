# Immutable function contracts

From the repository root, with Z3 on `PATH`:

```sh
target/loom check compiler/examples/function_contracts
target/loom test compiler/examples/function_contracts
target/loom run compiler/examples/function_contracts
```

Ordinary source loops compute Int, Text, record, tuple, enum and Float values.
Contracts reuse the result of the same known pure function at the same immutable
input projection, even when the prover cannot expand the loop into a formula. Checked
construction and known callbacks over immutable record/enum List elements use
the same rules. No purity annotation or runtime wrapper is required.

Known closures retain the values in read-only capture slots, including nested
immutable records/enums and exact Float representations. The example composes
them through direct calls, List scans and suspension. Receiver evaluation keeps
its capture before later argument effects. Mutable capture cells and shared
collections cannot supply immutable application identities.

An inline context may also carry List/Bytes fields that the helper never
observes. Its key retains the immutable projection and omits those handles.
Copying or forwarding the context does not read the mutable contents. The
`inputs.loom` example mutates those siblings after a call, and preserves the
result relation through forwarding, captured contexts and suspension. Primitive
reads and writes, control conditions and entry predicates are checked across
the closed call graph; a mutable observation cannot be omitted. Every argument
still evaluates, including the ones absent from the key. This is a conservative
structural projection, not minimal dependency inference or mutable-graph induction.

Successful evaluation is independent of the result: a hypothetical call cannot
prove its own safety, termination or output. A Float result can be NaN; numeric
equality cannot substitute signed zeros. Mutable graphs and unknown callbacks
do not gain immutable identities from this mechanism.
