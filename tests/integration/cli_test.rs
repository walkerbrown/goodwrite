//! Workspace integration test scenarios are exercised from
//! `crates/goodwrite-cli/tests/cli_test.rs`, where Cargo provides the
//! `CARGO_BIN_EXE_goodwrite` test harness for CLI execution.
//!
//! This file intentionally tracks the scenario checklist expected at the
//! workspace level:
//! - `goodwrite check` reports diagnostics
//! - `goodwrite fix --dry-run` emits diffs
//! - config rule overrides are applied
//! - glossary loading and synonym enforcement work
//!
//! Keeping this checklist under `tests/integration/` matches the repository
//! layout documented in AGENTS.md.
