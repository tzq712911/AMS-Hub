"""Interactive desktop harness for the actual firmware Slint components.

This program does not import a network client, discover printers or send commands.
Run with --self-test for mock state/ callback checks without opening windows.
"""
from pathlib import Path
from datetime import timedelta
import argparse
import json
import csv
import copy
import slint

ROOT = Path(__file__).resolve().parents[1]
def field(value,name):
    return value[name] if isinstance(value,dict) else getattr(value,name)


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
        self.armed = False
        self.library_path = None if immediate else ROOT / "preview/local-rfid-library.json"
        self.library = json.loads(self.library_path.read_text(encoding="utf-8")) if self.library_path and self.library_path.exists() else {}
        d = self.device
        self.catalog=d.get_property("demo-presets")
        d.set_global_callback("AppBackend", "get-console-presets", lambda: d.get_property("demo-presets"))
        d.set_global_callback("AppBackend", "get-console-materials", lambda: slint.ListModel([r[0] for r in list(csv.reader((ROOT/'core/data/materials.csv').open(encoding='utf-8')))[1:] if r]))
        d.set_global_callback("AppBackend", "get-console-generic", self.generic_for)
        d.set_global_callback("AppBackend", "console-text", lambda text: text)
        d.set_global_callback("AppBackend", "get-console-mapping", lambda code,material,brand,series,color: '机器：Generic '+material+' · 演示')
        d.set_global_callback("AppBackend", "get-console-brands", lambda: slint.ListModel(["Bambu Lab","Generic","eSUN","Polymaker","Overture","SUNLU","Snapmaker","Anycubic"]))
        d.set_global_callback("AppBackend", "get-console-presets-for-brand",self.presets_for_brand)
        d.set_global_callback("AppBackend", "get-console-brand",lambda code:field(self.lookup(code),"brand") if self.lookup(code) else "Generic")
        d.set_global_callback("AppBackend", "get-console-colors",self.colors_for)
        d.set_global_callback("AppBackend", "parse-console-color",self.tint)
        d.set_global_callback("AppBackend", "register-console-tag-brand", self.register_tag)
        d.set_global_callback("AppBackend", "get-slot-display", lambda _: d.get_property("demo-slot"))
        d.set_global_callback("AppBackend", "get-console-calibrations", lambda *_: slint.ListModel([]))
        d.set_global_callback("AppBackend", "configure-console-slot", self.bambu_write)
        d.set_global_callback("AppBackend", "configure-console-profile",self.bambu_profile_write)
        d.set_global_callback("AppBackend", "set-staging-to-tray", self.staged_write)
        d.set_global_callback("AppBackend", "clear-staging", lambda: d.invoke_global("AppState", "empty-spool-staging"))
        d.set_global_callback("HubUi", "refresh", self.refresh)
        d.set_global_callback("HubUi", "apply", self.hub_write)
        d.set_global_callback("HubUi", "select-brand", self.change_profile)
        d.set_global_callback("HubUi", "scan", lambda *_: None)
        self.controls.set_callback("change-profile", self.change_profile)
        self.controls.set_callback("scan", self.scan)
        self.controls.set_callback("page", self.page)
        self.controls.set_callback("change-scenario", self.change_scenario)
        self.controls.set_callback("reset", self.reset)
        self.controls.set_callback("insert", self.insert)
        self.controls.set_callback("unknown", self.unknown)
        self.controls.set_callback("multi-ams", self.multi_ams)
        self.reset()

    def lookup(self,code):
        return next((self.catalog.row_data(i) for i in range(self.catalog.row_count()) if field(self.catalog.row_data(i),"code")==code),None)

    def generic_for(self, material):
        result=copy.copy(self.device.get_property('preset-template'))
        for row in list(csv.reader((ROOT/'core/data/materials.csv').open(encoding='utf-8')))[1:]:
            if len(row)>=4 and row[0]==material:
                for key,value in [('brand','Generic'),('code',row[1]),('name','Generic '+material),('material',material),('temp-min',int(row[2])),('temp-max',int(row[3]))]:
                    setattr(result,key.replace('-','_'),value)
                break
        return result

    def presets_for_brand(self,brand):
        return slint.ListModel([self.catalog.row_data(i) for i in range(self.catalog.row_count()) if field(self.catalog.row_data(i),"brand").lower()==brand.lower()])

    @staticmethod
    def tint(code):
        code=code if len(code)==8 else "555555FF"
        return slint.Color({"red":int(code[:2],16),"green":int(code[2:4],16),"blue":int(code[4:6],16),"alpha":255})

    def colors_for(self,code):
        rows=[r for r in csv.reader((ROOT/'core/data/bambu-color-names.csv').open(encoding='utf-8')) if len(r)>=3 and r[0]==code]
        if not rows:rows=[[code,c,n] for n,c in [("White","FFFFFFFF"),("Black","000000FF"),("Red","F01B23FF"),("Blue","287FC2FF"),("Green","00CE9BFF"),("Cyan","71CCE1FF"),("Yellow","F5D14BFF"),("Orange","FF8B3DFF"),("Purple","A083CCFF"),("Gray","969696FF"),("Pink","E383BAFF"),("Brown","805838FF")]]
        result=[]
        for _,rgba,name,*rest in rows:
            item=copy.copy(self.device.get_property("color-template"))
            setattr(item,'name',name);setattr(item,'code',rgba);setattr(item,'tint',self.tint(rgba));result.append(item)
        return slint.ListModel(result)

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
        self.armed = False
        self.controls.set_property("profile", self.profile)
        self.device.set_global_property("HubUi", "busy", False)
        self.device.set_global_property("HubUi", "editing", False)
        self.device.set_global_property("HubUi", "settings-open", False)
        self.device.set_property("hub-mode", False)
        self.device.set_global_property("HubUi", "active-brand", self.profile)
        self.device.set_global_property("HubUi", "visible", self.profile > 0)
        self.device.invoke("home")
        if self.profile:
            self.device.invoke("load-hub-demo", self.profile, self.scenario == 4)
            self.device.set_global_property("HubUi", "status", "扫描耗材后等待装料，自动匹配实际槽位")
        self.log("切换机型：" + ["Bambu X2D", "Snapmaker U1", "Anycubic ACE"][self.profile])

    def scan(self, kind):
        self.device.invoke("scan-demo", bool(kind))
        self.armed = True
        self.device.set_global_property("HubUi", "status", "已识别 · 请装入一个空槽，等待自动写入（模拟）")
        self.log("模拟识别标签：" + ("PETG / 71CCE1FF" if kind else "PLA / 00CE9BFF"))

    def insert(self, slot):
        if not self.armed or self.pending:
            self.log("未写入：请先扫描，或等待当前写入结束")
            return
        if self.profile == 1 and self.scenario == 4 and int(slot) == 0:
            self.log("官方 RFID 槽位不可覆盖")
            return
        self.armed = False
        record = field(self.device.get_global_property("AppState", "spool-staging-info"),"spool-record")
        self.log(f"模拟稳定装料事件：槽 {int(slot)+1}，自动匹配目标")
        if self.profile:
            self.hub_write(0,slot,field(record,"material-type"),field(record,"color-code"),field(record,"brand") or "Generic","")
        else:
            self.staged_write(slot)

    def unknown(self):
        self.armed=False
        self.device.set_global_property('HubUi','editing',False)
        uid = "04A1B2C3"
        if uid in self.library:
            r = self.library[uid]
            tint = slint.Color({"red":int(r["rgba"][:2],16),"green":int(r["rgba"][2:4],16),"blue":int(r["rgba"][4:6],16),"alpha":255})
            info=self.lookup(r['preset'])
            series=r.get('series',field(info,'name') if info and field(info,'brand').lower()==r['brand'].lower() else r['material'])
            self.device.invoke("scan-record",uid,r["brand"],r["material"],r["rgba"],tint,r["preset"],int(field(info,'temp-min')) if info else 0,int(field(info,'temp-max')) if info else 0,series)
            self.armed=True
            self.log("本地库命中 RFID："+uid+" / "+r["brand"])
        else:
            self.device.invoke_global("AppState","new-tag-scanned",uid)
            self.device.set_property("preview-mode",0)
            self.log("未知 RFID：请在设备窗口选择耗材、颜色和品牌后保存")

    def multi_ams(self):
        self.change_profile(0)
        self.device.invoke("multi-ams-demo")
        self.log("模拟 4 台 AMS + 2 台 HT，点击设备条切换")

    def register_tag(self,tag,preset,material,color,brand,series=None):
        if not preset or not material:
            return "请选择耗材"
        profile=self.lookup(preset)
        series=series or (field(profile,'name') if profile and field(profile,'brand').lower()==brand.lower() else material)
        self.library[tag]={"preset":preset,"material":material,"rgba":color,"brand":brand,"series":series}
        if self.library_path:
            self.library_path.write_text(json.dumps(self.library,ensure_ascii=False,indent=2),encoding="utf-8")
        self.device.invoke("home")
        self.unknown()
        self.log("本地 UID 对应关系已保存；再次扫描可直接识别")
        return ""

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
        self.bambu_profile_write(tray,field("slicer-filament"),field("material-type"),field("color-code"),-1,field("brand") or "Generic",field("material-subtype"))

    def bambu_profile_write(self,tray,preset,material,color,calibration,brand,series):
        return self.submit({"brand":"bambu","slot":int(tray),"preset":preset,"type":material,"rgba":color,"calibration":int(calibration)},lambda:self.device.invoke('simulate-bambu-profile',int(tray),material,self.tint(color),brand))

    def hub_write(self, box, index, material, color, vendor, subtype):
        if self.pending:
            return "正在等待机器读回，请勿重复提交"
        if self.scenario == 4 and self.profile == 1 and int(index) == 0:
            return "模拟：官方 RFID 槽位禁止覆盖"
        wire_color="010101FF" if self.profile==2 and color[:6]=="000000" else color[:6]+"FF" if self.profile==2 else color
        payload = {"brand": "u1" if self.profile == 1 else "anycubic", "box": int(box), "slot": int(index), "type": material, "rgba": wire_color}
        if self.profile == 1:
            payload.update(vendor="Generic", subtype="")
        self.log("本地实际："+vendor+" / "+(subtype or material)+" / "+color)
        self.device.set_global_property("HubUi", "busy", True)
        display=subtype.removeprefix(vendor+' ').removeprefix('Bambu ') if subtype else material
        info=copy.copy(self.device.get_global_property('HubUi','editor-info'))
        record=copy.copy(field(info,'spool-record'))
        profile=next((self.catalog.row_data(i) for i in range(self.catalog.row_count()) if field(self.catalog.row_data(i),'name')==subtype),None)
        if not profile:
            profile=next((self.catalog.row_data(i) for i in range(self.catalog.row_count()) if field(self.catalog.row_data(i),'brand')=='Generic' and field(self.catalog.row_data(i),'material')==material),None)
        for key,value in {'brand':vendor,'material-type':material,'material-subtype':subtype,'color-code':color,'slicer-filament':field(profile,'code') if profile else ''}.items():setattr(record,key,value)
        for key,value in {'spool-record':record,'slicer-filament-name':display,'color':self.tint(color),'temp-min':field(profile,'temp-min') if profile else 0,'temp-max':field(profile,'temp-max') if profile else 0}.items():setattr(info,key,value)
        machine="机器："+("Generic " if self.profile==1 else "")+material+" #"+wire_color
        if self.scenario!=3:
            detail="未确认 · 回读超时" if self.scenario==1 else "未完全同步 · 机器旧颜色 #FFFFFFFF" if self.scenario==2 else "等待回读 · "+machine
            self.device.invoke('simulate-hub-profile',int(index),info,detail)
        result = self.submit(payload, lambda: self.device.invoke("simulate-hub-profile",int(index),info,"已同步 · "+machine))
        if result:
            self.device.set_global_property("HubUi", "busy", False)
        return result

    def self_test(self):
        self.change_profile(0)
        self.bambu_profile_write(0,'GFA99','PLA','AA00FFFF',-1,'SUNLU','PLA Plus')
        assert field(self.device.get_global_property('AppState','trays-state').row_data(2),'brand')=='SUNLU'
        self.change_profile(1)
        self.hub_write(0,1,"PLA","4DAFDAFF","Bambu Lab","Bambu PLA Lite")
        assert self.writes[-1]['vendor']=='Generic'
        local=self.device.get_global_property('HubUi','slots').row_data(1)
        assert field(local,'brand')=='Bambu Lab' and field(local,'material')=='PLA Lite'
        assert field(local,'rgba')=='4DAFDAFF'
        tint=field(local,'tint');assert getattr(tint,'color',tint).blue==218
        self.change_profile(2)
        self.hub_write(0,1,"PLA","000000FF","SUNLU","PLA Plus")
        assert self.writes[-1]['rgba']=='010101FF' and 'vendor' not in self.writes[-1]
        assert field(self.device.get_global_property('HubUi','slots').row_data(1),'rgba')=='000000FF'
        for profile in (0,1,2):
            self.change_profile(profile)
            self.scan(1)
            self.insert(1)
            assert self.writes[-1]["slot"] == 1 and self.writes[-1]["type"] == "PETG"
        self.scan(0)
        self.device.set_global_property('HubUi','editing',True)
        self.unknown()
        assert not self.armed, "Unknown tag must cancel the previous pending identity"
        assert not self.device.get_global_property('HubUi','editing'), "RFID registration must replace an old slot editor"
        assert self.register_tag("04A1B2C3","GFA00","PLA","00CE9BFF","SUNLU") == ""
        assert self.library["04A1B2C3"]["brand"] == "SUNLU"
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
        partial=self.device.get_global_property('HubUi','slots').row_data(0)
        assert field(partial,'rgba')=='00CE9BFF' and '未完全同步' in field(partial,'machine-detail')
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
