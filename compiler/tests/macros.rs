use std::{fs, process::Command};

mod common;
use common::{loom, success};

#[test]
fn static_macro_parameters_compose_with_proofs_callbacks_and_input_sequencing() {
    let package = "compiler/examples/static_macros";
    success(&loom(&["check", package]));
    let temporary = tempfile::tempdir().unwrap();
    for level in ["0", "2"] {
        let executable = common::executable(temporary.path(), "static-macros");
        success(
            &common::command(&["build", package])
                .arg("--output")
                .arg(&executable)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &Command::new(&executable)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
        success(
            &common::command(&["test", package])
                .args(["--no-run", "--output"])
                .arg(&executable)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &Command::new(&executable)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
    }
    let directory = common::root().join(package);
    let source = directory.join("main.loom");
    let text = fs::read_to_string(&source).unwrap();
    let query = loom(&[
        "editor-query",
        directory.to_str().unwrap(),
        "--at",
        source.to_str().unwrap(),
        &text.find("field!(\"upper\", value)").unwrap().to_string(),
    ]);
    success(&query);
    let report = String::from_utf8(query.stdout).unwrap();
    assert!(report.contains("\"types\":[\"Int\"]"), "{report}");
    assert!(
        report
            .replace("\\\\", "/")
            .contains("generators/generate.loom")
    );
}

#[test]
fn static_macro_cache_rechecks_configured_projection_contracts_and_privacy() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().canonicalize().unwrap();
    for name in ["main.loom", "loom.toml", "generators/generate.loom"] {
        let destination = directory.join(name);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(
            common::root()
                .join("compiler/examples/static_macros")
                .join(name),
            destination,
        )
        .unwrap();
    }
    let cache = directory.join("cache");
    let source = directory.join("main.loom");
    let original = fs::read_to_string(&source).unwrap();
    let checked = || {
        common::command(&["check"])
            .arg(&directory)
            .arg("--frontend-cache")
            .arg(&cache)
            .output()
            .unwrap()
    };
    success(&checked());
    let changed = original.replace(
        "left = field!(\"lower\", value)",
        "left = field!(\"upper\", value)",
    );
    assert_ne!(changed, original);
    fs::write(&source, changed).unwrap();
    let rejected = checked();
    assert!(
        !rejected.status.success(),
        "stale configured projection proved"
    );
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("postcondition"));
    fs::write(&source, &original).unwrap();
    success(&checked());
    fs::write(
        &source,
        original
            .replace("pub record BeforeFields", "record BeforeFields")
            .replace("pub type Before =", "type Before ="),
    )
    .unwrap();
    let rejected = checked();
    assert!(
        !rejected.status.success(),
        "private configured projection accepted"
    );
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("private"));
    fs::write(&source, original).unwrap();
    success(&checked());
}

#[test]
fn static_macro_callbacks_cannot_execute_io_or_read_runtime_captures() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("main.loom");
    let header = r#"
import std.reflect.Schema
import std.io.write_error
fn noisy(value Int) Int {
    discard write_error("must not run")
    value
}
fn generate(comptime action fn(Int) Int, types List[Schema]) Text {
    discard types
    discard action
    "$0"
}
fn main() { assert generate!(noisy, 42) == 42 }
"#;
    fs::write(&source, header).unwrap();
    let checked = || {
        common::command(&["check"])
            .arg(temporary.path())
            .output()
            .unwrap()
    };
    // Passing a target identity does not execute the target.
    let accepted = checked();
    success(&accepted);
    assert!(!String::from_utf8_lossy(&accepted.stderr).contains("must not run"));
    fs::write(
        &source,
        header.replace("discard action\n", "discard action(1)\n"),
    )
    .unwrap();
    let rejected = checked();
    assert!(
        !rejected.status.success(),
        "effectful generator callback executed"
    );
    assert!(!String::from_utf8_lossy(&rejected.stderr).contains("must not run"));
    fs::write(
        &source,
        header.replace(
            "fn main() { assert generate!(noisy, 42) == 42 }",
            "fn dynamic(offset Int) Int { generate!(fn(value Int) Int { value + offset }, 42) }\nfn main() { discard dynamic(1) }",
        ),
    )
    .unwrap();
    let rejected = checked();
    assert!(!rejected.status.success(), "runtime capture was frozen");
}

