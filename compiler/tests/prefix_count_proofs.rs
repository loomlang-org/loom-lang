use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn prefix_counts_recheck_extents_old_versions_and_scan_edits() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let counts = include_str!("../examples/smt_contracts/count_scopes.loom")
        .split("fn bounded_counts")
        .next()
        .unwrap();
    let prefixes = include_str!("../examples/smt_contracts/count_prefixes.loom");
    let program = format!("{counts}\n{prefixes}\nfn main() {{\n    count_prefix_cases()\n}}\n");
    let cache = package.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            package.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, &program).unwrap();
    success(&check());
    success(&check());
    for changed in [
        program.replace("count = count + 1", "count = count + 2"),
        program.replace("var index = 0", "var index = 1"),
        program.replace("index = index + 1", "index = index + 2"),
        program.replace("if values[end] == needle", "if values[end] != needle"),
        program.replace("values[end] = needle", "values[0] = needle"),
        program.replace("push(values, needle)", "push(values, needle + 1)"),
        format!(
            "{counts}\nfn wrong(values List[Int], needle Int, end Int) Bool\nrequires end >= 0 && end <= length(values)\nensures prefix_occurrences(values, needle, end + 1) >= 0 {{\n    true\n}}\n"
        ),
    ] {
        assert_ne!(changed, program, "test edit did not apply");
        fs::write(&source, &changed).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound prefix counts: {changed}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proved") || error.contains("proof"),
            "{error}"
        );
    }
    fs::write(&source, &program).unwrap();
    success(&check());
    for mode in ["test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "prefix-counts");
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

#[test]
fn merge_proves_order_and_multiset_union_without_retaining_loop_scans() {
    let package = tempfile::tempdir().unwrap();
    let source = package.path().join("main.loom");
    let program = include_str!("../examples/merge_contracts/main.loom");
    fs::write(&source, &program).unwrap();
    for mode in ["check", "test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "merge-counts");
    let ir = package.path().join("merge-counts.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--output")
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
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
        assert_eq!(invariant_lines, 8);
        success(
            &Command::new(&artifact)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
    for changed in [
        program.replace("push(output, left[i])", "push(output, 0)"),
        program.replace("i = i + 1", "i = i + 2"),
    ] {
        assert_ne!(changed, program, "test edit did not apply");
        fs::write(&source, changed).unwrap();
        let output = common::loom(&["check", package.path().to_str().unwrap()]);
        assert!(!output.status.success(), "incorrect merge was accepted");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proved") || error.contains("proof"),
            "{error}"
        );
    }
}
