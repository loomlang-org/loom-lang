use std::{fs, process::Command};

mod common;
use common::success;

#[test]
fn contracted_writes_keep_effects_eager_arguments_and_fault_cleanup() {
    let package = tempfile::tempdir().unwrap();
    let path = package.path().join("main.loom");
    let source = r#"import std.io.write_text
import std.io.write_bytes
import std.bytes.new
import std.bytes.push
import std.bytes.length
import std.process.arguments
import std.list.length

fn normal(value Text) Int
ensures result == 0 {
    defer {
        discard write_text(value)
    }
    0
}
fn binary(value Bytes) Int
ensures result == 0 && std.bytes.length(value) == old(std.bytes.length(value)) {
    discard write_bytes(value)
    0
}
fn visible() Bool
ensures result {
    discard write_text("visible\n")
    true
}
fn checked() Int
ensures result == 0 {
    assert visible()
    0
}
fn eager() Int
ensures result == 7 {
    var value = 0
    discard write_text({
        value = 7
        "eager\n"
    })
    value
}
fn fail(divisor Int) Int
ensures result == 0 {
    defer {
        discard write_text("cleanup\n")
    }
    discard (1 / divisor)
    0
}
fn main() {
    assert normal("normal\n") == 0
    let bytes = new()
    push(bytes, 0)
    push(bytes, 255)
    assert binary(bytes) == 0
    assert checked() == 0
    assert eager() == 7
    if length(arguments()) > 1 {
        discard fail(0)
    }
}
test fn inline_contract() {
    assert normal("") == 0
}
"#;
    fs::write(&path, source).unwrap();
    success(&common::loom(&["check", package.path().to_str().unwrap()]));
    success(
        &common::command(&["test", package.path().to_str().unwrap()])
            .env("LOOM_OPT_LEVEL", "0")
            .output()
            .unwrap(),
    );
    let artifact = common::executable(package.path(), "io-contracts");
    let expected = [b"normal\n".as_slice(), &[0, 255], b"visible\neager\n"].concat();
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
        success(&output);
        assert_eq!(output.stdout, expected);
        let failed = Command::new(&artifact)
            .arg("fail")
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        assert!(!failed.status.success());
        assert_eq!(failed.stdout, [expected.as_slice(), b"cleanup\n"].concat());
        assert!(String::from_utf8_lossy(&failed.stderr).contains("division by zero"));
    }
    let output = common::loom(&["run", package.path().to_str().unwrap()]);
    success(&output);
    assert_eq!(output.stdout, expected);
    // A worker build cannot frame arbitrary shared Bytes across blocking I/O.
    fs::write(
        &path,
        format!(
            "import std.task.worker.run\n{}",
            source.replace(
                "fn main() {",
                "async fn main() {\n    discard run(fn() Int {\n        7\n    }).await"
            )
        ),
    )
    .unwrap();
    let shared = common::loom(&["check", package.path().to_str().unwrap()]);
    assert!(!shared.status.success());
    assert!(String::from_utf8_lossy(&shared.stderr).contains("interference-safe"));
}
