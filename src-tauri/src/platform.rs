use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// RIME 安装信息
///
/// `installed` 的判定标准只有一条：**真的找得到可执行的部署器**。
/// 配置目录存在不算数 —— 小狼毫卸载时不会删 `%APPDATA%\Rime`，
/// 光看目录会把「卸载后的残留」误判成「已安装」。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RimeInstallInfo {
    pub installed: bool,
    pub version: Option<String>,
    /// 程序目录（里面有 WeaselDeployer.exe / Squirrel.app）
    pub install_dir: Option<String>,
    /// 用户配置目录
    pub config_dir: String,
    pub platform: String,
    /// 没装程序，但配置目录里有上一次留下的 RIME 配置
    pub has_leftover: bool,
    /// 找得到卸载入口
    pub can_uninstall: bool,
}

/// 平台特定的 RIME 操作 trait（预留 Linux 扩展）
pub trait RimePlatform {
    fn detect(&self) -> RimeInstallInfo;
    fn config_dir(&self) -> PathBuf;
    /// 触发重新部署。返回 true 表示我们等到它真的跑完了。
    fn deploy(&self, install_dir: Option<&str>) -> AppResult<bool>;
    /// 启动官方卸载程序
    fn uninstall(&self) -> AppResult<()>;
    /// 停掉输入法后台服务
    ///
    /// 换输入方案要先停它：服务把「当前是哪个方案」缓在内存里，改文件它不看；
    /// 更麻烦的是它退出时会**把内存里那份写回 user.yaml**，先改文件再停服
    /// 等于白改。所以必须停服 → 改文件 → 启动，顺序不能换。
    fn stop_service(&self, install_dir: Option<&str>) -> AppResult<()>;
    /// 启动输入法后台服务
    fn start_service(&self, install_dir: Option<&str>) -> AppResult<()>;
}

/// 配置目录里出现这些文件，才算「有 RIME 配置残留」，
/// 而不是某个程序随手建了个空 Rime 文件夹
const LEFTOVER_MARKERS: [&str; 5] = [
    "default.yaml",
    "default.custom.yaml",
    "installation.yaml",
    "user.yaml",
    "build",
];

fn has_rime_leftover(config_dir: &Path) -> bool {
    if !config_dir.is_dir() {
        return false;
    }
    if LEFTOVER_MARKERS.iter().any(|m| config_dir.join(m).exists()) {
        return true;
    }
    // 兜底：目录里有任何 *.schema.yaml 也算
    std::fs::read_dir(config_dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(".schema.yaml")
            })
        })
        .unwrap_or(false)
}

// ─── Windows 实现 ───

