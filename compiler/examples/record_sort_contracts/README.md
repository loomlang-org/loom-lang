# Record column contracts

With the compiler and compile-time Z3 solver available:

```sh
target/loom check compiler/examples/record_sort_contracts
target/loom test compiler/examples/record_sort_contracts
target/loom run compiler/examples/record_sort_contracts
```

The ordinary source-library sort preserves the entry counts of nested `key.id`
and sibling `amount` fields, even when comparisons change and grow the original
input. A copy loop also extracts the keys into `List[Int]` with checked entry
length and occurrence counts.

Histograms retain their storage version, complete inline-field path and prefix.
Each requested column has its own checked conservation equation. Equal counts
in separate columns do not prove that complete rows, or their field pairings,
are preserved. General element equality and comparator ordering laws remain open;
mutable descendants retain the existing conservative effect rules.
