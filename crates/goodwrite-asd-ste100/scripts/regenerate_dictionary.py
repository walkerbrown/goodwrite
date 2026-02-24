#!/usr/bin/env python3
"""Generate ASD-STE100 dictionary TOML from Part 2 PDF pages."""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

SCRIPT_DIR = Path(__file__).resolve().parent
CRATE_ROOT = SCRIPT_DIR.parent
REPO_ROOT = CRATE_ROOT.parent.parent
DEFAULT_PDF = REPO_ROOT / "ASD-STE100_ISSUE9.pdf"
DEFAULT_OUT = CRATE_ROOT / "data" / "dictionary.toml"

HEADER_PATTERNS = [
    "ASD-STE100 Simplified Technical English",
    "ASD STE100 Simplified Technical English",
    "Part 2 - Dictionary",
    "Issue 9",
    "2025-01-15",
]

VALID_POS = {"n", "v", "adj", "adv", "prep", "conj", "pron", "art", "tn", "tv"}
POS_MAP = {
    "n": "noun",
    "v": "verb",
    "adj": "adjective",
    "adv": "adverb",
    "prep": "preposition",
    "conj": "conjunction",
    "pron": "pronoun",
    "art": "determiner",
    "tn": "technical-noun",
    "tv": "technical-verb",
}


@dataclass
class Entry:
    c1_lines: List[str] = field(default_factory=list)
    c2_lines: List[str] = field(default_factory=list)
    c3_lines: List[str] = field(default_factory=list)
    c4_lines: List[str] = field(default_factory=list)


@dataclass
class ParsedEntry:
    word_display: str
    word: str
    pos: str
    forms: List[str]
    approved: bool
    c2: str
    c3: str
    c4: str
    alternatives: List[Tuple[str, Optional[str]]]


class TableParser:
    def __init__(self) -> None:
        self.col2 = 40
        self.col3 = 68
        self.col4 = 97

    def _update_columns(self, line: str) -> None:
        if "(part of speech)" not in line or "ALTERNATIVES" not in line or "STE EXAMPLE" not in line:
            return
        p2 = line.find("ALTERNATIVES")
        p3 = line.find("STE EXAMPLE")
        p4 = line.find("Non-STE example")
        if p2 >= 0 and p3 >= 0 and p4 >= 0:
            self.col2 = p2
            self.col3 = p3
            self.col4 = p4

    def split_line(self, line: str) -> tuple[str, str, str, str]:
        self._update_columns(line)
        line = line.ljust(self.col4 + 10)
        
        col2 = self.col2
        col3 = self.col3
        col4 = self.col4

        if line[col2 - 1] != " " or line[col2] != " ":
            m = re.search(r"\)\s+(?=[A-Z])", line[:col3])
            if m:
                col2 = m.end()
            else:
                m = re.search(r"\s{2,}", line[:col3])
                if m and m.end() < col3:
                    col2 = m.end()
                else:
                    m = re.search(r"(?<=\S)\s+(?=[A-Z][a-z])", line[:col3])
                    if m:
                        col2 = m.end()

        c1 = line[:col2].strip()
        c2 = line[col2:col3].strip()
        c3 = line[col3:col4].strip()
        c4 = line[col4:].strip()

        # If a list marker leaked into column 1, move it back to column 2.
        leaked = re.match(r"^(.*\([^)]+\),?)\s+(\d+)$", c1)
        if leaked:
            c1 = leaked.group(1).strip()
            c2 = f"{leaked.group(2)} {c2}".strip()

        # Recover STE text if it leaked into column 2.
        c2_to_c3 = re.search(r"\s{2,}([A-Z0-9][A-Z0-9 ,+\-/.]+)$", c2)
        if c2_to_c3:
            lhs = c2[: c2_to_c3.start()].strip()
            rhs = c2_to_c3.group(1).strip()
            if lhs:
                c2 = lhs
                c3 = f"{rhs} {c3}".strip()

        # Recover non-STE text if it leaked into column 3.
        c3_to_c4 = re.search(r"\s{2,}([A-Z]?[a-z][A-Za-z0-9 ,+\-/.]+)$", c3)
        if c3_to_c4:
            lhs = c3[: c3_to_c4.start()].strip()
            rhs = c3_to_c4.group(1).strip()
            if lhs:
                c3 = lhs
                c4 = f"{rhs} {c4}".strip()

        return c1, c2, c3, c4


