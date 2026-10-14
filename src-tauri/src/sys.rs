use std::path::Path;
use std::process::{Command, Stdio};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// 注册表导出 / 导入依赖系统自带的 reg.exe，输出重定向后无窗口副作用最小。
fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn run_hidden(program: &str, args: &[&str]) -> bool {
    // 系统目录里的 reg.exe / taskkill.exe 不太可能不存在
    let args = args
        .iter()
        .map(|a| a.replace('"', "\"\""))
        .collect::<Vec<_>>();
    hidden_command(program)
        .args(&args)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// 导出注册表键到 .reg 文件；键不存在或平台不支持时返回 false。
pub fn export_registry(key: &str, file: &Path) -> bool {
    #[cfg(not(windows))]
    {
        let _ = (key, file);
        return false;
    }
    #[cfg(windows)]
    {
        if key.trim().is_empty() {
            return false;
        }
        if let Some(dir) = file.parent() {
            if std::fs::create_dir_all(dir).is_err() {
                return false;
            }
        }
        let Some(target) = file.to_str() else {
            return false;
        };
        run_hidden("reg.exe", &["export", key, target, "/y"])
    }
}

/// 从 .reg 文件导入注册表。
pub fn import_registry(file: &Path) -> bool {
    #[cfg(not(windows))]
    {
        let _ = file;
        return false;
    }
    #[cfg(windows)]
    {
        if !file.exists() {
            return false;
        }
        let Some(target) = file.to_str() else {
            return false;
        };
        run_hidden("reg.exe", &["import", target])
    }
}

fn process_running(process_name: &str) -> bool {
    let target = process_name.trim().to_ascii_lowercase();
    if target.is_empty() {
        return false;
    }
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    system
        .processes()
        .values()
        .any(|p| p.name().to_string_lossy().to_ascii_lowercase() == target)
}

/// 游戏进程是否在运行。
pub fn is_game_running(process_name: &str) -> bool {
    process_running(process_name)
}

/// 强制结束游戏进程，返回是否确实结束了。
pub fn kill_game(process_name: &str) -> bool {
    let target = process_name.trim().to_ascii_lowercase();
    if target.is_empty() {
        return false;
    }
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    let targets: Vec<_> = system
        .processes()
        .iter()
        .filter(|(_, p)| p.name().to_string_lossy().to_ascii_lowercase() == target)
        .map(|(pid, _)| *pid)
        .collect();
    if targets.is_empty() {
        return false;
    }
    for pid in targets {
        if let Some(process) = system.process(pid) {
            process.kill();
        }
    }
    // 等进程真正退出，避免调用方紧接着覆盖存档时被文件占用
    for _ in 0..20 {
        if !process_running(&target) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    !process_running(&target)
}
