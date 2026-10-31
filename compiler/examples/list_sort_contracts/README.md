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
The generic wrapper covers integers, recursive records with shared cyclic
children, and closures that capture the original input. Copies keep sharing
their element graphs; the comparator can mutate those graphs. The example also
constructs a length-three type directly from the checked result.

Element-graph exclusions keep private buffer headers separate from callback
effects. Stores, publication and every loop backedge must preserve that evidence;
an inserted backreference can invalidate it. This supplies no comparator law or
general ordering/permutation guarantee. The standard library's runtime sorting
tests cover its other documented behavior.
