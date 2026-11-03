# Generate a package with Loom

This ordinary source-library client builds a record from reflected field names,
parses a tracked declaration input, and emits formatted Loom source. A helper
package combines a generic collection, a known captured callback, a
heterogeneous parameter-pack call and a typed macro. The generic call
checks its explicit callable requirement through the ordinary language rules.

`definition.txt` supplies the generated function and its required postcondition.
The input is captured during compilation; the built generator retains that
snapshot. Rebuilding after an input edit must refresh the generated declaration,
including through a warm frontend cache. Generated code is checked separately
as an ordinary package.

```sh
target/loom build compiler/examples/ast_generation --output target/ast-generator
mkdir -p target/generated-example
target/ast-generator > target/generated-example/main.loom
target/loom check target/generated-example
target/loom test target/generated-example
target/loom run target/generated-example
```

The integration test runs the native generator with GC stress, checks and runs
its output, and edits the tracked declaration and reflected record. Incorrect
generated contracts and incomplete record constructions reject; restoring the
inputs restores identical output. Private helper imports reject too.

This explicit generation step uses the public AST, parser and formatter APIs.
The generated declarations retain normal visibility, contracts, test isolation
and build rules.
