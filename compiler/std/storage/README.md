# Storage rows

Source-only storage descriptors and typed row codecs; no database driver or
deployment policy is built into this package. IDs are application-supplied stable
identities, independent of physical SQL names.

## Record codecs

`record_codec[T](table[, aliases])` returns `Result[PreparedRowCodec[T], CodecError]`.
For a visible record, ordinary compile-time field iteration generates the field
encoders and checked assembly. Visible refinements also retain their complete
constructor check after field decoding. Records used from another package must
be `pub`; private types remain opaque. Normal resource-construction restrictions
still apply.

By default, a field name selects the column with that stable ID. Optional
`List[(Text, Text)]` aliases specify `(field_name, column_id)`. Preparation requires
a bijection covering all fields and columns with matching physical kinds and
nullability. Unknown or duplicate aliases, missing/duplicate column selections,
unsupported field codecs and shape mismatches return `InvalidSchema`.
Physical column order determines row order, not record declaration order.

The prepared codec privately snapshots the descriptor and resolved positions.
Later edits to the caller's descriptor or aliases do not change it. Row width,
kind and nullability are still validated at encode/decode boundaries.
`codec_table(codec)` returns an independent metadata copy for drivers to select
the same physical columns in the decoder's order.

`ColumnValue` defines each field's representation:

- `column_kind()` and `column_nullable()` describe its physical cell.
- `to_column(self Self)` encodes a value.
- `from_column(value Value)` returns a checked `Result[Self, CodecError]`.

Implementations are supplied for Int, Float, Text, Bytes, Bool (integer 0/1), and
`Option[T ColumnValue]` (NULL for None). Some(NULL) is rejected because a single
cell cannot distinguish it from None. Bytes retain ordinary sharing, not a deep
snapshot. User-defined/refined types provide an explicit implementation;
decoders use their normal checked constructors. Test-only implementations do not
become discoverable from production generic code.

For a visible refinement over an unrefined `Base ColumnValue`, implementations
can delegate to `encode_refined_column[T, Base](value)` and
`decode_refined_column[T, Base](cell)`. Encoding uses the base codec; decoding
passes the base value through `T`'s ordinary checked constructor and maps a
failed constraint to `CodecError.Rejected`. An always-proved constructor still
returns `Result.Ok`, without an extra check. Opaque types and mismatched bases
reject; visibility is the library's lexical visibility. The conformance and its
physical kind/nullability remain explicit. Delegation does not prove that an
arbitrary base codec preserves values or that deployments are compatible.

For a refined `T`, `record_codec[T]` obtains the visible non-refined base identity
through `std.meta.refinement_base`, then derives its ordinary record mapping.
`T` must admit weakening to that base, and the field codecs and mapping must pass
the ordinary checks. An opaque or non-record base returns `InvalidSchema`;
names and similar fields do not substitute for the actual base identity.

Encoding delegates through the base record. Decoding first checks all fields,
then uses `T`'s ordinary constructor to check the complete record predicate,
including relationships between fields. A failed predicate returns `Rejected`;
field errors retain their original categories. Redundant refinements can use a
statically proved constructor, and the same derivation works in CTFE. Constructor
evaluation and continuing alias constraints still apply. No implicit isolation
or new protection for shared Bytes is added.

See the [runnable example](../../examples/storage/README.md), including stable-ID
mapping, a constrained amount decoder and a whole-record relationship between
the ID and amount. No per-record handwritten encoder or runtime type registry
is required.

## Custom mappings

`RowCodec[T]` remains available for explicit flattening and application-specific
policies. `encode_row` / `decode_row` validate caller-owned mutable descriptors;
`prepare_codec` freezes a descriptor for repeated `encode_prepared_row` /
`decode_prepared_row` calls. Callback state and Bytes may still be shared.
`validate_table` and `validate_row` can validate descriptors and physical rows
without a logical codec.

A valid descriptor or successful round trip does not prove an upgrade compatible.
Deployed-state comparisons still require explicit migration, failure and recovery
policies.

## Source-derived SQLite checks

`sqlite_record_check[T](table[, aliases])` returns a SQL `CHECK` clause as
`Result[Text, SqliteCheckError]`. It uses the same visible refined-record mapping
as `record_codec[T]`, then translates the direct source predicate. Stable IDs and
aliases select quoted physical columns, including reordered fields. Preparation
freezes the mapping and rejects names containing NUL or colliding under SQLite's
ASCII-insensitive identifier comparison.

The supported expressions use exact built-in `Int` and `Bool` fields, integer and
Boolean literals, integer comparisons, same-type equality/inequality, `!`, `&&`
and `||`. Signed literals include both Int limits. Emitted comparisons retain
explicit parentheses, and referenced cells must have integer storage class;
Bool cells must also be 0 or 1. The final `IS 1` rejects SQL NULL rather than
allowing an unknown CHECK result. No partial arithmetic, helper invocation,
custom/refined field encoding or unresolved syntax is translated. Unsupported
types/predicates return an error, without executing a predicate to sample it.
The helper also works in ordinary CTFE.

See SQLite's [CHECK semantics](https://www.sqlite.org/lang_createtable.html#check_constraints)
and [storage classes](https://www.sqlite.org/datatype3.html#storage_classes_and_datatypes).

The caller installs the clause in its own schema; the helper neither executes
SQL nor inspects an existing database. SQLite affinity and explicit disabling of
constraints remain database policy. This clause enforces the modeled predicate
on referenced scalar cells, not every field's decodability, arbitrary application
writers, artifact identity or deployment compatibility. Other fields retain
their ordinary codec and database validation. See the
[real SQLite trial](../../../tools/deployment/sqlite_rows_trial/README.md) for
raw INSERT/UPDATE rejection alongside typed decoding and insertion.

## Canonical SQLite record codecs

`sqlite_record_codec[T](table[, aliases])` prepares a `SqliteRecordCodec[T]`.
`sqlite_codec_rows(codec)` supplies its frozen `PreparedRowCodec[T]` to typed row
readers and writers; `sqlite_codec_check(codec)` supplies the matching SQL
`CHECK` clause. Both come from the same visible field types and stable-ID mapping.

This factory selects a canonical representation: non-null `Int` cells for `Int`,
integer 0/1 for `Bool`, and those same representations for visible direct
refinements over either built-in. It uses checked constructors when decoding
refinements and does not call application `ColumnValue` implementations. Choose
`record_codec` when application-defined field conversion is required.

Every field contributes its storage-class and value constraints, including
fields unused by a whole-record predicate. Direct scalar field predicates and
the optional whole-record predicate use the expression fragment described above.
Plain visible records are supported too. Unsupported field types or predicates,
inaccessible definitions, nullable mappings and ambiguous names return an error;
the factory does not silently omit a field's requirements. In particular, it
does not certify arbitrary custom codecs or nullable/text/blob/float fields.

The caller installs the clause in its own database. With SQLite constraints
enabled, the clause enforces the admitted canonical row domain on stored cells;
affinity may convert incoming values before they are checked. This API does not
inspect an existing schema, prevent constraints from being disabled, bind an
application artifact, authorize migration, or prove deployment compatibility.
The [SQLite trial](../../../tools/deployment/sqlite_rows_trial/README.md) compares
source constructors with real raw writes and typed reads.
