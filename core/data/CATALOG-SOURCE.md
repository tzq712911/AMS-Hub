# 目录来源

alpha.3 使用本机安装的 Bambu Studio `resources/profiles/BBL/filament` 中的公开系统预设事实字段（名称、ID、品牌、基础类型、温度范围），以及 `filaments_color_codes.json` 的单色色卡。

由 `scripts/import_studio_catalog.py` 提取并与原有索引合并；101 条预设、5 个预设品牌。混合色不能用一个 RGB 准确表示，因此未冒充单色导入。没有读取账户、访问码或用户项目。

这是 Studio 资源目录快照，不是通过 LAN 从打印机枚举得到的目录，也不保证等于任意打印机当前固件支持的完整名单。可重新执行导入脚本更新事实字段。未收录品牌使用明确标记的通用基础类型，实际品牌和自选 RGB 仍在 RFID 对应库保留。
