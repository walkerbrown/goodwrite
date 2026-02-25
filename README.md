# goodwrite

[![Build](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/ci.yml?branch=main&label=build)](https://github.com/walkerbrown/goodwrite/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/ci.yml?branch=main&label=tests)](https://github.com/walkerbrown/goodwrite/actions/workflows/ci.yml)
[![Clippy](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/ci.yml?branch=main&label=clippy)](https://github.com/walkerbrown/goodwrite/actions/workflows/ci.yml)
[![Fmt](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/ci.yml?branch=main&label=fmt)](https://github.com/walkerbrown/goodwrite/actions/workflows/ci.yml)
[![Rule Accountability](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/rule-accountability.yml?branch=main&label=rule-accountability)](https://github.com/walkerbrown/goodwrite/actions/workflows/rule-accountability.yml)
[![Rule Tests](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/walkerbrown/goodwrite/main/.github/badges/rule_coverage.json)](https://raw.githubusercontent.com/walkerbrown/goodwrite/main/.github/badges/rule_coverage.json)
[![Security](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/security.yml?branch=main&label=security)](https://github.com/walkerbrown/goodwrite/actions/workflows/security.yml)
[![Site](https://img.shields.io/github/actions/workflow/status/walkerbrown/goodwrite/site.yml?branch=main&label=site)](https://github.com/walkerbrown/goodwrite/actions/workflows/site.yml)
[![MSRV](https://img.shields.io/badge/msrv-1.85%2B-informational)](https://github.com/walkerbrown/goodwrite)
[![Maturity](https://img.shields.io/badge/maturity-alpha-orange)](https://github.com/walkerbrown/goodwrite)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#License)

[goodwrite](https://goodwrite.dev) is a linter and language server for engineering requirements and technical documentation written in Typst or Markdown.

> "The great enemy of clear language is insincerity. When there is a gap between one's real and one's declared aims, one turns as it were instinctively to long words and exhausted idioms, like a cuttlefish spurting out ink."
>
> "What is above all needed is to let the meaning choose the word, and not the other way about."
>
> "If you simplify your English, you are freed from the worst follies of orthodoxy. You cannot speak any of the necessary dialects, and when you make a stupid remark its stupidity will be obvious, even to yourself."
>
> -George Orwell, ["Politics and the English Language"](https://www.orwellfoundation.com/the-orwell-foundation/orwell/essays-and-other-works/politics-and-the-english-language/) (1946)

> 🚧 Pardon our dust! Work in progress. 🚧

The project offers both a CLI tool and text editor integrations to provide clippy-style guidance and automated enforcement of:
- Simplified Technical English ([ASD-STE100](https://www.asd-ste100.org))
- Requirements grammars ([EARS](https://alistairmavin.com/ears/), by default)
- Project glossary for consistent terms of art
- Rules banning ambiguity and wrongwrite

## Install

```bash
# Install latest release
curl -fsSL https://raw.githubusercontent.com/walkerbrown/goodwrite/main/scripts/install.sh | bash

# Homebrew install coming soon!
# brew install goodwrite
```

## Quick Start

```bash
# Check files, directories, or globs
goodwrite check docs/**/*.md docs/**/*.typ

# Apply machine-applicable fixes
goodwrite fix docs/manual.md
```

Without `goodwrite.toml`, goodwrite enables `asd-ste100`, `ears`, and `glossary`.

## Optional Configuration

```toml
[profiles]
enable = ["asd-ste100", "ears", "glossary"]

[glossary]
path = "glossary.toml"

[unsafe]
ignore = ["docs/legacy/**/*.md"]
```

## Requirement Syntax

```markdown
<!-- goodwrite:requirement -->
When the pilot presses the button, the system shall record the event.
<!-- goodwrite:requirement:end -->

<!-- goodwrite:requirement:event-driven -->
When voltage drops below 22.0 V, the controller shall issue an alert.
<!-- goodwrite:requirement:end -->
```

```typst
#requirement[
When the pilot presses the button,
  the system shall record the event.
]

#requirement_event[
When voltage drops below 22.0 V,
  the controller shall issue an alert.
]
```

## Dictionary

The standard for technical documentation [ASD-STE100](https://www.asd-ste100.org/STE_downloads.html) is available free of charge upon request.

The script shown below generates a baseline `dictionary.toml` from the standards document.

```bash
python3 crates/goodwrite-asd-ste100/scripts/regenerate_dictionary.py \
  --pdf ASD-STE100_ISSUE9.pdf \
  --out crates/goodwrite-asd-ste100/data/dictionary.toml
```

Users may augment this dictionary in compliance with ASD-STE100 by adding their own project and industry specific _Technical Nouns_ and _Technical Verbs_ in `glossary.toml`, following the provided templates.

## License

Copyright 2026 Dylan Walker Brown

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
