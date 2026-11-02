# Source-library integer sort contracts

This [ordinary source program](main.loom) calls `std.list.sorted` with an `Int`
less-than callback and proves both normal-return guarantees:

- Adjacent elements of the result are in nondecreasing order.
- Each input value has the same occurrence count in the result as at entry.

The contract predicates use ordinary loops. They are proof-only and do not reach
the native program; all six sorting-loop assertions are also removed. The
implementation keeps its stable two-buffer merge sort, with O(n log n)
comparisons and O(n) extra storage. The example exercises empty and single-entry
inputs, duplicates, signed integer extremes and the fresh result container.

Unknown or changing comparison order cannot establish the ordering guarantee.
Permutation remains independent of comparison order, including mutable captured
state. Required unsupported or exhausted proofs reject. Expansion, candidate,
encoding, query and solver-time limits are unchanged.

From the repository root, with the proof solver on `PATH`:

```sh
target/loom check compiler/examples/std_sort_contracts
target/loom test compiler/examples/std_sort_contracts
target/loom run compiler/examples/std_sort_contracts
```

The [integration trial](../../tests/sort_permutation.rs) also checks warm reuse,
equivalent and invalid source edits, proof-predicate elimination, GC-stressed
O0/O2 execution and permutation-only controls for unknown and changing callbacks.