#[test]
fn typed_macros_execute_without_a_runtime_generator() {
    let package = "compiler/examples/typed_macros";
    success(&loom(&["check", package]));
    success(&loom(&["test", package]));
    let temporary = tempfile::tempdir().unwrap();
    let executable = common::executable(temporary.path(), "macros");
    let ir = temporary.path().join("macros.ll");
    for level in ["0", "2"] {
        success(
            &common::command(&["build", package])
                .arg("--output")
                .arg(&executable)
                .arg("--emit-ir")
                .arg(&ir)
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &Command::new(&executable)
                .env("LOOM_GC_STRESS", "1")
                .output()
                .unwrap(),
        );
        let lowered = fs::read_to_string(&ir).unwrap();
        assert!(!lowered.contains("$0"));
        assert!(!lowered.contains("loom_rt_macro"));
    }
    let directory = common::root().join(package);
    let source = directory.join("main.loom");
    let text = fs::read_to_string(&source).unwrap();
    let output = loom(&[
        "editor-query",
        directory.to_str().unwrap(),
        "--at",
        source.to_str().unwrap(),
        &text.find("helper_call!(40)").unwrap().to_string(),
    ]);
    success(&output);
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(report.contains("\"types\":[\"Int\"]"), "{report}");
    assert!(
        report
            .replace("\\\\", "/")
            .contains("generators/generate.loom"),
        "{report}"
    );
}

#[test]
fn generated_code_keeps_proof_and_one_shot_obligations() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("main.loom");
    let cases = [
        (
            r#"
import std.reflect.Schema
import std.resource.MustScope
import std.resource.Dispose

record Guard {
    value Int
}

impl MustScope for Guard {}

impl Dispose for Guard {
    fn dispose(self Guard) {
        discard self.value
    }
}

fn ignore(types List[Schema]) Text {
    discard types
    "true"
}

fn main() {
    discard ignore!(Guard { value = 1 })
}
"#,
            "macro resource input must enter scoped",
        ),
        (
            r#"
import std.reflect.Schema
import std.resource.NoSuspend

record Guard {
    value Int
}

impl NoSuspend for Guard {}

fn wait(types List[Schema]) Text {
    discard types
    "$0.await"
}

async fn task() Int {
    1
}

async fn main() {
    let guard = Guard { value = 0 }
    discard wait!(task())
    discard guard
}
"#,
            "NoSuspend value is still in scope",
        ),
        (
            r#"
import std.reflect.Schema

fn generate(types List[Schema]) Text {
    discard types
    "0"
}

fn positive() Int
ensures result > 0
{
    generate!()
}
"#,
            "not proved",
        ),
        (
            r#"
import std.reflect.Schema

fn generate(types List[Schema]) Text {
    discard types
    "($0, $0)"
}

async fn task() Int {
    1
}

async fn main() {
    let (first, second) = generate!(task())
    discard first.await
    discard second.await
}
"#,
            "task",
        ),
        (
            r#"
import std.reflect.Schema
import std.io.write_text

fn generate(types List[Schema]) Text {
    discard types
    discard write_text("must not execute during compilation")
    "1"
}

fn main() {
    discard generate!()
}
"#,
            "compile-time",
        ),
    ];
    for (text, diagnostic) in cases {
        fs::write(&source, text).unwrap();
        let output = common::command(&["check"])
            .arg(temporary.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains(diagnostic), "{message}");
    }
}

#[test]
fn structured_macro_output_requires_valid_source_syntax() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("main.loom");
    for (kind, value, message) in [
        ("Name", "f()", "syntax tree"),
        ("Binary", "+", "missing a child"),
        ("ExpansionId", "fake", "internal macro syntax"),
    ] {
        fs::write(
            &path,
            format!(
                r#"
import std.reflect.Schema
import std.loom.ast.Node
import std.loom.ast.NodeKind
import std.loom.source.Span
import std.list.new

fn generate(types List[Schema]) Node {{
    discard types
    Node {{
        kind = NodeKind.{kind}
        value = "{value}"
        span = Span {{
            start = 0
            end = 0
        }}
        children = new[Node]()
    }}
}}

fn main() {{
    discard generate!()
}}
"#
            ),
        )
        .unwrap();
        let result = common::command(&["check"])
            .arg(temporary.path())
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(message),
            "{result:?}"
        );
    }
}

