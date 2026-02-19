#!/usr/bin/env python3
"""Regenerate canonical rule index skeleton from currently registered rules."""

from __future__ import annotations

import json
import pathlib
import subprocess
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
RULE_INDEX = ROOT / "crates" / "goodwrite-core" / "data" / "rule_index.toml"

DEFAULTS = {
    "standard": "TBD",
    "part": "TBD",
    "section_number": "TBD",
    "section_name": "TBD",
    "rule_number": "TBD",
    "citation": "TBD",
    "test_pass": "TBD",
    "test_fail": "TBD",
}


def load_registered_rules() -> list[dict[str, object]]:
    completed = subprocess.run(
        ["cargo", "run", "-q", "-p", "goodwrite-cli", "--", "list-rules", "--format", "json"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def load_existing() -> dict[str, dict[str, str]]:
    if not RULE_INDEX.exists():
        return {}
    rules = tomllib.loads(RULE_INDEX.read_text()).get("rules", [])
    return {entry["id"]: entry for entry in rules}


def render(entries: list[dict[str, str]]) -> str:
    lines: list[str] = []
    for entry in entries:
        lines.append("[[rules]]")
        for key in [
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
        ]:
            value = entry[key].replace('"', '\\"')
            lines.append(f'{key} = "{value}"')
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main() -> None:
    registered = load_registered_rules()
    existing = load_existing()
    out: list[dict[str, str]] = []

    for row in registered:
        rule_id = str(row["id"])
        profile_names = row.get("profiles") or []
        profile = str(profile_names[0]) if profile_names else "unknown"
        title = str(row["name"])
        merged = {
            "id": rule_id,
            "profile": profile,
            "title": title,
            **DEFAULTS,
            **{k: str(v) for k, v in existing.get(rule_id, {}).items() if k in DEFAULTS},
        }
        out.append(merged)

    out.sort(key=lambda entry: entry["id"])
    RULE_INDEX.parent.mkdir(parents=True, exist_ok=True)
    RULE_INDEX.write_text(render(out))
    print(f"Wrote {RULE_INDEX} ({len(out)} rules)")


if __name__ == "__main__":
    main()
