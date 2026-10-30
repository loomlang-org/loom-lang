use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn finite_order_composes_distant_reads_and_erases_merge_invariant_scans() {
    let fixture = "compiler/tests/fixtures/order_chains";
    let program = include_str!("fixtures/order_chains/main.loom");
    for mode in ["check", "test", "run"] {
        success(&common::loom(&[mode, fixture]));
    }
    let directory = tempfile::tempdir().unwrap();
    let artifact = common::executable(directory.path(), "order-chains");
    let ir = directory.path().join("order-chains.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", fixture])
                .arg("--output")
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        // Check the source assertion's location: optimized function names alone
        // cannot distinguish elision from a scan inlined into the loop.
        let mut invariant_lines = 0;
        for (line, text) in program.lines().enumerate() {
            if text.starts_with("        assert ") {
                invariant_lines += 1;
                assert!(
                    !lowered.contains(&format!(":{}:9: assertion failed", line + 1)),
                    "retained invariant at line {} (O{level})",
                    line + 1
                );
            }
        }
        assert_eq!(invariant_lines, 5);
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
}

#[test]
fn finite_order_rechecks_premises_versions_columns_and_domains() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("main.loom");
    let cache = directory.path().join("cache");
    let program = include_str!("fixtures/order_chains/main.loom");
    let scalar = program.split("record Item").next().unwrap();
    let check = || {
        common::loom(&[
            "check",
            directory.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, scalar).unwrap();
    success(&check());
    success(&check());
    let invalid = [
        scalar.replace("ordered(values) && ", ""),
        scalar.replace("requires ordered(values)", "requires !ordered(values)"),
        scalar
            .replace(
                "values[index - 1] > values[index]",
                "values[index - 1] == values[index]",
            )
            .replace(
                "values[first] <= values[second]",
                "values[first] != values[second]",
            ),
        scalar.replace(
            "    values[first] <= values[second]",
            "    values[first] = values[second] + 1\n    values[first] <= values[second]",
        ),
        scalar
            .replace("index < length(values)", "index < 2")
            .replace(
                "requires ordered(values)",
                "requires length(values) == 3 && first == 0 && second == 2 && ordered(values)",
            ),
        scalar
            .replace("index < length(values)", "index < length(values) / 2 + 1")
            .replace(
                "values[index - 1] > values[index]",
                "values[2 * index - 2] > values[2 * index - 1]",
            )
            .replace(
                "requires ordered(values)",
                "requires length(values) == 4 && first == 0 && second == 3 && ordered(values)",
            ),
        scalar.replace("values List[Int]", "values List[Float]"),
        // A faulting body has no normal-return promise. Instead make the body
        // return normally and require evaluation of the out-of-range contract.
        scalar
            .replace("second < length(values)", "second <= length(values)")
            .replace(
                "ensures result {\n    values[first] <= values[second]",
                "ensures values[first] <= values[second] {\n    true",
            ),
        program.replace(
            "values[first].key <= values[second].key",
            "values[first].payload <= values[second].payload",
        ),
    ];
    for changed in invalid {
        assert_ne!(changed, scalar, "test edit did not apply");
        fs::write(&source, &changed).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound finite order: {changed}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proved") || error.contains("proof"),
            "unexpected rejection: {error}"
        );
    }
    fs::write(&source, scalar).unwrap();
    success(&check());
    for (guard, comparison) in [("<", ">="), (">=", "<"), ("!=", "==")] {
        let changed = scalar
            .replace(
                "values[index - 1] > values[index]",
                &format!("values[index - 1] {guard} values[index]"),
            )
            .replace(
                "values[first] <= values[second]",
                &format!("values[first] {comparison} values[second]"),
            );
        fs::write(&source, changed).unwrap();
        success(&check());
    }
}
