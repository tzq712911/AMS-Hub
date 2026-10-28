# SpoolEase 精简触屏版：阶段一历史记录

本文保留第一阶段的分析和最初修改记录。后续工具链安装、中文界面、编译和验证状态请以 [README-CN.md](README-CN.md) 为准。

## 基线与范围

- 上游： https://github.com/yanshay/SpoolEase
- 固定提交：3532f8d962dd1a95c7d4ebb37beddca5bbefd39a
- 源码包版本：core/cargo.toml 中的 0.5.5-b.4。不是官网 0.7.6 或 0.7.7-b.5 的对应源码。
- 硬件：WT32-SC01 Plus（ESP32-S3）+ PN532 SPI；当前上游仍依赖 microSD。
- 目标：识别标签 → 匹配耗材 → 确定 AMS 槽位 → 写入材质/颜色/喷嘴对应 K 配置 → 核对设备回报。
- 非目标：网页库存、仓位、称重、余量和消耗统计、切片功能。
- 标签编号到耗材参数的最小本地档案需要保留，不能随库存 UI 一起删除。

## 首批修改

core/ui/app.slint 中 AppConsts.console-lite 默认 true，设为 false 可恢复这些界面分支的原行为。

- controlpanel.slint：隐藏称重状态；待机提示使用固定颜色，停止待机变色计时器；槽位选择不再十秒自动取消。
- spoolstaging.slint：隐藏暂存耗材余量，优先显示 K 配置可用性；隐藏空闲卡片中的官方升级提示和入口。
- trays.slint：槽位卡片不再以余量覆盖状态显示。

这是第一阶段的界面精简，不代表库存后端、称重任务或官方 OTA 通道已经删除。设置菜单和详情页仍有原功能入口；后续应一起梳理，不能宣称已完成全面裁剪。

## 核心调用关系

- core/src/main.rs：WT32-SC01 Plus 显示/触控、SD、PN532 SPI 初始化。
- shared/src/nfc.rs、pn532_ext.rs、spool_tag.rs：标签识别与读写。
- core/src/view_model.rs：标签、UI 回调、耗材档案与机器操作的连接层。
- core/src/bambu.rs：set_tray_filament、匹配校准记录、创建校准记录及设备状态处理。
- core/src/bambu_api.rs：ams_filament_setting、extrusion_cali_sel、extrusion_cali_set 等消息结构。
- core/src/store.rs、spool_record.rs：本地耗材记录；需要先拆分最小档案，再移除统计职责。
- core/ui/*.slint：480×320 触屏界面。

## 构建与验证状态

本机检查时没有找到 cargo/rustup；尚未编译、烧录或进行实机验证。当前改动只完成源代码检查，不可视为可刷入版本。

上游构建使用 Slint 1.11.0、xtensa-esp32s3-none-elf 目标、名为 esp1 的工具链和多个作者维护的 Git 依赖。工具链与依赖可用性需要实际验证，不能假设常规桌面 Rust 即可编译。首次构建需要先对未修改基线进行复现，再编译定制版。

## 后续工作顺序

1. 复现原版构建，固定编译器及 Cargo.lock 依赖；记录工具链安装步骤。
2. 核对固件连接 X2D 的行为，验证 TLS、AMS/喷嘴映射、K 配置查询和写入回报。新版本 TLS 修复对连接有实际影响，不能因为是维护版本而略过。
3. 建立中文资源和字库子集，处理 Rust 动态提示、Slint 页面文字、截断和内存占用。中文输入优先采用最小配置网页，是否保留由交互设计确定。
4. 重排主页和扫码流程；已知标签尽量少步，未知标签有完整绑定入口。
5. 将写入区分为等待、发送、设备确认、失败/超时；避免仅凭发布 MQTT 就显示成功。
6. 在最小档案可独立使用后，裁剪库存 API、称重/消耗任务和无关页面；设置定制升级通道。
7. 实机覆盖扫描、重复扫描、空槽/多槽变化、左右喷嘴、断网重连、重启、写入拒绝及 Studio 状态同步。

## 许可

保留上游 LICENSE.md、版权及归属声明。本文件记录定制修改。上游根许可证为 Apache 2.0 加 Commons Clause；core/cargo.toml 的许可证元数据与根文件不一致，不能据此认定衍生固件不受 Commons Clause 限制。依赖许可证需在分发前分别核对。
