# Checked analysis

## Native compiler input

`native_input(analysis)` returns the encoded native backend input prepared from
the checked program retained by an `Analysis`. It uses the same assertion
optimization, executable-root selection, lowering and encoding as the compiler's
ordinary build path. Preparation works on a deep copy, including async dispatch
and type tables, and leaves the retained analysis unchanged. It does not invoke
application functions.

The SHA-256 of these bytes can be compared with `checked-sha256` in a trusted
local build receipt. The checked AST matters: editing its body without changing
`SourceFile.text` can change native input. Matching source-file hashes alone
cannot establish this link.

Analysis must use the build's test mode, observed inputs, target options and
proof backend. This query includes all executable roots retained by the analysis
and applies no test-name filter. Optional proof outcomes can affect preparation;
different input must remain a mismatch. The encoded format is an internal
compiler/backend boundary, not a stable artifact interchange format.

An equal hash establishes equality of prepared backend input, subject to the
ordinary SHA-256 assumption and trust in that receipt. Artifact bytes, receipt
provenance, module/configuration/input freshness and storage/writer compatibility
still need their own checks. See the
[native input example](../../../examples/analysis_native_input/README.md).

## Checked call closure

`call_closure(analysis, root, boundaries)` inspects the checked source IR retained
by an `Analysis`. `root` and each boundary are function indices in that same
analysis. `snapshot(analysis).functions` maps these indices to source symbols;
`function_at` exposes their detached bodies and concrete type arguments.
Different generic instances keep different indices.

The result is `Option[CallClosure]`. Invalid indices return `None`. A successful
query returns detached metadata:

- `functions`: function bodies visited, including the root.
- `calls`: direct call sites, their callers, targets, spans and boundary flags.
- `operations`: declared runtime-function names and typed intrinsic labels.
- `issues`: execution paths that inspection could not resolve or finish.

The root is always inspected. A direct call into a listed boundary records an
opaque call and stops at that callee. Its arguments and capture construction are
still inspected in the caller. Boundaries do not assert that those functions are
safe, pure, authorized, or associated with a particular database. A function's
identity also does not freeze the values in its live capture environment.

Inspection includes entry conditions and cleanup, visits all branches, and
handles recursive direct calls without assuming termination. It may include
operations from branches that will not run, or repeated cleanup representations.
An unused function value does not invoke its body. Neither the query nor the
returned metadata invokes application functions.

Erased calls, dynamic dispatch, implicit resource traversals, awaited Tasks,
unresolved source selections and inspection limits leave explicit issues.
Callers must retain those gaps when interpreting the result. Runtime names and
function indices belong to this checked program; they are not stable external
identities or a complete description of machine-level behavior.

Changing the returned Lists cannot change the retained analysis. Editing source
requires a new analysis and an ordinary freshness check. Relating a report to a
built artifact additionally needs its actual source, dependency, configuration
and tracked-input basis. This query supplies call metadata, not argument-domain,
result, storage-mapping, compatibility or deployment evidence.
