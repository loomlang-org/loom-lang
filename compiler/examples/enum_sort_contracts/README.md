# Enum row permutation

With the compiler and compile-time Z3 solver available:

```sh
target/loom check compiler/examples/enum_sort_contracts
target/loom test compiler/examples/enum_sort_contracts
target/loom run compiler/examples/enum_sort_contracts
```

`same_signal` uses ordinary source matches to compare `Empty`, `Number(Int)` and
`Label(Text)`. `same_row` also compares the integer key. The occurrence-count
contract preserves complete rows and duplicates through an unknown comparator,
including comparisons that overwrite and append to the original input.
`arranged_ascending` additionally proves adjacent key order with that same entry
permutation, while its known comparison changes and grows the input.

Finite mathematical keys keep each payload with its selected nominal tag and
the other fields of the same row. Inactive payload padding is canonicalized in
those keys, rather than requiring arbitrary unobservable values to survive a
copy. Count needles, array maps and checked store laws share that representation;
source storage and native enum layout are unchanged. Source evaluation checks,
including selected-payload access and arithmetic faults, remain separate.

The runtime cases distinguish empty Text, zero-valued Number and payloadless
variants, retain duplicates and check stable ties. The integration test also
rejects tag/payload corruption, moving signals between different row keys and a
potentially undefined count predicate. Removing each false contract supplies a
valid-source control. Native O0/O2 runs use GC stress.

This is bounded finite counting over immutable inline `Int`, `Bool` and `Text`
keys and nominal alternatives. Float equality, mutable descendant equality,
arbitrary predicate induction and general comparator laws remain separate work.
An entry List freezes its outer values, without freezing shared children.
