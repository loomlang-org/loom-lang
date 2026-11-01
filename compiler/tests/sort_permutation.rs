use std::{fs, process::Command};

mod common;
use common::success;

fn permutation_trial(
    program: &str,
    equivalent: &[String],
    invalid: &[String],
    check_elision: bool,
) {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let cache = package.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            package.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, program).unwrap();
    success(&check());
    success(&check());
    for changed in equivalent {
        assert_ne!(changed, program);
        fs::write(&source, changed).unwrap();
        success(&check());
    }
    for (index, changed) in invalid.iter().enumerate() {
        assert_ne!(changed, program);
        fs::write(&source, format!("import std.list.clone\n{changed}")).unwrap();
        let output = check();
        assert!(
            !output.status.success(),
            "invalid permutation variant {index} was proved"
        );
    }
    fs::write(&source, program).unwrap();
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[
            mode,
            package.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ]));
    }
    let artifact = common::executable(package.path(), "permutation");
    let ir = package.path().join("permutation.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--frontend-cache")
                .arg(&cache)
                .arg("--output")
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        if check_elision {
            let lowered = fs::read_to_string(&ir).unwrap();
            for (line, text) in include_str!("../std/list/sorting.loom").lines().enumerate() {
                if text.trim_start().starts_with("assert ") {
                    let column = text.len() - text.trim_start().len() + 1;
                    assert!(
                        !lowered.contains(&format!(":{}:{column}: assertion failed", line + 1)),
                        "retained sort invariant at O{level}: {text}"
                    );
                }
            }
        }
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
}

#[test]
fn library_sort_preserves_entry_counts_through_unknown_effectful_comparisons() {
    let program = include_str!("../examples/sort_permutation/main.loom");
    let invalid = [
        "let changed = std.list.clone(values)\n    if length(changed) > 0 { changed[0] = 0 }\n    changed",
        "discard sorted(values, less)\n    values",
    ].map(|body| program.replace("    sorted(values, less)", body));
    permutation_trial(program, &[], &invalid, true);
}

#[test]
fn record_sort_tracks_nested_and_sibling_columns_without_mixing_them() {
    let program = include_str!("../examples/record_sort_contracts/main.loom");
    let altered_key = "let changed = std.list.clone(values)\n    if length(changed) > 0 {\n        let row = changed[0]\n        changed[0] = Row { key = Key { id = 0 }, ..row }\n    }\n    changed";
    let invalid = [
        program.replace("    sorted(values, less)", altered_key),
        program.replace(
            "    sorted(values, less)",
            "discard sorted(values, less)\n    values",
        ),
        program.replacen(
            "count(second, first[index].key.id)",
            "count_amount(second, first[index].key.id)",
            1,
        ),
        program.replacen("index < length(values)", "index < length(values) - 1", 1),
        program.replace(
            "push(ids, values[index].key.id)",
            "push(ids, values[index].amount)",
        ),
    ];
    permutation_trial(program, &[], &invalid, false);
}

#[test]
fn complete_record_counts_keep_field_pairings_and_normalize_equivalent_equalities() {
    let program = include_str!("../examples/value_sort_contracts/main.loom");
    let swap = "let changed = std.list.clone(values)\n    if length(changed) > 1 {\n        let first = changed[0]\n        let second = changed[1]\n        changed[0] = Row { amount = second.amount, ..first }\n        changed[1] = Row { amount = first.amount, ..second }\n    }\n    changed";
    let invalid = [
        program.replace("    sorted(values, less)", swap),
        program.replace(
            "    sorted(values, less)",
            "discard sorted(values, less)\n    values",
        ),
        program.replace(
            "same_row(values[index], needle)",
            "same_row(Row { label = values[0].label, ..values[index] }, needle)",
        ),
    ];
    let start = program.find("fn count(").unwrap();
    let end = start + program[start..].find("\n}\n").unwrap() + 3;
    let reordered = program[start..end].replace("fn count(", "fn count_reordered(")
        .replace("same_row(values[index], needle)",
            "needle.label == values[index].label && needle.active == values[index].active && needle.amount == values[index].amount && needle.key.id == values[index].key.id");
    let equivalent = format!("{program}\n{reordered}").replace(
        "count(second, first[index])",
        "count_reordered(second, first[index])",
    );
    permutation_trial(program, &[equivalent], &invalid, true);
}

#[test]
fn boolean_count_algebra_keeps_overlap_complements_and_entry_guards() {
    let program = include_str!("../examples/predicate_sort_contracts/main.loom");
    let equivalent = program.replace(
        "row.id == id || row.label == label",
        "!(row.id != id && row.label != label)",
    );
    let invalid = [
        program.replace(
            "    sorted(values, less)",
            "let changed = std.list.clone(values)\n    if length(changed) > 0 { changed[0] = Row { id = id, label = label } }\n    changed",
        ),
        program.replace(
            "selected(values[index], id, label)",
            "selected(values[index], id, label) && values[0].id == id",
        ),
    ];
    permutation_trial(program, &[equivalent], &invalid, true);

    let package = tempfile::tempdir().unwrap();
    let prefix = program.split("pub fn sorted_matching").next().unwrap();
    fs::write(
        package.path().join("main.loom"),
        format!(
            r#"{prefix}
fn count_id(values List[Row], id Int) Int {{
    var index = 0
    var found = 0
    while index < length(values) {{
        if values[index].id == id {{ found = found + 1 }}
        index = index + 1
    }}
    found
}}
fn count_label(values List[Row], label Text) Int {{
    var index = 0
    var found = 0
    while index < length(values) {{
        if values[index].label == label {{ found = found + 1 }}
        index = index + 1
    }}
    found
}}
pub fn wrong(values List[Row], id Int, label Text) Int
requires length(values) <= 1
ensures result == count_id(values, id) + count_label(values, label) {{
    count(values, id, label)
}}
"#
        ),
    )
    .unwrap();
    let output = common::loom(&["check", package.path().to_str().unwrap()]);
    assert!(
        !output.status.success(),
        "union counted overlapping matches twice"
    );
}
