# Composed count predicates

```sh
target/loom check compiler/examples/predicate_sort_contracts
target/loom test compiler/examples/predicate_sort_contracts
target/loom run compiler/examples/predicate_sort_contracts
```

The ordinary source predicate selects a row by its ID **or** label. Its count
is preserved by `std.list.sorted`, even when comparisons modify the original
List. Duplicate rows and overlapping matches still count once. This promises
the declared filtered count, not ordering under an arbitrary comparator.

The prover composes finite indicator laws: intersection, complement, union
with its intersection subtracted, and guarded alternatives. `partition_size`
combines equality and inequality counts under a Boolean parameter;
`empty_prefix` proves zero for a nonpositive scan bound. The original evaluation
and access checks remain separate from this mathematical arithmetic.

Atoms use the existing same-index Int/Bool/Text equality keys with stable needles. Boolean element
reads and `!=` reuse those rules. Expansion keeps its existing work and tree
limits; exhausted reasoning remains unproved. This adds neither arbitrary
predicate induction nor Float, enum or mutable-graph count keys.
