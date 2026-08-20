use crate::error::{code, AppError, AppResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// 输入法状态图标
///
/// 小狼毫会在托盘和候选框里画这几个图标，路径**先在用户配置目录找、再去程序目录找**
/// （见 RimeWithWeasel.cpp 的 `load_icon`）。所以不用改小狼毫的源码 ——
/// 把图片拷进配置目录，再往方案的 custom 里写一行就行。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IconKind {
    /// 中文状态
    Zhung,
    /// 英文（ASCII）状态
    Ascii,
    /// 全角
    Full,
    /// 半角
    Half,
}

impl IconKind {
    fn parse(s: &str) -> AppResult<Self> {
        match s {
            "zhung" => Ok(Self::Zhung),
            "ascii" => Ok(Self::Ascii),
            "full" => Ok(Self::Full),
            "half" => Ok(Self::Half),
            _ => Err(AppError::with(code::ICON_KIND_UNKNOWN, s)),
        }
    }

    /// 方案配置里的键名。中文状态那个键就叫 `schema/icon`，没有 zhung 前缀。
    fn patch_key(self) -> &'static str {
        match self {
            Self::Zhung => "schema/icon",
            Self::Ascii => "schema/ascii_icon",
            Self::Full => "schema/full_icon",
            Self::Half => "schema/half_icon",
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Self::Zhung => "zhung",
            Self::Ascii => "ascii",
            Self::Full => "full",
            Self::Half => "half",
        }
    }
}

const ALL: [IconKind; 4] = [
    IconKind::Zhung,
    IconKind::Ascii,
    IconKind::Full,
    IconKind::Half,
];

/// 小狼毫画的是 Win32 图标，`.ico` 最稳
const ALLOWED_EXT: [&str; 2] = ["ico", "png"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaIcon {
    pub kind: String,
    /// 配置里写的相对路径，空 = 没设
    pub path: String,
    /// 那个文件真的在不在
    pub exists: bool,
}

fn icons_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("icons")
}

/// 图标装到所有启用中的方案上
///
/// 小狼毫是按方案存图标的，但用户要的是「换了图标，打什么方案都是这个图标」。
/// 只装当前方案的话，一切到别的方案图标就变回自带的那个 —— 看着像坏了。
fn target_schemas(config_dir: &str) -> Vec<String> {
    let list = crate::dict::enabled_schemas(config_dir);
    if list.is_empty() {
        vec![crate::settings::detect_primary_schema(Path::new(config_dir))]
    } else {
        list
    }
}

#[tauri::command]
pub fn read_schema_icons(config_dir: String) -> AppResult<Vec<SchemaIcon>> {
    let dir = Path::new(&config_dir);
    let schema = crate::settings::current_schema(dir);
    let custom = dir.join(format!("{}.custom.yaml", schema));
    let patch = std::fs::read_to_string(&custom)
        .ok()
        .and_then(|raw| serde_yaml::from_str::<serde_yaml::Value>(&raw).ok())
        .and_then(|doc| doc.get("patch").cloned());

    Ok(ALL
        .iter()
        .map(|kind| {
            let path = patch
                .as_ref()
                .and_then(|p| p.get(kind.patch_key()))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            SchemaIcon {
                exists: !path.is_empty() && dir.join(&path).is_file(),
                kind: kind.slug().to_string(),
                path,
            }
        })
        .collect())
}

/// 把用户挑的图片拷进配置目录，并写进方案的 custom
#[tauri::command]
pub fn set_schema_icon(
    config_dir: String,
    kind: String,
    source: String,
) -> AppResult<Vec<SchemaIcon>> {
    let kind = IconKind::parse(&kind)?;
    let src = PathBuf::from(&source);

    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .filter(|e| ALLOWED_EXT.contains(&e.as_str()))
        .ok_or_else(|| AppError::with(code::ICON_FORMAT_UNSUPPORTED, source.clone()))?;

    if !src.is_file() {
        return Err(AppError::with(code::FILE_CREATE_FAILED, source));
    }

    let dir = Path::new(&config_dir);
    let target_dir = icons_dir(dir);
    std::fs::create_dir_all(&target_dir).map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;

    // 所有方案共用同一份文件，名字不带方案
    let name = format!("xgrime-{}.{}", kind.slug(), ext);
    std::fs::copy(&src, target_dir.join(&name))
        .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

    // 小狼毫是按「相对配置目录」来找的，所以这里写相对路径
    let relative = format!("icons/{}", name);
    for schema in target_schemas(&config_dir) {
        crate::settings::merge_patch(
            &dir.join(format!("{}.custom.yaml", schema)),
            [(
                kind.patch_key().to_string(),
                serde_yaml::Value::String(relative.clone()),
            )],
            &[],
        )?;
    }

    read_schema_icons(config_dir)
}

