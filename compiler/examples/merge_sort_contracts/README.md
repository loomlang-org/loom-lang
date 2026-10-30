# Checked recursive merge sort

Run from the repository root with a built compiler and the compile-time Z3 solver:

```sh
target/loom check compiler/examples/merge_sort_contracts
target/loom test compiler/examples/merge_sort_contracts
target/loom run compiler/examples/merge_sort_contracts
target/loom build compiler/examples/merge_sort_contracts --output target/merge-sort
```

`sorted_merge` splits an integer List into fresh Lists, recursively sorts them,
and calls the separate `merge_contracts` package. Its mandatory postconditions
prove ordering and preservation of the input's entry-state occurrence counts,
including duplicate values. The merge package states its result guarantees over
`old(left)` and `old(right)`, so callers can compose them without assuming the
callee preserves current input storage.

The example uses ordinary functions, contracts and leading loop assertions.
It introduces no trusted sort axiom or source annotation. Runtime cases also
check that the input is unchanged. This is an integer example, not a proof of
arbitrary comparators or the general standard-library sorting implementation.
