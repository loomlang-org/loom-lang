use std::{fs, process::Command, thread, time::Duration};

mod common;
use common::success;

#[test]
fn proven_assertions_leave_native_code_without_removing_effects() {
    let package = tempfile::tempdir().unwrap();
    let source = r#"
fn yes() Bool {
    true
}
fn ignored(value Bool) Bool {
    true
}
fn proven(value Int) Int
requires value >= 0 && value < 100
ensures result == value + 1 {
    assert value >= 0
    assert yes()
    value + 1
}
fn update(state List[Int]) Bool
ensures result {
    state[0] = 7
    true
}
fn effectful(state List[Int]) Int
ensures result == 0 {
    assert update(state)
    0
}
fn eager() Int
ensures result == 7 {
    var changed = 0
    assert ignored({
        changed = 7
        true
    })
    changed
}
fn exercise() {
    assert proven(7) == 8
    let state = [0]
    assert effectful(state) == 0
    assert state[0] == 7
    assert eager() == 7
}
fn main() {
    exercise()
}
test fn evaluation_order_and_effects_are_preserved() {
    exercise()
}
"#;
    fs::write(package.path().join("main.loom"), source).unwrap();
    for mode in ["check", "test", "run"] {
        success(&common::loom(&[mode, package.path().to_str().unwrap()]));
    }
    let artifact = common::executable(package.path(), "assertions");
    let ir = package.path().join("assertions.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .args(["--output"])
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        for assertion in ["assert value >= 0", "assert yes()"] {
            let line = source
                .lines()
                .position(|line| line.trim() == assertion)
                .unwrap()
                + 1;
            assert!(
                !lowered.contains(&format!(":{line}:5: assertion failed")),
                "proved assertion remained at O{level}: {assertion}"
            );
        }
        success(&common::run_tasks(&artifact));
    }
}

#[test]
fn assertion_elision_retains_partial_and_diverging_operands() {
    let package = tempfile::tempdir().unwrap();
    let file = package.path().join("main.loom");
    let artifact = common::executable(package.path(), "partial-assertion");
    for (helper, assertion, error) in [
        (
            "fn condition(value Int) Bool { true }",
            "condition(1 / value)",
            "division by zero",
        ),
        (
            r#"fn condition(value Int) Bool {
    let ignored = 1 / value
    true
}"#,
            "condition(value)",
            "division by zero",
        ),
        (
            r#"fn condition(value Int) Bool
ensures result {
    assert value > 0
    true
}"#,
            "condition(value)",
            "assertion failed",
        ),
    ] {
        fs::write(
            &file,
            format!(
                r#"import std.io.write_text
{helper}
fn execute(value Int) Int
ensures result == 0 {{
    assert {assertion}
    0
}}
fn main() {{
    defer {{
        discard write_text("cleaned")
    }}
    discard execute(0)
}}
"#
            ),
        )
        .unwrap();
        for level in ["0", "2"] {
            success(
                &common::command(&["build", package.path().to_str().unwrap()])
                    .arg("--output")
                    .arg(&artifact)
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            let output = common::run_tasks(&artifact);
            assert!(!output.status.success(), "operand fault was removed");
            assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "cleaned");
            let diagnostic = String::from_utf8_lossy(&output.stderr);
            assert!(diagnostic.contains(error), "{diagnostic}");
        }
    }
    fs::write(
        &file,
        r#"
fn condition() Bool
ensures result {
    while true {
    }
    true
}
fn execute() Int
ensures result == 0 {
    assert condition()
    0
}
fn main() {
    discard execute()
}
"#,
    )
    .unwrap();
    success(&common::loom(&[
        "build",
        package.path().to_str().unwrap(),
        "--output",
        artifact.to_str().unwrap(),
    ]));
    let mut child = Command::new(&artifact).spawn().unwrap();
    thread::sleep(Duration::from_millis(200));
    let exited = child.try_wait().unwrap();
    if exited.is_none() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    assert!(exited.is_none(), "diverging assertion operand was removed");
}

