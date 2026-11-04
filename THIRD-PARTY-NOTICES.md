# 第三方许可与发布要求

检查日期：2026-11-04。本文件记录已识别的义务和阻塞项，不是完整依赖 SBOM、法律意见或“全部合规”的认证。源码公开与固件分发须分别检查。

## 当前发布阻塞：Slint

`core/Cargo.toml` 固定使用 Slint / slint-build 1.11.0。该版本提供 GPLv3、Royalty-free 2.0 和商业许可三条路线。Royalty-free 2.0 第 3 节明确排除嵌入式系统；WT32-SC01 Plus 上的 ESP32-S3 控制台属于需要按嵌入式场景审查的应用，不能以桌面预览可运行推定固件获许可。

维护者已确认尚未取得覆盖本项目的商业许可或书面例外授权。若选择 GPLv3 路线，分发组合程序须满足 GPLv3 对整体及对应源码的要求，不得叠加其不允许的限制；当前仍包含 Commons Clause 受限代码，不能通过给仓库改名或改写根许可证来消除冲突。

因此，在取得适用授权、完成兼容许可替换，或完成独立实现并重新审查前，不应将现有固件作为已获许可的公开或商业交付包继续分发。免费发放不自动免除该义务。仓库中已有二进制、源码归档、历史提交、Release 附件和外部副本也需要纳入后续处理；添加本说明不会收回副本或追溯补齐授权。

来源：

- [Slint 1.11.0 许可入口](https://github.com/slint-ui/slint/blob/v1.11.0/LICENSE.md)
- [Royalty-free 2.0 原文](https://github.com/slint-ui/slint/blob/v1.11.0/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md)
- [Slint 随附 GPLv3，第 5、6、10、12 节](https://github.com/slint-ui/slint/blob/v1.11.0/LICENSES/GPL-3.0-only.txt)

上游即使持有自己的商业许可，也不能据此推定该许可自动覆盖 AMS-Hub 或下游使用者。当前选择逐步替换受限依赖；迁移完成前保留发布阻塞。

## SpoolEase 及修改

保留根目录 `LICENSE.md` 的完整 Apache-2.0 + Commons Clause 原文。Apache-2.0 第 4 节要求分发许可副本、在修改文件上标明变更，并保留适用的版权及归属信息。`NOTICE.md` 是辅助记录，不能替代文件内声明和完整许可。

Commons Clause 的限制取决于产品或服务价值是否全部或主要来自受限软件的功能。官方 FAQ 允许某些提供实质新增价值的产品，不等于所有衍生产品都禁止销售，也不等于增加功能就一定豁免。商业化前需按实际实现和商业模式判断，疑义应取得权利人书面确认或由专业律师复核。

- [Commons Clause 原文与 FAQ](https://commonsclause.com/)
- [Apache-2.0 第 4 节与商标条款](https://www.apache.org/licenses/LICENSE-2.0)

## 已识别的其他组件

- `core/ui/fonts/ConsoleSans.ttf`：Noto Sans SC 派生子集，保留 `core/ui/fonts/OFL.txt`。字体改名不免除 OFL 义务。
- `vendor/esp-mbedtls`：保留 `LICENSE-MIT`、`LICENSE-APACHE` 及文件内归属。实际构建还需按锁文件和构建配置确认使用本地还是 Git 修订。
- `vendor/esp-hal-app-framework`：Cargo 元数据声明 `MIT OR Apache-2.0`。这与 SpoolEase 主程序的组合许可不同，不能仅因作者或仓库来源相同就视为相同限制；分发前需补齐实际采用修订的许可原文及修改记录。
- Espressif HAL、embassy、网络、TLS、RFID、解析与其他 Rust 依赖：以各构建的锁文件、实际启用特性、所选许可和第三方代码为准。此处尚未完成全依赖逐项清点。
- 品牌图标、图片、材料数据库及其他资源：公开可下载不等于有再分发或商标授权，需独立核对来源和用途。示意图不构成外观专利、商标或产品安全审查。

## 发布操作要求

每次交付应保存所构建源码提交、锁文件、许可清单及所需授权证据，并随包附完整许可和适用声明。授权合同等私密证据不要直接提交到公开仓库。`scripts/package.ps1` 的当前发布阻塞仅是防止误打包的工程措施，不是法律授权；其他打包入口、手动复制和旧包不因此自动合规。

独立硬件设计、软件实现、商标与专利应分别审查。源码归属说明无法保证硬件外观或供料机构不涉及第三方权利；在量产、收费或签约前，请熟悉软件许可和目标销售地区的律师复核。
