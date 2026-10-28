# 交互预览

`scripts/simulate_ui.py` 直接加载 `core/ui/console-preview.slint`，其首页、编辑器、设置、多品牌槽位页均来自固件使用的组件。`preview/simulator-controls.slint` 仅提供桌面测试操作区。

运行根目录 `启动交互模拟器.cmd`，或执行 `scripts/simulate-ui.ps1`。

1. 在模拟控制台选择机型并扫描 PLA / PETG。
2. 拓竹：点击设备槽位，修改预设/颜色，确认。
3. 主流程：扫描后点击“装入槽 1–4”，自动匹配模拟装料事件；U1 / ACE 仍可手选槽位。
4. 切换超时、不一致、离线、RFID 锁定场景，观察状态及按钮禁用。
5. 点击“重置演示”清除状态。

左上角的品牌 Logo 可直接切换，不必依赖控制台下拉框。“多 AMS + HT”展示四组 AMS、两个 HT 单槽。

“未知 RFID / 再扫描”首次打开真实建档组件，选择预设、颜色、品牌保存后，再点击即可命中对应关系。模拟库保存在 `preview/local-rfid-library.json`，已被 Git 忽略；真实设备使用原 SD 卡数据库。

模拟器没有联网能力，不测试真实认证、MQTT、机器屏幕或切片软件。Bambu 的 K 选择界面使用真实组件，但模拟数据不提供真实校准记录。

`preview/src/main.rs` 是离屏渲染及坐标点击检查工具，会输出截图并验证材质、颜色、参数提交和重复点击保护。