def extract_text(pdf_path: Path, first_page: int, last_page: int) -> str:
    try:
        import pdftotext  # type: ignore
    except ImportError:
        print("ERROR: Missing required Python package 'pdftotext'.")
        print("Please install it by running: pip install pdftotext")
        sys.exit(1)

    with open(pdf_path, "rb") as f:
        pdf = pdftotext.PDF(f, physical=True)

    text = []
    # pdftotext is 0-indexed, whereas PDF pages are 1-indexed
    for page_idx in range(first_page - 1, last_page):
        if page_idx < len(pdf):
            text.append(pdf[page_idx])

    return "\n".join(text)


def is_header_or_footer(line: str) -> bool:
    s = line.strip()
    if not s:
        return True
    if s.startswith("Page 2-"):
        return True
    if s.startswith("Word") or s.startswith("(part of speech)"):
        return True
    if "Word" in s and "Approved meaning/" in s:
        return True
    return any(pat in s for pat in HEADER_PATTERNS)


def normalize_ws(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def header_has_pos(text: str) -> bool:
    return any(m.group(1).strip().lower() in VALID_POS for m in re.finditer(r"\(([^)]+)\)", text))


def extract_entries(lines: List[str]) -> List[Entry]:
    parser = TableParser()
    entries: List[Entry] = []
    cur: Optional[Entry] = None
    expect_c1_continuation = False

    for raw in lines:
        c1, c2, c3, c4 = parser.split_line(raw)

        if is_header_or_footer(raw):
            continue

        if c1:
            is_c1_pos_only = bool(re.fullmatch(r"\([^)]*\),?", c1))
            is_c1_form_like = bool(re.fullmatch(r"[A-Za-z0-9'\-/]+,?", c1))
            is_likely_continuation = (
                bool(re.fullmatch(r"[A-Z][A-Z0-9 ,\-/()]+", c1)) and not header_has_pos(c1)
            )

            if cur is None:
                cur = Entry()
                cur.c1_lines.append(c1)
                expect_c1_continuation = c1.endswith(",") or not header_has_pos(" ".join(cur.c1_lines))
            else:
                if expect_c1_continuation and (
                    is_c1_pos_only or is_c1_form_like or is_likely_continuation
                ):
                    cur.c1_lines.append(c1)
                    expect_c1_continuation = c1.endswith(",") or not header_has_pos(
                        " ".join(cur.c1_lines)
                    )
                else:
                    entries.append(cur)
                    cur = Entry()
                    cur.c1_lines.append(c1)
                    expect_c1_continuation = c1.endswith(",") or not header_has_pos(
                        " ".join(cur.c1_lines)
                    )

        if cur is None:
            continue

        if c2:
            cur.c2_lines.append(c2)
        if c3:
            cur.c3_lines.append(c3)
        if c4:
            cur.c4_lines.append(c4)

    if cur is not None:
        entries.append(cur)

    return entries


def parse_header(c1_lines: List[str]) -> tuple[Optional[str], Optional[str], List[str]]:
    header = normalize_ws(" ".join(c1_lines))
    matches = list(re.finditer(r"\(([^)]+)\)", header))
    if not matches:
        return None, None, []

    pos_match = None
    for m in reversed(matches):
        candidate = m.group(1).strip().lower()
        if candidate in VALID_POS:
            pos_match = m
            break
    if pos_match is None:
        for m in matches:
            candidate = m.group(1).strip().lower()
            if candidate in VALID_POS:
                pos_match = m
                break
    if pos_match is None:
        return None, None, []

    pos_raw = pos_match.group(1).strip().lower()
    word = header[: pos_match.start()].strip(" ,")
    if not word:
        return None, None, []

    forms: List[str] = []
    tail = header[pos_match.end() :].strip(" ,")
    tail = re.sub(r"\(also[^)]*\)", "", tail, flags=re.IGNORECASE)
    if tail:
        for token in tail.split(","):
            form = token.strip(" ,")
            if form and re.fullmatch(r"[A-Za-z0-9'\-/ ]+", form):
                forms.append(form)

    if len(c1_lines) > 1:
        for line in c1_lines[1:]:
            line = normalize_ws(line).strip(" ,")
            if not line:
                continue
            if line.startswith("(") and line.endswith(")"):
                continue
            if re.fullmatch(r"[A-Za-z0-9'\-/ ]+", line):
                forms.append(line)

    dedup_forms: List[str] = []
    seen = set()
    for form in forms:
        lower = form.lower()
        if lower in seen or lower == word.lower():
            continue
        seen.add(lower)
        dedup_forms.append(lower)

    return word, pos_raw, dedup_forms


ALT_WITH_POS_RE = re.compile(r"([A-Z][A-Z0-9+\-/\. ]*?)\s*\(([^)]+)\)")


def extract_alternatives(c2_lines: List[str]) -> List[Tuple[str, Optional[str]]]:
    found: List[Tuple[str, Optional[str]]] = []

    for line in c2_lines:
        for m in ALT_WITH_POS_RE.finditer(line):
            word = normalize_ws(m.group(1).strip(" ,")).lower()
            pos_raw = m.group(2).strip().lower()
            pos = POS_MAP.get(pos_raw, pos_raw)
            if word:
                found.append((word, pos))

        stripped = normalize_ws(line.strip(" ,"))
        if stripped and re.search(r"[A-Z]", stripped) and not re.search(r"[a-z]", stripped):
            if "(" not in stripped and ")" not in stripped:
                found.append((stripped.lower(), None))

    dedup: List[Tuple[str, Optional[str]]] = []
    seen = set()
    for item in found:
        if item in seen:
            continue
        seen.add(item)
        dedup.append(item)
    return dedup


def classify_approved(word_display: str) -> bool:
    alpha = "".join(ch for ch in word_display if ch.isalpha())
    if not alpha:
        return False
    return alpha.upper() == alpha


def to_parsed(entry: Entry) -> Optional[ParsedEntry]:
    word_display, pos_raw, forms = parse_header(entry.c1_lines)
    if word_display is None or pos_raw is None:
        return None

    if word_display.startswith("("):
        return None

    word = normalize_ws(word_display).lower()
    if not re.search(r"[A-Za-z]", word):
        return None

    if len(word) < 2 and word not in {"a", "i"}:
        return None

    pos = POS_MAP.get(pos_raw, pos_raw)
    approved = classify_approved(word_display)

    c2 = normalize_ws(" ".join(entry.c2_lines))
    c3 = normalize_ws(" ".join(entry.c3_lines))
    c4 = normalize_ws(" ".join(entry.c4_lines))
    alternatives = extract_alternatives(entry.c2_lines)

    return ParsedEntry(
        word_display=word_display,
        word=word,
        pos=pos,
        forms=forms,
        approved=approved,
        c2=c2,
        c3=c3,
        c4=c4,
        alternatives=alternatives,
    )


GOODWRITE_WORD_RE = re.compile(r"[A-Za-z][A-Za-z'/-]*")


def normalize_goodwrite_example(text: str) -> str:
    value = normalize_ws(text)
    if not value:
        return ""

    letters = [ch for ch in value if ch.isalpha()]
    if not letters:
        return value

    upper_ratio = sum(1 for ch in letters if ch.isupper()) / len(letters)
    if upper_ratio < 0.65:
        return value

    def lower_screaming_word(match: re.Match[str]) -> str:
        token = match.group(0)
        token_letters = [ch for ch in token if ch.isalpha()]
        if not token_letters:
            return token
        if not all(ch.isupper() for ch in token_letters):
            return token
        if any(ch.isdigit() for ch in token):
            return token
        return token.lower()

    normalized = GOODWRITE_WORD_RE.sub(lower_screaming_word, value)

    chars = list(normalized)
    cap_next = True
    for index, ch in enumerate(chars):
        if cap_next and ch.isalpha():
            chars[index] = ch.upper()
            cap_next = False
        if ch in ".!?":
            cap_next = True

    return "".join(chars)


def build_entries(text: str) -> List[ParsedEntry]:
    parsed: List[ParsedEntry] = []
    for raw in extract_entries(text.splitlines()):
        item = to_parsed(raw)
        if item is None:
            continue
        parsed.append(item)

    merged: Dict[Tuple[str, str, bool], ParsedEntry] = {}
    order: List[Tuple[str, str, bool]] = []

    for item in parsed:
        key = (item.word, item.pos, item.approved)
        existing = merged.get(key)
        if existing is None:
            merged[key] = item
            order.append(key)
            continue

        existing.forms = sorted(set(existing.forms) | set(item.forms))
        if not existing.c2 and item.c2:
            existing.c2 = item.c2
        if not existing.c3 and item.c3:
            existing.c3 = item.c3
        if not existing.c4 and item.c4:
            existing.c4 = item.c4

        alt_seen = set(existing.alternatives)
        for alt in item.alternatives:
            if alt in alt_seen:
                continue
            existing.alternatives.append(alt)
            alt_seen.add(alt)

    return [merged[key] for key in order]


def toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=True)


