use std::{fs, path::Path, process::Command};
mod common;
use common::{loom, success};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn source_path_dependencies_pass_native_commands_without_dependency_tests() {
    for relative in [
        "compiler/examples/modules/app",
        "compiler/examples/module_configuration/app",
    ] {
        let package = common::root().join(relative);
        let package = package.to_str().unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let executable = common::executable(temporary.path(), "modules");
        let ir = temporary.path().join("modules.ll");
        success(&loom(&["check", package]));
        success(
            &common::command(&[
                "build",
                package,
                "--output",
                executable.to_str().unwrap(),
                "--emit-ir",
                ir.to_str().unwrap(),
            ])
            .env("LOOM_OPT_LEVEL", "0")
            .output()
            .unwrap(),
        );
        success(&Command::new(executable).output().unwrap());
        success(&loom(&["run", package]));
        let tests = loom(&["test", package]);
        success(&tests);
        assert_eq!(tests.stdout, b"1 tests passed\n");
        assert!(!fs::read_to_string(ir).unwrap().contains("9001"));
    }
}

#[test]
fn dependency_visibility_and_module_identity_follow_the_declaring_manifest() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(
        root,
        "app/loom.toml",
        "[module]\nname='app'\n[dependencies.codec]\npath='../codec'\n[dependencies.other]\npath='../other'\n[dependencies.unused]\npath='../not-present'\n",
    );
    let main = "import codec.answer\nimport codec.token\nimport other.answer\nimport other.consume\nfn main() { assert codec.answer() == 42 && other.answer() == 42\nassert consume(token()) == 42 }";
    write(root, "app/main.loom", main);
    write(root, "app/main_test.loom", "test fn app_test() { main() }");
    write(
        root,
        "codec/loom.toml",
        "[module]\nname='codec'\n[dependencies.seed]\npath='../seed'\n",
    );
    write(
        root,
        "codec/漢漢.loom",
        "import seed.value\nimport seed.Token\npub fn answer() Int { value() }\npub fn token() Token { Token { value = 42 } }\ntest fn embedded() { assert false }",
    );
    write(
        root,
        "codec/main_test.loom",
        "test fn excluded() { assert false }",
    );
    write(
        root,
        "other/loom.toml",
        "[module]\nname='other'\n[dependencies.seed]\npath='../seed/.'\n",
    );
    write(
        root,
        "other/main.loom",
        "import seed.value\nimport seed.Token\npub fn answer() Int { value() }\npub fn consume(value Token) Int { value.value }",
    );
    write(root, "seed/loom.toml", "[module]\nname='seed'\n");
    write(
        root,
        "seed/main.loom",
        "pub record Token { value Int }\npub fn value() Int { 42 }",
    );
    let package = root.join("app");
    let package = package.to_str().unwrap();
    success(&loom(&["run", package]));
    let output = loom(&["test", package]);
    success(&output);
    assert_eq!(output.stdout, b"1 tests passed\n");

    // Loading a transitive dependency does not make it a direct dependency.
    write(
        root,
        "app/main.loom",
        "import codec.answer\nimport seed.value\nfn main() { assert answer() == value() }",
    );
    let output = loom(&["check", package]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("seed"));

    // Distinct instances coexist, but equal names/layouts do not unify types.
    write(
        root,
        "app/main.loom",
        "import codec.answer\nimport other.answer\nfn main() { assert codec.answer() == 42 && other.answer() == 99 }",
    );
    write(
        root,
        "other/loom.toml",
        "[module]\nname='other'\n[dependencies.seed]\npath='../alternate'\n",
    );
    write(root, "alternate/loom.toml", "[module]\nname='seed'\n");
    write(
        root,
        "alternate/main.loom",
        "pub record Token { value Int }\npub fn value() Int { 99 }",
    );
    success(&loom(&["run", package]));
    write(root, "app/main.loom", main);
    let output = loom(&["check", package]);
    assert_eq!(output.status.code(), Some(1));
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    let expected = format!(
        "expected seed.Token (from {})",
        root.join("alternate").canonicalize().unwrap().display()
    );
    let found = format!(
        "found seed.Token (from {})",
        root.join("seed").canonicalize().unwrap().display()
    );
    assert!(
        diagnostic.contains(&expected) && diagnostic.contains(&found),
        "{output:?}"
    );
}

