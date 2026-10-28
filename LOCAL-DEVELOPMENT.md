# AMS-Hub 本机开发入口

源码来自 `https://github.com/tzq712911/AMS-Hub`，保留 Git 版本历史。

## 工具

- Rustup：用户目录 `.cargo/bin`。
- ESP32-S3 工具链：`esp1`，Xtensa Rust 1.90.0.0。
- 官方环境变量入口：用户目录 `export-esp.ps1`。
- espup 0.19.0：本仓库 `.local-tools/espup.exe`。
- espflash 4.6.0：本仓库 `.local-tools/espflash/espflash.exe`。
- 还需要安装完成的 Visual Studio 2022 C++ Build Tools 和 Windows SDK。

`.local-tools/`、`target/`、`dist/` 不提交到 Git。

## 使用

在此项目目录打开 PowerShell，启用命令行工具：

```powershell
. ./scripts/enter-local.ps1
cargo +esp1 --version
espflash --version
```

编译（脚本自动加载工具路径；使用仓库锁文件）：

```powershell
./scripts/build-local.ps1
```

如果 crates.io 下载不畅，可使用原工程支持的镜像：

```powershell
./scripts/build-local.ps1 -Mirror
```

成功后生成空白 WT32-SC01 Plus 所需的完整镜像：

```powershell
./scripts/package-local.ps1
```

输出是 `dist/local/AMS-Hub-full.bin` 和 SHA256 校验文件。完整镜像的烧录起始地址为 `0x0`；此约定不适用于仓库已有的 `*-app.bin` 应用镜像。

脚本不会自动写入开发板。首次烧录前仍需核实连接的是 WT32-SC01 Plus / ESP32-S3 / 16 MiB Flash，并确认串口。编译通过不代表固件通过实机测试。

## 状态说明

新增脚本已进行 PowerShell 语法检查。只有实际 release 编译和打包成功后，才能认为本机开发环境已完成验证；文件存在本身不代表安装、编译或实机验证成功。
