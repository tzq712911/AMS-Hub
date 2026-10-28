"""Build the embedded Chinese UI font from the OFL-licensed Noto Sans SC source."""
import argparse
from pathlib import Path
from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

parser = argparse.ArgumentParser()
parser.add_argument("font", type=Path)
parser.add_argument("--output", type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
text = "".join(p.read_text(encoding="utf-8") for directory in ("core/ui", "core/src", "shared/src")
               for p in (root / directory).rglob("*") if p.suffix in (".slint", ".rs"))
characters = set(chr(c) for c in range(32, 127))
characters.update(c for c in text if ord(c) >= 160)
font = TTFont(args.font)
if "fvar" in font:
    font = instantiateVariableFont(font, {"wght": 500}, inplace=True)
# The upstream variable font's default family is Thin. Give the subset its own
# family and PostScript name so both desktop and MCU resolve the same font.
for name_id, value in {1: "Console Sans", 2: "Medium", 4: "Console Sans Medium", 6: "ConsoleSans-Medium"}.items():
    for record in font["name"].names:
        if record.nameID == name_id:
            record.string = value.encode(record.getEncoding())
options = subset.Options()
options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]
subsetter = subset.Subsetter(options=options)
subsetter.populate(text="".join(sorted(characters)))
subsetter.subset(font)
output = args.output or root / "core/ui/fonts/ConsoleSans.ttf"
output.parent.mkdir(parents=True, exist_ok=True)
font.save(output)
print(f"UI font: {len(characters)} requested characters, {output.stat().st_size} bytes")
