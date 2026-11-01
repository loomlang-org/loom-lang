# Complete record permutation

With the compiler and compile-time Z3 solver available:

```sh
target/loom check compiler/examples/value_sort_contracts
target/loom test compiler/examples/value_sort_contracts
target/loom run compiler/examples/value_sort_contracts
```

`same_row` compares every field of an immutable record using ordinary source
code: a nested integer key, amount, Boolean flag and Text label. The sorting
wrapper proves entry length and full-row occurrence counts, including duplicates,
through an unknown comparator that can change and grow the original input.

The proof joins the fields at the same sequence index into one mathematical key.
It preserves their association; separate column counts would allow amounts to
move between different rows. Conjunction and equality-operand order normalize
without changing the source's evaluation checks. Finite histogram laws are shared
by direct integer keys and structural keys, with no runtime histogram or tag.

The supported count fragment is a conjunction of `Int`, `Bool` and `Text`
equalities over one element's inline fields and stable needles. Float equality,
enum alternatives, mutable descendants and arbitrary equality predicates remain
separate work. Comparator ordering laws are not inferred by permutation.
