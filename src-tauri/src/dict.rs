use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// 一个方案要下载的仓库来源
///
/// RIME 的方案很少是自给自足的：`jyut6ping3.schema.yaml` 里引用了
/// `luna_pinyin` / `stroke` / `cangjie5` / `loengfan` 四本反查词典，
/// 少一本部署就报错。官方是用 plum 按配方装的，这里把配方内联进来。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictSource {
    pub repo: String,
    pub branch: String,
}

impl DictSource {
    fn zip_url(&self) -> String {
        format!(
            "https://github.com/{}/archive/refs/heads/{}.zip",
            self.repo, self.branch
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictInfo {
    pub id: String,
    /// RIME 里的方案标识，也是 schema_list 里写的那个
    pub schema_id: String,
    /// 大类：sound = 按读音打，shape = 按字形打，extra = 进阶 / 小众
    pub group: String,
    /// 小类键，显示名由前端按界面语言翻译
    pub category: String,
    /// 首屏「新手推荐」里露出的那几个
    pub recommended: bool,
    pub homepage: String,
    pub sources: Vec<DictSource>,
    /// 全部来源加起来要下多少字节，UI 上直接告诉用户
    pub total_bytes: u64,
    pub installed: bool,
    /// 卸得掉吗。输入法自带的方案文件在程序目录里，那是它的安装内容，
    /// 我们既没权也没道理去删 —— 对这种只给「停用」，不给「卸载」。
    pub removable: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
    /// 第几个来源（从 1 开始）/ 一共几个
    pub step: usize,
    pub step_total: usize,
    pub step_name: String,
}

/// 安装清单：装了哪些文件，卸载时照着删；顺带记下装的是哪一版
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    dict_id: String,
    schema_id: String,
    /// 相对配置目录的路径
    files: Vec<String>,
    /// 主仓库（sources 的最后一个）装的时候是哪个提交。
    /// 只跟主仓库，是因为依赖包几乎不动，而查一次要花一次 GitHub 匿名额度。
    #[serde(default)]
    main_repo: String,
    #[serde(default)]
    main_branch: String,
    #[serde(default)]
    main_sha: Option<String>,
}

/// 所有会用到的上游包。体积是实测的压缩包大小，不是拍脑袋写的。
///
/// key 只在本文件内部用来串方案和包，不出现在界面上。
const PACKAGES: &[(&str, &str, &str, u64)] = &[
    // key,             仓库,                        分支,      字节数
    ("prelude",         "rime/rime-prelude",         "master",    19_763),
    ("essay",           "rime/rime-essay",           "master", 2_577_546),
    ("luna",            "rime/rime-luna-pinyin",     "master",   397_373),
    ("stroke",          "rime/rime-stroke",          "master", 1_067_820),
    ("pinyin-simp",     "rime/rime-pinyin-simp",     "master",   545_626),
    ("terra",           "rime/rime-terra-pinyin",    "master",   732_440),
    ("bopomofo",        "rime/rime-bopomofo",        "master",    11_875),
    ("double-pinyin",   "rime/rime-double-pinyin",   "master",    23_377),
    ("cangjie",         "rime/rime-cangjie",         "master",   402_527),
    ("quick",           "rime/rime-quick",           "master",    39_755),
    ("scj",             "rime/rime-scj",             "master",   228_180),
    ("array",           "rime/rime-array",           "master", 1_719_828),
    ("wubi",            "rime/rime-wubi",            "master", 1_095_147),
    ("wugniu",          "rime/rime-wugniu",          "master",   267_995),
    ("soutzoe",         "rime/rime-soutzoe",         "master",    36_961),
    ("middle-chinese",  "rime/rime-middle-chinese",  "master",   137_937),
    ("ipa",             "rime/rime-ipa",             "master",    10_864),
    ("combo",           "rime/rime-combo-pinyin",    "master",    28_496),
    ("jyutping",        "rime/rime-jyutping",        "master", 3_125_665),
    ("cantonese",       "rime/rime-cantonese",       "main",   4_816_833),
    ("emoji",           "rime/rime-emoji",           "master",    53_270),
    ("emoji-cantonese", "rime/rime-emoji-cantonese", "master",    47_709),
    ("loengfan",        "CanCLID/rime-loengfan",     "main",     563_384),
    ("rime-ice",        "iDvel/rime-ice",            "main",  16_934_146),
    ("oh-my-rime",      "Mintimate/oh-my-rime",      "main",  24_679_433),
];

fn package(key: &str) -> &'static (&'static str, &'static str, &'static str, u64) {
    PACKAGES
        .iter()
        .find(|p| p.0 == key)
        .unwrap_or_else(|| panic!("方案表引用了不存在的包：{}", key))
}

struct Entry {
    id: &'static str,
    schema_id: &'static str,
    group: &'static str,
    category: &'static str,
    recommended: bool,
    home: &'static str,
    deps: &'static [&'static str],
}

/// 裸方案仓库都要这两个垫底：prelude 给 default.yaml / symbols / punctuation，
/// essay 给词频语料。自带完整配置的发行版（雾凇 / 薄荷）不需要。
const BASE: [&str; 2] = ["prelude", "essay"];

