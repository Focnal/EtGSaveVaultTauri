use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// 回档点保留份数上限，防止配置文件被改成离谱的值
pub const MAX_ROLLBACK_KEEP: u32 = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSettings {
    /// 游戏存档目录（备份来源）
    pub source_path: String,
    /// 随存档导出的注册表键
    pub registry_key: String,
    /// 恢复前检测用的游戏进程名
    pub process_name: String,
    /// 备份内数据目录的名字
    pub data_dir_name: String,
    /// 备份内注册表文件名
    pub reg_file_name: String,
    /// 回档点保留份数，0 表示关闭
    pub rollback_keep: u32,
}

impl Default for GameSettings {
    fn default() -> Self {
        let source_path = std::env::var("USERPROFILE")
            .map(|home| {
                PathBuf::from(home)
                    .join("AppData")
                    .join("LocalLow")
                    .join("Dodge Roll")
                    .join("Enter the Gungeon")
            })
            .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Public"))
            .to_string_lossy()
            .into_owned();
        Self {
            source_path,
            registry_key: r"HKCU\Software\Dodge Roll\Enter the Gungeon".to_string(),
            process_name: "etg.exe".to_string(),
            data_dir_name: "Enter the Gungeon".to_string(),
            reg_file_name: "Enter the Gungeon.reg".to_string(),
            rollback_keep: 1,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub workspaces: Vec<Workspace>,
    pub game: GameSettings,
}

/// 设置面板提交的增量补丁，未出现的字段保持原值。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GamePatch {
    pub source_path: Option<String>,
    pub registry_key: Option<String>,
    pub process_name: Option<String>,
    pub data_dir_name: Option<String>,
    pub reg_file_name: Option<String>,
    pub rollback_keep: Option<u32>,
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位应用数据目录：{e}"))?;
    Ok(dir.join("vault-config.json"))
}

/// 读配置；缺字段用默认值补齐，损坏则整体回退到默认值。
pub fn load(app: &AppHandle) -> AppConfig {
    let defaults = AppConfig::default();
    let Ok(path) = config_path(app) else {
        return defaults;
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return defaults;
    };
    let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) else {
        return defaults;
    };

    let game = raw.get("game").cloned().unwrap_or(serde_json::Value::Null);
    let mut merged = serde_json::to_value(&defaults.game).unwrap_or(serde_json::Value::Null);
    if let (Some(target), Some(patch)) = (merged.as_object_mut(), game.as_object()) {
        for (key, value) in patch {
            if !value.is_null() {
                target.insert(key.clone(), value.clone());
            }
        }
    }
    let mut game: GameSettings = serde_json::from_value(merged).unwrap_or(defaults.game);
    // 保留份数只允许 0..=MAX，其余取值回到默认
    if !text.is_empty() && game.rollback_keep > MAX_ROLLBACK_KEEP {
        game.rollback_keep = GameSettings::default().rollback_keep;
    }

    let workspaces = raw
        .get("workspaces")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value::<Workspace>(item.clone()).ok())
                .filter(|w| !w.id.is_empty() && !w.path.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    AppConfig { workspaces, game }
}

pub fn save(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("无法创建配置目录：{e}"))?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| format!("无法写入配置：{e}"))
}
