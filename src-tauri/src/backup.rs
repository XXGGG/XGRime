//! 把设置打包带走，换台机器再装回来。
//!
//! 只收「用户自己调出来的东西」：各种 `*.custom.yaml`、状态图标、XGRime 存的预设。
//! 词库本体和编译产物一概不收 —— 那些几十上百 MB，装回去也是重新下一遍更省事。

use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, Write};
use std::path::{Component, Path, PathBuf};

/// 备份里分几类，导出导入都按类勾选
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupParts {
    /// 候选框外观：weasel/squirrel 的 custom 加状态图标，再加 XGRime 自己存的预设
    pub theme: bool,
    /// 方案选单、按键、模糊音这些：default.custom.yaml 和各方案的 custom
    pub schemas: bool,
    /// 自造词
    pub phrases: bool,
    /// 打字积累的词频。几十 MB 且跟着方案版本走，默认不带
    pub userdb: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// 备份格式版本，将来改结构靠它判断
    pub format: u32,
    /// 导出时的 XGRime 版本
    pub app: String,
    pub parts: BackupParts,
    pub files: Vec<String>,
}

const FORMAT: u32 = 1;
const MANIFEST: &str = "xgrime-backup.json";
/// XGRime 存的用户预设，不在 RIME 配置目录里，单独收进来
const PRESETS_IN_ZIP: &str = "xgrime/presets.json";

fn is_theme_config(name: &str) -> bool {
    name == "weasel.custom.yaml" || name == "squirrel.custom.yaml"
}

fn is_schema_config(name: &str) -> bool {
    name.ends_with(".custom.yaml") && !is_theme_config(name)
}

/// 挑出该收进备份的文件，返回「zip 里的相对路径 → 磁盘绝对路径」
fn collect(config_dir: &Path, parts: BackupParts) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();

    let entries: Vec<String> = std::fs::read_dir(config_dir)
        .map(|it| {
            it.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();

    for name in &entries {
        let path = config_dir.join(name);
        if !path.is_file() {
            continue;
        }
        let take = (parts.theme && is_theme_config(name))
            || (parts.schemas && is_schema_config(name))
            || (parts.phrases && name == "custom_phrase.txt");
        if take {
            out.push((name.clone(), path));
        }
    }

    if parts.theme {
        push_dir(&mut out, config_dir, "icons");
        if let Some(p) = crate::prefs::presets_path() {
            if p.is_file() {
                out.push((PRESETS_IN_ZIP.to_string(), p));
            }
        }
    }
    if parts.userdb {
        for name in &entries {
            if name.ends_with(".userdb") {
                push_dir(&mut out, config_dir, name);
            }
        }
    }

    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn push_dir(out: &mut Vec<(String, PathBuf)>, config_dir: &Path, name: &str) {
    let root = config_dir.join(name);
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Ok(rel) = p.strip_prefix(config_dir) {
                out.push((rel.to_string_lossy().replace('\\', "/"), p));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

#[tauri::command]
pub fn export_settings(
    config_dir: String,
    target: String,
    parts: BackupParts,
) -> AppResult<ExportSummary> {
    let dir = Path::new(&config_dir);
    let picked = collect(dir, parts);
    if picked.is_empty() {
        return Err(AppError::new(code::BACKUP_EMPTY));
    }

    let file = std::fs::File::create(&target).map_err(|e| AppError::with(code::FILE_CREATE_FAILED, e))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let manifest = Manifest {
        format: FORMAT,
        app: env!("CARGO_PKG_VERSION").to_string(),
        parts,
        files: picked.iter().map(|(n, _)| n.clone()).collect(),
    };
    zip.start_file(MANIFEST, opts)
        .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
    zip.write_all(
        serde_json::to_string_pretty(&manifest)
            .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?
            .as_bytes(),
    )
    .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

    for (name, path) in &picked {
        let body = std::fs::read(path).map_err(|e| AppError::with(code::FILE_READ_FAILED, e))?;
        zip.start_file(name.as_str(), opts)
            .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
        zip.write_all(&body)
            .map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
    }
    zip.finish().map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

    let bytes = std::fs::metadata(&target).map(|m| m.len()).unwrap_or(0);
    Ok(ExportSummary {
        path: target,
        files: picked.len(),
        bytes,
    })
}

fn open_zip(path: &str) -> AppResult<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path).map_err(|e| AppError::with(code::FILE_READ_FAILED, e))?;
    zip::ZipArchive::new(file).map_err(|e| AppError::with(code::BACKUP_UNREADABLE, e))
}

fn read_manifest<R: Read + Seek>(zip: &mut zip::ZipArchive<R>) -> AppResult<Manifest> {
    let mut entry = zip
        .by_name(MANIFEST)
        .map_err(|_| AppError::new(code::BACKUP_NOT_XGRIME))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| AppError::with(code::BACKUP_UNREADABLE, e))?;
    let m: Manifest =
        serde_json::from_str(&text).map_err(|e| AppError::with(code::BACKUP_UNREADABLE, e))?;
    if m.format > FORMAT {
        return Err(AppError::with(code::BACKUP_TOO_NEW, m.format.to_string()));
    }
    Ok(m)
}

/// 先看看这个包里有什么，别一上来就往用户目录里写
#[tauri::command]
pub fn inspect_backup(path: String) -> AppResult<Manifest> {
    read_manifest(&mut open_zip(&path)?)
}

/// zip 里的路径不能跳出配置目录（`../`、绝对路径、盘符一律拒绝）
fn safe_relative(relative: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in Path::new(relative).components() {
        match c {
            Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub files: usize,
    /// 覆盖前把原文件挪去了这里，空表示没有东西被覆盖
    pub backup_dir: String,
}

#[tauri::command]
pub fn import_settings(
    config_dir: String,
    path: String,
    parts: BackupParts,
) -> AppResult<ImportSummary> {
    let dir = Path::new(&config_dir);
    let mut zip = open_zip(&path)?;
    let manifest = read_manifest(&mut zip)?;

    // 覆盖之前先把原来的挪走。用户按错了还能自己捞回来
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_dir = dir.join(format!("xgrime-导入前备份-{}", stamp));
    let mut backed_up = false;
    let mut written = 0usize;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AppError::with(code::BACKUP_UNREADABLE, e))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if name == MANIFEST {
            continue;
        }
        if !wanted(&name, parts, &manifest) {
            continue;
        }

        let target = if name == PRESETS_IN_ZIP {
            match crate::prefs::presets_path() {
                Some(p) => p,
                None => continue,
            }
        } else {
            let Some(rel) = safe_relative(&name) else {
                return Err(AppError::with(code::BACKUP_UNSAFE_PATH, name));
            };
            dir.join(rel)
        };

        if target.is_file() {
            let rel = target.strip_prefix(dir).unwrap_or(Path::new(&name));
            let keep = backup_dir.join(rel);
            if let Some(parent) = keep.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;
            }
            std::fs::copy(&target, &keep).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
            backed_up = true;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;
        }
        let mut body = Vec::new();
        entry
            .read_to_end(&mut body)
            .map_err(|e| AppError::with(code::BACKUP_UNREADABLE, e))?;
        std::fs::write(&target, &body).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;
        written += 1;
    }

    if written == 0 {
        return Err(AppError::new(code::BACKUP_EMPTY));
    }

    Ok(ImportSummary {
        files: written,
        backup_dir: if backed_up {
            backup_dir.to_string_lossy().to_string()
        } else {
            String::new()
        },
    })
}

