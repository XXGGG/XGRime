use crate::error::AppResult;
use crate::platform::{current_platform, RimeInstallInfo};

#[tauri::command]
pub fn detect_rime() -> AppResult<RimeInstallInfo> {
    Ok(current_platform().detect())
}

/// 启动官方卸载程序（Windows 会弹 UAC）
#[tauri::command]
pub fn uninstall_rime() -> AppResult<()> {
    current_platform().uninstall()
}

/// 把卸载后残留的旧配置目录改名备份，返回备份到哪了
#[tauri::command]
pub fn backup_leftover_config(config_dir: String) -> AppResult<String> {
    crate::platform::backup_leftover(&config_dir)
}
