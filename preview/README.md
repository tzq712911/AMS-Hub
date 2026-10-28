# 交互预览

`scripts/simulate_ui.py` 直接加载 `core/ui/console-preview.slint`，其首页、编辑器、设置、多品牌槽位页均来自固件使用的组件。`preview/simulator-controls.slint` 仅提供桌面测试操作区。

运行根目录 `启动交互模拟器.cmd`，或执行 `scripts/simulate-ui.ps1`。

1. 在模拟控制台选择机型并扫描 PLA / PETG。
2. 拓竹：点击设备槽位，修改预设/颜色，确认。
3. U1 / ACE：读取状态，选择槽位，确认写入。
4. 切换超时、不一致、离线、RFID 锁定场景，观察状态及按钮禁用。
5. 点击“重置演示”清除状态。

模拟器没有联网能力，不测试真实认证、MQTT、机器屏幕或切片软件。Bambu 的 K 选择界面使用真实组件，但模拟数据不提供真实校准记录。

`preview/src/main.rs` 是离屏渲染及坐标点击检查工具，会输出截图并验证材质、颜色、参数提交和重复点击保护。
