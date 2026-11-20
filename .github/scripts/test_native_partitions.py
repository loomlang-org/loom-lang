import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import xml.etree.ElementTree as ET

spec = importlib.util.spec_from_file_location(
    "partition_audit", Path(__file__).with_name("verify-native-partitions.py"))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


def manifest():
    def suite(binary, names):
        return {"binary-id": binary, "status": "listed", "testcases": {
            name: {"ignored": ignored, "filter-match": {"status": "matches"}}
            for name, ignored in names}}
    return {"test-count": 5, "rust-suites": {
        "library": suite("library", [("one", False), ("two & 雪", False), ("ignored", True)]),
        "integration": suite("integration", [("one", False), ("two", False)])}}


def junit(cases, failure=None):
    root = ET.Element("testsuites")
    for binary, name in cases:
        suite = ET.SubElement(root, "testsuite", name=binary)
        case = ET.SubElement(suite, "testcase", name=name)
        if failure:
            ET.SubElement(case, failure)
    return ET.tostring(root)


class PartitionAuditTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parts = [Path(self.temporary.name) / name for name in ["first 雪", "second"]]
        self.cases = [[("library", "one"), ("integration", "two")],
                      [("library", "two & 雪"), ("integration", "one")]]
        for index in range(2):
            self.write(index)

    def write(self, index, data=None, cases=None, failure=None, source="a" * 40):
        root = self.parts[index]
        (root / "nextest/ci").mkdir(parents=True, exist_ok=True)
        (root / "native-test-manifest.json").write_text(json.dumps(manifest() if data is None else data))
        (root / "nextest/ci/junit.xml").write_bytes(
            junit(self.cases[index] if cases is None else cases, failure))
        (root / "native-test-source.txt").write_text(source + "\n")

    def rejects(self):
        with self.assertRaises(ValueError):
            audit.audit_partitions(self.parts, "a" * 40)

    def test_full_disjoint_union_and_checked_source(self):
        result = audit.audit_partitions(self.parts, "a" * 40)
        self.assertEqual(result["executed"], 4)
        self.assertEqual(result["listed"], 5)
        self.assertEqual(result["ignored"], 1)
        self.assertEqual(result["partitions"], [2, 2])
        self.assertEqual(result["sourceCommit"], "a" * 40)
        self.assertEqual(len(result["manifestSha256"]), 64)

    def test_missing_case(self):
        self.write(1, cases=self.cases[1][1:])
        self.rejects()

    def test_duplicate_case_between_partitions(self):
        self.write(1, cases=self.cases[1] + self.cases[0][:1])
        self.rejects()

    def test_duplicate_case_within_partition(self):
        self.write(0, cases=self.cases[0] * 2)
        self.rejects()

    def test_ignored_or_unknown_case_cannot_replace_enabled_case(self):
        for replacement in [("library", "ignored"), ("library", "unknown"), ("other", "one")]:
            with self.subTest(replacement=replacement):
                self.write(0, cases=[replacement] + self.cases[0][1:])
                self.rejects()

    def test_failure_error_and_skip_are_not_success(self):
        for failure in ["failure", "error", "skipped"]:
            with self.subTest(failure=failure):
                self.write(0, failure=failure)
                self.rejects()

    def test_manifests_must_match_including_ignore_status(self):
        data = manifest()
        data["rust-suites"]["library"]["testcases"]["one"]["ignored"] = True
        self.write(1, data=data)
        self.rejects()

    def test_unlisted_binary_or_filtered_cases_cannot_prove_full_coverage(self):
        for filtered in [False, True]:
            data = manifest()
            if filtered:
                data["rust-suites"]["library"]["testcases"]["one"]["filter-match"]["status"] = "mismatch"
            else:
                data["rust-suites"]["library"]["status"] = "skipped"
            with self.subTest(filtered=filtered):
                self.write(0, data=data)
                self.write(1, data=data)
                self.rejects()

    def test_manifest_count_and_binary_identity_are_checked(self):
        for field, value in [("test-count", 4), ("binary-id", "other")]:
            data = manifest()
            if field == "test-count":
                data[field] = value
            else:
                data["rust-suites"]["library"][field] = value
            with self.subTest(field=field):
                self.write(0, data=data)
                self.rejects()

    def test_wrong_source_or_missing_source_record(self):
        self.write(1, source="b" * 40)
        self.rejects()
        (self.parts[1] / "native-test-source.txt").unlink()
        self.rejects()

    def test_duplicate_manifest_keys_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            audit.manifest_cases('{"test-count": 1, "test-count": 2}')

    def test_incomplete_or_malformed_manifest_is_not_a_full_test_list(self):
        for raw in ["[]", "{}", '{"test-count": 0, "rust-suites": {}}']:
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                audit.manifest_cases(raw)

    def test_duplicate_artifact_files_are_rejected(self):
        (self.parts[0] / "extra").mkdir()
        (self.parts[0] / "extra/junit.xml").write_bytes(junit(self.cases[0]))
        self.rejects()


if __name__ == "__main__":
    unittest.main()
