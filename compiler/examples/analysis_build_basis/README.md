# Checked build basis

This filesystem analysis client prints a source/graph/input basis and a native
input hash without invoking application functions. Compare them with
`basis-sha256` and `checked-sha256` in an actual v5 build receipt:

```sh
target/loom build compiler/examples/analysis_build_basis --output target/inspect-basis
target/loom build compiler/examples/scalar --output target/scalar --receipt target/scalar.receipt
target/inspect-basis compiler/examples/scalar compiler/std
```

The client uses production analysis with no external input reader, target reader,
build options or proof backend. Projects using those capabilities must supply
their actual observations through `analyze_with_inputs`. Optional native proof
outcomes can affect the checked-input hash; a mismatch remains a mismatch.

The basis includes exact source trees as well as source bytes and resolved import
edges. A local fork can change a source refinement in an unused generic function
without changing emitted code or the old v4 receipt fields. Native input alone
cannot identify that semantic context.

The receipt and actual artifact need separate verification. The source basis is
snapshot identity, not filesystem freshness, a signature, semantic equivalence,
storage-mapping evidence or deployment permission.
