use std::{fs, process::Command};

mod common;
use common::{loom, success};

#[test]
fn bounded_datagram_application_preserves_contracts_privacy_and_real_io() {
    let package = "compiler/examples/bounded_datagrams";
    success(&loom(&["check", package]));
    for (path, expected) in [
        (package.to_owned(), "1 tests passed"),
        (format!("{package}/protocol"), "2 tests passed"),
    ] {
        let output = loom(&["test", &path]);
        success(&output);
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
    }
    let temporary = tempfile::tempdir().unwrap();
    let executable = common::executable(temporary.path(), "bounded-datagrams");
    let tests = common::executable(temporary.path(), "bounded-tests");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package])
                .arg("--output")
                .arg(&executable)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        for input in ["checked message".to_owned(), "é".repeat(32)] {
            let output = common::run_task_command(
                Command::new(&executable)
                    .arg(&input)
                    .env("LOOM_GC_STRESS", "1"),
            );
            success(&output);
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                format!("{input}\n")
            );
        }
        let rejected = common::run_task_command(
            Command::new(&executable)
                .arg("é".repeat(33))
                .env("LOOM_GC_STRESS", "1"),
        );
        assert_eq!(rejected.status.code(), Some(1));
        assert!(rejected.stdout.is_empty());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("exceeds 64 UTF-8 bytes"));
        for scope in [package.to_owned(), format!("{package}/protocol")] {
            success(
                &common::command(&["test", &scope, "--no-run"])
                    .arg("--output")
                    .arg(&tests)
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            success(&common::run_tasks(&tests));
        }
    }

    let directory = common::root().join(package);
    let main = directory.join("main.loom");
    let source = fs::read_to_string(&main).unwrap();
    let offset = source.find("message(input)").unwrap().to_string();
    let query = common::command(&[
        "editor-query",
        directory.to_str().unwrap(),
        "--at",
        main.to_str().unwrap(),
        &offset,
    ])
    .arg("--std")
    .arg(common::root().join("compiler/std"))
    .output()
    .unwrap();
    success(&query);
    let report = String::from_utf8(query.stdout).unwrap();
    assert!(report.contains("\"diagnostics\":[]"), "{report}");
    assert!(
        report.contains("fn(Text) std.result.Result[bounded_datagrams.protocol.Message"),
        "{report}"
    );
    assert!(
        report
            .replace("\\\\", "/")
            .contains("protocol/message.loom"),
        "{report}"
    );

    let fixture = temporary.path().join("fixture");
    fs::create_dir_all(fixture.join("protocol")).unwrap();
    for name in [
        "loom.toml",
        "main.loom",
        "transport.loom",
        "protocol/message.loom",
    ] {
        fs::copy(directory.join(name), fixture.join(name)).unwrap();
    }
    let protocol = fixture.join("protocol/message.loom");
    let original = fs::read_to_string(&protocol).unwrap();
    let cache = temporary.path().join("frontend-cache");
    let checked = || {
        common::command(&["check"])
            .arg(&fixture)
            .arg("--frontend-cache")
            .arg(&cache)
            .output()
            .unwrap()
    };
    success(&checked());
    let broken = format!(
        "import std.text.concat\n{}",
        original.replace(
            "    to_bytes(text(value))",
            "    to_bytes(concat(text(value), \"x\"))"
        )
    );
    assert_ne!(broken, original);
    fs::write(&protocol, broken).unwrap();
    let rejected = checked();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("required postcondition is not proved")
    );
    let edited = common::command(&["editor-check", fixture.to_str().unwrap()])
        .arg("--std")
        .arg(common::root().join("compiler/std"))
        .output()
        .unwrap();
    assert_eq!(edited.status.code(), Some(1));
    assert!(edited.stderr.is_empty());
    let report = String::from_utf8(edited.stdout).unwrap();
    assert!(
        report.starts_with('{') && report.ends_with("}\n"),
        "{report}"
    );
    assert!(
        report.contains("required postcondition is not proved"),
        "{report}"
    );
    fs::write(&protocol, original).unwrap();
    success(&checked());
    fs::write(fixture.join("escape.loom"), "import bounded_datagrams.protocol.Message\nfn expose(value Message) Text { value.payload.body }\n").unwrap();
    let rejected = checked();
    assert_eq!(rejected.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .contains("cannot inspect a private type from another package")
    );
}
