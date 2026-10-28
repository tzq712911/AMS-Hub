"""Package reproducible sources and licenses, excluding local build caches."""
from pathlib import Path
import zipfile

root = Path(__file__).resolve().parents[1]
output = root / "dist/AMS-Hub-0.6.0-alpha.3-source.zip"
output.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        if relative == Path("preview/local-rfid-library.json") or path.suffix == ".log":
            continue
        if any(part in {"target", "dist", "releases", ".git", "__pycache__"} for part in relative.parts):
            continue
        archive.write(path, Path("AMS-Hub-0.6.0-alpha.3") / relative)
print(output)
