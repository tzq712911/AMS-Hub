"""Desktop preview of the real Slint components, with explicitly simulated data."""
from pathlib import Path
import argparse
import slint

parser = argparse.ArgumentParser()
parser.add_argument("--editor", action="store_true")
args = parser.parse_args()
compiler = slint.native.Compiler()
compiler.style = "fluent-dark"
result = compiler.build_from_path(Path(__file__).resolve().parents[1] / "core/ui/console-preview.slint")
errors = [d for d in result.diagnostics if d.level == slint.native.DiagnosticLevel.Error]
for diagnostic in result.diagnostics:
    print(diagnostic)
if errors:
    raise SystemExit(1)
window = result.component("ConsolePreview").create()
window.set_global_callback("AppBackend", "get-console-presets", lambda: window.get_property("demo-presets"))
window.set_global_callback("AppBackend", "get-slot-display", lambda _tray: window.get_property("demo-slot"))
window.set_global_callback("AppBackend", "get-console-calibrations", lambda *_: slint.ListModel([]))
window.set_global_callback("AppBackend", "configure-console-slot", lambda *_: "演示模式，不向机器写入")
window.set_global_callback("AppBackend", "set-staging-to-tray", lambda *_: None)
window.set_property("preview-mode", 1 if args.editor else 0)
window.run()
