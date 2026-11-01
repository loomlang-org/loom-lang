# Finite relation contracts

```sh
target/loom check compiler/examples/relation_contracts
target/loom test compiler/examples/relation_contracts
target/loom run compiler/examples/relation_contracts
target/loom build compiler/examples/relation_contracts --output target/relation-contracts
```

The endpoint functions prove a relation between any two distinct, increasing
indices from adjacent observations. The relations are ordinary source predicates:
Text length order and a record's lexicographic order. Their transitivity helpers
declare `ensures result`, which the compiler proves for arbitrary input Lists.
The endpoint functions therefore need only bounds and adjacency as preconditions.
Each returns `true`; its declared endpoint relation is established statically.

The finite induction rule preserves the complete predicate, captured values,
field projections, content versions and enclosing ranges. It applies to admitted
indexed observations without classifying user types or naming comparison helpers.
Its transitivity premise must follow from the encoded relation or established
contract facts. The law helper declarations alone supply no proof.
Nested `all` calls retain their actual checked callbacks. Rebinding or mutating
the input cannot reuse an earlier content version's facts.

The step relation has a loop whose result is modeled independently of successful
evaluation. Its endpoint contract explicitly requires comparability, adjacency
and transitivity over the input. These are executed source requirements, not
assumed properties of every step relation or a proof of its loop's termination.
The endpoint proof keeps both the returned value and call-safety obligations.
Relation queries defer eager quantified instantiation within the ordinary solver
time limit; they do not remove premises or retry a timeout as proof.

Reflexivity, successful evaluation and termination remain separate obligations.
More complex partial-function queries can still exhaust proof search and reject.
Generic comparator parameters need an explicit purity requirement; this example
uses closed source relations. The ordering/permutation proof for
`std.list.sorted` remains open.
