# Entry shape through source-library sorting

```sh
target/loom check compiler/examples/list_sort_contracts
target/loom test compiler/examples/list_sort_contracts
target/loom run compiler/examples/list_sort_contracts
target/loom build compiler/examples/list_sort_contracts --output target/list-sort-contracts
```

`preserved_count` proves its result length from the ordinary `std.list.sorted`
body. The comparison callback may modify the original input; the result keeps
its entry count because sorting first copies the input into private buffers.
The example also constructs a length-three type directly from the checked result.

This proof covers integer elements. It supplies no comparator law, ordering or
permutation guarantee, or a frame for arbitrary mutable element graphs. The
standard library's runtime sorting tests cover its other documented behavior.
