//! 跟操作系统打交道的零碎：开机自启、打开系统设置页。

use crate::error::{code, AppError, AppResult};

/// 注册表里那一项的名字。改名会导致旧的那条清不掉，别动。
#[cfg(windows)]
const RUN_KEY: &str = "XGRime";
#[cfg(windows)]
const RUN_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[tauri::command]
pub fn get_autostart() -> AppResult<bool> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        Ok(RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(RUN_PATH)
            .and_then(|k| k.get_value::<String, _>(RUN_KEY))
            .is_ok())
    }
    #[cfg(not(windows))]
    Ok(mac_plist().map(|p| p.is_file()).unwrap_or(false))
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> AppResult<bool> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_WRITE};
        use winreg::RegKey;

        let run = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(RUN_PATH, KEY_WRITE)
            .map_err(|e| AppError::with(code::AUTOSTART_FAILED, e))?;
        if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| AppError::with(code::AUTOSTART_FAILED, e))?;
            // 路径带引号，不然装在「Program Files」这种带空格的目录下会被拆开
            run.set_value(RUN_KEY, &format!("\"{}\"", exe.display()))
                .map_err(|e| AppError::with(code::AUTOSTART_FAILED, e))?;
        } else {
            // 本来就没有也算成功，别让用户对着一个关不掉的开关发愁
            let _ = run.delete_value(RUN_KEY);
        }
    }
    #[cfg(not(windows))]
    {
        let path = mac_plist().ok_or_else(|| AppError::new(code::AUTOSTART_FAILED))?;
        if enabled {
            let exe = std::env::current_exe()
                .map_err(|e| AppError::with(code::AUTOSTART_FAILED, e))?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;
            }
            let plist = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.xxggg.xgrime</string>
  <key>ProgramArguments</key><array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
                exe.display()
            );
            std::fs::write(&path, plist)
                .map_err(|e| AppError::with(code::AUTOSTART_FAILED, e))?;
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
    get_autostart()
}

#[cfg(not(windows))]
fn mac_plist() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents/com.xxggg.xgrime.plist"))
}

/// 打开系统里的某个设置页
///
/// 目前只用来开 Windows 的「高级键盘设置」—— 换默认输入法、按应用切输入法
/// 这些开关在系统里，输入法自己改不了，只能把用户送过去。
#[tauri::command]
pub fn open_system_setting(which: String) -> AppResult<()> {
    #[cfg(windows)]
    {
        let uri = match which.as_str() {
            "keyboard-advanced" => "ms-settings:keyboard-advanced",
            "language" => "ms-settings:regionlanguage",
            _ => return Err(AppError::with(code::SETTING_PAGE_UNKNOWN, which)),
        };
        // 走 explorer 而不是 cmd：不会闪一下黑窗口
        std::process::Command::new("explorer")
            .arg(uri)
            .spawn()
            .map_err(|e| AppError::with(code::LAUNCH_FAILED, e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        // macOS 没有对应的页面，前端也不会显示这个入口
        Err(AppError::with(code::SETTING_PAGE_UNKNOWN, which))
    }
}