/// 这个文件属不属于用户勾选的那几类
fn wanted(name: &str, parts: BackupParts, manifest: &Manifest) -> bool {
    let base = name.rsplit('/').next().unwrap_or(name);
    if name == PRESETS_IN_ZIP || name.starts_with("icons/") {
        return parts.theme && manifest.parts.theme;
    }
    if name.contains(".userdb/") {
        return parts.userdb && manifest.parts.userdb;
    }
    if base == "custom_phrase.txt" {
        return parts.phrases && manifest.parts.phrases;
    }
    if is_theme_config(base) {
        return parts.theme && manifest.parts.theme;
    }
    if is_schema_config(base) {
        return parts.schemas && manifest.parts.schemas;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts_all() -> BackupParts {
        BackupParts {
            theme: true,
            schemas: true,
            phrases: true,
            userdb: true,
        }
    }

    #[test]
    fn theme_and_schema_configs_go_to_different_buckets() {
        assert!(is_theme_config("weasel.custom.yaml"));
        assert!(is_theme_config("squirrel.custom.yaml"));
        assert!(!is_schema_config("weasel.custom.yaml"));
        assert!(is_schema_config("default.custom.yaml"));
        assert!(is_schema_config("rime_ice.custom.yaml"));
        assert!(!is_schema_config("rime_ice.schema.yaml"));
    }

    /// 只勾了皮肤就别把方案设置也盖掉 —— 那是两码事，混着导会把人坑惨
    #[test]
    fn import_only_touches_the_checked_buckets() {
        let m = Manifest {
            format: FORMAT,
            app: "test".into(),
            parts: parts_all(),
            files: vec![],
        };
        let only_theme = BackupParts {
            theme: true,
            schemas: false,
            phrases: false,
            userdb: false,
        };
        assert!(wanted("weasel.custom.yaml", only_theme, &m));
        assert!(wanted("icons/xgrime-rime_ice-zhung.ico", only_theme, &m));
        assert!(wanted(PRESETS_IN_ZIP, only_theme, &m));
        assert!(!wanted("default.custom.yaml", only_theme, &m));
        assert!(!wanted("custom_phrase.txt", only_theme, &m));
        assert!(!wanted("rime_ice.userdb/data.mdb", only_theme, &m));
    }

    /// 包里没带的东西，勾了也变不出来
    #[test]
    fn cannot_import_what_the_backup_does_not_have() {
        let m = Manifest {
            format: FORMAT,
            app: "test".into(),
            parts: BackupParts {
                theme: true,
                schemas: false,
                phrases: false,
                userdb: false,
            },
            files: vec![],
        };
        assert!(!wanted("default.custom.yaml", parts_all(), &m));
        assert!(wanted("weasel.custom.yaml", parts_all(), &m));
    }

    #[test]
    fn zip_slip_is_blocked() {
        assert!(safe_relative("../../evil.yaml").is_none());
        assert!(safe_relative("/etc/passwd").is_none());
        assert!(safe_relative("C:/Windows/x.yaml").is_none());
        assert!(safe_relative("icons/ok.ico").is_some());
    }
}
