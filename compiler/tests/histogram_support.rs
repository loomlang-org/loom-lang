use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn histogram_support_preserves_premises_ranges_versions_and_columns() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let program = include_str!("../examples/histogram_contracts/main.loom");
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
    for changed in [
        program.replace("requires nonnegative(values) && ", "requires "),
        program.replace("index < length(values)", "index < length(values) - 1"),
        program.replace("values[index] < 0", "values[index] < -index"),
        program.replace("prefix_occurrences(values, values[index], end) > 0", "prefix_occurrences(values, values[index], end) > 1"),
        program.replace("prefix_occurrences(values, values[index], end) > 0", "prefix_occurrences(values, values[index], index) > 0"),
        program.replace("    prefix_occurrences(values, values[index], end) > 0", "    let saved = values[index]\n    values[index] = saved + 1\n    prefix_occurrences(values, saved, end) > 0"),
        program
            .replace("fn positive_member(values List[Int], needle Int) Bool\nrequires nonnegative(values)", "fn positive_member(values List[Int], other List[Int], needle Int) Bool\nrequires nonnegative(other)")
            .replace("positive_member([0, 3, 9], 3)", "positive_member([0, 3, 9], [0], 3)")
            .replace("positive_member([0], 0)", "positive_member([0], [0], 0)"),
    ] {
        assert_ne!(changed, program, "test edit did not apply");
        fs::write(&source, changed).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsupported membership was proved");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("proof") || error.contains("proved"), "{error}");
    }
    fs::write(&source, program).unwrap();
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "histogram-support");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--output")
                .arg(&artifact)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
}
