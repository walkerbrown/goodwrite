# CI Scripts

This directory defines the canonical CI entrypoint for local runs and GitHub workflows.

## Script

- `scripts/ci/run.sh`

## Modes

- `core`: build + test + (`fmt`/`clippy` on stable)
- `accountability`: rule-index and linkage gates
- `site`: site data freshness checks
- `smoke`: benchmark smoke + cargo-install smoke
- `full`: runs all modes in sequence

## Local Usage

From repository root:

```bash
./scripts/ci/run.sh full --offline
```

Targeted runs:

```bash
./scripts/ci/run.sh core --channel stable
./scripts/ci/run.sh accountability
./scripts/ci/run.sh site
./scripts/ci/run.sh smoke --offline
```

`--offline` affects install smoke only.

## GitHub Usage

Workflows call this script directly so local and CI behavior stays aligned:

- `.github/workflows/ci.yml`
- `.github/workflows/rule-accountability.yml`
- `.github/workflows/site.yml`
