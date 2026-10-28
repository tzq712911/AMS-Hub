"""Apply reviewed exact UI string translations; safe to rerun after upstream sync."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
translations = json.loads((root / "core/ui/zh-CN.json").read_text(encoding="utf-8"))
changes = 0
for path in (root / "core/ui").rglob("*.slint"):
    original = path.read_text(encoding="utf-8")
    updated = original
    for source, translated in translations.items():
        # The catalogue escapes Slint's interpolation/newline backslashes.
        source = source.replace("\\\\", "\\")
        translated = translated.replace("\\\\", "\\")
        updated = updated.replace('"' + source + '"', '"' + translated + '"')
    if updated != original:
        path.write_text(updated, encoding="utf-8", newline="\n")
        changes += 1
print(f"Localized {changes} UI files")
