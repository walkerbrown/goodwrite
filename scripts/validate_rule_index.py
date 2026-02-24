#!/usr/bin/env python3
"""Validate canonical rule index metadata for CI and local checks."""

from __future__ import annotations

import pathlib
import re
import sys
import tomllib
import typing


ROOT = pathlib.Path(__file__).resolve().parents[1]
RULE_INDEX = ROOT / "crates" / "goodwrite-core" / "data" / "rule_index.toml"

REQUIRED_FIELDS = [
    "id",
    "profile",
    "title",
    "standard",
    "part",
    "section_number",
    "section_name",
    "rule_number",
    "citation",
    "test_pass",
    "test_fail",
]

ASD_SECTION_EXPECTATIONS = {
    "1": "Words",
    "2": "Multi-word nouns",
    "3": "Verbs",
    "4": "Sentences",
    "5": "Procedural writing",
    "6": "Descriptive writing",
    "7": "Safety instructions",
    "8": "Punctuation and word count",
    "GR": "Grammar recommendations",
}


def fail(message: str) -> typing.NoReturn:
    print(f"rule-index validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def main() -> None:
    if not RULE_INDEX.exists():
        fail(f"missing file: {RULE_INDEX}")

    data = tomllib.loads(RULE_INDEX.read_text())
    rules = data.get("rules", [])
    if not isinstance(rules, list) or not rules:
        fail("rule_index.toml must define non-empty [[rules]] entries")

    seen_ids: set[str] = set()
    for entry in rules:
        for field in REQUIRED_FIELDS:
            value = entry.get(field)
            if not isinstance(value, str) or not value.strip():
                fail(f"entry `{entry.get('id', '<missing>')}` missing field `{field}`")

        rule_id = entry["id"]
        if rule_id in seen_ids:
            fail(f"duplicate rule id: {rule_id}")
        seen_ids.add(rule_id)

        for path_field in ("test_pass", "test_fail"):
            fixture_path = ROOT / entry[path_field]
            if not fixture_path.exists():
                fail(f"{rule_id} -> {path_field} path does not exist: {entry[path_field]}")

        if entry["profile"] == "asd-ste100":
            if not entry["citation"].startswith("ASD-STE100 "):
                fail(f"{rule_id} has malformed ASD citation: {entry['citation']}")

            if entry["rule_number"].startswith("GR-"):
                if entry["section_number"] != "GR":
                    fail(
                        f"{rule_id} GR entry must use section_number `GR`, got "
                        f"`{entry['section_number']}`"
                    )
                if entry["section_name"] != ASD_SECTION_EXPECTATIONS["GR"]:
                    fail(
                        f"{rule_id} GR entry must use section "
                        f"`{ASD_SECTION_EXPECTATIONS['GR']}`, "
                        f"got `{entry['section_name']}`"
                    )
                expected_citation = f"ASD-STE100 {entry['rule_number']}"
                if entry["citation"] != expected_citation:
                    fail(
                        f"{rule_id} citation mismatch: expected `{expected_citation}`, "
                        f"got `{entry['citation']}`"
                    )
            else:
                match = re.match(r"(\d+)\.\d+$", entry["rule_number"])
                if not match:
                    fail(f"{rule_id} has invalid ASD rule_number: {entry['rule_number']}")
                section_key = match.group(1)
                expected = ASD_SECTION_EXPECTATIONS.get(section_key)
                if expected is None:
                    fail(f"{rule_id} maps to unsupported ASD section key: {section_key}")
                if entry["section_number"] != section_key:
                    fail(
                        f"{rule_id} section_number mismatch: expected `{section_key}`, "
                        f"got `{entry['section_number']}`"
                    )
                if entry["section_name"] != expected:
                    fail(
                        f"{rule_id} section mismatch: expected `{expected}`, got `{entry['section_name']}`"
                    )
                expected_citation = f"ASD-STE100 Rule {entry['rule_number']}"
                if entry["citation"] != expected_citation:
                    fail(
                        f"{rule_id} citation mismatch: expected `{expected_citation}`, "
                        f"got `{entry['citation']}`"
                    )

    print(f"Validated rule index entries: {len(rules)}")


if __name__ == "__main__":
    main()
