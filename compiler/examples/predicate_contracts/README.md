# Predicate contracts

From the repository root:

```sh
target/loom check compiler/examples/predicate_contracts
target/loom build compiler/examples/predicate_contracts --output target/predicate-contracts
target/loom test compiler/examples/predicate_contracts
target/loom run compiler/examples/predicate_contracts
```

The parenthesis validator uses an ordinary state machine. Contracts reuse its
successful result for the same immutable value without deriving a closed formula
for the state machine. The examples cover enums, records, Text equality, checked
construction, unrelated writes, loop preservation, compile-time execution,
async suspension and known List callbacks in both scan directions.

`observed_nonempty` deliberately accepts a partial callback: an empty element
would fault, but a false earlier element stops the scan before that suffix.
Successful evaluation and Boolean truth remain separate proof facts. Removing
the required evidence, changing the argument or inventing safety for an
unexecuted callback does not prove a contract. General mutable-graph predicates
and unknown callbacks remain outside this rule.
