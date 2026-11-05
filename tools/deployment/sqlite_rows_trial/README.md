# Typed SQLite rows

Use a fresh disposable database path and an explicit SQLite CLI executable:

```sh
trial_dir="$(mktemp -d)"
loom run tools/deployment/sqlite_rows_trial -- /usr/bin/sqlite3 "$trial_dir/rows.sqlite"
```

`deployment.read_sqlite_rows[T](tool, database, codec)` reads the physical columns
of a frozen `std.storage.PreparedRowCodec[T]` and returns
`Result[List[T], SqliteRowError]`. The trial derives a codec for an ordinary
record, including an explicit constrained-amount decoder, optional text, Bool,
Bytes and Float. It exercises reordered columns, stable-ID aliases, quoted
identifiers, empty/binary/multiline values, integer limits and a subnormal Float.
Invalid constraints, kinds, NULLs and UTF-8 are rejected at the typed boundary.

The trial also derives `record_codec[OrderedBounds]` for a
whole-record `lower <= upper` predicate. It reads valid rows through reordered
stable-ID aliases, rejects a stored row whose two valid integer fields violate
that relationship, then inserts and reads a checked boundary value. A source
postcondition uses the decoded record's complete constraint.

The source-check trial installs a `std.storage.sqlite_record_check[SourceBounds]`
clause derived from `!self.active || self.lower <= self.upper`, using the same
stable-ID mapping as its typed codec. The real database rejects invalid active
rows, NULLs, wrong storage classes, non-Boolean integers and an UPDATE that would
reactivate invalid bounds. Valid inactive rows, Int limits and a typed insertion
still decode to the required source invariant. The fixture uses columns without
affinity so wrong input classes cannot be coerced. This is enforcement of the
supported scalar predicate, not general schema or application-writer proof.
Fifty source-constructor versus real-database INSERT controls cover every admitted
operator, both field-selection forms and both Int limits. They are regression
examples, not a universal compatibility proof.

The canonical-record trial prepares `sqlite_record_codec[CanonicalRow]` once
and uses both its row codec and SQL clause. Its positive Amount field has no
`ColumnValue` implementation: the factory selects the canonical Int
representation and checks the source constructor when decoding. The complete
row also has an unreferenced Bool and an `id >= 0 && amount >= id` predicate.
Forty source-constructor versus raw INSERT controls, wrong classes, NULLs,
non-Boolean integers and invalid UPDATEs exercise the real database; accepted
rows and typed insertion decode through the same prepared codec. This closes
the unreferenced-field gap for the admitted canonical Int/Bool record domain,
without certifying arbitrary codecs or application/deployment compatibility.

The canonical nullable-record trial uses the same factory for binary fields and
one `Option` layer over canonical Int, Bool, Bytes or an admitted refinement.
It checks each field's storage class and nullability against the frozen mapping.
SQL NULL represents `None`; empty blobs remain `Some(empty)` for optional Bytes.
Twenty-four constructor versus raw INSERT controls, eleven wrong-class/domain
controls, typed insertion/readback and invalid UPDATEs exercise real SQLite.
Nested options, same-shaped user enums, untranslated optional/content predicates
and unsupported scalar types reject. Bytes retain their ordinary mutable alias
semantics; this factory does not freeze content or prove writer compatibility.

The trusted CLI runs one read-only SELECT, with literal arguments and quoted
identifiers; user startup commands are disabled. Cells use tagged hexadecimal
payloads, including SQLite CLI's
[`ieee754_to_blob`](https://sqlite.org/floatingpoint.html) binary64 representation.
The CLI must provide that function. No database/JSON support is added to the
language runtime. Errors contain only categories, zero-based row/column indices
and codec errors, not stored values, SQL or stderr.
Physical column names colliding under SQLite's ASCII-insensitive identifier
comparison reject as `Mapping`; stable IDs remain case-sensitive.

All rows are loaded into memory, without an ordering promise. Codec callbacks
retain their ordinary effects. Successful reads are not schema compatibility
evidence, proof of application writer behavior, or authorization to migrate;
general migration execution and recovery remain separate work.

## Batch insertion

`insert_sqlite_rows[T](tool, database, codec, values)` returns
`Result[Bool, SqliteRowError]`. It validates/encodes every row before invoking
SQLite, then executes one `INSERT OR ABORT` in one transaction against an
existing database. Empty input is a successful no-op without starting SQLite.
Codec failures report `Encode(row, error)`; NaN returns `Encoding` instead of
silently becoming SQL NULL. Strings and blobs use hexadecimal literals; Float
uses the source [binary64 codec](../../../compiler/std/encoding/binary64/README.md).
SQL fragments are joined once, without repeatedly copying a growing batch.

Encoding validates outgoing cells, not the stored result: SQLite affinity,
constraints and triggers retain their semantics. In particular, SQLite storage
may canonicalize zero signs. Codec callbacks keep their ordinary effects.
An execution failure returns `WriteOutcomeUnknown`, even when a particular SQL
error would roll back: the process may instead have committed and lost its
acknowledgement. Inspect authoritative state; the API never retries automatically.
Success acknowledges the SQL command, not a deployment or compatibility proof.
Database arguments are literal filesystem filenames, not SQLite URI options or
`:memory:` connections. Transport selects URI `mode=ro`, `rw` or `rwc` itself,
escaping filename delimiters; an insertion never creates a missing database.

The trial also tests real batch insertion, constraint rollback, pre-execution
codec/NaN rejection, and a real commit with lost acknowledgement. The latter
reuses the existing Node.js fault-injection fixture from the migration trial.
