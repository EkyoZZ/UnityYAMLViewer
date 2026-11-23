# PS Tile 插件安装器

独立 Rust / egui 工程，用于部署、更新和移除当前用户的 PS Tile 插件。

## 行为

- EXE 内嵌插件清单、面板脚本与图标等十个运行文件，无需旁带插件文件。
- 固定安装到当前用户 `%APPDATA%\Adobe\UXP\Plugins\External\unityyamlviewer.isometric-preview`，不修改 Photoshop 安装目录、注册表、文件关联或 UXP 安装数据库，不申请管理员权限。
- 安装、更新和卸载前检测 `Photoshop.exe`；运行中或无法确认进程状态时拒绝操作，要求用户关闭后重试，不自动关闭 PS。
- 更新先在同级暂存目录准备并进行 SHA-256 校验，再重命名切换；替换失败时尝试恢复旧目录，恢复异常时显示路径。不强行删除失败现场。
- 旧目录必须有本插件 ID 的清单；额外普通文件会复制并保留，目录中存在子目录、符号链接／重解析点，或超过 64 个文件／32 MiB 时拒绝更新。卸载只移除十个插件文件，其他内容保留。
- 操作中不允许关闭窗口；同一目标通过独占锁避免多个本安装器同时修改。若进程被强制终止，可能保留 `.unityyamlviewer.isometric-preview.lock` 或暂存／备份目录；确认没有其他安装器运行后再人工检查恢复，不自动清除锁。
- 软件列表不注册卸载项。再次打开安装器，点击“卸载插件”即可卸载；用户另行放到其他位置的副本不在管理范围内。

## 图标

先复用主项目 `assets/app_icon_*.png`，构建时生成多尺寸 ICO 并嵌入 Windows 资源；窗口使用 256px PNG。图标不是运行必需项，可以后续替换，不需要先制作新图标。

## 检查与生成

允许不生成目标 EXE 的检查：

```powershell
cargo check --offline --tests --manifest-path plugins/photoshop-isometric-tile/installer/Cargo.toml
```

工程约定要求生成 EXE 前取得用户明确确认。确认后才运行：

```powershell
./plugins/photoshop-isometric-tile/installer/build.ps1 -ConfirmExecutableBuild
```

脚本先检查安装器进程与固定输出 EXE 是否占用；有占用时停止，关闭后仍须用户再次确认再编译。主程序不参与此次构建。源码和依赖分别维护，`Cargo.lock` 随工程保留。

使用 UXP 用户目录部署方式。PS 需要支持当前插件所声明的最低版本；面板绑定与预览能力见上级目录说明。

安装器按已部署版本显示安装、更新或覆盖安装；同版本允许重新部署并校验。已部署版本更高时停止覆盖，避免误降级。读取或版本比较失败会显示原因。

The installer offers Install, Update or Overwrite according to the deployed version. Equal versions may be redeployed and verified. A newer deployed version blocks replacement by an older installer; unreadable or invalid version information is reported.
