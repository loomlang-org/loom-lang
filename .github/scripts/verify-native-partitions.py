#!/usr/bin/env python3
"""Verify complete nextest manifests against two successful JUnit partitions."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, f"duplicate JSON key: {key}")
        value[key] = item
    return value


def manifest_cases(raw):
    manifest = json.loads(raw, object_pairs_hook=unique_object)
    require(isinstance(manifest, dict), "expected a complete manifest object")
    suites = manifest.get("rust-suites")
    require(isinstance(suites, dict), "missing complete rust-suites manifest")
    cases = {}
    for binary, suite in suites.items():
        require(bool(binary) and isinstance(suite, dict), "invalid test binary")
        require(suite.get("binary-id") == binary, "binary ID differs from suite key")
        require(suite.get("status") == "listed", f"unlisted test binary: {binary}")
        tests = suite.get("testcases")
        require(isinstance(tests, dict), f"missing test cases: {binary}")
        for name, test in tests.items():
            require(bool(name) and isinstance(test, dict), "invalid test case")
            ignored = test.get("ignored")
            require(type(ignored) is bool, f"missing ignore status: {binary}/{name}")
            if not ignored:
                match = test.get("filter-match")
                require(isinstance(match, dict) and match.get("status") == "matches",
                        f"filtered manifest cannot prove full coverage: {binary}/{name}")
            cases[(binary, name)] = ignored
    require(type(manifest.get("test-count")) is int and manifest["test-count"] == len(cases),
            "manifest test-count differs from listed cases")
    require(any(not ignored for ignored in cases.values()), "no enabled tests in manifest")
    return cases


def junit_cases(raw):
    root = ET.fromstring(raw)
    require(root.tag in {"testsuites", "testsuite"}, "unrecognized JUnit root")
    for element in root.iter():
        require(element.tag not in {"failure", "error", "skipped"},
                f"unsuccessful JUnit result: {element.tag}")
    cases = set()
    for suite in root.iter("testsuite"):
        binary = suite.get("name")
        require(bool(binary), "JUnit suite has no binary ID")
        for case in suite.findall("testcase"):
            name = case.get("name")
            require(bool(name), "JUnit test has no name")
            identity = (binary, name)
            require(identity not in cases, f"duplicate JUnit case: {binary}/{name}")
            cases.add(identity)
    require(sum(1 for _ in root.iter("testcase")) == len(cases),
            "JUnit case outside a test suite")
    return cases


def artifact_file(directory, name):
    directory = Path(directory).resolve()
    matches = list(directory.rglob(name))
    require(len(matches) == 1 and matches[0].is_file(), f"expected one {name} in {directory}")
    require(matches[0].resolve().is_relative_to(directory), "artifact file escapes its directory")
    return matches[0].read_bytes()


def audit_partitions(directories, source=None):
    require(len(directories) == 2, "exactly two native partitions are required")
    manifests = [manifest_cases(artifact_file(path, "native-test-manifest.json"))
                 for path in directories]
    require(manifests[0] == manifests[1], "partition manifests list different test sets")
    required = {identity for identity, ignored in manifests[0].items() if not ignored}
    executed = [junit_cases(artifact_file(path, "junit.xml")) for path in directories]
    require(not (executed[0] & executed[1]), "test executed in both partitions")
    require((executed[0] | executed[1]) == required,
            "executed union differs from every enabled manifest case")
    if source is not None:
        require(bool(re.fullmatch(r"[0-9a-f]{40}", source)), "invalid expected source commit")
        for directory in directories:
            observed = artifact_file(directory, "native-test-source.txt").decode("ascii").strip()
            require(observed == source, "native worker source differs from current checkout")
    canonical = [[binary, name, ignored]
                 for (binary, name), ignored in sorted(manifests[0].items())]
    digest = hashlib.sha256(json.dumps(canonical, separators=(",", ":")).encode()).hexdigest()
    return {"status": "PASS", "sourceCommit": source, "listed": len(canonical),
            "ignored": len(canonical) - len(required), "executed": len(required),
            "partitions": list(map(len, executed)), "manifestSha256": digest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    parser.add_argument("--require-checkout", action="store_true",
                        help="require both worker commits to match git rev-parse HEAD")
    args = parser.parse_args()
    source = None
    if args.require_checkout:
        source = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    try:
        result = audit_partitions([args.first, args.second], source)
    except (ValueError, KeyError, TypeError, OSError, ET.ParseError) as error:
        print(f"Native partition audit failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
