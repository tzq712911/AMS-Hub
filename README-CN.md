# SpoolEase Console 中文定制版

定制版本：0.5.5-b.4+console.zh.1。基于公开提交 3532f8d962dd1a95c7d4ebb37beddca5bbefd39a，不是官网 0.7 系列的对应源码。

## 已实现

- WT32-SC01 Plus + PN532 原硬件路线；保留 microSD 存储。
- 480×320 深色主页、AMS 槽位卡片、外置料架切换、绿色确认按钮。
- 扫描耗材后选择槽位，核对扫描值与机器当前值。
- 选择内置耗材预设、12 色快捷色板及匹配喷嘴/耗材的现有 K 校准记录。
- 未知标签可以直接在触屏建立最小本地耗材档案；可解析的兼容标签预填已有数据。保存 UID 绑定，不修改原标签。
- 常用扫码、错误、取消和完成提示汉化；Noto Sans SC 派生的 Console Sans 字体子集及 OFL 声明。
- 已发送提示与设备接受的事实区分；新编辑器校验连接、开发者模式、槽位、颜色、参数及校准记录匹配关系。
- 主页、设置及新标签流程不依赖网页库存界面。配置网页仍用于 Wi-Fi/打印机连接设置。

## 尚需实机验证/功能边界

- 没有向实际打印机发送命令，也没有刷写开发板。
- 机器接受写入、X2D 最新固件 TLS 兼容、左右喷嘴映射及 Studio 回显均待实机确认。
- 当前提示“已发送”不代表打印机已经接受；完整逐请求确认和状态回读仍需进一步实现。
- 不执行进退料、加热或打印。喷嘴温度栏显示预设/读取的数据，当前不支持手动修改温度范围。
- 不新建任意 K 数字；可以使用既有匹配记录。未修改的扫描档案继续沿用上游匹配逻辑；手动编辑未选择校准记录时使用默认 K。
- 编辑槽位不会自动改写原标签或本地档案中的参数。首次绑定没有 K 档案时仍需在槽位选择校准记录。
- 库存/称重界面入口已从主要流程移除；上游存储格式、称重任务与部分后台 API 尚未全部裁剪。
- 上游 0.7 的连接修复未全面移植。不承诺仅凭编译成功就兼容所有当前固件。
- 字库覆盖界面和源码字符；任意中文自定义名称可能需要扩展字库。可重新运行 scripts/prepare_font.py。
- U1/Anycubic 只完成调研，未实现品牌驱动。见 COMPATIBILITY_RESEARCH.md。

## 本机已安装的工具

- Rustup/Cargo；Espressif Xtensa Rust 1.90.0，工具链名 esp1，目标 xtensa-esp32s3-none-elf。
- Xtensa LLVM/GCC；espflash 4.6.0。
- Visual Studio 2022 C++ Build Tools、Windows SDK、CMake。安装器返回 3010，未执行重启；实际基线构建已成功。
- Slint 1.11 Python 绑定、fontTools/brotli 用于 UI 检查和字体裁剪。

Rust/ESP 和 Python 辅助包位于工作区 tools 目录；不依赖系统全局 PATH。标准 stable Rust 的独立下载曾超时，但 esp1 自带的编译器及 Cargo 已通过运行验证。

## 构建

本工作区布局为 tools/、baseline/ 和 source/本工程/。

```powershell
.\scripts\build.ps1 -Mirror
```

脚本在工程 core 目录使用锁文件编译，-Mirror 使用 rsproxy 处理本机 crates.io 超时。它不执行烧录。基线与定制版共用 baseline 工程下的 target 缓存，便于复现。

构建修复：将上游小写 cargo.toml 规范为 Cargo.toml；build.rs 显式搜索现有 memory.x，使链接器使用与两块 7 MiB OTA 分区一致的 Flash 映射。否则 esp-hal 默认 4 MiB 代码区不足以容纳定制版。

## 验证

- 原版 release 构建成功。
- 定制版 release 构建成功记录后续随 dist 构建报告保存。
- Slint 1.11 实际 AppWindow 及所有导入组件编译检查通过。
- console_validation.rs 的四组边界测试通过。
- preview/ 使用同一套 UI 组件和软件渲染器，生成 480×320 PNG，全部为明确的演示数据。
- 离屏交互验证通过：耗材选择、色板选择、发送参数检查、防重复确认。
- 预览不是硬件性能测试或真实设备回报。Python 的桌面包装层不完整支持本工程的嵌套类型，优先用 Rust 离屏预览。

## 许可

保留 LICENSE.md 的 Apache 2.0 + Commons Clause 及上游归属。定制 Cargo 元数据已改为引用根许可证。字体源来自 https://github.com/google/fonts/tree/main/ofl/notosanssc ，派生字体名称为 Console Sans，许可在 core/ui/fonts/OFL.txt。分发前仍须按使用方式核对 Slint 及其他依赖的许可；本工作未获得额外商业许可。
