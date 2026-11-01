# Source-library sorting permutation

With the compiler and compile-time Z3 solver available, run:

```sh
target/loom check compiler/examples/sort_permutation
target/loom test compiler/examples/sort_permutation
target/loom run compiler/examples/sort_permutation
```

`sorted_permutation` proves that `std.list.sorted` preserves the entry length
and every input integer's occurrence count, including duplicates. The comparator
is an ordinary unknown function and may modify or grow the original input.
The library sorts private snapshots; the test exercises those comparison effects.

The proof uses checked source bodies, finite histogram algebra and inductive
storage frames. No sorting axiom, comparator-purity requirement or increased
proof budget is used. Numeric loop assertions in the library can be statically
removed; an unproved assertion retains its runtime behavior.
Ordering laws and permutation for arbitrary element types remain separate work.
