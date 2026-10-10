"""Validate the seven-language catalog and regenerate Rust format templates.

Run: python -X utf8 scripts/generate_localized_formats.py
This generates source text only; it does not compile an executable.
"""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1] / "src"
PLACEHOLDER = re.compile(r"\{([^{}]*)\}")

def canonical(template, names=None):
    if names is None:
        names = list(dict.fromkeys(
            match[1].split(":")[0] for match in PLACEHOLDER.finditer(template)
            if match[1].split(":")[0] and not match[1].split(":")[0].isdigit()
        ))
    count = sum(not match[1].split(":")[0] for match in PLACEHOLDER.finditer(template))
    implicit = 0
    def replace(match):
        nonlocal implicit
        name, separator, spec = match[1].partition(":")
        if not name:
            index = implicit
            implicit += 1
        elif name.isdigit():
            index = int(name)
        else:
            index = count + names.index(name)
        return "{" + str(index) + (separator + spec if separator else "") + "}"
    return PLACEHOLDER.sub(replace, template), names

def rust_string(value):
    return json.dumps(value.replace("\\n", "\n"), ensure_ascii=False)

rows = [line.split("\t") for line in (ROOT / "translations.tsv").read_text(encoding="utf-8").splitlines()]
assert all(len(row) == 7 and all(row) for row in rows), "Each entry needs seven nonempty translations"
assert len({row[0] for row in rows}) == len(rows), "Duplicate translation key"
arms = []
for row in rows:
    if not PLACEHOLDER.search(row[0]):
        continue
    key, names = canonical(row[0])
    translations = [canonical(text, names)[0] for text in row]
    expected = sorted(match[1] for match in PLACEHOLDER.finditer(key))
    assert all(sorted(match[1] for match in PLACEHOLDER.finditer(text)) == expected for text in translations), row[0]
    arms.append("    (" + rust_string(key) + " $(, $arg:expr)* $(,)?) => { match $crate::localization::language_index() {\n" +
                "\n".join("        " + str(i) + " => format!(" + rust_string(text) + " $(, $arg)*)," for i, text in enumerate(translations[1:], 1)) +
                "\n        _ => format!(" + rust_string(translations[0]) + " $(, $arg)*),\n    }};")
output = "// Translated format templates; indices preserve argument order.\n#[macro_export]\nmacro_rules! localized_format {\n" + "\n".join(arms) + "\n}\n"
(ROOT / "localized_format.rs").write_text(output, encoding="utf-8")
print(f"Validated {len(rows)} entries in seven languages; generated {len(arms)} format templates.")
