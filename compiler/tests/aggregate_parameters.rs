use std::{fs, process::Command};
mod common;
use common::success;

const SOURCE: &str = r#"import std.list.new
import std.list.push
import std.list.length
import std.text.concat
import std.time.sleep_ms

record Payload {
    label Text
    left Text
    right Text
    values List[Int]
    number Int
}

fn payload(number Int, values List[Int]) Payload {
    Payload {
        label = "value"
        left = "left"
        right = "right"
        values = values
        number = number
    }
}

fn inspect(value Payload) Int {
    push(value.values, value.number)
    assert concat(value.label, concat(value.left, value.right)) == "valueleftright"
    assert value.values[length(value.values) - 1] == value.number
    value.number
}

fn after(value Payload, ignored Int) Int {
    assert ignored == 0
    inspect(value)
}

fn paired(first Payload, second Payload) Int {
    inspect(first) + inspect(second)
}

fn invoke(callback fn(Payload) Int, value Payload) Int {
    callback(value)
}

fn retain(value Payload) fn() Int {
    fn() Int {
        inspect(value)
    }
}

concept Combine {
    fn combine(self Self, other Payload) Int
}

impl Combine for Payload {
    fn combine(self Self, other Payload) Int {
        let before = self.number
        discard inspect(self)
        before + inspect(other)
    }
}

async fn later(value Payload) Int {
    sleep_ms(1).await
    inspect(value)
}

enum Choice {
    Small(Int)
    Large(Payload)
    Pair(Payload, Payload)
}

fn chosen(value Choice) Int {
    match value {
        Choice.Small(number) => number
        Choice.Large(data) => inspect(data)
        Choice.Pair(first, second) => inspect(first) + inspect(second)
    }
}

async fn main() {
    let values = new[Int]()
    let original = payload(10, values)
    assert inspect(original) == 10
    assert length(values) == 1 && values[0] == 10
    var current = original
    let replace = fn() Int {
        current = payload(99, values)
        0
    }
    assert after(current, replace()) == 10
    assert current.number == 99
    assert invoke(inspect, original) == 10
    let captured = fn(value Payload) Int {
        inspect(original) + inspect(value)
    }
    assert invoke(captured, current) == 109
    let view dyn Combine = original
    assert view.combine(current) == 109
    let work = later(current)
    current = payload(123, values)
    assert work.await == 99
    assert original.number == 10 && current.number == 123
    assert chosen(Choice.Small(3)) == 3
    assert chosen(Choice.Large(original)) == 10
    assert chosen(Choice.Pair(original, current)) == 133
    assert paired(original, current) == 133
    let saved = new[fn() Int]()
    var index = 0
    while index < 2 {
        push(saved, retain(payload(20 + index, values)))
        index = index + 1
    }
    assert saved[0]() == 20 && saved[1]() == 21
    assert length(values) == 15
}
"#;

#[test]
fn aggregate_parameters_preserve_snapshots_shared_children_and_moving_roots() {
    let package = tempfile::tempdir().unwrap();
    fs::write(package.path().join("main.loom"), SOURCE).unwrap();
    let executable = common::executable(package.path(), "aggregate-parameters");
    for level in ["0", "2"] {
        success(
            &common::command(&[
                "build",
                package.path().to_str().unwrap(),
                "--output",
                executable.to_str().unwrap(),
            ])
            .env("LOOM_OPT_LEVEL", level)
            .output()
            .unwrap(),
        );
        let output = common::run_task_command(Command::new(&executable).env("LOOM_GC_STRESS", "1"));
        success(&output);
        assert!(output.stdout.is_empty());
    }
}
