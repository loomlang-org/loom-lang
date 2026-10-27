# Immutable function contracts

From the repository root, with Z3 on `PATH`:

```sh
target/loom check compiler/examples/function_contracts
target/loom test compiler/examples/function_contracts
target/loom run compiler/examples/function_contracts
```

Ordinary source loops compute Int, Text, record, tuple, enum and Float values.
Contracts reuse the result of the same known pure function at the same immutable
arguments, even when the prover cannot expand the loop into a formula. Checked
construction and known callbacks over immutable record/enum List elements use
the same rules. No purity annotation or runtime wrapper is required.

Successful evaluation is independent of the result: a hypothetical call cannot
prove its own safety, termination or output. A Float result can be NaN; numeric
equality cannot substitute signed zeros. Mutable graphs and unknown callbacks
do not gain immutable identities from this mechanism.
