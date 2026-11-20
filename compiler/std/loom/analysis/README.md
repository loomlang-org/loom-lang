# Checked analysis

## Build basis

`build_basis(analysis)` returns the SHA-256 recorded as `basis-sha256` in a v5
build receipt. Compiler publication and analysis use the same source-written
encoding. It binds the selected modules, configuration, package identities and
resolved import edges, file ownership/flags, source bytes and ASTs, test mode,
and observed file, option and target inputs. It reads the retained private
snapshot and does not invoke application functions or reread the filesystem.

Resolved edges matter even if the module/source lists and native code are
unchanged: a local fork can change the meaning of a refinement in an unused
generic declaration. ASTs matter independently of `SourceFile.text`. This hash
is separate from `native_input` and the artifact digest. A matching native input
does not establish matching source contracts.

Receipts are produced by `build`, so a matching analysis uses production mode
and the build's exact observations. A legacy v3/v4 receipt has no complete basis
digest. Filesystem freshness, receipt provenance and actual artifact bytes still
need their own checks. Matching hashes do not establish storage/writer or
deployment compatibility.

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

`execution_roots(analysis)` returns the actual checked source entry (an
`Option[Int]`), test function IDs and their names, and export IDs. These are the
unfiltered roots supplied to the same preparation path as `native_input`, not
guesses from names or public visibility. Production and test analysis retain
their respective selections. All returned Lists are detached.

The IDs still address source functions in this analysis. Async lowering can add
private entry/test wrappers with different IDs; a source root ID is not a native
address or a promise that its original body survives optimization. Selection
does not mean the root was invoked, establish termination, or summarize all
machine-level effects. A call closure started at a selected source root retains
the limits below.

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
