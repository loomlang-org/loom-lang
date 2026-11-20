# Artifact reproduction

`deployment.reproduce_artifact` explicitly builds a candidate with the compiler
and native images selected by the caller. Their expected SHA-256 hashes are
checked before and after the build. The request supplies the source, standard
library, build options and child process options, including its environment.
Use the toolchain and inputs that produced the claimed build.

The claimed receipt must have a v5 build basis. A new output directory retains
the rebuilt artifact and receipt. Success requires the actual artifact bytes,
checked input, complete build basis and backend identity to match. Receipt hashes
may differ because the output paths differ. A failed or nonreproducible build
remains unverified. The original receipt is read again after the build to detect
changes to the claim. Existing output directories are preserved and rejected.

This closes a concrete local-evidence gap: a receipt reader can validate actual
artifact A bytes accompanied by program B's otherwise well-formed metadata.
Rebuilding B and comparing the actual artifact rejects that mixed claim.
Reproduction relies on the caller's selected toolchain; it does not issue a
signature or prove application writer behavior, storage mappings, hotfix
compatibility or complete migration handling.

This directory contains the executable regression fixture used by
`compiler/tests/artifact_reproduction.rs`. It passes literal build-option values
and explicit child process settings, and never runs the candidate application.
The tests exercise O0/O2 inspectors under GC, genuine reproduction, mixed
artifacts, changed options with identical artifact bytes, wrong tool hashes,
legacy receipts, compiler failure, artifact tampering, mid-build receipt changes
and existing-directory preservation. The general API accepts a list of build options; the fixture uses
one option named `mode` to make input identity changes observable.
