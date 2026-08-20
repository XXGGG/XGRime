use crate::config::{LayoutConfig, ThemeConfig};
use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 用户自己存下来的一套外观
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPreset {
    /// 用创建时间当 id，够唯一也够简单
    pub id: String,
    pub name: String,
    pub theme: ThemeConfig,
    pub layout: LayoutConfig,
}

/// XGRime 自己的东西放这儿，跟 RIME 的配置目录分开
///
/// 分开是有意的：RIME 那个目录归输入法管，我们往里塞自己的文件会在
/// 「卸载方案」「清理残留」时纠缠不清。备份的时候两边一起打包就好。
pub fn prefs_dir() -> AppResult<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::new(code::CONFIG_PATH_INVALID))?
        .join("XGRime");
    std::fs::create_dir_all(&dir).map_err(|e| AppError::with(code::CONFIG_DIR_CREATE_FAILED, e))?;
    Ok(dir)
}

fn presets_file() -> AppResult<PathBuf> {
    Ok(prefs_dir()?.join("presets.json"))
}

/// 备份要连它一起打包，所以给外面一个入口。拿不到目录就返回 None
pub fn presets_path() -> Option<PathBuf> {
    presets_file().ok()
}

/// 读全部预设
///
/// 文件在但读不懂时**报错而不是当空的返回** —— 返回空的话，用户下一次保存
/// 就把整份文件覆盖掉，他之前存的全没了。宁可这一次失败。
fn read_all() -> AppResult<Vec<UserPreset>> {
    let path = presets_file()?;
    let raw = match std::fs::read_to_string(&path) {
        Ok(r) => r,
        Err(_) => return Ok(vec![]), // 还没存过，正常
    };
    if raw.trim().is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str(&raw).map_err(|e| {
        AppError::with(
            code::PRESETS_UNREADABLE,
            format!("{} — {}", path.display(), e),
        )
    })
}

fn write_all(list: &[UserPreset]) -> AppResult<()> {
    let json = serde_json::to_string_pretty(list)
        .map_err(|e| AppError::with(code::YAML_SERIALIZE_FAILED, e))?;
    std::fs::write(presets_file()?, json).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))
}

#[tauri::command]
pub fn list_user_presets() -> AppResult<Vec<UserPreset>> {
    read_all()
}

/// 存一套。id 已存在就当成改名 / 覆盖，不重复添加。
#[tauri::command]
pub fn save_user_preset(preset: UserPreset) -> AppResult<Vec<UserPreset>> {
    let mut list = read_all()?;
    match list.iter_mut().find(|p| p.id == preset.id) {
        Some(existing) => *existing = preset,
        None => list.push(preset),
    }
    write_all(&list)?;
    Ok(list)
}

#[tauri::command]
pub fn delete_user_preset(id: String) -> AppResult<Vec<UserPreset>> {
    let mut list = read_all()?;
    list.retain(|p| p.id != id);
    write_all(&list)?;
    Ok(list)
}

/// 状态图标的选择
///
/// 存在 XGRime 自己的目录里，不进 RIME 配置 —— 「跟着任务栏自动换」这件事
/// 是我们的功能，输入法本身没有这个概念。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconPref {
    /// `auto` = 跟着任务栏深浅换；其余就是某一套的 id；空 = 没设过
    pub mode: String,
    /// 上次实际装进去的是哪一套，用来判断要不要重装
    pub applied: String,
}

fn icon_pref_file() -> AppResult<PathBuf> {
    Ok(prefs_dir()?.join("icon-pref.json"))
}

#[tauri::command]
pub fn read_icon_pref() -> AppResult<IconPref> {
    let path = icon_pref_file()?;
    // 读不出来就当没设过：这里只是个偏好，坏了不该挡住整页
    Ok(std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default())
}

pub fn write_icon_pref(pref: &IconPref) -> AppResult<()> {
    let path = icon_pref_file()?;
    let text =
        serde_json::to_string_pretty(pref).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
    std::fs::write(path, text).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))
}
