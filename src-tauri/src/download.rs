use crate::error::{code, AppError, AppResult};
use tauri::{AppHandle, Emitter};
use serde::Deserialize;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    percentage: f64,
}

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
    tag_name: String,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

/// 从 GitHub API 获取最新 release 中的安装包 URL
async fn get_installer_url() -> AppResult<(String, String, u64)> {
    #[cfg(target_os = "windows")]
    let api_url = "https://api.github.com/repos/rime/weasel/releases/latest";
    #[cfg(target_os = "macos")]
    let api_url = "https://api.github.com/repos/rime/squirrel/releases/latest";

    let client = reqwest::Client::builder()
        .user_agent("XGRime/0.1.0")
        .build()
        .map_err(|e| AppError::with(code::HTTP_CLIENT_FAILED, e))?;

    let release: GithubRelease = client
        .get(api_url)
        .send()
        .await
        .map_err(|e| AppError::with(code::GITHUB_REQUEST_FAILED, e))?
        .json()
        .await
        .map_err(|e| AppError::with(code::GITHUB_PARSE_FAILED, e))?;

    // 找到安装包 asset
    let asset = release.assets.iter().find(|a| {
        #[cfg(target_os = "windows")]
        { a.name.ends_with("-installer.exe") }
        #[cfg(target_os = "macos")]
        { a.name.ends_with(".pkg") }
    }).ok_or_else(|| AppError::with(code::INSTALLER_ASSET_NOT_FOUND, &release.tag_name))?;

    Ok((asset.browser_download_url.clone(), asset.name.clone(), asset.size))
}

#[tauri::command]
pub async fn download_rime(app: AppHandle) -> AppResult<String> {
    // 1. 获取最新安装包 URL
    let _ = app.emit("rime-download-progress", DownloadProgress {
        downloaded: 0, total: 0, percentage: 0.0,
    });

    let (url, filename, expected_size) = get_installer_url().await?;

    // 2. 准备下载目录：系统临时目录/XGRime/
    let temp_dir = std::env::temp_dir().join("XGRime");
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| AppError::with(code::TEMP_DIR_FAILED, e))?;
    }
    let file_path = temp_dir.join(&filename);

    // 如果已经下载过同名文件且大小匹配，直接使用
    if file_path.exists() {
        if let Ok(meta) = std::fs::metadata(&file_path) {
            if meta.len() == expected_size {
                run_as_admin(&file_path)?;
                return Ok(file_path.to_string_lossy().to_string());
            }
        }
        // 文件不完整，删除重下
        let _ = std::fs::remove_file(&file_path);
    }

    // 3. 流式下载
    let client = reqwest::Client::builder()
        .user_agent("XGRime/0.1.0")
        .build()
        .map_err(|e| AppError::with(code::HTTP_CLIENT_FAILED, e))?;

    let resp = client.get(&url)
        .send()
        .await
        .map_err(|e| AppError::with(code::DOWNLOAD_FAILED, e))?;

    let total = resp.content_length().unwrap_or(expected_size);
    let mut downloaded: u64 = 0;

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut file = std::fs::File::create(&file_path)
        .map_err(|e| AppError::with(code::FILE_CREATE_FAILED, e))?;

    use std::io::Write;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::with(code::DOWNLOAD_INTERRUPTED, e))?;
        file.write_all(&chunk)
            .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

        downloaded += chunk.len() as u64;
        let percentage = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let _ = app.emit("rime-download-progress", DownloadProgress {
            downloaded,
            total,
            percentage,
        });
    }

    drop(file);

    // 4. 启动安装器
    run_as_admin(&file_path)?;

    Ok(file_path.to_string_lossy().to_string())
}

/// 以管理员身份运行一个程序（安装器 / 卸载器都走这里）
///
/// Windows 用 ShellExecuteW 的 "runas" 动词触发 UAC；macOS 直接 `open`。
pub fn run_as_admin(path: &std::path::Path) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::ffi::OsStr;

        // ShellExecuteW 使用 "runas" 动词来请求管理员权限（UAC 提示）
        let operation: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();
        let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let null_ptr = std::ptr::null();

        let result = unsafe {
            winapi::um::shellapi::ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                null_ptr,
                null_ptr,
                winapi::um::winuser::SW_SHOWNORMAL,
            )
        };

        // ShellExecuteW 返回值 <= 32 表示失败，其中 5 是「用户在 UAC 弹窗点了否」
        if (result as isize) <= 32 {
            return if (result as isize) == 5 {
                Err(AppError::new(code::UAC_DENIED))
            } else {
                Err(AppError::with(code::LAUNCH_FAILED, result as isize))
            };
        }
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::with(code::LAUNCH_FAILED, e))?;
    }
    Ok(())
}
