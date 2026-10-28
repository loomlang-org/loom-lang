use std::{fs, process::Command};
mod common;
use common::success;

const FUNCTIONS: &str = include_str!("../examples/function_contracts/main.loom");
const FLOATS: &str = include_str!("../examples/function_contracts/floats.loom");
const CAPTURES: &str = include_str!("../examples/function_contracts/captures.loom");
const INPUTS: &str = include_str!("../examples/function_contracts/inputs.loom");
const SEQUENCES: &str = include_str!("../examples/function_contracts/sequences.loom");

const SCOPED_FUNCTIONS: &str = r#"
fn negative(value Int) Bool {
    var index = 0
    var answer = false
    while index < 2 {
        answer = value < 0
        index = index + 1
    }
    answer
}

fn positive(value Int) Bool {
    var index = 0
    var answer = false
    while index < 2 {
        answer = value >= 0
        index = index + 1
    }
    answer
}

fn observed(value Int) Bool
ensures result == negative(value) {
    negative(value)
}

fn warm(value Int) Bool
ensures result == negative(value) {
    observed(value)
}

type Negative = Int where negative(self)

fn selected(comptime positive Bool, value Int) Bool {
    var index = 0
    var answer = false
    while index < 2 {
        answer = comptime if positive {
            value >= 0
        } else {
            value < 0
        }
        index = index + 1
    }
    answer
}

fn preserved(value Negative) Bool
ensures negative(value) {
    true
}

fn specialization(value Int) Bool
ensures result == selected(false, value) {
    selected(false, value)
}

fn main() {
    assert !warm(1)
    assert preserved(Negative(-1))
    assert !specialization(1)
}
"#;

#[test]
fn proof_identities_survive_private_queues_without_equating_distinct_instances() {
    let package = tempfile::tempdir().unwrap();
    let path = package.path().to_str().unwrap();
    let source = package.path().join("main.loom");
    let cache = package.path().join("cache");
    let check = || common::loom(&["check", path, "--frontend-cache", cache.to_str().unwrap()]);
    fs::write(&source, SCOPED_FUNCTIONS).unwrap();
    success(&check());
    success(&check());
    success(&common::loom(&["run", path]));
    for (basis, extra) in [
        // warm has already certified observed in another private queue.
        (
            SCOPED_FUNCTIONS.split("type Negative").next().unwrap(),
            r#"
fn wrong(value Int) Bool
requires negative(value) == negative(value) && positive(value)
ensures result == positive(value) {
    observed(value)
}
"#,
        ),
        // The invariant's queue starts at negative; this one starts at positive.
        (
            SCOPED_FUNCTIONS,
            r#"
fn wrong(value Negative) Bool
ensures positive(value) {
    false
}
"#,
        ),
        // One source declaration still denotes distinct static specializations.
        (
            SCOPED_FUNCTIONS,
            r#"
fn wrong(value Int) Bool
requires selected(false, value) == selected(false, value) && selected(true, value)
ensures result == selected(true, value) {
    selected(false, value)
}
"#,
        ),
    ] {
        fs::write(&source, format!("{basis}\n{extra}")).unwrap();
        let output = check();
        assert!(
            !output.status.success(),
            "conflated proof identities: {extra}"
        );
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("required postcondition is not proved"),
            "{error}"
        );
        assert!(!error.contains("RuntimeFault"), "{error}");
        fs::write(&source, SCOPED_FUNCTIONS).unwrap();
        success(&check());
    }
}