def write_dictionary(path: Path, entries: List[ParsedEntry]) -> None:
    approved = [entry for entry in entries if entry.approved]
    not_approved = [entry for entry in entries if not entry.approved]

    lines: List[str] = []
    lines.append('notice = "ASD-STE100 standard is available free of charge at https://asd-ste100.org"')
    lines.append("")

    for entry in approved:
        lines.append("[[approved]]")
        lines.append(f"word = {toml_string(entry.word)}")
        lines.append(f"pos = {toml_string(entry.pos)}")
        if entry.forms:
            forms = ", ".join(toml_string(form) for form in entry.forms)
            lines.append(f"forms = [{forms}]")
        lines.append(f"approved_meaning = {toml_string(entry.c2)}")
        lines.append(f"goodwrite_example = {toml_string(normalize_goodwrite_example(entry.c3))}")
        lines.append(f"wrongwrite_example = {toml_string(entry.c4)}")
        lines.append("")

    for entry in not_approved:
        lines.append("[[not_approved]]")
        lines.append(f"word = {toml_string(entry.word)}")
        lines.append(f"pos = {toml_string(entry.pos)}")

        if entry.alternatives:
            serialized = []
            for word, pos in entry.alternatives:
                if pos:
                    serialized.append(
                        "{ word = "
                        + toml_string(word)
                        + ", pos = "
                        + toml_string(pos)
                        + " }"
                    )
                else:
                    serialized.append("{ word = " + toml_string(word) + " }")
            lines.append("alternatives = [" + ", ".join(serialized) + "]")

        lines.append(f"approved_meaning = {toml_string(entry.c2)}")
        lines.append(f"goodwrite_example = {toml_string(normalize_goodwrite_example(entry.c3))}")
        lines.append(f"wrongwrite_example = {toml_string(entry.c4)}")
        lines.append("")

    path.write_text("\n".join(lines) + "\n")


