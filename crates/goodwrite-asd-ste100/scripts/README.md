# ASD-STE100 Dictionary Generation

This directory contains the script used to parse and generate the canonical `dictionary.toml` from the official ASD-STE100 standard PDF (Part 2).

## Prerequisites

The extraction script relies on the `pdftotext` Python package, which uses sophisticated layout-preservation algorithms necessary for parsing the tightly constrained STE PDF tables.

**CRITICAL:** `pdftotext` requires the `poppler` C++ library headers to be installed on your system *before* you can install the Python environment.

**macOS:**
```bash
brew install poppler
```

**Ubuntu/Debian:**
```bash
sudo apt-get install build-essential libpoppler-cpp-dev pkg-config python3-dev
```

## Usage

It is recommended to use [uv](https://github.com/astral-sh/uv) to manage the Python environment:

```bash
# Create a virtual environment and install requirements
uv venv
source .venv/bin/activate
uv pip install -r crates/goodwrite-asd-ste100/scripts/requirements.txt

# Run the script, pointing it to your copy of the ASD-STE100 PDF
python3 crates/goodwrite-asd-ste100/scripts/regenerate_dictionary.py --pdf path/to/ASD-STE100_ISSUE9.pdf
```

The script will extract the dictionary pages, safely recover any multi-line or overflowing columns, and output a freshly updated TOML file to `crates/goodwrite-asd-ste100/data/dictionary.toml`.
