use crate::error::{code, AppError, AppResult};
use crate::platform::current_platform;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployOutcome {
    /// 我们确实等到部署跑完了。false 表示只是发了指令，
    /// 结束时间不知道（macOS，或者已经有另一次部署在跑）。
    pub confirmed: bool,
}

/// 重新部署
///
/// 编译大词库要几十秒，所以扔到阻塞线程池里等，别占着 Tauri 的执行线程。
#[tauri::command]
pub async fn deploy_rime() -> AppResult<DeployOutcome> {
    let confirmed = tauri::async_runtime::spawn_blocking(|| {
        let platform = current_platform();
        let info = platform.detect();
        platform.deploy(info.install_dir.as_deref())
    })
    .await
    .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))??;

    Ok(DeployOutcome { confirmed })
}

/// 停掉输入法后台服务
///
/// 换输入方案的第一步。**不能先改文件再停服** —— 服务退出时会把内存里的
/// 当前方案写回 user.yaml，把刚写进去的新方案盖掉。
#[tauri::command]
pub async fn stop_rime_service() -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(|| {
        let platform = current_platform();
        let info = platform.detect();
        platform.stop_service(info.install_dir.as_deref())
    })
    .await
    .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?
}

/// 启动输入法后台服务
#[tauri::command]
pub async fn start_rime_service() -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(|| {
        let platform = current_platform();
        let info = platform.detect();
        platform.start_service(info.install_dir.as_deref())
    })
    .await
    .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?
}
