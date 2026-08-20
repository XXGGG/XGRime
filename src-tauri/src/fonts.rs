use crate::error::AppResult;
use std::collections::BTreeSet;

#[tauri::command]
pub fn get_system_fonts() -> AppResult<Vec<String>> {
    let mut fonts = BTreeSet::new();

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts") {
            for (name, _) in key.enum_values().filter_map(|r| r.ok()) {
                // 注册表值名格式: "Arial (TrueType)" → 提取 "Arial"
                if let Some(font_name) = name.split(" (").next() {
                    let font_name = font_name.trim();
                    if !font_name.is_empty() {
                        fonts.insert(font_name.to_string());
                    }
                }
            }
        }

        // 也读用户字体
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts") {
            for (name, _) in key.enum_values().filter_map(|r| r.ok()) {
                if let Some(font_name) = name.split(" (").next() {
                    let font_name = font_name.trim();
                    if !font_name.is_empty() {
                        fonts.insert(font_name.to_string());
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: 用 system_profiler 或直接扫描字体目录
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["SPFontsDataType", "-json"])
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                // 简单解析：找所有 "family": "xxx" 的值
                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with("\"family\"") {
                        if let Some(name) = line.split('"').nth(3) {
                            if !name.starts_with('.') {
                                fonts.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 备选：扫描常见字体目录
        if fonts.is_empty() {
            for dir in &["/System/Library/Fonts", "/Library/Fonts", "~/Library/Fonts"] {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            fonts.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(fonts.into_iter().collect())
}