#[test]
fn reused_assertions_reprove_changed_helper_bodies() {
    let package = tempfile::tempdir().unwrap();
    let main = package.path().join("main.loom");
    let helper = package.path().join("condition.loom");
    fs::write(
        &main,
        r#"fn main()
ensures true {
    assert condition()
}

"#,
    )
    .unwrap();
    let cache = package.path().join("cache");
    let artifact = common::executable(package.path(), "cached-assertion");
    let ir = package.path().join("cached-assertion.ll");
    let build = || {
        common::command(&["build", package.path().to_str().unwrap()])
            .arg("--output")
            .arg(&artifact)
            .arg("--frontend-cache")
            .arg(&cache)
            .arg("--emit-ir")
            .arg(&ir)
            .env("LOOM_OPT_LEVEL", "0")
            .env("LOOM_NATIVE_TIMINGS", "1")
            .output()
            .unwrap()
    };
    for (index, value) in ["true", "false", "true"].into_iter().enumerate() {
        fs::write(
            &helper,
            format!("fn condition() Bool\nensures true {{\n    {value}\n}}\n"),
        )
        .unwrap();
        success(&build());
        let output = common::run_tasks(&artifact);
        if value == "true" {
            success(&output);
            assert!(
                !fs::read_to_string(&ir)
                    .unwrap()
                    .contains("assertion failed")
            );
        } else {
            assert!(!output.status.success(), "cached proof hid changed helper");
            assert!(String::from_utf8_lossy(&output.stderr).contains("assertion failed"));
        }
        if index == 0 {
            // Force a whole-closure miss while reusing both checked bodies.
            // New declarations also move check-local function identities.
            fs::write(
                package.path().join("aaa.loom"),
                "fn unrelated() Int {\n    9\n}\n",
            )
            .unwrap();
            let reused = build();
            success(&reused);
            let trace = String::from_utf8_lossy(&reused.stderr);
            assert!(trace.contains("frontend miss"), "{trace}");
            assert!(trace.contains("bodies reused 2"), "{trace}");
            assert!(
                !fs::read_to_string(&ir)
                    .unwrap()
                    .contains("assertion failed")
            );
            success(&common::run_tasks(&artifact));
        }
    }
}

#[test]
fn adding_workers_rechecks_sequential_assertion_evidence() {
    let package = tempfile::tempdir().unwrap();
    let source = r#"import std.list.length

pub fn checked(values List[Int]) Int
requires length(values) == 1
ensures result == 0 {
    assert length(values) == 1
    0
}
async fn main() {
    trigger().await
    discard checked([0])
}
"#;
    fs::write(package.path().join("main.loom"), source).unwrap();
    let trigger = package.path().join("trigger.loom");
    let cache = package.path().join("cache");
    let artifact = common::executable(package.path(), "worker-assertions");
    let ir = package.path().join("worker-assertions.ll");
    let assertion = source
        .lines()
        .position(|line| line.trim() == "assert length(values) == 1")
        .unwrap()
        + 1;
    let diagnostic = format!(":{assertion}:5: assertion failed");
    for (body, retained) in [
        ("async fn trigger() {\n}\n", false),
        (
            r#"import std.task.worker.run

async fn trigger() {
    assert run(fn() Int { 7 }).await == 7
}
"#,
            true,
        ),
    ] {
        fs::write(&trigger, body).unwrap();
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--output")
                .arg(&artifact)
                .arg("--emit-ir")
                .arg(&ir)
                .arg("--frontend-cache")
                .arg(&cache)
                .env("LOOM_OPT_LEVEL", "0")
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        assert_eq!(lowered.contains(&diagnostic), retained, "{diagnostic}");
        success(&common::run_tasks(&artifact));
    }
}

#[test]
fn cleanup_assertions_keep_fault_path_state() {
    let package = tempfile::tempdir().unwrap();
    fs::write(
        package.path().join("main.loom"),
        r#"import std.io.write_text

fn execute(state List[Int], divisor Int) Int
ensures result == 0 {
    defer {
        assert divisor != 0
        state[0] = 7
    }
    discard 1 / divisor
    0
}
fn main() {
    let state = [0]
    defer {
        if state[0] == 0 {
            discard write_text("cleaned")
        } else {
            discard write_text("wrong")
        }
    }
    discard execute(state, 0)
}
"#,
    )
    .unwrap();
    let artifact = common::executable(package.path(), "fault-cleanup");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package.path().to_str().unwrap()])
                .arg("--output")
                .arg(&artifact)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let output = common::run_tasks(&artifact);
        assert!(!output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "cleaned");
        assert!(String::from_utf8_lossy(&output.stderr).contains("division by zero"));
    }
}
