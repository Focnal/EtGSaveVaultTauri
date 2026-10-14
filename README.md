# EtG Save Vault

Enter the Gungeon 存档备份管理工具。这是 [electron 版](https://github.com/Focnal/EtGSaveVault) 的 **Tauri 2 重构版**：
后端从 Node.js 主进程换成 Rust，前端仍是 TypeScript + Vue 3，界面与交互保持一致。

## 功能

- **工作区**：可添加多个备份目录，树形展开查看其中的备份
- **备份**：新建（复制存档目录 + 导出注册表键）、恢复、重命名、删除、在资源管理器中打开
- **备份详情**：创建时间、文件数、总大小、注册表标记、可编辑备注、存档文件预览
- **回档点**：恢复前自动把当前存档保留为回档点，恢复完成的提示里可直接「撤销本次恢复」
- **设置**：存档来源目录、注册表键、游戏进程名、回档点保留份数均可配置
- **单实例**：重复启动只会聚焦已有窗口，不会开出第二个实例

## 环境要求

- Windows 10 1803+ / Windows 11（自带 WebView2 Runtime）
- 开发需要：Node.js >= 20、Rust stable（MSVC 工具链）、Visual Studio C++ 生成工具

## 开发与构建

```bash
npm install            # 前端依赖（不再下载 Electron 二进制）
npm run dev            # vite 开发服务器 + tauri dev（Rust 侧改动会自动重编译重启）
npm run typecheck      # vue-tsc 类型检查
npm run build          # 发布构建：vite build + cargo build --release，只出便携版 exe
```

产物只有一个：

- `src-tauri/target/release/etg-save-vault.exe`（独立 exe，无需安装）

Rust 侧检查：

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test  --manifest-path src-tauri/Cargo.toml --lib
```

## 目录结构

```
src-tauri/
  src/main.rs        进程入口
  src/lib.rs         Tauri 装配 + 全部 #[tauri::command]
  src/config.rs      配置模型与读写（AppConfig / GamePatch）
  src/backups.rs     备份的列出、详情、新建、恢复、重命名、删除、备注
  src/rollback.rs    回档点：捕获、列举、还原、清理、按份数裁剪
  src/sys.rs         注册表导入导出（reg.exe）、游戏进程检测与结束（sysinfo）
  src/fsutil.rs      路径存在性、递归统计、递归复制、时间戳
  src/error.rs       命令层统一错误类型
  capabilities/      窗口权限（dialog / opener / core）
src/
  api.ts             invoke 封装：与 Rust 命令签名一一对应
  types.ts           与 Rust serde 结构对应的类型（后端统一 camelCase）
  store.ts           全局状态、Toast、右键菜单、业务流程
  components/        Sidebar / WorkspaceView / BackupDetail / 各弹窗 / 右键菜单
  style.css          与原项目一致的样式
scripts/
  fixture.mjs        生成冒烟测试用的工作区与配置
  cdp.mjs            CDP 连接、求值、截图的共用部分
  ui-smoke.mjs       只读冒烟：逐界面断言渲染
  e2e.mjs            完整业务链路，逐步核对磁盘结果
docs/                界面截图
```

## 数据布局

备份是工作区下的一个文件夹，格式与原项目完全一致，可以直接复用已有备份：

```
<工作区>/<备份名>/
  Enter the Gungeon/      存档数据
  Enter the Gungeon.reg   注册表导出（可选）
  vault-meta.json         创建时间与备注
```

「回档点」单独存放在 `%LOCALAPPDATA%\EtG Save Vault\rollback\<来源路径摘要>\`，不进入备份树；
与存档来源同卷时零拷贝捕获（目录 rename），跨卷时退化为复制，默认只保留最近 1 份。

应用配置与 Electron 版共用同一个文件：`%APPDATA%\EtG Save Vault\vault-config.json`