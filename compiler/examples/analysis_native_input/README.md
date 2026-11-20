# Analysis and native compiler input

This example compares the native input of a checked filesystem project with an
in-memory edit of its `main` body. It prints two SHA-256 values while leaving the
source files unchanged. The original hash can be compared with `checked-sha256`
in an ordinary local build receipt of that project.

```sh
target/loom build compiler/examples/analysis_native_input --output target/inspect-input
target/loom build compiler/examples/call_closure --output target/subject --receipt target/subject.receipt
target/inspect-input compiler/examples/call_closure compiler/std
```

The inspected project must have a nonempty `main` body that can be replaced by
an empty body. The example checks that the edited AST has different native input,
even though both analyses retain identical source text, and that the original
analysis remains current against a fresh filesystem load. It analyzes and
prepares the programs without invoking their application functions.

The example uses ordinary analysis without an external proof backend or custom
build inputs. A project needing those capabilities must supply the same observed
inputs, target options and proof backend as its build. Optional proof outcomes
can also affect prepared input. A mismatch must remain a mismatch.

The native input format belongs to the compiler/backend boundary. A matching
hash links input bytes to a trusted local receipt; callers must separately
verify the receipt, artifact bytes and relevant source/configuration/input
basis. It provides no storage or deployment compatibility permission.