#[test]
fn selected_root_identity_does_not_leak_through_a_same_named_dependency() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(
        root,
        "app/loom.toml",
        "[module]\nname='app'\n[dependencies.other]\npath='../other'\n",
    );
    write(
        root,
        "app/main.loom",
        "import other.answer\nfn main() { assert answer() == 7 }\ntest fn selected() { main() }",
    );
    write(
        root,
        "other/loom.toml",
        "[module]\nname='other'\n[dependencies.app]\npath='../nested'\n",
    );
    write(
        root,
        "other/main.loom",
        "import app.value\npub fn answer() Int { value() }",
    );
    write(root, "nested/loom.toml", "[module]\nname='app'\n");
    write(
        root,
        "nested/main.loom",
        "pub fn value() Int { 7 }\nfn main() { assert false }\ntest fn excluded() { assert false }",
    );
    write(
        root,
        "nested/main_test.loom",
        "test fn excluded_file() { assert false }",
    );
    let package = root.join("app");
    let package = package.to_str().unwrap();
    success(&loom(&["run", package]));
    let output = loom(&["test", package]);
    success(&output);
    assert_eq!(output.stdout, b"1 tests passed\n");

    // Only the nested app imports other, producing a real instance cycle.
    write(
        root,
        "nested/loom.toml",
        "[module]\nname='app'\n[dependencies.other]\npath='../other'\n",
    );
    write(
        root,
        "nested/main.loom",
        "import other.answer\npub fn value() Int { answer() }",
    );
    let output = loom(&["check", package]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cycle"));
}

#[test]
fn import_completion_respects_nested_modules_and_directory_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(root, "app/loom.toml", "[module]\nname='app'\n");
    write(root, "app/main.loom", "import app.");
    write(
        root,
        "app/normal/main.loom",
        "pub fn visible() {}\nfn hidden() {}\n",
    );
    write(root, "app/nested/loom.toml", "[module]\nname='nested'\n");
    write(root, "app/nested/main.loom", "pub fn excluded() {}\n");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("app/normal"), root.join("app/alias")).unwrap();
    let package = root.join("app");
    let source = package.join("main.loom");
    for (text, expected) in [
        ("import app.", "\"label\":\"normal\""),
        ("import app.normal.", "\"label\":\"visible\""),
        ("import app.nested.", "\"items\":[]"),
        ("import app.alias.", "\"items\":[]"),
    ] {
        fs::write(&source, text).unwrap();
        let output = common::command(&["editor-complete", package.to_str().unwrap()])
            .arg("--std")
            .arg(common::root().join("compiler/std"))
            .arg("--at")
            .arg(&source)
            .arg(text.len().to_string())
            .output()
            .unwrap();
        success(&output);
        let json = String::from_utf8(output.stdout).unwrap();
        assert!(json.contains(expected), "{json}");
        for excluded in ["alias", "nested", "hidden", "excluded"] {
            assert!(
                !json.contains(&format!("\"label\":\"{excluded}\"")),
                "{json}"
            );
        }
    }
}

#[test]
fn module_configuration_binds_instances_caches_and_artifact_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    write(
        root,
        "app/loom.toml",
        "[module]\nname='app'\n[configuration]\nmode='application'\n[dependencies.search]\npath='../search'\n[dependencies.export]\npath='../export'\n",
    );
    let app = r#"import search.value