#[tauri::command]
pub fn clear_schema_icon(config_dir: String, kind: String) -> AppResult<Vec<SchemaIcon>> {
    let kind = IconKind::parse(&kind)?;
    let dir = Path::new(&config_dir);

    for schema in target_schemas(&config_dir) {
        crate::settings::merge_patch(
            &dir.join(format!("{}.custom.yaml", schema)),
            [],
            &[kind.patch_key()],
        )?;
    }

    // 拷进来的那份也删掉，别在目录里留垃圾。
    // 两种名字都扫：`xgrime-<用途>` 是现在的，`xgrime-<方案>-<用途>` 是早期按方案存的。
    if let Ok(entries) = std::fs::read_dir(icons_dir(dir)) {
        let slug = kind.slug();
        for e in entries.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("xgrime-") && name.contains(slug) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    read_schema_icons(config_dir)
}

/// 内置的一套状态图标
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconSet {
    pub id: String,
    /// 四个状态是不是都齐了
    pub complete: bool,
}

/// 跟应用一起打包的图标套，见 scripts/gen-status-icons.py
const BUILTIN_SETS: [&str; 4] = ["plain_dark", "plain_light", "badge_blue", "badge_ink"];
/// 「跟着任务栏深浅自动换」不是某一套图标，是个选择
pub const AUTO: &str = "auto";

/// 任务栏深色就用白字那套，浅色就用黑字那套
fn resolve(set: &str) -> String {
    if set != AUTO {
        return set.to_string();
    }
    if crate::platform::taskbar_dark_mode() {
        "plain_light".into()
    } else {
        "plain_dark".into()
    }
}

fn builtin_dir(app: &tauri::AppHandle, set: &str) -> AppResult<PathBuf> {
    use tauri::Manager;
    app.path()
        .resolve(
            format!("resources/status-icons/{}", set),
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| AppError::with(code::ICON_SET_UNKNOWN, e))
}

#[tauri::command]
pub fn list_builtin_icon_sets(app: tauri::AppHandle) -> AppResult<Vec<IconSet>> {
    Ok(BUILTIN_SETS
        .iter()
        .map(|id| {
            let complete = builtin_dir(&app, id)
                .map(|d| ALL.iter().all(|k| d.join(format!("{}.ico", k.slug())).is_file()))
                .unwrap_or(false);
            IconSet {
                id: (*id).to_string(),
                complete,
            }
        })
        .collect())
}

/// 清掉不再被引用的图标文件
///
/// 早期是按方案存的（`xgrime-<方案>-<用途>.ico`），现在所有方案共用
/// `xgrime-<用途>.ico`。装新的时候把旧命名那批扫掉，别在目录里烂着。
fn prune_stale_icons(config_dir: &Path, keep: &[String]) {
    let Ok(entries) = std::fs::read_dir(icons_dir(config_dir)) else {
        return;
    };
    for e in entries.filter_map(|e| e.ok()) {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("xgrime-") && !keep.iter().any(|k| k == &name) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// 一次把整套内置图标装上
#[tauri::command]
pub fn apply_builtin_icon_set(
    app: tauri::AppHandle,
    config_dir: String,
    set: String,
) -> AppResult<Vec<SchemaIcon>> {
    if set != AUTO && !BUILTIN_SETS.contains(&set.as_str()) {
        return Err(AppError::with(code::ICON_SET_UNKNOWN, set));
    }
    let actual = resolve(&set);
    let src_dir = builtin_dir(&app, &actual)?;
    let dir = Path::new(&config_dir);
    let target_dir = icons_dir(dir);
    std::fs::create_dir_all(&target_dir).map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;

    let mut patch = Vec::new();
    let mut keep = Vec::new();
    for kind in ALL {
        let src = src_dir.join(format!("{}.ico", kind.slug()));
        if !src.is_file() {
            return Err(AppError::with(code::ICON_SET_UNKNOWN, src.display().to_string()));
        }
        let name = format!("xgrime-{}.ico", kind.slug());
        std::fs::copy(&src, target_dir.join(&name))
            .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
        keep.push(name.clone());
        // 小狼毫是按「相对配置目录」找的，所以写相对路径
        patch.push((
            kind.patch_key().to_string(),
            serde_yaml::Value::String(format!("icons/{}", name)),
        ));
    }

    for schema in target_schemas(&config_dir) {
        crate::settings::merge_patch(
            &dir.join(format!("{}.custom.yaml", schema)),
            patch.clone(),
            &[],
        )?;
    }
    prune_stale_icons(dir, &keep);
    // 记下选的是哪一套（可能是 auto）和实际装的是哪一套 —— 下次开机比对用
    crate::prefs::write_icon_pref(&crate::prefs::IconPref {
        mode: set,
        applied: actual,
    })?;
    read_schema_icons(config_dir)
}

/// 开机 / 打开这一页时对一次：选了「自动」而任务栏深浅变了，就悄悄换过来
///
/// 返回 true 表示真换了，调用方据此决定要不要重新部署。
#[tauri::command]
pub fn sync_status_icons(app: tauri::AppHandle, config_dir: String) -> AppResult<bool> {
    let pref = crate::prefs::read_icon_pref()?;
    if pref.mode != AUTO {
        return Ok(false);
    }
    let want = resolve(AUTO);
    if want == pref.applied {
        return Ok(false);
    }
    apply_builtin_icon_set(app, config_dir, AUTO.to_string())?;
    Ok(true)
}

/// 应用一启动就对一次
///
/// 只在这一页里对的话，用户得先打开 XGRime 再点进状态图标，「自动」名不副实。
/// 放到启动时，配上设置里的开机自启，开机就跟着任务栏换好了。
///
/// 扔到后台线程：这里要读注册表、拷文件、还可能触发一次部署，不能挡着开窗。
pub fn sync_on_startup(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let platform = crate::platform::current_platform();
        let info = platform.detect();
        if !info.installed {
            return;
        }
        if let Ok(true) = sync_status_icons(app, info.config_dir) {
            let _ = platform.deploy(info.install_dir.as_deref());
        }
    });
}

/// 四个状态一起清掉
#[tauri::command]
pub fn clear_all_schema_icons(config_dir: String) -> AppResult<Vec<SchemaIcon>> {
    crate::prefs::write_icon_pref(&crate::prefs::IconPref::default())?;
    for kind in ALL {
        clear_schema_icon(config_dir.clone(), kind.slug().to_string())?;
    }
    read_schema_icons(config_dir)
}