#[test]
fn immutable_function_results_compose_and_reject_unestablished_evidence() {
    let package = tempfile::tempdir().unwrap();
    let path = package.path().to_str().unwrap();
    let source = package.path().join("main.loom");
    fs::write(&source, FUNCTIONS).unwrap();
    fs::write(package.path().join("floats.loom"), FLOATS).unwrap();
    fs::write(package.path().join("captures.loom"), CAPTURES).unwrap();
    fs::write(package.path().join("inputs.loom"), INPUTS).unwrap();
    fs::write(package.path().join("sequences.loom"), SEQUENCES).unwrap();
    let cache = package.path().join("cache");
    let check = || common::loom(&["check", path, "--frontend-cache", cache.to_str().unwrap()]);
    success(&check());
    success(&check());
    for extra in [
        // No call or entry predicate has established the function's domain.
        "fn bad(value Text) Bool\nensures count_open(value) == count_open(value)\n{\n    true\n}\n",
        "fn forever(value Text) Int {\n    forever(value)\n}\nfn bad(value Text) Bool\nensures forever(value) == forever(value)\n{\n    true\n}\n",
        // A successful call at another argument supplies no result equation.
        "fn bad(value Text, other Text) Int\nensures result == count_open(value)\n{\n    count_open(other)\n}\n",
        // Different result fields must not collapse to the same symbol.
        "fn bad(value Text) Bool\nrequires pair(value).0 > 0\nensures pair(value).0 == 0\n{\n    true\n}\n",
        // The same immutable result is reusable, but is not necessarily zero.
        "fn bad(value Text) Int\nensures result == 0\n{\n    count_open(value)\n}\n",
        // A returned Float may be NaN; successful evaluation is not IEEE equality.
        "fn bad(value Float) Float\nensures result == reciprocal(value)\n{\n    reciprocal(value)\n}\n",
        // IEEE numerical equality does not preserve a reciprocal's sign at zero.
        "fn bad(a Float, b Float) Float\nrequires a == b && reciprocal(a) > 0.0\nensures reciprocal(result) > 0.0\n{\n    b\n}\n",
        // Exact aggregate binding cannot establish unrelated output fields.
        "fn bad(value Text) Summary\nensures result.count == 0\n{\n    summarize(value)\n}\n",
        // Capture values, not allocations, distinguish known closure calls.
        r#"
fn bad(value Int, left Int, right Int) Int
ensures result == captured_sum(left)(value) {
    captured_sum(right)(value)
}
"#,
        r#"
fn bad(value Int, offset Int) Bool
ensures captured_sum(offset)(value) == captured_sum(offset)(value) {
    true
}
"#,
        r#"
fn bad(value Int) Int
ensures result == captured_sum(value + 1)(value + 1) {
    observed_capture_arguments(value)
}
"#,
        // IEEE equality cannot replace an exact Float capture representation.
        r#"
fn bad(value Int, left Float, right Float) Bool
requires left == right && captured_sign(left)(value)
ensures captured_sign(right)(value) {
    true
}
"#,
        // A mutable capture cell is state, not an immutable argument key.
        r#"
fn counter(start Int) fn() Int {
    var count = start
    fn() Int {
        count = count + 1
        count
    }
}
fn bad(start Int) Int
ensures result == counter(start)() {
    let callback = counter(start)
    discard callback()
    callback()
}
"#,
        // Immutable projections must retain every observable scalar component.
        r#"
fn bad(context SharedContext, other SharedContext) Int
ensures result == inspect_context(context) {
    inspect_context(other)
}
"#,
        // Reading a mutable sibling is not an immutable value computation.
        r#"
fn bad(context SharedContext) Int
requires context_has_first(context)
ensures result == reads_context(context) {
    let value = reads_context(context)
    replace_context_first(context, value + 1)
    value
}
"#,
        // Entry predicates participate in the read footprint too.
        r#"
fn bad(context SharedContext) Int
requires context_entry_valid(context)
ensures result == context_entry(context) {
    let value = context_entry(context)
    replace_context_first(context, -1)
    value
}
"#,
        // Dropping an argument from the key does not establish its evaluation.
        r#"
fn bad(value Int, values List[Int], nested List[List[Int]], index Int) Int
ensures result == eager_ignored_values(value, nested, index) {
    ignores_values(value, values)
}
"#,
        // A successful read binds a content version, not the shared handle.
        r#"
fn bad(values List[Int]) Int
requires length(values) > 0
ensures result == sequence_total(values) {
    let total = sequence_total(values)
    values[0] = 99
    total
}
"#,
        // Another parameter may be the same storage, even with equal lengths.
        r#"
fn bad(values List[Int], alias List[Int]) Int
requires length(alias) > 0
ensures result == sequence_total(values) {
    let total = sequence_total(values)
    alias[0] = 99
    total
}
"#,
        // A hypothetical content-dependent call still lacks its own domain.
        r#"
fn bad(values List[Int]) Bool
ensures sequence_total(values) == sequence_total(values) {
    true
}
"#,
        // Equal extents do not identify the contents of different inputs.
        r#"
fn bad(values List[Int], other List[Int]) Int
requires length(values) == length(other)
ensures result == sequence_total(values) {
    sequence_total(other)
}
"#,
        // Type facts belong to the constrained input, not another sequence.
        r#"
fn bad(values PositiveSequence, other List[Int]) Int
ensures result > 0 {
    sequence_total(other)
}
"#,
        // A type predicate is an entry fact, not an arbitrary result axiom.
        r#"
fn bad(values PositiveSequence) Int
ensures result < 0 {
    sequence_total(values)
}
"#,
        // Every constrained store still has to preserve the full predicate.
        r#"
fn bad(values PositiveSequence) Int
requires length(values) > 0
ensures result > 0 {
    values[0] = -1000
    sequence_total(values)
}
"#,
        // Child mutation need not change the outer sequence's version.
        r#"
fn nested_total(values List[List[Int]]) Int {
    var total = 0
    var index = 0
    while index < length(values) {
        total = total + sequence_total(get(values, index))
        index = index + 1
    }
    total
}
fn bad(values List[List[Int]], child List[Int]) Int
requires length(child) > 0
ensures result == nested_total(values) {
    let total = nested_total(values)
    child[0] = 99
    total
}
"#,
        // Contract results retain the actual function target's identity.
        "fn different(value Text) Int {\n    count_open(value) + 1\n}\nfn bad(value Text) Int\nensures result == different(value)\n{\n    count_open(value)\n}\n",
    ] {
        fs::write(&source, format!("{FUNCTIONS}\n{extra}")).unwrap();
        let output = check();
        assert!(!output.status.success(), "unsound function proof: {extra}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("proof")
                || error.contains("proved")
                || error.contains("pure Boolean scan"),
            "{error}\n{extra}"
        );
        assert!(!error.contains("RuntimeFault"), "{error}");
    }
    fs::write(&source, FUNCTIONS).unwrap();
    success(&check());
    let artifact = common::executable(package.path(), "functions");
    for level in ["0", "2"] {
        success(
            &common::command(&["test", path])
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        success(
            &common::command(&["build", path, "--output", artifact.to_str().unwrap()])
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
        // An argument omitted from a mathematical key still executes, even
        // when native optimization can see that the callee ignores its value.
        let eager = FUNCTIONS.replace(
            "fn main() {",
            "fn main() {\n    discard eager_ignored_values(3, [], 0)",
        );
        assert_ne!(eager, FUNCTIONS);
        fs::write(&source, eager).unwrap();
        success(
            &common::command(&["build", path, "--output", artifact.to_str().unwrap()])
                .env("LOOM_OPT_LEVEL", level)
                .output()
                .unwrap(),
        );
        let fault = Command::new(&artifact).output().unwrap();
        assert!(!fault.status.success());
        assert!(String::from_utf8_lossy(&fault.stderr).contains("list index out of bounds"));
        fs::write(&source, FUNCTIONS).unwrap();
    }
    success(&common::loom(&["run", path]));
}
