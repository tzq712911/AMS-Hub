"""Interactive desktop harness for the actual firmware Slint components.

This program does not import a network client, discover printers or send commands.
Run with --self-test for mock state/ callback checks without opening windows.
"""
from pathlib import Path
from datetime import timedelta
import argparse
import json
import slint

ROOT = Path(__file__).resolve().parents[1]


def compile_ui(path, component):
    compiler = slint.native.Compiler()
    compiler.style = "fluent-dark"
    result = compiler.build_from_path(path)
    for diagnostic in result.diagnostics:
        if diagnostic.level == slint.native.DiagnosticLevel.Error:
            raise RuntimeError(str(diagnostic))
    return result.component(component).create()


class Simulator:
    def __init__(self, immediate=False):
        self.immediate = immediate
        self.device = compile_ui(ROOT / "core/ui/console-preview.slint", "ConsolePreview")
        self.controls = compile_ui(ROOT / "preview/simulator-controls.slint", "SimulatorControls")
        self.profile = 0
        self.scenario = 0
        self.pending = False
        self.generation = 0
        self.writes = []
        self.lines = []
        self.timers = []
        d = self.device
        d.set_global_callback("AppBackend", "get-console-presets", lambda: d.get_property("demo-presets"))
        d.set_global_callback("AppBackend", "get-slot-display", lambda _: d.get_property("demo-slot"))
        d.set_global_callback("AppBackend", "get-console-calibrations", lambda *_: slint.ListModel([]))
        d.set_global_callback("AppBackend", "configure-console-slot", self.bambu_write)
        d.set_global_callback("AppBackend", "set-staging-to-tray", self.staged_write)
        d.set_global_callback("AppBackend", "clear-staging", lambda: d.invoke_global("AppState", "empty-spool-staging"))
        d.set_global_callback("HubUi", "refresh", self.refresh)
        d.set_global_callback("HubUi", "apply", self.hub_write)
        self.controls.set_callback("change-profile", self.change_profile)
        self.controls.set_callback("scan", self.scan)
        self.controls.set_callback("page", self.page)
        self.controls.set_callback("change-scenario", self.change_scenario)
        self.controls.set_callback("reset", self.reset)
        self.reset()

    def log(self, text):
        self.lines.append(text)
        self.lines = self.lines[-18:]
        self.controls.set_property("log", "\n".join(self.lines))
        print(text, flush=True)

    def later(self, action):
        generation = self.generation
        def guarded():
            if generation == self.generation:
                action()
        if self.immediate:
            guarded()
        else:
            timer = slint.Timer()
            self.timers.append(timer)
            timer.start(slint.TimerMode.SingleShot, timedelta(milliseconds=1100), guarded)

    def reset(self):
        self.generation += 1
        for timer in self.timers:
            timer.stop()
        self.timers.clear()
        self.pending = False
        self.writes.clear()
        self.lines.clear()
        self.controls.set_property("profile", self.profile)
        self.device.set_global_property("HubUi", "busy", False)
        self.device.invoke("home")
        self.change_profile(self.profile)
        self.scan(0)
        self.log("模拟已重置；未建立任何真实打印机连接。")

    def change_profile(self, profile):
        self.generation += 1
        self.pending = False
        self.profile = int(profile)
        self.device.set_global_property("HubUi", "busy", False)
        self.device.set_property("hub-mode", self.profile > 0)
        self.device.set_global_property("HubUi", "visible", self.profile > 0)
        self.device.invoke("home")
        if self.profile:
            self.device.invoke("load-hub-demo", self.profile, self.scenario == 4)
            self.device.set_global_property("HubUi", "status", "演示状态：请点击读取，选择目标槽位")
        self.log("切换机型：" + ["Bambu X2D", "Snapmaker U1", "Anycubic ACE"][self.profile])

    def scan(self, kind):
        self.device.invoke("scan-demo", bool(kind))
        self.log("模拟识别标签：" + ("PETG / 71CCE1FF" if kind else "PLA / 00CE9BFF"))

    def page(self, mode):
        self.device.invoke("home")
        if self.profile and mode == 1:
            self.log("U1 / ACE 在槽位页面设置；品牌、温度、K 不作为 ACE 写入字段。")
            return
        self.device.set_property("preview-mode", int(mode))

    def change_scenario(self, scenario):
        self.scenario = int(scenario)
        self.change_profile(self.profile)
        self.log("场景：" + ["读回成功", "读回超时", "读回不一致", "连接断开", "U1 官方 RFID 锁定"][self.scenario])

    def refresh(self):
        if self.pending:
            return
        self.device.set_global_property("HubUi", "busy", True)
        self.device.set_global_property("HubUi", "status", "模拟连接，等待真实状态格式…")
        def finish():
            self.device.set_global_property("HubUi", "busy", False)
            if self.scenario == 3:
                self.device.set_global_property("HubUi", "status", "模拟：连接断开，未获得槽位状态")
            else:
                self.device.invoke("load-hub-demo", self.profile, self.scenario == 4)
                self.device.set_global_property("HubUi", "status", "模拟：已读取 4 个槽位")
        self.later(finish)

    def submit(self, payload, finish):
        if self.pending:
            return "正在等待机器读回，请勿重复提交"
        if self.scenario == 3:
            self.log("拒绝提交：模拟连接断开")
            return "模拟：连接断开，未写入"
        self.pending = True
        self.writes.append(payload)
        self.log("发送（模拟）：" + json.dumps(payload, ensure_ascii=False))
        def complete():
            self.pending = False
            self.device.set_global_property("HubUi", "busy", False)
            result = {1: "模拟：命令已发送，但机器读回超时", 2: "模拟：机器读回不一致，不能确认写入"}.get(self.scenario, "模拟：机器读回一致，写入已确认")
            if self.scenario not in (1, 2):
                finish()
            self.device.set_global_property("HubUi", "status", result)
            if not self.profile:
                self.device.invoke("home")
                self.device.invoke("show-message", result)
            self.log(result)
        self.later(complete)
        return ""

    def bambu_write(self, tray, preset, material, color, calibration):
        tint = slint.Color({"red": int(color[0:2], 16), "green": int(color[2:4], 16),
                            "blue": int(color[4:6], 16), "alpha": int(color[6:8], 16)})
        return self.submit({"brand": "bambu", "slot": int(tray), "preset": preset, "type": material, "rgba": color, "calibration": int(calibration)},
                           lambda: self.device.invoke("simulate-slot", int(tray), material, tint))

    def staged_write(self, tray):
        info = self.device.get_global_property("AppState", "spool-staging-info")
        record = info["spool-record"] if isinstance(info, dict) else getattr(info, "spool-record")
        def field(name):
            return record[name] if isinstance(record, dict) else getattr(record, name)
        self.bambu_write(tray, field("slicer-filament"), field("material-type"), field("color-code"), -1)

    def hub_write(self, box, index, material, color, vendor, subtype):
        if self.pending:
            return "正在等待机器读回，请勿重复提交"
        if self.scenario == 4 and self.profile == 1 and int(index) == 0:
            return "模拟：官方 RFID 槽位禁止覆盖"
        payload = {"brand": "u1" if self.profile == 1 else "anycubic", "box": int(box), "slot": int(index), "type": material, "rgba": color}
        if self.profile == 1:
            payload.update(vendor=vendor, subtype=subtype)
        self.device.set_global_property("HubUi", "busy", True)
        result = self.submit(payload, lambda: self.device.invoke("simulate-hub-slot", int(index), material, color))
        if result:
            self.device.set_global_property("HubUi", "busy", False)
        return result

    def self_test(self):
        for profile in (0, 1, 2):
            self.change_profile(profile)
            self.scenario = 0
            if profile:
                assert self.hub_write(0, 1, "PETG", "71CCE1FF", "Generic", "") == ""
            else:
                assert self.bambu_write(0, "GFA00", "PLA", "00CE9BFF", -1) == ""
        self.change_profile(2)
        self.scenario = 1
        self.hub_write(0, 0, "PLA", "00CE9BFF", "Generic", "")
        assert "超时" in self.device.get_global_property("HubUi", "status")
        self.scenario = 2
        self.hub_write(0, 0, "PLA", "00CE9BFF", "Generic", "")
        assert "不一致" in self.device.get_global_property("HubUi", "status")
        self.scenario = 3
        n = len(self.writes)
        assert self.hub_write(0, 0, "PLA", "00CE9BFF", "Generic", "")
        assert len(self.writes) == n
        self.change_profile(1)
        self.scenario = 4
        assert self.hub_write(0, 0, "PLA", "00CE9BFF", "Generic", "")
        self.pending = True
        self.scenario = 0
        assert self.hub_write(0, 1, "PLA", "00CE9BFF", "Generic", "")
        assert len(self.writes) == n
        print("PASS: 3 profiles, success/timeout/mismatch/offline/RFID-lock/duplicate-submit; mock only")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--smoke-test", action="store_true", help="Open windows, exercise delayed callbacks, then close")
    args = parser.parse_args()
    app = Simulator(immediate=args.self_test)
    if args.self_test:
        app.self_test()
    else:
        app.device.show()
        app.controls.show()
        errors = []
        if args.smoke_test:
            def start():
                app.controls.invoke("change-profile", 2)
                app.controls.invoke("scan", 1)
                app.device.invoke_global("HubUi", "refresh")
            def write():
                app.device.invoke_global("HubUi", "apply", 0, 1, "PETG", "71CCE1FF", "Generic", "")
            def verify():
                if "写入已确认" not in app.device.get_global_property("HubUi", "status") or app.pending:
                    errors.append("delayed readback failed")
                else:
                    print("PASS: visible windows, control callbacks, async readback, busy released", flush=True)
                slint.native.quit_event_loop()
            for ms, callback in [(100, start), (1500, write), (2900, verify)]:
                timer = slint.Timer()
                app.timers.append(timer)
                timer.start(slint.TimerMode.SingleShot, timedelta(milliseconds=ms), callback)
        slint.native.run_event_loop()
        if errors:
            raise SystemExit("; ".join(errors))
