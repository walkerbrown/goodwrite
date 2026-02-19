# goodwrite Agent Guide

This file provides current development context for coding agents working in this repository.

## Comments

These comment policies apply globally, please use reasonableness in applying them

- Prefer detailed comments and always keep them in sync with code changes.
- Avoid adding self-reflective comments on changes you're currently making.
- Avoid referencing previous implementations or strategies.

<!--Remove after 1.0 release-->
We're before initial release, so please don't add migration advice on breaking changes.

## Scope

`goodwrite` is a Rust CLI tool and language server for linting technical documentation written in Typst and Markdown.

Primary profiles:
- `asd-ste100`: Simplified Technical English checks
- requirement rulesets: EARS is the default built-in ruleset
- `glossary`: project terminology checks

Primary binaries:
- `goodwrite` (CLI)
- `goodwrite-lsp` (language server)

## Repository Map

- `crates/goodwrite-core`: rule interfaces, diagnostics, config, span annotations, canonical rule index loader
- `crates/goodwrite-extract`: Typst/Markdown extraction, annotation parsing, source spans
- `crates/goodwrite-tokenize`: sentence splitting, tokenization, POS helpers, STE word counting
- `crates/goodwrite-asd-ste100`: ASD-STE100 rules + dictionary lookups
- `crates/goodwrite-ears`: EARS requirement rules
- `crates/goodwrite-glossary`: glossary parsing/rules
- `crates/goodwrite-cli`: CLI commands/output
- `crates/goodwrite-lsp`: LSP server and diagnostics mapping
- `templates/`: Typst helpers
- `tests/fixtures/` and `tests/rulecases/`: integration and rule-accountability fixtures
- `site/`: static website and generated rule index JSON

## Annotation Model

`SpanAnnotations` carries source metadata used by profile dispatch.

Supported source-facing annotations:
- writing mode: `goodwrite:mode:<value>`
- requirement block start/end: `goodwrite:requirement` and `goodwrite:requirement:end`
- optional requirement type: `goodwrite:requirement:<type>`

Typst helpers:
- `#requirement[...]`
- `#requirement_<type>[...]`

Source files can mark requirement presence/type, but cannot select requirement rulesets.
Ruleset selection is controlled only via `goodwrite.toml` or CLI args.

Source annotations in the `goodwrite:ears:*` namespace are rejected.

## Rule and Test Contract

For every rule change:
- keep stable rule IDs
- keep diagnostics source-mapped and actionable
- update canonical metadata in `crates/goodwrite-core/data/rule_index.toml`
- keep linked pass/fail fixtures valid (`test_pass` and `test_fail`)

Hard requirement: every indexed rule must have both linked cases and must be exercised in `rule_accountability` CI.

## Canonical Rule Index

Single source of truth:
- `crates/goodwrite-core/data/rule_index.toml`

Validation tools:
- `scripts/validate_rule_index.py`
- `scripts/update_rule_coverage_badge.py`
- `scripts/export_rule_index_json.py`

Website data is generated from this index:
- `site/data/rule_index.json`

## Build and Quality Gates

Run before finalizing changes:

```bash
cargo build --workspace --locked
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/validate_rule_index.py
python3 scripts/update_rule_coverage_badge.py --check
python3 scripts/export_rule_index_json.py --check
cargo test -p goodwrite-cli --test rule_accountability -- --nocapture
```

## Dictionary Workflow

Regeneration script:
- `crates/goodwrite-asd-ste100/scripts/regenerate_dictionary.py`

Usage:

```bash
python3 crates/goodwrite-asd-ste100/scripts/regenerate_dictionary.py \
  --pdf ASD-STE100_ISSUE9.pdf \
  --out crates/goodwrite-asd-ste100/data/dictionary.toml
```

## Do NOT commit the ASD-STE100 standard PDF

Keep ASD standard PDFs local-only and gitignored.
