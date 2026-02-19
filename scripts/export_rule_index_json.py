#!/usr/bin/env python3
"""Export the canonical TOML rule index as deterministic JSON for website usage."""

from __future__ import annotations

import argparse
import json
import pathlib
import tomllib


ROOT = pathlib.Path(__file__).resolve().parents[1]
RULE_INDEX = ROOT / "crates" / "goodwrite-core" / "data" / "rule_index.toml"
SITE_RULES_JSON = ROOT / "site" / "data" / "rule_index.json"


def build_payload() -> dict[str, object]:
    parsed = tomllib.loads(RULE_INDEX.read_text())
    rules = parsed.get("rules", [])
    if not isinstance(rules, list):
        raise SystemExit("invalid rule_index.toml shape: expected `rules` array")

    normalized = sorted(rules, key=lambda rule: (str(rule.get("profile", "")), str(rule.get("id", ""))))
    return {
        "rule_count": len(normalized),
        "rules": normalized,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero when site/data/rule_index.json is stale",
    )
    args = parser.parse_args()

    payload = build_payload()
    rendered = json.dumps(payload, indent=2, sort_keys=True) + "\n"

    if args.check:
        existing = SITE_RULES_JSON.read_text() if SITE_RULES_JSON.exists() else ""
        if existing != rendered:
            raise SystemExit(
                "site/data/rule_index.json is stale; run scripts/export_rule_index_json.py"
            )
        return

    SITE_RULES_JSON.parent.mkdir(parents=True, exist_ok=True)
    SITE_RULES_JSON.write_text(rendered)
    print(f"Wrote {SITE_RULES_JSON}")


if __name__ == "__main__":
    main()