/// 方案表
///
/// `deps` 是逐个翻 `schema.yaml` 核对出来的真实依赖，不是猜的 ——
/// 方案里 `dictionary:` / `opencc_config:` 点名要的东西，少一样 RIME 部署就报错。
/// 几个容易想当然的地方：明月拼音和所有双拼都要 `stroke`（笔画反查），
/// 五笔要的是 `pinyin_simp` 而不是 `luna_pinyin`，行列要 `emoji`。
///
/// 顺序即解压顺序，主方案永远放最后，免得被依赖包里的同名文件盖掉。
const ENTRIES: &[Entry] = &[
    // ─────────── 按读音打 ───────────
    Entry { id: "rime-ice", schema_id: "rime_ice", group: "sound", category: "mandarin",
        recommended: true, home: "https://github.com/iDvel/rime-ice",
        deps: &["rime-ice"] },
    Entry { id: "mint", schema_id: "mint", group: "sound", category: "mandarin",
        recommended: false, home: "https://github.com/Mintimate/oh-my-rime",
        deps: &["oh-my-rime"] },
    Entry { id: "luna-pinyin", schema_id: "luna_pinyin", group: "sound", category: "mandarin",
        recommended: false, home: "https://github.com/rime/rime-luna-pinyin",
        deps: &["stroke", "luna"] },
    Entry { id: "pinyin-simp", schema_id: "pinyin_simp", group: "sound", category: "mandarin",
        recommended: false, home: "https://github.com/rime/rime-pinyin-simp",
        deps: &["stroke", "pinyin-simp"] },
    Entry { id: "terra-pinyin", schema_id: "terra_pinyin", group: "sound", category: "mandarin",
        recommended: false, home: "https://github.com/rime/rime-terra-pinyin",
        deps: &["stroke", "terra"] },

    Entry { id: "double-pinyin-flypy", schema_id: "double_pinyin_flypy", group: "sound", category: "double",
        recommended: false, home: "https://github.com/rime/rime-double-pinyin",
        deps: &["stroke", "luna", "double-pinyin"] },
    Entry { id: "double-pinyin-mspy", schema_id: "double_pinyin_mspy", group: "sound", category: "double",
        recommended: false, home: "https://github.com/rime/rime-double-pinyin",
        deps: &["stroke", "luna", "double-pinyin"] },
    Entry { id: "double-pinyin-ziran", schema_id: "double_pinyin", group: "sound", category: "double",
        recommended: false, home: "https://github.com/rime/rime-double-pinyin",
        deps: &["stroke", "luna", "double-pinyin"] },
    Entry { id: "double-pinyin-abc", schema_id: "double_pinyin_abc", group: "sound", category: "double",
        recommended: false, home: "https://github.com/rime/rime-double-pinyin",
        deps: &["stroke", "luna", "double-pinyin"] },
    Entry { id: "double-pinyin-pyjj", schema_id: "double_pinyin_pyjj", group: "sound", category: "double",
        recommended: false, home: "https://github.com/rime/rime-double-pinyin",
        deps: &["stroke", "luna", "double-pinyin"] },

    Entry { id: "bopomofo", schema_id: "bopomofo", group: "sound", category: "bopomofo",
        recommended: true, home: "https://github.com/rime/rime-bopomofo",
        deps: &["stroke", "terra", "bopomofo"] },
    Entry { id: "bopomofo-tw", schema_id: "bopomofo_tw", group: "sound", category: "bopomofo",
        recommended: false, home: "https://github.com/rime/rime-bopomofo",
        deps: &["stroke", "terra", "bopomofo"] },

    Entry { id: "jyutping", schema_id: "jyut6ping3", group: "sound", category: "cantonese",
        recommended: true, home: "https://github.com/rime/rime-cantonese",
        deps: &["luna", "cangjie", "quick", "stroke", "loengfan", "emoji-cantonese", "cantonese"] },

    // ─────────── 按字形打 ───────────
    Entry { id: "wubi86", schema_id: "wubi86", group: "shape", category: "wubi",
        recommended: false, home: "https://github.com/rime/rime-wubi",
        deps: &["pinyin-simp", "wubi"] },
    Entry { id: "wubi-pinyin", schema_id: "wubi_pinyin", group: "shape", category: "wubi",
        recommended: false, home: "https://github.com/rime/rime-wubi",
        deps: &["pinyin-simp", "wubi"] },
    Entry { id: "cangjie5", schema_id: "cangjie5", group: "shape", category: "cangjie",
        recommended: false, home: "https://github.com/rime/rime-cangjie",
        deps: &["luna", "cangjie"] },
    Entry { id: "quick5", schema_id: "quick5", group: "shape", category: "cangjie",
        recommended: false, home: "https://github.com/rime/rime-quick",
        deps: &["luna", "quick"] },
    Entry { id: "scj6", schema_id: "scj6", group: "shape", category: "cangjie",
        recommended: false, home: "https://github.com/rime/rime-scj",
        deps: &["luna", "scj"] },
    Entry { id: "stroke", schema_id: "stroke", group: "shape", category: "stroke",
        recommended: false, home: "https://github.com/rime/rime-stroke",
        deps: &["luna", "stroke"] },
    Entry { id: "array30", schema_id: "array30", group: "shape", category: "array",
        recommended: false, home: "https://github.com/rime/rime-array",
        deps: &["luna", "emoji", "array"] },

    // ─────────── 进阶 / 小众 ───────────
    Entry { id: "jyutping-plain", schema_id: "jyutping", group: "extra", category: "cantonese",
        recommended: false, home: "https://github.com/rime/rime-jyutping",
        deps: &["luna", "cangjie", "stroke", "jyutping"] },
    Entry { id: "yale", schema_id: "yale", group: "extra", category: "cantonese",
        recommended: false, home: "https://github.com/rime/rime-jyutping",
        deps: &["luna", "jyutping"] },
    Entry { id: "hkcantonese", schema_id: "hkcantonese", group: "extra", category: "cantonese",
        recommended: false, home: "https://github.com/rime/rime-jyutping",
        deps: &["luna", "jyutping"] },
    Entry { id: "wugniu", schema_id: "wugniu_lopha", group: "extra", category: "dialect",
        recommended: false, home: "https://github.com/rime/rime-wugniu",
        deps: &["luna", "wugniu"] },
    Entry { id: "soutzoe", schema_id: "soutzoe", group: "extra", category: "dialect",
        recommended: false, home: "https://github.com/rime/rime-soutzoe",
        deps: &["luna", "soutzoe"] },
    Entry { id: "zyenpheng", schema_id: "zyenpheng", group: "extra", category: "ancient",
        recommended: false, home: "https://github.com/rime/rime-middle-chinese",
        deps: &["luna", "middle-chinese"] },
    Entry { id: "ipa-xsampa", schema_id: "ipa_xsampa", group: "extra", category: "phonetic",
        recommended: false, home: "https://github.com/rime/rime-ipa",
        deps: &["ipa"] },
    Entry { id: "combo-pinyin", schema_id: "combo_pinyin", group: "extra", category: "chord",
        recommended: false, home: "https://github.com/rime/rime-combo-pinyin",
        deps: &["luna", "combo"] },
];

