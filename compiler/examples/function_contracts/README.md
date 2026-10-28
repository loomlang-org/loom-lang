# Immutable function contracts

From the repository root, with Z3 on `PATH`:

```sh
target/loom check compiler/examples/function_contracts
target/loom test compiler/examples/function_contracts
target/loom run compiler/examples/function_contracts
```

Ordinary source loops compute Int, Text, record, tuple, enum and Float values.
Contracts reuse the result of the same known pure function at the same immutable
input projection, even when the prover cannot expand the loop into a formula. Checked
construction and known callbacks over immutable record/enum List elements use
the same rules. No purity annotation or runtime wrapper is required.

Known closures retain the values in read-only capture slots, including nested
immutable records/enums and exact Float representations. The example composes
them through direct calls, List scans and suspension. Receiver evaluation keeps
its capture before later argument effects. Shared collections require versioned
observations rather than immutable handle identities.
A read from a newly constructed captured binding can use its initialized value,
with eager evaluation checks retained. Existing environments and shared descendants
continue to use ordinary storage snapshots and alias invalidation.

An inline context may also carry List/Bytes fields that the helper never
observes. Its key retains the immutable projection and omits those handles.
Copying or forwarding the context does not read the mutable contents. The
`inputs.loom` example mutates those siblings after a call, and preserves the
result relation through forwarding, captured contexts and suspension. Primitive
reads and writes, control conditions and entry predicates are checked across
the closed call graph; a mutable observation cannot be omitted. Every argument
still evaluates, including the ones absent from the key. This is a conservative
structural projection, not minimal dependency inference or mutable-graph induction.

`sequences.loom` also composes read-only helpers over current List/Bytes contents.
Their keys contain the logical outer content version and observed extent, never
the shared handle. Direct calls, forwarding and read-only capture slots use the
same rule; argument evaluation finishes before the invocation snapshot. Immutable
elements may be read, including records/enums. Writes and possible alias writes
invalidate the current relation, while `old(values)` retains its entry version
across writes and suspension without a runtime copy. Unprotected worker data and
reads through mutable child graphs cannot supply this whole-call snapshot.
Distinct versions are not assumed equal even when their lengths match.
`sum_inputs.loom` uses the same structural projection for enum tags and payloads,
including recursive List-backed branches. Unobserved children remain shared;
observed List/Bytes payloads retain their content versions. Inactive concrete
alternatives need no storage snapshot. Guards and entry predicates still count
as reads, and hypothetical alternatives establish no successful-call evidence.
`PositiveSequence` also consumes a loop-based predicate from a constrained input
type: its positive total needs no repeated `requires`. Unknown construction still
returns a checked `Result`, and each constrained store must preserve the predicate.

`graphs.loom` also composes read-only functions over tracked mutable roots and
their descendants. A logical heap version binds one coherent invocation;
recursive data, shared edges and cycles require no runtime copy. Every possible
write, unknown call, hot Task creation and suspension invalidates that version.
Closed read-only calls and loops preserve it. Root names imply no separation.
This coarse rule rejects current graph observations in shared-worker proofs;
it does not prove recursive algorithms, termination or arbitrary graph invariants.

`cell_captures.loom` composes closed read-only callbacks over mutable bindings
whose contents are immutable values, including records and Float representations.
Loops use the same sampled contents rule as direct reads. A setter changes later
observations while an earlier result retains its sampled value. Getter/setter
pairs still share one actual cell, and private captures can cross suspension.
Shared-worker reads require private or read-only storage. Mutable descendants
need graph snapshots; unknown or writing callbacks do not acquire result facts.
`cell_collections.loom` applies the same projection to captured Lists and recursive
records. Replacing a captured handle changes its later reads; writes through a
shared child invalidate content observations even when the binding is unchanged.
The runnable example also checks a shared self edge without a separation axiom.
Unobserved mutable siblings remain omitted, including inside a mutable captured
record; sampling a binding does not imply reading every descendant.

Successful evaluation is independent of the result: a hypothetical call cannot
prove its own safety, termination or output. A Float result can be NaN; numeric
equality cannot substitute signed zeros. Mutable graphs need their logical heap
snapshot; unknown callbacks supply no guessed behavior.
