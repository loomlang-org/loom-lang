mod common;
use common::success;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

fn package(root: &Path, name: &str, manifest: &str, source: &str) {
    let directory = root.join(name);
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("loom.toml"), manifest).unwrap();
    fs::write(directory.join("main.loom"), source).unwrap();
}

#[test]
fn build_basis_binds_fork_edges_even_when_native_code_and_old_receipt_fields_match() {
    let temporary = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temporary.path()).unwrap();
    let inspector = common::executable(&root, "inspect-basis");
    success(
        &common::command(&["build", "compiler/examples/analysis_build_basis"])
            .arg("--output")
            .arg(&inspector)
            .env("LOOM_OPT_LEVEL", "2")
            .output()
            .unwrap(),
    );
    for (suffix, minimum) in [("a", 0), ("b", 1)] {
        package(
            &root,
            &format!("shared-{suffix}"),
            "[module]\nname = \"shared\"\n",
            &format!(
                "pub fn accepted(value Int) Bool {{ value >= {minimum} }}\npub fn constant() Int {{ 2 }}\n"
            ),
        );
    }
    for (name, suffix) in [("alpha", "a"), ("beta", "b")] {
        package(
            &root,
            name,
            &format!(
                "[module]\nname = \"{name}\"\n[dependencies.shared]\npath = \"../shared-{suffix}\"\n"
            ),
            "import shared.constant\npub fn constant() Int { shared.constant() }\n",
        );
    }
    package(
        &root,
        "selected",
        "[module]\nname = \"selected\"\n[dependencies.shared]\npath = \"../shared-a\"\n",
        "import shared.accepted\npub type Amount = Int where shared.accepted(self)\n",
    );
    package(
        &root,
        "app",
        "[module]\nname = \"edge_subject\"\n[dependencies.alpha]\npath = \"../alpha\"\n[dependencies.beta]\npath = \"../beta\"\n[dependencies.selected]\npath = \"../selected\"\n",
        "import alpha.constant\nimport beta.constant\nimport selected.Amount\npub fn keep[T](value selected.Amount) selected.Amount { value }\nfn main() { discard alpha.constant()\ndiscard beta.constant() }\n",
    );
    let artifact = common::executable(&root, "artifact");
    let receipt = root.join("artifact.receipt");
    let mut versions = Vec::new();
    for suffix in ["a", "b"] {
        fs::write(
            root.join("selected/loom.toml"),
            format!("[module]\nname = \"selected\"\n[dependencies.shared]\npath = \"../shared-{suffix}\"\n"),
        )
        .unwrap();
        success(
            &common::command(&["build"])
                .arg(root.join("app"))
                .arg("--output")
                .arg(&artifact)
                .arg("--receipt")
                .arg(&receipt)
                .env("LOOM_OPT_LEVEL", "2")
                .output()
                .unwrap(),
        );
        let raw = fs::read_to_string(&receipt).unwrap();
        assert!(raw.starts_with("loom-build-receipt 5\n"));
        let field = |prefix: &str| {
            raw.lines()
                .find_map(|line| line.strip_prefix(prefix))
                .unwrap()
        };
        let artifact_hash = format!("{:x}", Sha256::digest(fs::read(&artifact).unwrap()));
        assert_eq!(field("artifact-sha256 "), artifact_hash);
        let output = Command::new(&inspector)
            .arg(root.join("app"))
            .arg(common::root().join("compiler/std"))
            .env("LOOM_GC_STRESS", "1")
            .output()
            .unwrap();
        success(&output);
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 2, "{text}");
        assert_eq!(
            lines[0].strip_prefix("basis ").unwrap(),
            field("basis-sha256 ")
        );
        assert_eq!(
            lines[1].strip_prefix("checked ").unwrap(),
            field("checked-sha256 ")
        );
        let old_fields = raw
            .lines()
            .filter(|line| !line.starts_with("basis-sha256 "))
            .collect::<Vec<_>>()
            .join("\n");
        versions.push((field("basis-sha256 ").to_owned(), old_fields));
    }
    assert_ne!(versions[0].0, versions[1].0);
    assert_eq!(versions[0].1, versions[1].1);
}
