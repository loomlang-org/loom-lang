use std::{fs, process::Command};

mod common;

fn trial_success(phase: &str, output: &std::process::Output) {
    assert!(
        output.status.success(),
        "{phase}: stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn permutation_trial(
    program: &str,
    equivalent: &[String],
    invalid: &[String],
    check_elision: bool,
    proof_only_lines: Option<usize>,
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
    trial_success("cold check", &check());
    trial_success("warm check", &check());
    for (index, changed) in equivalent.iter().enumerate() {
        assert_ne!(changed, program);
        fs::write(&source, changed).unwrap();
        trial_success(&format!("equivalent variant {index}"), &check());
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
    trial_success("restored check", &check());
    if let Some(last_proof_line) = proof_only_lines {
        let output = common::loom(&[
            "emit-checked",
            package.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ]);
        trial_success("native proof-scan elision", &output);
        let native =
            loom_native::native_input::decode(std::str::from_utf8(&output.stdout).unwrap())
                .unwrap();
        let selected = fs::canonicalize(&source).unwrap();
        let mut selected_sites = 0;
        for position in native.source_locations.sites.values() {
            if fs::canonicalize(&native.source_locations.paths[position.file])
                .ok()
                .as_deref()
                == Some(selected.as_path())
            {
                selected_sites += 1;
                assert!(
                    position.line > last_proof_line,
                    "proof-only predicate reached native code at line {}",
                    position.line
                );
            }
        }
        assert!(selected_sites > 0, "native source map omitted the trial");
    }
    for mode in ["test", "run"] {
        trial_success(
            mode,
            &common::loom(&[
                mode,
                package.path().to_str().unwrap(),
                "--frontend-cache",
                cache.to_str().unwrap(),
            ]),
        );
    }
    let artifact = common::executable(package.path(), "permutation");
    let ir = package.path().join("permutation.ll");
    for level in ["0", "2"] {
        trial_success(
            &format!("build O{level}"),
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
        trial_success(
            &format!("run O{level}"),
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
}

#[test]
fn source_library_integer_sort_composes_order_and_permutation() {
    let program = include_str!("../examples/std_sort_contracts/main.loom");
    let equivalent = [program.replace("left < right", "!(left >= right)")];
    let invalid = [
        program.replace("left < right", "left > right"),
        program.replace(
            "    sorted(values,",
            "    let changed = std.list.clone(values)\n    if length(changed) > 0 { changed[0] = 0 }\n    sorted(changed,",
        ),
    ];
    let proof_lines = program[..program.find("pub fn sorted_values").unwrap()]
        .lines()
        .count();
    permutation_trial(program, &equivalent, &invalid, true, Some(proof_lines));
}

#[test]
fn source_library_sort_cannot_assume_unknown_or_changing_comparison_order() {
    let program = include_str!("../examples/std_sort_contracts/main.loom");
    let prefix = program.split("pub fn sorted_values").next().unwrap();
    let claims = "ensures ordered(result)\nensures permutation(old(values), result)";
    let variants = [
        format!(
            "pub fn unknown(values List[Int], less fn(Int, Int) Bool) List[Int]\n{claims}\n{{ sorted(values, less) }}"
        ),
        format!(
            "pub fn changing(values List[Int]) List[Int]\n{claims}\n{{\n    let reverse = [false]\n    sorted(values, fn(left Int, right Int) Bool {{\n        reverse[0] = !reverse[0]\n        if reverse[0] {{ left > right }} else {{ left < right }}\n    }})\n}}"
        ),
    ];
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    for variant in variants {
        fs::write(&source, format!("{prefix}{variant}\n")).unwrap();
        let output = common::loom(&["check", package.path().to_str().unwrap()]);
        assert!(
            !output.status.success(),
            "unproved comparator order was accepted"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof") || error.contains("proved"),
            "{error}"
        );
        fs::write(
            &source,
            format!(
                "{prefix}{}\n",
                variant.replace("ensures ordered(result)\n", "")
            ),
        )
        .unwrap();
        trial_success(
            "permutation-only comparator control",
            &common::loom(&["check", package.path().to_str().unwrap()]),
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
    permutation_trial(program, &[], &invalid, true, None);
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
    permutation_trial(program, &[], &invalid, false, None);
}

#[test]
fn complete_record_counts_keep_field_pairings_and_normalize_equivalent_equalities() {
    let program = include_str!("../examples/value_sort_contracts/main.loom");
    let corrupted = program
        .replacen("    sorted(values, fn(a Row, b Row) Bool {", "    let output = sorted(values, fn(a Row, b Row) Bool {", 1)
        .replacen("            a.key.id < b.key.id\n        })\n}", "            a.key.id < b.key.id\n        })\n    if length(output) > 0 { output[0] = Row { amount = 99, ..output[0] } }\n    output\n}", 1);
    assert!(corrupted.contains("\n    output\n}"));
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
        program.replacen("a.key.id < b.key.id", "a.key.id > b.key.id", 1),
        program.replacen("a.key.id > b.key.id", "a.key.id < b.key.id", 1),
        corrupted,
        format!(
            r#"{program}
pub fn unknown_order(values List[Row], less fn(Row, Row) Bool) List[Row]
ensures ascending(result)
ensures permutation(old(values), result) {{
    sorted(values, less)
}}
"#
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
    let reversed_comparisons = program
        .replace("a.key.id < b.key.id", "b.key.id > a.key.id")
        .replace("a.key.id > b.key.id", "b.key.id < a.key.id");
    permutation_trial(
        program,
        &[equivalent, reversed_comparisons],
        &invalid,
        true,
        None,
    );
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
    permutation_trial(program, &[equivalent], &invalid, true, None);

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
    let source = package.path().join("main.loom");
    let without_claim = fs::read_to_string(&source).unwrap().replace(
        "ensures result == count_id(values, id) + count_label(values, label)",
        "",
    );
    fs::write(source, without_claim).unwrap();
    trial_success(
        "overlap control without the false claim",
        &common::loom(&["check", package.path().to_str().unwrap()]),
    );
}

#[test]
fn enum_row_permutation_keeps_selected_payloads_tags_and_field_pairings() {
    let program = include_str!("../examples/enum_sort_contracts/main.loom");
    let equivalent = program.replace(
        "first.key == second.key && same_signal(first.signal, second.signal)",
        "same_signal(second.signal, first.signal) && second.key == first.key",
    );
    permutation_trial(program, &[equivalent], &[], true, None);

    let prefix = program.split("fn ascending(").next().unwrap();
    let copy = "let changed = std.list.clone(values)\n    if length(changed) > 0 {\n        changed[0] = Row { signal = REPLACEMENT, ..changed[0] }\n    }\n    changed";
    let swap = "let changed = std.list.clone(values)\n    if length(changed) > 1 {\n        let first = changed[0]\n        let second = changed[1]\n        changed[0] = Row { signal = second.signal, ..first }\n        changed[1] = Row { signal = first.signal, ..second }\n    }\n    changed";
    let mut invalid = [
        "Signal.Empty()",
        "Signal.Number(99)",
        "Signal.Label(\"changed\")",
    ]
    .map(|replacement| {
        prefix.replace(
            "    sorted(values, less)",
            &copy.replace("REPLACEMENT", replacement),
        )
    })
    .to_vec();
    invalid.push(prefix.replace("    sorted(values, less)", swap));
    invalid.push(prefix.replace("value == other", "1 / value == 1 / other"));
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let cache = package.path().join("cache");
    for (index, changed) in invalid.iter().enumerate() {
        assert_ne!(changed, prefix);
        let changed = format!("import std.list.clone\n{changed}");
        fs::write(&source, &changed).unwrap();
        let check = || {
            common::loom(&[
                "check",
                package.path().to_str().unwrap(),
                "--frontend-cache",
                cache.to_str().unwrap(),
            ])
        };
        let output = check();
        assert!(
            !output.status.success(),
            "invalid enum permutation variant {index} was proved"
        );
        let without_claim = changed.replace("ensures permutation(old(values), result)", "");
        fs::write(&source, without_claim).unwrap();
        trial_success(
            &format!("enum variant {index} without false claim"),
            &check(),
        );
    }
}
