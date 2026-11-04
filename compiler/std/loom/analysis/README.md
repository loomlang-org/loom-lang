# Checked call closure

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