fn builtin_dicts() -> Vec<DictInfo> {
    ENTRIES
        .iter()
        .map(|e| {
            let standalone = matches!(e.id, "rime-ice" | "mint");
            let keys: Vec<&str> = if standalone {
                e.deps.to_vec()
            } else {
                BASE.iter().copied().chain(e.deps.iter().copied()).collect()
            };

            let sources: Vec<DictSource> = keys
                .iter()
                .map(|k| {
                    let (_, repo, branch, _) = package(k);
                    DictSource {
                        repo: (*repo).into(),
                        branch: (*branch).into(),
                    }
                })
                .collect();

            DictInfo {
                id: e.id.into(),
                schema_id: e.schema_id.into(),
                group: e.group.into(),
                category: e.category.into(),
                recommended: e.recommended,
                homepage: e.home.into(),
                total_bytes: keys.iter().map(|k| package(k).3).sum(),
                sources,
                installed: false,
                removable: false,
                active: false,
            }
        })
        .collect()
}

// ═══════════════════════ 安装清单 ═══════════════════════

fn manifest_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(".xgrime")
}

fn manifest_path(config_dir: &Path, dict_id: &str) -> PathBuf {
    manifest_dir(config_dir).join(format!("{}.json", dict_id))
}

fn read_manifest(config_dir: &Path, dict_id: &str) -> Option<Manifest> {
    let raw = std::fs::read_to_string(manifest_path(config_dir, dict_id)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_manifest(config_dir: &Path, m: &Manifest) -> AppResult<()> {
    let dir = manifest_dir(config_dir);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::with(code::MANIFEST_WRITE_FAILED, e))?;
    let json = serde_json::to_string_pretty(m).map_err(|e| AppError::with(code::MANIFEST_WRITE_FAILED, e))?;
    std::fs::write(manifest_path(config_dir, &m.dict_id), json)
        .map_err(|e| AppError::with(code::MANIFEST_WRITE_FAILED, e))
}

// ═══════════════════════ 解压过滤 ═══════════════════════

/// 这些顶层目录是仓库自己的杂物，不是 RIME 配置，倒进去只会污染用户目录
const JUNK_DIRS: [&str; 14] = [
    ".github", ".ci", ".vscode", ".build", ".idea", "demo", "docs", "doc", "scripts", "tools",
    "others", "preview", "plum", "test",
];

/// RIME 认得的文件类型
/// - yaml：方案 / 词典 / 配置
/// - txt：词频语料（essay.txt）、自定义短语、OpenCC 字表
/// - lua：脚本过滤器
/// - json：OpenCC 配置（emoji、繁简转换）—— 漏了它 emoji 和港台字形就全废
/// - gram：八股文语言模型
const ALLOWED_EXT: [&str; 5] = ["yaml", "txt", "lua", "json", "gram"];

/// 这些是「全局配置」，装词库时不该覆盖 —— 用户或别的方案可能已经改过了
const PROTECTED: [&str; 6] = [
    "default.yaml",
    "weasel.yaml",
    "squirrel.yaml",
    "ibus_rime.yaml",
    "user.yaml",
    "installation.yaml",
];

enum Decision {
    Skip,
    Write,
    /// 只在文件还不存在时写
    WriteIfAbsent,
}

fn decide(relative: &str) -> Decision {
    let path = Path::new(relative);

    // 顶层杂物目录
    if let Some(Component::Normal(first)) = path.components().next() {
        let first = first.to_string_lossy().to_lowercase();
        if JUNK_DIRS.contains(&first.as_str()) {
            return Decision::Skip;
        }
        // build/ 是编译产物，RIME 自己会重新生成
        if first == "build" {
            return Decision::Skip;
        }
    }

    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return Decision::Skip,
    };
    let lower = name.to_lowercase();

    // 仓库说明文件
    if lower.starts_with("readme") || lower.starts_with("license") || lower.starts_with("authors") {
        return Decision::Skip;
    }
    // 用户自己的 patch，绝对不能被下载的包覆盖
    if lower.ends_with(".custom.yaml") {
        return Decision::Skip;
    }
    // plum 的配方文件，RIME 本身用不上
    if lower.ends_with(".recipe.yaml") {
        return Decision::Skip;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if !ALLOWED_EXT.contains(&ext.as_str()) {
        return Decision::Skip;
    }

    if PROTECTED.contains(&lower.as_str()) {
        return Decision::WriteIfAbsent;
    }
    Decision::Write
}

/// 防目录穿越：ZIP 里的路径不许是绝对路径，也不许含 `..`
fn safe_relative(relative: &str) -> Option<PathBuf> {
    let p = Path::new(relative);
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Normal(part) => out.push(part),
            // 其余全部拒绝：RootDir / Prefix / ParentDir / CurDir
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

// ═══════════════════════ 查询 ═══════════════════════

/// 从 default.yaml 读取当前 schema_list 中的所有 schema_id
fn get_active_schemas(config_dir: &str) -> Vec<String> {
    let default_yaml = Path::new(config_dir).join("default.yaml");
    if let Ok(content) = std::fs::read_to_string(&default_yaml) {
        if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(list) = doc.get("schema_list").and_then(|v| v.as_sequence()) {
                return list
                    .iter()
                    .filter_map(|item| {
                        item.get("schema")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
            }
        }
    }
    vec![]
}

/// 小狼毫 / 鼠须管自带一批方案（明月拼音、注音、仓颉、五笔画、地球拼音），
/// 它们放在程序目录的 `data/` 里，不在用户配置目录。不看那儿的话，
/// 这些方案会显示成「未安装」，可它们明明就在方案选单里用着。
///
/// 探测一次要读注册表加扫目录，**列表里每个方案都探一次就太贵了**，
/// 所以由调用方探好传进来。
pub fn shared_data_dir() -> Option<PathBuf> {
    let info = crate::platform::current_platform().detect();
    let dir = PathBuf::from(info.install_dir?);
    #[cfg(target_os = "macos")]
    let dir = dir.join("Contents/SharedSupport");
    #[cfg(not(target_os = "macos"))]
    let dir = dir.join("data");
    dir.is_dir().then_some(dir)
}

/// 这个方案装了没，以及是不是我们能动的
///
/// 返回 `(装着, 卸得掉)`。输入法自带的那批（明月拼音、注音、仓颉、五笔画、
/// 地球拼音）文件在程序目录，装着但卸不掉。
fn check_installed(
    config_dir: &Path,
    shared: Option<&Path>,
    dict_id: &str,
    schema_id: &str,
) -> (bool, bool) {
    let file = format!("{}.schema.yaml", schema_id);

    // 有清单 = 我们装的，删得干净
    if manifest_path(config_dir, dict_id).exists() {
        return (true, true);
    }
    // 用户目录里有文件：旧版本装的，或者用户自己手动放进去的，也归我们管
    if config_dir.join(&file).exists() {
        return (true, true);
    }
    // 程序自带的：能用，但不该去删人家的安装文件
    if shared.is_some_and(|d| d.join(&file).exists()) {
        return (true, false);
    }
    (false, false)
}

#[tauri::command]
pub fn list_available_dicts() -> AppResult<Vec<DictInfo>> {
    Ok(builtin_dicts())
}

#[tauri::command]
pub fn list_installed_dicts(config_dir: String) -> AppResult<Vec<DictInfo>> {
    let dir = Path::new(&config_dir);
    let mut dicts = builtin_dicts();
    let active = read_custom_schema_list(&config_dir);
    // 探一次就够，28 个方案各探一次会把这个调用拖慢一大截
    let shared = shared_data_dir();
    for dict in &mut dicts {
        let (installed, removable) =
            check_installed(dir, shared.as_deref(), &dict.id, &dict.schema_id);
        dict.installed = installed;
        dict.removable = removable;
        dict.active = active.contains(&dict.schema_id);
    }
    Ok(dicts)
}

// ═══════════════════════ 安装 ═══════════════════════

#[tauri::command]
pub async fn install_dict(
    app: AppHandle,
    dict_id: String,
    config_dir: String,
) -> AppResult<()> {
    install_dict_with(&dict_id, &config_dir, |p| {
        let _ = app.emit("dict-download-progress", p);
    })
    .await
}

/// 安装的实际逻辑。进度用回调传出去，这样不依赖 AppHandle，测试里能直接跑。
pub async fn install_dict_with<F>(
    dict_id: &str,
    config_dir: &str,
    on_progress: F,
) -> AppResult<()>
where
    F: Fn(DownloadProgress),
{
    let dicts = builtin_dicts();
    let dict = dicts
        .iter()
        .find(|d| d.id == dict_id)
        .ok_or_else(|| AppError::with(code::SCHEMA_NOT_FOUND, dict_id))?
        .clone();

    let config_path = PathBuf::from(config_dir);
    std::fs::create_dir_all(&config_path).map_err(|e| AppError::with(code::CONFIG_DIR_CREATE_FAILED, e))?;

    let client = reqwest::Client::builder()
        .user_agent(concat!("XGRime/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::with(code::HTTP_CLIENT_FAILED, e))?;

    let step_total = dict.sources.len();
    let mut written: Vec<String> = Vec::new();

    for (i, source) in dict.sources.iter().enumerate() {
        let step = i + 1;
        let step_name = source.repo.clone();

        let zip_path =
            download_to_temp(&on_progress, &client, source, step, step_total, &step_name).await?;
        let files = extract_zip(&zip_path, &config_path)
            .map_err(|e| AppError::with(code::ZIP_CORRUPT, format!("{} — {}", source.repo, e)))?;
        let _ = std::fs::remove_file(&zip_path);
        written.extend(files);
    }

    written.sort();
    written.dedup();

    // 记下主仓库当前的提交号，之后才知道词库有没有更新。
    // 查不到就存 None —— 没网或超额度不该让安装失败。
    let main = dict.sources.last().cloned().unwrap_or(DictSource {
        repo: String::new(),
        branch: String::new(),
    });
    let main_sha = if main.repo.is_empty() {
        None
    } else {
        crate::update::head_sha(&main.repo, &main.branch).await
    };

    write_manifest(
        &config_path,
        &Manifest {
            dict_id: dict.id.clone(),
            schema_id: dict.schema_id.clone(),
            files: written,
            main_repo: main.repo,
            main_branch: main.branch,
            main_sha,
        },
    )?;

    // 装完直接挂进方案列表 —— 不然用户在输入法里根本看不到它
    set_schema_enabled(config_dir, &dict.schema_id, true)?;

    Ok(())
}

async fn download_to_temp<F>(
    on_progress: &F,
    client: &reqwest::Client,
    source: &DictSource,
    step: usize,
    step_total: usize,
    step_name: &str,
) -> AppResult<PathBuf>
where
    F: Fn(DownloadProgress),
{
    use futures_util::StreamExt;
    use std::io::Write;

    let resp = client
        .get(source.zip_url())
        .send()
        .await
        .map_err(|e| AppError::with(code::DOWNLOAD_FAILED, format!("{} — {}", source.repo, e)))?;

    if !resp.status().is_success() {
        return Err(AppError::with(
            code::DOWNLOAD_FAILED,
            format!("{} — HTTP {}", source.repo, resp.status()),
        ));
    }

    let total = resp.content_length().unwrap_or(0);
    let temp_dir = std::env::temp_dir().join("XGRime");
    std::fs::create_dir_all(&temp_dir).map_err(|e| AppError::with(code::TEMP_DIR_FAILED, e))?;
    let zip_path = temp_dir.join(format!("{}.zip", source.repo.replace('/', "_")));

    // 落盘而不是堆内存：雾凇的包有几十 MB，全塞 Vec 里没必要
    let mut file =
        std::fs::File::create(&zip_path).map_err(|e| AppError::with(code::TEMP_FILE_FAILED, e))?;
    let mut downloaded: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::with(code::DOWNLOAD_INTERRUPTED, format!("{} — {}", source.repo, e)))?;
        file.write_all(&chunk)
            .map_err(|e| AppError::with(code::TEMP_FILE_FAILED, e))?;
        downloaded += chunk.len() as u64;

        let percentage = if total > 0 {
            (downloaded as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        on_progress(DownloadProgress {
            downloaded,
            total,
            percentage,
            step,
            step_total,
            step_name: step_name.to_string(),
        });
    }
    file.flush().map_err(|e| AppError::with(code::TEMP_FILE_FAILED, e))?;
    drop(file);

    Ok(zip_path)
}

/// 解压并返回实际写入的文件（相对配置目录的路径）
fn extract_zip(zip_path: &Path, config_path: &Path) -> AppResult<Vec<String>> {
    let file = std::fs::File::open(zip_path).map_err(|e| AppError::with(code::ZIP_OPEN_FAILED, e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::with(code::ZIP_CORRUPT, e))?;

    // GitHub 的源码包统一多套一层 `仓库名-分支名/`，要剥掉
    let prefix = archive
        .file_names()
        .next()
        .and_then(|name| name.split('/').next())
        .map(|s| format!("{}/", s))
        .unwrap_or_default();

    let mut written = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::with(code::ZIP_ENTRY_FAILED, e))?;

        if entry.is_dir() {
            continue;
        }

        // enclosed_name 已经挡掉了绝对路径和 `..`，这里再按字符串剥前缀
        let name = match entry.enclosed_name() {
            Some(n) => n.to_string_lossy().replace('\\', "/"),
            None => continue, // 路径可疑，直接丢弃
        };
        let relative = name.strip_prefix(&prefix).unwrap_or(&name);

        let dest_rel = match decide(relative) {
            Decision::Skip => continue,
            Decision::Write => match safe_relative(relative) {
                Some(p) => p,
                None => continue,
            },
            Decision::WriteIfAbsent => match safe_relative(relative) {
                Some(p) if !config_path.join(&p).exists() => p,
                _ => continue,
            },
        };

        let dest = config_path.join(&dest_rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;
        }

        let mut out =
            std::fs::File::create(&dest).map_err(|e| AppError::with(code::FILE_CREATE_FAILED, format!("{} — {}", dest.display(), e)))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

        written.push(dest_rel.to_string_lossy().replace('\\', "/"));
    }

    Ok(written)
}

// ═══════════════════════ 启用 / 停用 ═══════════════════════

/// 方案选单里当前启用的全部方案
pub fn enabled_schemas(config_dir: &str) -> Vec<String> {
    read_custom_schema_list(config_dir)
}

/// 读 default.custom.yaml 里的 schema_list；没有就退回 default.yaml 的原始列表
fn read_custom_schema_list(config_dir: &str) -> Vec<String> {
    let custom_yaml = Path::new(config_dir).join("default.custom.yaml");
    if let Ok(content) = std::fs::read_to_string(&custom_yaml) {
        if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(list) = doc
                .get("patch")
                .and_then(|p| p.get("schema_list"))
                .and_then(|v| v.as_sequence())
            {
                return list
                    .iter()
                    .filter_map(|item| {
                        item.get("schema")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
            }
        }
    }
    get_active_schemas(config_dir)
}

/// 改 schema_list，其余 patch 键原样保留
pub fn set_schema_enabled(config_dir: &str, schema_id: &str, enable: bool) -> AppResult<()> {
    use serde_yaml::Value;

    let mut schemas = read_custom_schema_list(config_dir);
    if enable {
        if !schemas.iter().any(|s| s == schema_id) {
            schemas.push(schema_id.to_string());
        }
    } else {
        schemas.retain(|s| s != schema_id);
    }

    // 一个都不剩的话 RIME 会起不来，兜一个最低限度的方案
    if schemas.is_empty() {
        schemas.push("luna_pinyin".to_string());
    }

    let list: Vec<Value> = schemas
        .iter()
        .map(|id| {
            let mut m = serde_yaml::Mapping::new();
            m.insert(Value::String("schema".into()), Value::String(id.clone()));
            Value::Mapping(m)
        })
        .collect();

    crate::settings::merge_patch(
        &Path::new(config_dir).join("default.custom.yaml"),
        [("schema_list".to_string(), Value::Sequence(list))],
        &[],
    )
}

#[tauri::command]
pub fn toggle_dict(dict_id: String, config_dir: String, enable: bool) -> AppResult<()> {
    let dicts = builtin_dicts();
    let dict = dicts
        .iter()
        .find(|d| d.id == dict_id)
        .ok_or_else(|| AppError::with(code::SCHEMA_NOT_FOUND, dict_id))?;

    set_schema_enabled(&config_dir, &dict.schema_id, enable)
}

// ═══════════════════════ 卸载 ═══════════════════════

#[tauri::command]
pub fn remove_dict(dict_id: String, config_dir: String) -> AppResult<()> {
    let dicts = builtin_dicts();
    let dict = dicts
        .iter()
        .find(|d| d.id == dict_id)
        .ok_or_else(|| AppError::with(code::SCHEMA_NOT_FOUND, dict_id))?
        .clone();

    let config_path = PathBuf::from(&config_dir);

    // 自带的方案文件在程序目录里，那是输入法自己的安装内容。
    // 与其假装卸载成功（什么都没删，界面也不会变），不如明说卸不了。
    let (_, removable) = check_installed(
        &config_path,
        shared_data_dir().as_deref(),
        &dict.id,
        &dict.schema_id,
    );
    if !removable {
        return Err(AppError::new(code::SCHEMA_NOT_REMOVABLE));
    }

    // 先从方案列表摘掉，避免删完文件 RIME 还想加载它
    set_schema_enabled(&config_dir, &dict.schema_id, false)?;

    // 别的方案还在用的文件不能删（明月拼音的词典粤拼也要用）
    let shared_dir = shared_data_dir();
    let shared: HashSet<String> = dicts
        .iter()
        .filter(|d| d.id != dict.id)
        .filter(|d| check_installed(&config_path, shared_dir.as_deref(), &d.id, &d.schema_id).0)
        .filter_map(|d| read_manifest(&config_path, &d.id))
        .flat_map(|m| m.files)
        .collect();

    match read_manifest(&config_path, &dict.id) {
        Some(manifest) => {
            for rel in &manifest.files {
                if shared.contains(rel) {
                    continue;
                }
                // 全局配置是公共设施，谁都可能靠它，卸一个方案不该把它带走
                if Path::new(rel)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| PROTECTED.contains(&n.to_lowercase().as_str()))
                    .unwrap_or(false)
                {
                    continue;
                }
                let path = config_path.join(rel);
                if path.is_file() {
                    let _ = std::fs::remove_file(&path);
                }
            }
            // 只清这次删过文件的那几个目录，别把用户自己建的空目录也扫了
            let touched: HashSet<PathBuf> = manifest
                .files
                .iter()
                .filter_map(|rel| Path::new(rel).parent().map(|p| config_path.join(p)))
                .filter(|p| p != &config_path)
                .collect();
            prune_empty_dirs(&config_path, &touched);
            let _ = std::fs::remove_file(manifest_path(&config_path, &dict.id));
        }
        None => {
            // 没有清单（旧版本装的）：只删明确属于这个方案的文件，宁可留下也别误删
            let _ = std::fs::remove_file(config_path.join(format!("{}.schema.yaml", dict.schema_id)));
            if let Ok(entries) = std::fs::read_dir(&config_path) {
                let head = format!("{}.", dict.schema_id);
                for e in entries.filter_map(|e| e.ok()) {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with(&head) && name.ends_with(".dict.yaml") {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
        }
    }

    Ok(())
}

// ═══════════════════════ 更新检查 ═══════════════════════

/// 逐个问已装方案的主仓库有没有新提交
///
/// 只查装了的，而且每个方案只花一次请求 —— GitHub 匿名接口一小时 60 次，
/// 挥霍不起。任何一个查不动就跳过它，不影响其余的。
#[tauri::command]
pub async fn check_dict_updates(config_dir: String) -> AppResult<Vec<String>> {
    let config_path = PathBuf::from(&config_dir);
    let shared = shared_data_dir();
    let mut stale = Vec::new();

    for dict in builtin_dicts() {
        if !check_installed(&config_path, shared.as_deref(), &dict.id, &dict.schema_id).0 {
            continue;
        }
        let Some(manifest) = read_manifest(&config_path, &dict.id) else {
            continue;
        };
        // 旧版本装的没记提交号，没法比，跳过而不是谎报有更新
        let (Some(old_sha), false) = (manifest.main_sha, manifest.main_repo.is_empty()) else {
            continue;
        };

        if let Some(now) = crate::update::head_sha(&manifest.main_repo, &manifest.main_branch).await
        {
            if now != old_sha {
                stale.push(dict.id);
            }
        }
    }

    Ok(stale)
}

/// 清掉这次卸载留下的空目录（opencc/ lua/ 之类）
///
/// 只动 `touched` 里列的那几个 —— 之前是把配置目录整棵树扫一遍删空目录，
/// 用户自己建的空文件夹也会被顺手删掉。
fn prune_empty_dirs(root: &Path, touched: &HashSet<PathBuf>) {
    let mut dirs: Vec<&PathBuf> = touched.iter().collect();
    // 先删深的，父目录才可能跟着变空
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));

    for dir in dirs {
        if !dir.starts_with(root) || dir == root {
            continue;
        }
        if dir.file_name().map(|n| n == ".xgrime").unwrap_or(false) {
            continue;
        }
        let empty = std::fs::read_dir(dir)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if empty {
            let _ = std::fs::remove_dir(dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_write(d: Decision) -> bool {
        matches!(d, Decision::Write)
    }
    fn is_skip(d: Decision) -> bool {
        matches!(d, Decision::Skip)
    }

    #[test]
    fn opencc_json_must_survive_the_filter() {
        // 这两个漏掉的话，emoji 候选和港台字形转换会静默失效
        assert!(is_write(decide("opencc/emoji.json")));
        assert!(is_write(decide("opencc/t2hkf.json")));
        assert!(is_write(decide("opencc/HKVariantsFull.txt")));
    }

    #[test]
    fn rime_data_files_pass() {
        for f in [
            "jyut6ping3.schema.yaml",
            "jyut6ping3.chars.dict.yaml",
            "essay.txt",
            "lua/search.lua",
            "zh-hant-t-essay-bgw.gram",
            "symbols_cantonese.yaml",
        ] {
            assert!(is_write(decide(f)), "{} 应该被装进去", f);
        }
    }

    #[test]
    fn repo_junk_is_dropped() {
        for f in [
            ".github/workflows/ci.yaml",
            "scripts/build.lua",
            "others/patch_examples/foo.yaml",
            "demo/tone.txt",
            "build/rime_ice.table.bin",
            "README.md",
            "LICENSE",
            "emoji_cantonese.recipe.yaml",
        ] {
            assert!(is_skip(decide(f)), "{} 不该被装进去", f);
        }
    }

    #[test]
    fn user_patches_are_never_overwritten() {
        // 用户自己写的 custom 文件是他的设置，压缩包里的同名文件不许盖
        assert!(is_skip(decide("jyut6ping3.custom.yaml")));
        assert!(is_skip(decide("default.custom.yaml")));
        // 全局配置只在缺失时写，避免装第二个方案时把第一个的 default.yaml 换掉
        assert!(matches!(decide("default.yaml"), Decision::WriteIfAbsent));
        assert!(matches!(decide("weasel.yaml"), Decision::WriteIfAbsent));
    }

    fn temp_config(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xgrime-dict-{}-{}", tag, nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 卸载只该删这个方案自己的东西：
    /// 公共配置留下，别的方案还在用的文件也留下
    #[test]
    fn uninstall_only_removes_what_it_owns() {
        let dir = temp_config("uninstall");
        let path = dir.to_string_lossy().to_string();

        for f in ["default.yaml", "luna_pinyin.dict.yaml", "jyut6ping3.schema.yaml"] {
            std::fs::write(dir.join(f), "x").unwrap();
        }
        std::fs::create_dir_all(dir.join("opencc")).unwrap();
        std::fs::write(dir.join("opencc/t2hkf.json"), "x").unwrap();

        // 粤拼装了这四个文件，明月拼音也用着那本 luna_pinyin 词典
        write_manifest(&dir, &Manifest {
            dict_id: "jyutping".into(),
            schema_id: "jyut6ping3".into(),
            files: vec![
                "default.yaml".into(),
                "luna_pinyin.dict.yaml".into(),
                "jyut6ping3.schema.yaml".into(),
                "opencc/t2hkf.json".into(),
            ],
            ..Default::default()
        }).unwrap();
        write_manifest(&dir, &Manifest {
            dict_id: "luna-pinyin".into(),
            schema_id: "luna_pinyin".into(),
            files: vec!["luna_pinyin.dict.yaml".into()],
            ..Default::default()
        }).unwrap();

        remove_dict("jyutping".into(), path).unwrap();

        assert!(!dir.join("jyut6ping3.schema.yaml").exists(), "自己的方案文件该删掉");
        assert!(!dir.join("opencc/t2hkf.json").exists(), "自己的 opencc 配置该删掉");
        assert!(dir.join("default.yaml").exists(), "公共配置不该被带走");
        assert!(dir.join("luna_pinyin.dict.yaml").exists(), "明月拼音还用着，不该删");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn zip_slip_is_blocked() {
        assert!(safe_relative("../../evil.yaml").is_none());
        assert!(safe_relative("a/../../evil.yaml").is_none());
        assert!(safe_relative("/etc/evil.yaml").is_none());
        assert!(safe_relative("opencc/t2hkf.json").is_some());
    }

    #[test]
    fn catalog_is_consistent() {
        use std::collections::HashSet;
        let dicts = builtin_dicts(); // 引用了不存在的包会在这里 panic

        let mut ids = HashSet::new();
        let mut schema_ids = HashSet::new();
        for d in &dicts {
            assert!(ids.insert(d.id.clone()), "方案 id 重复：{}", d.id);
            assert!(
                schema_ids.insert(d.schema_id.clone()),
                "schema_id 重复：{}",
                d.schema_id
            );
            assert!(!d.sources.is_empty(), "{} 没有下载来源", d.id);
            assert!(!d.installed && !d.removable, "{} 的初始状态该是未安装", d.id);
            assert!(d.total_bytes > 0, "{} 体积算成 0 了", d.id);
            assert!(
                ["sound", "shape", "extra"].contains(&d.group.as_str()),
                "{} 的分组不认识：{}",
                d.id,
                d.group
            );
        }
        assert!(dicts.iter().any(|d| d.recommended), "至少要有一个新手推荐");
    }

    /// 依赖表错一个就是部署报错，这几条是逐个翻 schema.yaml 核对出来的
    #[test]
    fn verified_dependencies_are_not_lost() {
        let by_id = |id: &str| builtin_dicts().into_iter().find(|d| d.id == id).unwrap();
        let has = |d: &DictInfo, repo: &str| d.sources.iter().any(|s| s.repo == repo);

        // 明月拼音和双拼都要笔画反查，漏了就部署报错
        for id in ["luna-pinyin", "double-pinyin-flypy", "double-pinyin-mspy"] {
            assert!(has(&by_id(id), "rime/rime-stroke"), "{} 缺 stroke 词典", id);
        }
        // 五笔反查用的是袖珍简化字拼音，不是明月拼音
        let wubi = by_id("wubi86");
        assert!(has(&wubi, "rime/rime-pinyin-simp"), "五笔缺 pinyin_simp 词典");
        // 行列的 emoji 滤镜要 opencc/emoji.json
        assert!(has(&by_id("array30"), "rime/rime-emoji"), "行列缺 emoji 包");

        // 粤拼的四本反查词典
        let jyut = by_id("jyutping");
        for repo in [
            "rime/rime-cantonese",
            "rime/rime-luna-pinyin",
            "rime/rime-cangjie",
            "rime/rime-stroke",
            "CanCLID/rime-loengfan",
        ] {
            assert!(has(&jyut, repo), "粤语拼音缺依赖 {}", repo);
        }
        // 主方案必须排最后，否则会被依赖包里的同名文件盖掉
        assert_eq!(jyut.sources.last().unwrap().repo, "rime/rime-cantonese");
        assert_eq!(by_id("rime-ice").sources.last().unwrap().repo, "iDvel/rime-ice");
    }
}

/// 真正连网装一遍。默认不跑（要下几十 MB），验证时手动：
/// `cargo test --lib -- --ignored --nocapture`
#[cfg(test)]
mod net_tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn install_cantonese_end_to_end() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xgrime-e2e-{}", nanos));
        let path = dir.to_string_lossy().to_string();

        install_dict_with("jyutping", &path, |p| {
            if p.total > 0 && p.downloaded == p.total {
                println!("  [{}/{}] {} 完成", p.step, p.step_total, p.step_name);
            }
        })
        .await
        .expect("安装粤语拼音失败");

        // 方案本体 + schema 里点名要的四本反查词典，少一本部署就报错
        for f in [
            "jyut6ping3.schema.yaml",
            "jyut6ping3.dict.yaml",
            "jyut6ping3.chars.dict.yaml",
            "luna_pinyin.dict.yaml",
            "stroke.dict.yaml",
            "cangjie5.dict.yaml",
            "loengfan.dict.yaml",
            "essay.txt",
            "default.yaml",
            "symbols.yaml",
            "punctuation.yaml",
        ] {
            assert!(dir.join(f).is_file(), "缺文件：{}", f);
        }

        // 之前被过滤掉的 opencc 配置，现在必须在
        assert!(dir.join("opencc/t2hkf.json").is_file(), "缺 opencc/t2hkf.json");
        assert!(
            dir.join("opencc/emoji_cantonese.json").is_file(),
            "缺 opencc/emoji_cantonese.json"
        );

        // 仓库杂物不该被倒进来
        assert!(!dir.join(".github").exists(), "把 .github 装进去了");
        assert!(!dir.join("scripts").exists(), "把 scripts 装进去了");
        assert!(!dir.join("README.md").exists(), "把 README 装进去了");

        // 装完自动挂进方案列表
        let custom = std::fs::read_to_string(dir.join("default.custom.yaml")).unwrap();
        assert!(custom.contains("jyut6ping3"), "没有自动启用：{}", custom);

        // 卸载要能把装进去的东西清干净
        remove_dict("jyutping".into(), path.clone()).unwrap();
        assert!(!dir.join("jyut6ping3.schema.yaml").exists(), "卸载没删干净");
        assert!(!dir.join("opencc/t2hkf.json").exists(), "卸载没删干净");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    #[ignore]
    async fn install_rime_ice_end_to_end() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xgrime-e2e-ice-{}", nanos));
        let path = dir.to_string_lossy().to_string();

        install_dict_with("rime-ice", &path, |_| {}).await.expect("安装雾凇拼音失败");

        for f in [
            "rime_ice.schema.yaml",
            "rime_ice.dict.yaml",
            "default.yaml",
            "opencc/emoji.json",
            "custom_phrase.txt",
        ] {
            assert!(dir.join(f).is_file(), "缺文件：{}", f);
        }
        assert!(dir.join("lua").is_dir(), "缺 lua 目录");
        // 仓库里的示例配置不该倒进用户目录
        assert!(!dir.join("others").exists(), "把 others 装进去了");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 打印本机检测结果，用来核对残留判定
    #[test]
    #[ignore]
    fn show_local_detection() {
        let info = crate::platform::current_platform().detect();
        println!("{:#?}", info);
    }

    /// 依赖表写错就是部署报错。挑几个最容易错的真装一遍，
    /// 确认方案 schema.yaml 点名要的词典都真的落了地。
    #[tokio::test]
    #[ignore]
    async fn dependencies_actually_land_on_disk() {
        let cases: &[(&str, &[&str])] = &[
            // 明月拼音要笔画反查
            ("luna-pinyin", &["luna_pinyin.schema.yaml", "luna_pinyin.dict.yaml", "stroke.dict.yaml"]),
            // 五笔的反查用袖珍简化字拼音，不是明月拼音
            ("wubi86", &["wubi86.schema.yaml", "wubi86.dict.yaml", "pinyin_simp.dict.yaml"]),
            // 注音要地球拼音的词典
            ("bopomofo", &["bopomofo.schema.yaml", "terra_pinyin.dict.yaml", "stroke.dict.yaml"]),
            // 行列的 emoji 滤镜要 opencc/emoji.json
            ("array30", &["array30.schema.yaml", "array30.dict.yaml", "opencc/emoji.json"]),
        ];

        for (id, expected) in cases {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("xgrime-dep-{}-{}", id, nanos));
            let path = dir.to_string_lossy().to_string();

            install_dict_with(id, &path, |_| {})
                .await
                .unwrap_or_else(|e| panic!("{} 装不上：{}", id, e));

            for f in *expected {
                assert!(dir.join(f).is_file(), "{} 装完之后缺 {}", id, f);
            }
            // 每个都要自动挂进方案列表
            let custom = std::fs::read_to_string(dir.join("default.custom.yaml")).unwrap();
            let schema_id = builtin_dicts().into_iter().find(|d| d.id == *id).unwrap().schema_id;
            assert!(custom.contains(&schema_id), "{} 没自动启用", id);

            println!("  {} ✓", id);
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