def validate_entries(entries: List[ParsedEntry]) -> None:
    if not entries:
        raise ValueError("no dictionary entries were parsed")

    approved = [entry for entry in entries if entry.approved]
    not_approved = [entry for entry in entries if not entry.approved]
    if len(approved) < 800:
        raise ValueError(f"approved entry count too low: {len(approved)}")
    if len(not_approved) < 1200:
        raise ValueError(f"not-approved entry count too low: {len(not_approved)}")

    seen = set()
    for entry in entries:
        key = (entry.word, entry.pos, entry.approved)
        if key in seen:
            raise ValueError(f"duplicate entry detected for {key}")
        seen.add(key)

        if not entry.word.strip():
            raise ValueError("entry with empty word field found")
        if not entry.pos.strip():
            raise ValueError(f"entry `{entry.word}` has empty part-of-speech")
        if not entry.approved and not entry.alternatives and not entry.c2.strip():
            raise ValueError(f"not-approved entry `{entry.word}` has no alternatives and no guidance")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--pdf",
        type=Path,
        default=DEFAULT_PDF,
        help="Path to ASD-STE100 PDF",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help="Output dictionary.toml path",
    )
    parser.add_argument("--first-page", type=int, default=149)
    parser.add_argument("--last-page", type=int, default=434)
    args = parser.parse_args()

    text = extract_text(args.pdf, args.first_page, args.last_page)
    entries = build_entries(text)
    validate_entries(entries)

    write_dictionary(args.out, entries)

    approved = sum(1 for entry in entries if entry.approved)
    not_approved = sum(1 for entry in entries if not entry.approved)
    print(f"generated {len(entries)} entries")
    print(f"approved entries: {approved}")
    print(f"not-approved entries: {not_approved}")


if __name__ == "__main__":
    main()