import search.make
import export.value
import export.consume
import std.build.configuration_text
import std.build.option
import std.io.write_text
fn main() {
    assert configuration_text("mode") == "application"
    assert option("mode", "fallback") == "global"
    discard write_text(search.value())
    discard write_text("|")
    discard write_text(export.value())
}
test fn selected_application() { main() }
"#;
    write(root, "app/main.loom", app);
    write(
        root,
        "codec/loom.toml",
        "[module]\nname='codec'\n[configuration]\nmode='default'\nempty=''\n",
    );
    write(
        root,
        "codec/main.loom",
        r#"import std.build.configuration_text
import std.build.configuration_present
import std.build.option
import std.reflect.Schema
import std.text.concat
pub record Message { value Int }
pub fn make() Message { Message { value = 42 } }
pub fn consume(value Message) Int { value.value }
pub fn configured[T](value T) Text {
    assert configuration_present("mode") && configuration_present("empty")
    assert !configuration_present("absent") && configuration_text("empty") == ""
    assert option("mode", "fallback") == "global"
    configuration_text("mode")
}
pub fn callback() Text {
    let read = fn() Text { configuration_text("mode") }
    read()
}
pub fn generate(types List[Schema]) Text {
    discard types
    concat("\"", concat(configuration_text("mode"), "\""))
}
"#,
    );
    let importer = |name: &str, mode: &str, reversed: bool| {
        let base = format!(
            "[module]\nname='{name}'\n[dependencies.codec]\npath='../codec/.'\n[dependencies.codec.configuration]\n"
        );
        let settings = if reversed {
            format!("mode='{mode}'\nempty=''\n")
        } else {
            format!("empty=''\nmode='{mode}'\n")
        };
        write(root, &format!("{name}/loom.toml"), &(base + &settings));
    };
    for (name, mode) in [("search", "search"), ("export", "export")] {
        importer(name, mode, false);
        write(
            root,
            &format!("{name}/main.loom"),
            r#"import codec.configured
import codec.callback
import codec.generate
import codec.Message
import codec.make
import codec.consume
pub fn value() Text {
    let selected = configured(1)
    assert configured(true) == selected && callback() == selected
    assert generate!(1) == selected
    selected
}
pub fn make() Message { codec.make() }
pub fn consume(value Message) Int { codec.consume(value) }
"#,
        );
    }
    let package = root.join("app");
    let executable = common::executable(root, "configured");
    let cache = root.join("frontend");
    let receipt = root.join("build.receipt");
    let build = |level: &str| {
        common::command(&["build"])
            .arg(&package)
            .args(["--build-option", "mode=global", "--frontend-cache"])
            .arg(&cache)
            .arg("--output")
            .arg(&executable)
            .arg("--receipt")
            .arg(&receipt)
            .env("LOOM_OPT_LEVEL", level)
            .env("LOOM_NATIVE_TIMINGS", "1")
            .output()
            .unwrap()
    };
    for level in ["0", "2"] {
        success(&build(level));
        let output = Command::new(&executable)
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        assert_eq!(output.stdout, b"search|export");
    }
    let original = fs::read_to_string(&receipt).unwrap();
    assert!(original.starts_with("loom-build-receipt 5\n"));
    let codec_root = root.join("codec").canonicalize().unwrap();
    let hex_root: String = codec_root
        .to_str()
        .unwrap()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let instances: Vec<_> = original
        .lines()
        .filter(|line| line.starts_with(&format!("module 636f646563 {hex_root} ")))
        .collect();
    assert_eq!(instances.len(), 2, "{original}");
    assert_ne!(instances[0], instances[1]);

    // Reordering configuration keys retains the exact compiled closure.
    importer("export", "export", true);
    let reordered = build("2");
    success(&reordered);
    assert!(String::from_utf8_lossy(&reordered.stderr).contains("frontend hit"));
    // Same physical source and nominal layout, different configuration.
    write(
        root,
        "app/main.loom",
        &app.replace(
            "    assert configuration_text",
            "    assert consume(make()) == 42\n    assert configuration_text",
        ),
    );
    let invalid = build("0");
    assert_eq!(invalid.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("configuration sha256:"),
        "{invalid:?}"
    );
    assert_eq!(fs::read_to_string(&receipt).unwrap(), original);

    // Equal effective values reuse identity, including defaults and spelling.
    write(
        root,
        "export/loom.toml",
        "[module]\nname='export'\n[dependencies.codec]\npath='../codec'\n[dependencies.codec.configuration]\nmode='search'\n",
    );
    let normalized = build("2");
    success(&normalized);
    assert!(String::from_utf8_lossy(&normalized.stderr).contains("frontend miss"));
    let output = Command::new(&executable)
        .env("LOOM_GC_STRESS", "1")
        .output()
        .unwrap();
    success(&output);
    assert_eq!(output.stdout, b"search|search");
    let shared = fs::read_to_string(&receipt).unwrap();
    assert_eq!(
        shared
            .lines()
            .filter(|line| line.starts_with(&format!("module 636f646563 {hex_root} ")))
            .count(),
        1
    );
    success(
        &common::command(&["test"])
            .arg(&package)
            .args(["--build-option", "mode=global"])
            .output()
            .unwrap(),
    );

    // A missing key cannot reuse the previously successful cache or receipt.
    let codec_source = fs::read_to_string(root.join("codec/main.loom")).unwrap();
    write(
        root,
        "codec/main.loom",
        &codec_source.replacen(
            "configuration_text(\"mode\")",
            "configuration_text(\"missing\")",
            1,
        ),
    );
    let missing = build("0");
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&missing.stderr)
            .contains("module configuration has no supplied value"),
        "{missing:?}"
    );
    assert_eq!(fs::read_to_string(&receipt).unwrap(), shared);
    write(root, "codec/main.loom", &codec_source);
    success(&build("2"));
    success(
        &Command::new(&executable)
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap(),
    );
    // Root graph configuration must satisfy, rather than overwrite, an importer.
    let manifest = fs::read_to_string(root.join("app/loom.toml")).unwrap();
    write(
        root,
        "app/loom.toml",
        &(manifest.clone()
            + "[dependencies.codec]\npath='../codec'\nscope='graph'\n[dependencies.codec.configuration]\nmode='export'\n"),
    );
    let conflict = build("0");
    assert_eq!(conflict.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&conflict.stderr)
            .contains("incompatible module configuration requirements"),
        "{conflict:?}"
    );
    write(root, "app/loom.toml", &manifest);
    success(&build("2"));
}
