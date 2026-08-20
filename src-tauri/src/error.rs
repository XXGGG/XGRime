use serde::Serialize;
use std::fmt::Display;

/// 抛给前端的错误
///
/// 只给**错误码**，不给人话 —— 界面有四种语言，这句话该说成哪国话是界面的事。
/// `detail` 放技术细节（系统报的原文、路径、HTTP 状态码），这类东西本来也翻不了，
/// 直接原样显示在提示的第二行。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: &'static str) -> Self {
        Self { code, detail: None }
    }

    pub fn with(code: &'static str, detail: impl Display) -> Self {
        Self {
            code,
            detail: Some(detail.to_string()),
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(d) => write!(f, "{}: {}", self.code, d),
            None => write!(f, "{}", self.code),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// 全部错误码。
///
/// 四份界面语言里都必须有对应的 `errors.<码>`，`pnpm check:locales` 会照着这里核对，
/// 少一条构建就不过 —— 免得某个语言下用户看到的是一串 camelCase。
pub mod code {
    // 平台专属的码在另一个系统上编译时用不到，不算问题
    #![allow(dead_code)]

    // 安装 / 检测
    pub const RIME_NOT_FOUND: &str = "rimeNotFound";
    pub const DEPLOYER_LAUNCH_FAILED: &str = "deployerLaunchFailed";
    pub const UNINSTALLER_NOT_FOUND: &str = "uninstallerNotFound";
    pub const UNINSTALLER_MISSING: &str = "uninstallerMissing";
    pub const UNINSTALL_LAUNCH_FAILED: &str = "uninstallLaunchFailed";
    pub const UNINSTALL_CANCELLED: &str = "uninstallCancelled";
    pub const UAC_DENIED: &str = "uacDenied";
    pub const LAUNCH_FAILED: &str = "launchFailed";

    // 残留清理
    pub const LEFTOVER_NOT_FOUND: &str = "leftoverNotFound";
    pub const CONFIG_PATH_INVALID: &str = "configPathInvalid";
    pub const LEFTOVER_MOVE_FAILED: &str = "leftoverMoveFailed";
    pub const TOO_MANY_BACKUPS: &str = "tooManyBackups";

    // 网络
    pub const HTTP_CLIENT_FAILED: &str = "httpClientFailed";
    pub const GITHUB_REQUEST_FAILED: &str = "githubRequestFailed";
    pub const GITHUB_PARSE_FAILED: &str = "githubParseFailed";
    pub const INSTALLER_ASSET_NOT_FOUND: &str = "installerAssetNotFound";
    pub const DOWNLOAD_FAILED: &str = "downloadFailed";
    pub const DOWNLOAD_INTERRUPTED: &str = "downloadInterrupted";
    pub const UPDATE_CHECK_FAILED: &str = "updateCheckFailed";

    // 文件
    pub const TEMP_DIR_FAILED: &str = "tempDirFailed";
    pub const TEMP_FILE_FAILED: &str = "tempFileFailed";
    pub const DIR_CREATE_FAILED: &str = "dirCreateFailed";
    pub const FILE_CREATE_FAILED: &str = "fileCreateFailed";
    pub const FILE_WRITE_FAILED: &str = "fileWriteFailed";
    pub const FILE_DELETE_FAILED: &str = "fileDeleteFailed";
    pub const CONFIG_DIR_CREATE_FAILED: &str = "configDirCreateFailed";

    // 压缩包
    pub const ZIP_OPEN_FAILED: &str = "zipOpenFailed";
    pub const ZIP_CORRUPT: &str = "zipCorrupt";
    pub const ZIP_ENTRY_FAILED: &str = "zipEntryFailed";

    // 备份
    pub const FILE_READ_FAILED: &str = "fileReadFailed";
    pub const BACKUP_EMPTY: &str = "backupEmpty";
    pub const BACKUP_UNREADABLE: &str = "backupUnreadable";
    pub const BACKUP_NOT_XGRIME: &str = "backupNotXgrime";
    pub const BACKUP_TOO_NEW: &str = "backupTooNew";
    pub const BACKUP_UNSAFE_PATH: &str = "backupUnsafePath";

    // 系统
    pub const AUTOSTART_FAILED: &str = "autostartFailed";
    pub const SETTING_PAGE_UNKNOWN: &str = "settingPageUnknown";

    // 方案与配置
    pub const SCHEMA_NOT_FOUND: &str = "schemaNotFound";
    pub const SCHEMA_NOT_REMOVABLE: &str = "schemaNotRemovable";
    pub const SCHEMA_NOT_ENABLED: &str = "schemaNotEnabled";
    pub const SCHEMA_READ_FAILED: &str = "schemaReadFailed";
    pub const SCHEMA_PARSE_FAILED: &str = "schemaParseFailed";
    pub const MANIFEST_WRITE_FAILED: &str = "manifestWriteFailed";
    pub const YAML_SERIALIZE_FAILED: &str = "yamlSerializeFailed";
    pub const FONT_LIST_FAILED: &str = "fontListFailed";
    pub const PRESETS_UNREADABLE: &str = "presetsUnreadable";
    pub const ICON_KIND_UNKNOWN: &str = "iconKindUnknown";
    pub const ICON_SET_UNKNOWN: &str = "iconSetUnknown";
    pub const ICON_FORMAT_UNSUPPORTED: &str = "iconFormatUnsupported";
}
