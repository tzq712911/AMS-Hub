# AMS-Hub 来源与修改声明

AMS-Hub 是以 U1 耗材流程为重点的独立社区项目。当前代码来自 SpoolEase 的衍生开发，并非仅有创意相似，也尚未完成独立重写。

- 上游项目：https://github.com/yanshay/SpoolEase
- 基线版本：`3532f8d962dd1a95c7d4ebb37beddca5bbefd39a`
- 上游许可：[Apache-2.0 + Commons Clause v1.0](LICENSE.md)，许可人字段保留为 SpoolEase。
- AMS-Hub 的修改包括中文界面、耗材编辑与 RFID 建档、多品牌通信和 U1 流程适配；各提交及文件内修改声明记录具体变化。

版本标识用于追溯源码来源，不代表官方合作、兼容性认证或商业授权。所有原有版权、许可、专利及适用归属声明仍须保留。本项目与 SpoolEase、Snapmaker、Bambu Lab、Anycubic 均无此处所宣称的隶属或背书关系；品牌名称仅用于来源及兼容性描述。

2026-11-04 对已提交版本 `80354a75903cefd0799076105f76bad07556413c` 的 `core/`、`shared/` 路径与上游基线作 Git blob 比较：75 个路径完全相同、36 个同路径文件不同、33 个路径在上游未找到。该计数包括配置、数据、资源和源码，不是代码行数、原创比例或法律上的独立性判断；新文件名或不同哈希也不能证明独立创作。未提交改动不在该比较范围内。

保留的相同源码示例有 `core/src/app.rs`、`core/src/spool_record.rs`、`core/src/filament_staging.rs`、`shared/src/nfc.rs`、`shared/src/ndef.rs`、`shared/src/types.rs`。大量修改界面或增加 U1 功能，不会自动消除这些代码的许可义务。

本文件由 AMS-Hub 添加，不冒充上游 NOTICE，不修改许可条件，也不替代逐文件修改说明。基线上游目录中检测到 `LICENSE.md`，未检测到单独 NOTICE 文件；后续引入内容仍需重新检查。

第三方依赖及尚未解决事项见 [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)。未来独立实现的边界见 [INDEPENDENT-IMPLEMENTATION.md](INDEPENDENT-IMPLEMENTATION.md)。