#[test]
fn source_ast_tool_generates_a_normally_checked_package() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("compiler-client");
    for name in [
        "main.loom",
        "loom.toml",
        "definition.txt",
        "emitter/emitter.loom",
    ] {
        let target = source.join(name);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(
            common::root()
                .join("compiler/examples/ast_generation")
                .join(name),
            target,
        )
        .unwrap();
    }
    let generator = common::executable(temporary.path(), "generator");
    let cache = temporary.path().join("generator-cache");
    let receipt = temporary.path().join("generator.receipt");
    let build = |level: &str| {
        let output = common::command(&["build", source.to_str().unwrap()])
            .arg("--frontend-cache")
            .arg(&cache)
            .arg("--receipt")
            .arg(&receipt)
            .arg("--output")
            .arg(&generator)
            .env("LOOM_OPT_LEVEL", level)
            .output()
            .unwrap();
        success(&output);
        assert!(
            fs::read_to_string(&receipt)
                .unwrap()
                .contains("\ninput-count 1\n")
        );
    };
    let generate = || {
        let output = Command::new(&generator)
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        output.stdout
    };
    let mut initial = None;
    for level in ["0", "2"] {
        build(level);
        let generated = generate();
        if let Some(expected) = &initial {
            assert_eq!(&generated, expected);
        } else {
            initial = Some(generated);
        }
    }
    let initial = initial.unwrap();
    let input = source.join("definition.txt");
    let retained = fs::read(&input).unwrap();
    fs::remove_file(&input).unwrap();
    assert_eq!(
        generate(),
        initial,
        "generator reopened a captured build input"
    );
    fs::write(input, retained).unwrap();
    let package = temporary.path().join("generated");
    fs::create_dir(&package).unwrap();
    let generated_file = package.join("main.loom");
    fs::write(&generated_file, &initial).unwrap();
    let generated_cache = temporary.path().join("generated-cache");
    let checked = |operation: &str| {
        common::command(&[operation, package.to_str().unwrap()])
            .arg("--frontend-cache")
            .arg(&generated_cache)
            .output()
            .unwrap()
    };
    for operation in ["check", "test", "run"] {
        let output = checked(operation);
        success(&output);
        if operation == "test" {
            assert_eq!(output.stdout, b"1 tests passed\n");
        }
    }

    let declaration = source.join("definition.txt");
    let original = fs::read_to_string(&declaration).unwrap();
    let invalid = original.replace(
        "result == value.x + value.y",
        "result == value.x + value.y + 1",
    );
    assert_ne!(invalid, original);
    fs::write(&declaration, invalid).unwrap();
    build("0");
    let changed = generate();
    assert_ne!(
        changed, initial,
        "tracked input edit reused the old generator"
    );
    fs::write(&generated_file, changed).unwrap();
    let rejected = checked("check");
    assert!(
        !rejected.status.success(),
        "invalid generated postcondition was proved"
    );
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("postcondition"));
    fs::write(&declaration, original).unwrap();
    build("0");
    assert_eq!(generate(), initial);

    let emitter = source.join("emitter/emitter.loom");
    let original = fs::read_to_string(&emitter).unwrap();
    let changed = original.replace("    y Int\n", "    y Int\n    z Int\n");
    assert_ne!(changed, original);
    fs::write(&emitter, changed).unwrap();
    build("0");
    let changed = generate();
    assert!(String::from_utf8_lossy(&changed).contains("z Int"));
    fs::write(&generated_file, changed).unwrap();
    let rejected = checked("check");
    assert!(
        !rejected.status.success(),
        "missing reflected field was accepted"
    );
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("field"));
    fs::write(&emitter, original).unwrap();
    build("0");
    assert_eq!(generate(), initial);

    let main = source.join("main.loom");
    let original = fs::read_to_string(&main).unwrap();
    fs::write(
        &main,
        format!("import ast_generation.emitter.field_names\n{original}"),
    )
    .unwrap();
    let rejected = common::loom(&[
        "check",
        source.to_str().unwrap(),
        "--frontend-cache",
        cache.to_str().unwrap(),
    ]);
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("accessible declaration"));
    fs::write(&main, original).unwrap();
    build("0");
    assert_eq!(generate(), initial);
    fs::write(&generated_file, initial).unwrap();
    success(&checked("test"));
}
