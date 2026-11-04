mod common;
use common::success;
use std::fs;

#[test]
fn source_record_codecs_use_native_fields_and_checked_constructors() {
    let directory = tempfile::tempdir().unwrap();
    let executable = common::executable(directory.path(), "storage");
    let example = "compiler/examples/storage";
    success(&common::loom(&["check", example]));
    for level in ["0", "2"] {
        for (mode, package) in [
            ("test", "compiler/std/storage"),
            ("test", example),
            ("build", example),
        ] {
            let mut command = common::command(&[mode, package]);
            if mode == "test" {
                command.arg("--no-run");
            }
            success(
                &command
                    .arg("--output")
                    .arg(&executable)
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            success(&common::run_tasks(&executable));
        }
    }
    success(&common::loom(&["run", example]));
}

#[test]
fn refined_record_codecs_recheck_changed_record_predicates() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("main.loom");
    let program = include_str!("../examples/storage/main.loom");
    let cache = directory.path().join("cache");
    let check = || {
        common::loom(&[
            "check",
            directory.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    fs::write(&source, program).unwrap();
    success(&check());
    success(&check());

    let weakened = program.replace("self.id > 0 && self.id <= self.amount", "self.id > 0");
    assert_ne!(weakened, program);
    fs::write(&source, &weakened).unwrap();
    let rejected = check();
    assert!(!rejected.status.success());
    let message = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        message.contains("proof") || message.contains("proved"),
        "{message}"
    );

    // The changed source is otherwise well typed: only its false guarantee
    // prevents checking. This control is checked, not executed.
    let control = weakened.replace("ensures result >= value.id {", "{");
    assert_ne!(control, weakened);
    fs::write(&source, control).unwrap();
    success(&check());

    fs::write(&source, program).unwrap();
    success(&check());
    success(&common::loom(&[
        "run",
        directory.path().to_str().unwrap(),
        "--frontend-cache",
        cache.to_str().unwrap(),
    ]));
}

#[test]
fn sqlite_record_checks_refresh_static_predicates_and_visibility_through_cache() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("main.loom");
    let cache = directory.path().join("cache");
    let executable = common::executable(directory.path(), "sqlite-check");
    let program = r#"
import std.storage.sqlite_record_check
import std.storage.SqliteCheckError
import std.storage.Table
import std.storage.Column
import std.storage.ValueKind
import std.io.write_text
import std.result.Result

pub record Fields {
    lower Int
    upper Int
    active Bool
}
pub type Guarded = Fields where !self.active || self.lower <= self.upper

fn main() {
    let clause = comptime {
        match sqlite_record_check[Guarded](Table {
            id = "bounds"
            physical_name = "bounds"
            columns = [Column { id = "upper" physical_name = "hi" kind = ValueKind.Int nullable = false },
                Column { id = "lower" physical_name = "lo" kind = ValueKind.Int nullable = false },
                Column { id = "active" physical_name = "on" kind = ValueKind.Int nullable = false }]
        }) {
            Result.Ok(value) => value
            Result.Err(SqliteCheckError.UnsupportedType) => "opaque"
            Result.Err(_) => "unexpected error"
        }
    }
    discard write_text(clause)
}
"#;
    let check = || {
        common::loom(&[
            "check",
            directory.path().to_str().unwrap(),
            "--frontend-cache",
            cache.to_str().unwrap(),
        ])
    };
    for (version, expected) in [
        (program.to_owned(), "\"lo\" <= \"hi\""),
        (
            program.replace("self.lower <= self.upper", "self.lower < self.upper"),
            "\"lo\" < \"hi\"",
        ),
        (
            program.replace("pub type Guarded", "type Guarded"),
            "opaque",
        ),
        (program.to_owned(), "\"lo\" <= \"hi\""),
    ] {
        fs::write(&source, version).unwrap();
        success(&check());
        success(&check());
        for level in ["0", "2"] {
            success(
                &common::command(&["build", directory.path().to_str().unwrap()])
                    .arg("--frontend-cache")
                    .arg(&cache)
                    .arg("--output")
                    .arg(&executable)
                    .env("LOOM_OPT_LEVEL", level)
                    .output()
                    .unwrap(),
            );
            let output = common::run_tasks(&executable);
            success(&output);
            let sql = String::from_utf8(output.stdout).unwrap();
            assert!(sql.contains(expected), "{sql}");
            if expected != "opaque" {
                assert!(sql.contains("typeof(\"lo\") = 'integer'"), "{sql}");
                assert!(sql.contains("\"on\" IN (0, 1)"), "{sql}");
                assert!(sql.ends_with(") IS 1)"), "{sql}");
            }
        }
    }
}
