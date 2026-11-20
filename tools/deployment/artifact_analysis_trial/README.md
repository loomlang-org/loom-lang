# Artifact-associated source analysis

`deployment.artifact_analysis.reproduce_analysis(analysis, request)` associates a
retained `std.loom.analysis.Analysis` with an actual artifact reproduction.
It compares the complete v5 build basis and encoded native input before starting
the child compiler, then checks the actual reproduction and unchanged claim.
Both identities are required: unused build options can change the complete basis
without changing native input or executable bytes.

Use the same actual graph, options, file/target observations and proof capability
when constructing the Analysis. The selected toolchain and Analysis host retain
their existing assumptions. The [analysis client](../../../compiler/examples/analysis_build_basis/README.md)
and [reproduction API](../reproduction_trial/README.md) describe those inputs.
An absent, different, unknown or nonreproducible identity remains unverified.

The returned `ArtifactAnalysis` retains its analysis and reproduction evidence
in private state. `artifact_analysis_snapshot` returns detached editable source
metadata; `artifact_analysis_calls` runs the existing bounded source call query.
IDs belong to that retained analysis. Changing returned metadata does not change
it. Later source-file edits do not replace the retained snapshot: this allows
inspection of an older deployed build. Filesystem freshness is a separate query.

Each public query validates the original receipt and actual artifact identity.
Receipt edits, changed canonical artifact identity or changed artifact bytes
reject. Snapshot and call queries repeat that validation after computing their
answer. These are local observations, not signed evidence or an atomic lock over
files after the function returns.

Source call closure overapproximates checked branches. Arbitrary source roots
need not be emitted in the native artifact. Opaque cuts and runtime operations
retain their existing independent obligations. This association proves no
purity, application writer behavior, storage mapping, hotfix compatibility or
complete migration/recovery workflow, and grants no deployment permission.

The regression fixture is run against a disposable subject directory. It exercises
real O0/O2 reproduction under moving GC, a different unused-option basis with
identical native/artifact bytes, changed native metadata, legacy receipts,
private-state access rejection, detached-query mutation, retained source after an
edit, invalid root/cut IDs, receipt and artifact tampering, and restoration.
Only the fixture deliberately edits its disposable inputs; the library query APIs
read them and artifact reproduction writes the caller's new output directory.
