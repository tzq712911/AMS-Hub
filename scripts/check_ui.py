"""Compile the actual application UI with matching Slint 1.11 Python bindings."""
from pathlib import Path
import sys
import slint

root = Path(__file__).resolve().parents[1]
compiler = slint.native.Compiler()
compiler.style = "fluent-dark"
result = compiler.build_from_path(root / "core/ui/appwindow.slint")
errors = [d for d in result.diagnostics if d.level == slint.native.DiagnosticLevel.Error]
for diagnostic in result.diagnostics:
    print(diagnostic, file=sys.stderr)
if errors:
    sys.exit(1)
print("PASS: AppWindow and all imported UI components compiled with Slint 1.11")
