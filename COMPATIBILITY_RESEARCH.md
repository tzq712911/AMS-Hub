# Snapmaker U1 与 Anycubic 兼容性调研

调研日期：2026-10-27。仅调研；没有新增这两个品牌的驱动，没有连接、扫描或向任何打印机发送命令。

## 结论

SpoolEase 公开的主机固件仍以 Bambu 协议为核心。不能因为其他机器有 LAN 或管理员模式，就直接复用 Bambu 的命令。

- Snapmaker U1：材质/厂商/颜色后台写入有明确源码依据；官方固件的 Moonraker/G-Code 路径与第三方扩展固件的外部 RFID 路径要分开。
- Anycubic：Kobra 3 V2 + ACE Pro、Kobra X + ACE Pro 2 有第三方协议记录及实际代码，支持通过局域网设置槽位类型/颜色。不能由此推断所有 Anycubic FDM 型号或完整耗材配置都支持。
- K/压力提前量、重启持久化、官方切片软件设备页回显必须分别验证，不能将 MQTT 发送成功或第三方界面更新作为全部成功的依据。
- WT32-SC01 Plus + PN532 可复用为扫描/触屏硬件。ESP32-S3 实现 HTTP/WebSocket/MQTT 在工程上可行，但尚未验证跨品牌认证、内存与并发需求。

## SpoolEase 的 U1 关系

作者在讨论 #114 中最初说明没有直接支持 U1，随后提到对 U1 自定义固件及相关项目的集成尝试。这与 Console 已具备通用 U1 驱动不是同一件事。

作者创建的扩展固件 issue #271 是增加 SpoolEase 标签格式的请求。该 issue 被关闭的原因是维护者要求以 PR 提交，并非合并成功；不能把 closed 状态当成支持已发布。

来源：
- https://github.com/yanshay/SpoolEase/discussions/114
- https://github.com/paxx12-snapmaker-u1/SnapmakerU1-Extended-Firmware/issues/271

## Snapmaker U1

官方基于 Klipper、Moonraker、Fluidd，官方源码 print_task_config.py 定义 SET_PRINT_FILAMENT_CONFIG：

- CONFIG_EXTRUDER：0–3 的物理工具头。
- VENDOR / FILAMENT_TYPE / FILAMENT_SUBTYPE / FILAMENT_COLOR_RGBA 等耗材身份字段。
- 官方 RFID 耗材默认有不可手动编辑判断；具体操作应尊重原机状态，不默认强制覆盖。
- 可通过 Moonraker 的 printer.gcode.script 使用已有命令，不必为了基础材质/颜色编辑先假定要刷第三方固件。

第三方 U1 扩展固件 develop 文档另定义 POST /printer/filament_detect/set，可写通道对应的厂商、类型、颜色、温度等数据，并将部分身份字段映射到 print_task_config。

SpoolSense 提供扫描器直接写入此接口的工作流，明确要求对应的扩展固件；这条接口不能当成官方原版自带接口。该文档确认 Fluidd 回显，不等于已经验证官方 Snapmaker Orca 的所有同步行为。

K 值要按 U1 的工具头和压力提前量/校准系统另行适配。以上耗材身份命令不能证明已实现与 Bambu 等价的每卷 K 配置。

来源：
- https://wiki.snapmaker.com/en/FAQ/u1
- https://github.com/Snapmaker/u1-klipper/blob/main/klippy/extras/print_task_config.py
- https://github.com/paxx12-snapmaker-u1/SnapmakerU1-Extended-Firmware/blob/develop/docs/design/filament_detect.md
- https://spoolsense.org/installation/snapmaker-u1/
- https://github.com/TigerTag-Project/TigerTag-Studio-Manager/blob/main/renderer/printers/snapmaker/index.js

## Anycubic

官方 LAN 指南确认局域网切片软件通信，但没有据此保证任意第三方写入接口。

第三方实现使用 HTTP :18910 做本地握手，MQTT over TLS :9883 做状态和命令通信；认证过程与 Bambu :8883 不同。现有项目可以无需 root 连接部分原版固件。

TigerTag 的协议记录与代码有 multiColorBox / setInfo 的槽位类型和 RGB 写入。记录列明只有槽位索引、类型、颜色生效，品牌、温度等额外字段可能在返回成功时仍被忽略。外置料架可能使用独立 extfilbox 路径。

因此类型/颜色写入具有较高可行性；完整品牌预设、温度、每卷 K 选择不能承诺。Kobra 3、S1、X 等系列应逐型号/固件建立能力表，不对旧款或整个品牌给出统一结论。

来源：
- https://wiki.anycubic.com/fr/fdm-3d-printer/kobra-4-combo/lan-connection-guide
- https://github.com/chrisfore/anycubic_ha_local
- https://github.com/rvanderp3/kobra-connect
- https://github.com/TigerTag-Project/TigerTag-Studio-Manager/blob/main/renderer/printers/anycubic/PROTOCOL.md
- https://github.com/TigerTag-Project/TigerTag-Studio-Manager/blob/main/renderer/printers/anycubic/index.js

## 如后续授权适配

先定义能力层：读取槽位、写身份字段、选择校准记录、订阅/回读状态、认证方式。UI 按能力显示选项，品牌驱动分别实现通信；不将 Bambu 专用耗材 ID、AMS 编号和 cali_idx 转发给其他品牌。

验收要分开核实：机器读回、机器屏幕、官方切片软件设备页、客户端重新连接后状态、机器重启后状态。首次测试只改一个闲置槽位的元数据；进退料、加热、打印不属于本次调研范围。