#[cfg(target_os = "windows")]
pub struct WindowsPlatform;

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use winreg::enums::*;
    use winreg::RegKey;

    /// 小狼毫安装器是 32 位 NSIS，写 HKLM 会被重定向到 WOW6432Node。
    /// 两个视图都要读，谁先命中算谁。
    pub fn hklm_value(subkey: &str, name: &str) -> Option<String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        for flags in [KEY_READ | KEY_WOW64_32KEY, KEY_READ | KEY_WOW64_64KEY] {
            if let Ok(key) = hklm.open_subkey_with_flags(subkey, flags) {
                if let Ok(v) = key.get_value::<String, _>(name) {
                    if !v.trim().is_empty() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    pub fn hkcu_value(subkey: &str, name: &str) -> Option<String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(subkey)
            .ok()
            .and_then(|k| k.get_value::<String, _>(name).ok())
            .filter(|v| !v.trim().is_empty())
    }

    const RIME_KEY: &str = r"SOFTWARE\Rime\Weasel";
    const UNINST_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Weasel";

    /// 目录里有部署器才算数
    fn is_program_dir(dir: &Path) -> bool {
        dir.join("WeaselDeployer.exe").is_file()
    }

    /// 安装器把 InstallDir 记成用户选的**上级**目录，
    /// 真正的程序放在 `{InstallDir}\weasel-{版本}\` 里面，所以要往下找一层。
    fn resolve_program_dir(base: &Path) -> Option<PathBuf> {
        if is_program_dir(base) {
            return Some(base.to_path_buf());
        }
        let mut candidates: Vec<PathBuf> = std::fs::read_dir(base)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && p.file_name()
                        .map(|n| n.to_string_lossy().starts_with("weasel-"))
                        .unwrap_or(false)
            })
            .collect();
        // 按版本号比，不能按字符串比 —— 字符串比会认为 weasel-0.9.30 比
        // weasel-0.17.4 新（"9" > "1"）。
        let version_key = |p: &PathBuf| -> Vec<u64> {
            p.file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_prefix("weasel-"))
                .map(|v| {
                    v.split(['.', '-', '_'])
                        .map(|part| {
                            part.chars()
                                .take_while(|c| c.is_ascii_digit())
                                .collect::<String>()
                                .parse::<u64>()
                                .unwrap_or(0)
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        candidates.sort_by_key(version_key);
        candidates.into_iter().rev().find(|p| is_program_dir(p))
    }

    /// 按可靠度依次尝试，先命中先用
    pub fn find_program_dir() -> Option<PathBuf> {
        // 1. 卸载信息里的 DisplayIcon 直接指向 WeaselServer.exe，最准
        if let Some(icon) = hklm_value(UNINST_KEY, "DisplayIcon") {
            let cleaned = icon.trim().trim_matches('"');
            // 形如 "C:\...\weasel-0.17.4\WeaselServer.exe"，取它所在目录
            if let Some(parent) = Path::new(cleaned).parent() {
                if is_program_dir(parent) {
                    return Some(parent.to_path_buf());
                }
            }
        }
        // 2. 当前安装器写的 InstallDir（上级目录，要往下找 weasel-* ）
        if let Some(base) = hklm_value(RIME_KEY, "InstallDir") {
            if let Some(dir) = resolve_program_dir(Path::new(&base)) {
                return Some(dir);
            }
        }
        // 3. 老版本写的 WeaselRoot（直接指向程序目录）
        if let Some(root) = hklm_value(RIME_KEY, "WeaselRoot") {
            if let Some(dir) = resolve_program_dir(Path::new(&root)) {
                return Some(dir);
            }
        }
        // 4. 兜底：卸载程序所在目录
        if let Some(uninst) = uninstall_string() {
            let cleaned = uninst.trim().trim_matches('"');
            if let Some(parent) = Path::new(cleaned).parent() {
                if is_program_dir(parent) {
                    return Some(parent.to_path_buf());
                }
            }
        }
        None
    }

    pub fn version() -> Option<String> {
        hklm_value(UNINST_KEY, "DisplayVersion")
    }

    pub fn uninstall_string() -> Option<String> {
        hklm_value(UNINST_KEY, "UninstallString")
    }

    /// 用户配置目录：小狼毫自己就是这么找的
    /// （见 weasel/RimeWithWeasel/WeaselUtility.cpp 的 WeaselUserDataPath）
    pub fn user_data_dir() -> PathBuf {
        if let Some(dir) = hkcu_value(r"Software\Rime\Weasel", "RimeUserDir") {
            let p = PathBuf::from(dir);
            if !p.as_os_str().is_empty() {
                return p;
            }
        }
        dirs::config_dir()
            .map(|d| d.join("Rime"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Users\Default\AppData\Roaming\Rime"))
    }
}

#[cfg(target_os = "windows")]
impl RimePlatform for WindowsPlatform {
    fn detect(&self) -> RimeInstallInfo {
        let config_dir = self.config_dir();
        let program_dir = win::find_program_dir();
        let installed = program_dir.is_some();

        RimeInstallInfo {
            installed,
            version: if installed { win::version() } else { None },
            install_dir: program_dir.map(|p| p.to_string_lossy().to_string()),
            has_leftover: !installed && has_rime_leftover(&config_dir),
            can_uninstall: installed && win::uninstall_string().is_some(),
            config_dir: config_dir.to_string_lossy().to_string(),
            platform: "windows".to_string(),
        }
    }

    fn config_dir(&self) -> PathBuf {
        win::user_data_dir()
    }

    /// `WeaselDeployer.exe /deploy` 走的是 `Configurator::UpdateWorkspace()`，
    /// **编译完词库才退出**。所以等它退出就是等部署结束 ——
    /// 不等的话界面只能干转圈，永远不知道什么时候好。
    fn deploy(&self, install_dir: Option<&str>) -> AppResult<bool> {
        let dir = install_dir
            .map(PathBuf::from)
            .filter(|p| p.join("WeaselDeployer.exe").is_file())
            .or_else(win::find_program_dir)
            .ok_or_else(|| AppError::new(code::RIME_NOT_FOUND))?;

        let deployer = dir.join("WeaselDeployer.exe");
        let status = std::process::Command::new(&deployer)
            .arg("/deploy")
            .status()
            .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?;

        // 部署器有个独占锁：已经有一个在跑时它立刻退出并返回非 0。
        // 那不是失败，只是这次没轮到我们等，所以报「没确认」而不是报错。
        Ok(status.success())
    }

    fn stop_service(&self, _install_dir: Option<&str>) -> AppResult<()> {
        // 故意不用它自带的 `/q`：那条路径会把内存里的方案写回 user.yaml，
        // 正好盖掉我们马上要写进去的新方案。直接结束进程，让文件说了算。
        let _ = std::process::Command::new("taskkill")
            .args(["/IM", "WeaselServer.exe", "/F"])
            .status();
        std::thread::sleep(std::time::Duration::from_millis(400));
        Ok(())
    }

    fn start_service(&self, install_dir: Option<&str>) -> AppResult<()> {
        let dir = install_dir
            .map(PathBuf::from)
            .filter(|p| p.join("WeaselServer.exe").is_file())
            .or_else(win::find_program_dir)
            .ok_or_else(|| AppError::new(code::RIME_NOT_FOUND))?;

        std::process::Command::new(dir.join("WeaselServer.exe"))
            .spawn()
            .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?;
        Ok(())
    }

    fn uninstall(&self) -> AppResult<()> {
        let uninst = win::uninstall_string()
            .ok_or_else(|| AppError::new(code::UNINSTALLER_NOT_FOUND))?;
        let cleaned = uninst.trim().trim_matches('"').to_string();
        if !Path::new(&cleaned).is_file() {
            return Err(AppError::with(code::UNINSTALLER_MISSING, cleaned));
        }
        crate::download::run_as_admin(Path::new(&cleaned))
    }
}

// ─── macOS 实现 ───

#[cfg(target_os = "macos")]
pub struct MacOSPlatform;

#[cfg(target_os = "macos")]
impl MacOSPlatform {
    fn app_path() -> Option<PathBuf> {
        let system = PathBuf::from("/Library/Input Methods/Squirrel.app");
        if system.join("Contents/MacOS/Squirrel").is_file() {
            return Some(system);
        }
        let user = dirs::home_dir()?.join("Library/Input Methods/Squirrel.app");
        if user.join("Contents/MacOS/Squirrel").is_file() {
            return Some(user);
        }
        None
    }

    fn version(app: &Path) -> Option<String> {
        let plist = app.join("Contents/Info.plist");
        let out = std::process::Command::new("defaults")
            .arg("read")
            .arg(plist.with_extension("").to_string_lossy().to_string())
            .arg("CFBundleShortVersionString")
            .output()
            .ok()?;
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }
}

#[cfg(target_os = "macos")]
impl RimePlatform for MacOSPlatform {
    fn detect(&self) -> RimeInstallInfo {
        let config_dir = self.config_dir();
        let app = Self::app_path();
        let installed = app.is_some();

        RimeInstallInfo {
            version: app.as_deref().and_then(Self::version),
            installed,
            install_dir: app.map(|p| p.to_string_lossy().to_string()),
            has_leftover: !installed && has_rime_leftover(&config_dir),
            // 鼠须管是拖进 Input Methods 的，没有卸载程序，由 App 自己删
            can_uninstall: installed,
            config_dir: config_dir.to_string_lossy().to_string(),
            platform: "macos".to_string(),
        }
    }

    fn config_dir(&self) -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join("Library").join("Rime")
        } else {
            PathBuf::from("~/Library/Rime")
        }
    }

    fn deploy(&self, install_dir: Option<&str>) -> AppResult<bool> {
        let base = install_dir
            .map(PathBuf::from)
            .filter(|p| p.join("Contents/MacOS/Squirrel").is_file())
            .or_else(Self::app_path)
            .ok_or_else(|| AppError::new(code::RIME_NOT_FOUND))?;

        std::process::Command::new(base.join("Contents/MacOS/Squirrel"))
            .arg("--reload")
            .spawn()
            .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?;
        // 鼠须管的 --reload 只是发个通知就返回，真正的部署在 App 里跑，
        // 我们等不到结果，老实报「没确认」。
        Ok(false)
    }

    fn stop_service(&self, _install_dir: Option<&str>) -> AppResult<()> {
        std::process::Command::new("killall")
            .arg("Squirrel")
            .status()
            .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?;
        std::thread::sleep(std::time::Duration::from_millis(400));
        Ok(())
    }

    fn start_service(&self, _install_dir: Option<&str>) -> AppResult<()> {
        // 鼠须管由系统输入法框架托管，杀掉之后系统会自己把它拉起来
        Ok(())
    }

    fn uninstall(&self) -> AppResult<()> {
        let app = Self::app_path().ok_or_else(|| AppError::new(code::RIME_NOT_FOUND))?;
        // 先退出输入法进程，否则 .app 正在使用中删不掉
        let _ = std::process::Command::new("killall")
            .arg("Squirrel")
            .status();
        // 系统级目录要管理员权限，交给系统弹授权框
        let script = format!(
            "do shell script \"rm -rf '{}'\" with administrator privileges",
            app.to_string_lossy()
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .status()
            .map_err(|e| AppError::with(code::UNINSTALL_LAUNCH_FAILED, e))?;
        if !status.success() {
            return Err(AppError::new(code::UNINSTALL_CANCELLED));
        }
        Ok(())
    }
}

/// 获取当前平台实现
pub fn current_platform() -> Box<dyn RimePlatform + Send + Sync> {
    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsPlatform)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSPlatform)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        panic!("当前平台暂不支持")
    }
}

/// 把残留的配置目录改名备份，而不是直接删 —— 万一里面有用户自己加的词库
pub fn backup_leftover(config_dir: &str) -> AppResult<String> {
    let src = Path::new(config_dir);
    if !src.is_dir() {
        return Err(AppError::new(code::LEFTOVER_NOT_FOUND));
    }
    let parent = src.parent().ok_or_else(|| AppError::new(code::CONFIG_PATH_INVALID))?;
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Rime".into());

    // 同一天可能清理多次，加序号避免撞名
    for n in 0..100 {
        // 用中性的 ASCII 后缀：这是磁盘上的名字，界面却有四种语言
        let suffix = if n == 0 {
            "backup".to_string()
        } else {
            format!("backup{}", n + 1)
        };
        let dest = parent.join(format!("{}.{}", name, suffix));
        if dest.exists() {
            continue;
        }
        std::fs::rename(src, &dest)
            .map_err(|e| AppError::with(code::LEFTOVER_MOVE_FAILED, e))?;
        return Ok(dest.to_string_lossy().to_string());
    }
    Err(AppError::new(code::TOO_MANY_BACKUPS))
}

/// 系统是不是深色模式
///
/// 小狼毫会跟着系统切配色：深色下读 `style/color_scheme_dark`，浅色下读
/// `style/color_scheme`。写配置和读配置都得先知道现在是哪一边。
#[cfg(windows)]
pub fn system_dark_mode() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
        // 键不在就是浅色：Win10 1809 之前没有这一项
        .map(|v| v == 0)
        .unwrap_or(false)
}

/// 任务栏是不是深色
///
/// 跟 `system_dark_mode` 是两个注册表键：应用可以是浅色而任务栏是深色，
/// 反过来也行。状态图标是画在任务栏上的，要跟的是这一个。
#[cfg(windows)]
pub fn taskbar_dark_mode() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|k| k.get_value::<u32, _>("SystemUsesLightTheme"))
        .map(|v| v == 0)
        // 键不在就当深色：Win10 1809 之前任务栏本来就是深的
        .unwrap_or(true)
}

#[cfg(not(windows))]
pub fn system_dark_mode() -> bool {
    false
}

#[cfg(not(windows))]
pub fn taskbar_dark_mode() -> bool {
    false
}
