use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSettings {
    pub page_size: u32,
    pub shift_l_behavior: String,
    pub shift_r_behavior: String,
    pub page_keys: String,
}

impl Default for InputSettings {
    fn default() -> Self {
        Self {
            page_size: 5,
            shift_l_behavior: "commit_text".into(),
            shift_r_behavior: "noop".into(),
            page_keys: "minus_equal".into(),
        }
    }
}

/// 方案自带的一个开关（简繁、中英标点、Emoji 之类）
///
/// 这些不是我们造出来的功能，是每个方案 `schema.yaml` 里的 `switches:`。
/// 与其写死「简繁切换」再去猜每个方案怎么实现，不如把方案自己声明的开关读出来，
/// 有几个显示几个 —— 雾凇是「简/繁」，粤拼是「傳統/香港/臺灣/简化」四选一。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaSwitch {
    /// 在 switches 列表里的下标，写 patch 时要用
    pub index: usize,
    /// 只给键，显示成哪国话由前端按界面语言决定
    pub label_key: String,
    /// 认不出来的开关就把方案里的原名给前端兜底显示
    pub raw_name: Option<String>,
    /// 每个档位的显示名，直接来自方案
    pub states: Vec<String>,
    /// 当前默认停在第几档
    pub current: usize,
    /// 用户有没有显式设过（没设就是方案自带的默认值）
    pub configured: bool,
}

/// 方案选单里的一项，给前端做「调哪个方案」的选择器
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaBrief {
    pub schema_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOptions {
    pub schema_id: String,
    pub schema_name: String,
    /// 找不到方案文件时为 true，前端据此提示「先装一个输入方案」
    pub missing: bool,
    /// 模糊音只对拼音类方案有意义
    pub supports_fuzzy: bool,
    pub switches: Vec<SchemaSwitch>,
    /// 这个方案自己的模糊音设置
    pub fuzzy_pairs: Vec<String>,
    /// 方案选单里全部启用中的方案，装了两个以上时前端要给选择器
    pub available: Vec<SchemaBrief>,
}

// ═══════════════════════ YAML patch 读写 ═══════════════════════

/// 往 `*.custom.yaml` 的 `patch:` 里合并若干键，**保留其他键不动**
///
/// 之前这里是整份覆盖写，结果一存输入设置就把词库页写进去的 `schema_list` 抹掉了，
/// 已安装的方案会从输入法选单里凭空消失。
pub fn merge_patch<I>(path: &Path, set: I, remove: &[&str]) -> AppResult<()>
where
    I: IntoIterator<Item = (String, serde_yaml::Value)>,
{
    use serde_yaml::Value;

    let mut patch = serde_yaml::Mapping::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        // 文件在但读不懂时**不能**当成空的接着写 —— 那等于把用户手写的
        // schema_list、按键绑定全部truncate 掉。宁可报错让他自己看一眼。
        let doc: Value = serde_yaml::from_str(&content)
            .map_err(|e| AppError::with(code::SCHEMA_PARSE_FAILED, format!("{} — {}", path.display(), e)))?;
        if let Some(existing) = doc.get("patch").and_then(|v| v.as_mapping()) {
            patch = existing.clone();
        }
    }

    for key in remove {
        patch.remove(Value::String((*key).to_string()));
    }
    for (key, value) in set {
        patch.insert(Value::String(key), value);
    }

    if patch.is_empty() {
        // 没有任何自定义了，留个空文件没意义
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| AppError::with(code::FILE_DELETE_FAILED, e))?;
        }
        return Ok(());
    }

    let mut root = serde_yaml::Mapping::new();
    root.insert(Value::String("patch".into()), Value::Mapping(patch));

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AppError::with(code::DIR_CREATE_FAILED, e))?;
    }
    let yaml = serde_yaml::to_string(&Value::Mapping(root))
        .map_err(|e| AppError::with(code::YAML_SERIALIZE_FAILED, e))?;
    std::fs::write(path, yaml).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))
}

fn read_patch(path: &Path) -> Option<serde_yaml::Mapping> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc = serde_yaml::from_str::<serde_yaml::Value>(&content).ok()?;
    doc.get("patch").and_then(|v| v.as_mapping()).cloned()
}

// ═══════════════════════ 模糊音 / 翻页键 ═══════════════════════

/// 模糊音 ID → 方案里的 derive 规则
fn fuzzy_rules_for_id(id: &str) -> Vec<&'static str> {
    match id {
        "zh_z" | "ch_c" | "sh_s" => vec![
            "derive/^([zcs])h/$1/",
            "derive/^([zcs])([^h])/$1h$2/",
        ],
        "n_l" => vec!["derive/^n/l/", "derive/^l/n/"],
        "r_l" => vec!["derive/^r/l/", "derive/^l/r/"],
        "f_h" => vec!["derive/^f/h/", "derive/^h/f/"],
        "an_ang" => vec!["derive/ang$/an/", "derive/an$/ang/"],
        "en_eng" => vec!["derive/eng$/en/", "derive/en$/eng/"],
        "in_ing" => vec!["derive/in$/ing/", "derive/ing$/in/"],
        "ian_iang" => vec!["derive/ian$/iang/", "derive/iang$/ian/"],
        "uan_uang" => vec!["derive/uan$/uang/", "derive/uang$/uan/"],
        _ => vec![],
    }
}

/// 反向：从已写入的 derive 规则倒推出勾了哪些模糊音
fn fuzzy_ids_from_rules(rules: &[String]) -> Vec<String> {
    const ALL: [&str; 9] = [
        "zh_z", "n_l", "r_l", "f_h", "an_ang", "en_eng", "in_ing", "ian_iang", "uan_uang",
    ];
    let mut ids = Vec::new();
    for id in ALL {
        let expected = fuzzy_rules_for_id(id);
        if !expected.is_empty() && expected.iter().all(|r| rules.iter().any(|x| x == r)) {
            // 声母那组三个 ID 共用同一套规则，一起勾上
            if id == "zh_z" {
                ids.extend(["zh_z", "ch_c", "sh_s"].map(String::from));
            } else {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

const PAGE_KEY_SETS: [(&str, &str, &str); 4] = [
    ("minus_equal", "minus", "equal"),
    ("bracket", "bracketleft", "bracketright"),
    ("tab", "shift+Tab", "Tab"),
    ("comma_period", "comma", "period"),
];

fn page_key_bindings(id: &str) -> Vec<serde_yaml::Value> {
    use serde_yaml::Value;

    let make = |accept: &str, send: &str| -> Value {
        let mut m = serde_yaml::Mapping::new();
        m.insert(Value::String("when".into()), Value::String("has_menu".into()));
        m.insert(Value::String("accept".into()), Value::String(accept.into()));
        m.insert(Value::String("send".into()), Value::String(send.into()));
        Value::Mapping(m)
    };

    PAGE_KEY_SETS
        .iter()
        .find(|(key, _, _)| *key == id)
        .map(|(_, up, down)| vec![make(up, "Page_Up"), make(down, "Page_Down")])
        .unwrap_or_default()
}

/// 从已写入的 key_binder 绑定倒推出用的是哪套翻页键
fn page_keys_from_bindings(bindings: &serde_yaml::Value) -> Option<String> {
    let seq = bindings.as_sequence()?;
    let accepts: Vec<String> = seq
        .iter()
        .filter_map(|b| b.get("accept").and_then(|v| v.as_str()).map(String::from))
        .collect();

    PAGE_KEY_SETS
        .iter()
        .find(|(_, up, down)| {
            accepts.iter().any(|a| a == up) && accepts.iter().any(|a| a == down)
        })
        .map(|(id, _, _)| id.to_string())
}

// ═══════════════════════ 当前方案 ═══════════════════════

/// 当前主方案：优先看 default.custom.yaml（用户自己排的顺序），再退回 default.yaml
pub fn detect_primary_schema(config_dir: &Path) -> String {
    let read_first = |path: &Path, in_patch: bool| -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        let doc = serde_yaml::from_str::<serde_yaml::Value>(&content).ok()?;
        let node = if in_patch { doc.get("patch")? } else { &doc };
        node.get("schema_list")?
            .as_sequence()?
            .first()?
            .get("schema")?
            .as_str()
            .map(String::from)
    };

    read_first(&config_dir.join("default.custom.yaml"), true)
        .or_else(|| read_first(&config_dir.join("default.yaml"), false))
        .unwrap_or_else(|| "rime_ice".to_string())
}

fn supports_fuzzy(schema_id: &str) -> bool {
    // 模糊音那套 derive 规则是按汉语拼音写的，套到粤拼 / 五笔上纯属捣乱
    schema_id.contains("pinyin") || matches!(schema_id, "rime_ice" | "mint")
}

/// 把方案里的开关名归到一个我们认得的键上。
///
/// 方案写的 `name` 是给程序看的（ascii_punct 之类），直接显示用户看不懂；
/// 但翻成哪国话是界面的事，所以这里只负责给键。
fn switch_label_key(name: Option<&str>, states: &[String]) -> &'static str {
    match name {
        Some("ascii_mode") => "ascii_mode",
        Some("full_shape") => "full_shape",
        Some("ascii_punct") => "ascii_punct",
        Some("simplification") | Some("traditionalization") | Some("zh_simp") => "simplification",
        Some("emoji") | Some("emoji_cantonese_suggestion") => "emoji",
        Some("search_single_char") => "search_single_char",
        Some(_) => "other",
        // 没有 name 的是一组互斥选项，粤拼那个「傳統 / 香港 / 臺灣 / 简化」就是这种
        None => {
            let looks_like_variants = states.iter().any(|s| {
                s.contains('漢') || s.contains('汉') || s.contains('繁') || s.contains('简')
            });
            if looks_like_variants {
                "variants"
            } else {
                "other"
            }
        }
    }
}

// ═══════════════════════ 命令 ═══════════════════════

/// 找出方案文件在哪：先看用户目录，再看程序自带的
fn locate_schema_file(config_dir: &Path, schema_id: &str) -> Option<PathBuf> {
    let name = format!("{}.schema.yaml", schema_id);
    let user = config_dir.join(&name);
    if user.exists() {
        return Some(user);
    }
    let shared = crate::dict::shared_data_dir()?.join(&name);
    shared.exists().then_some(shared)
}

fn schema_display_name(config_dir: &Path, schema_id: &str) -> String {
    let read = || -> Option<String> {
        let path = locate_schema_file(config_dir, schema_id)?;
        let raw = std::fs::read_to_string(path).ok()?;
        let doc: serde_yaml::Value = serde_yaml::from_str(&raw).ok()?;
        doc.get("schema")?
            .get("name")?
            .as_str()
            .map(String::from)
    };
    // 读不出名字就用 id 顶着，总好过界面上空一块
    read().unwrap_or_else(|| schema_id.to_string())
}

/// 读某个方案能调的东西
///
/// `schema` 不传就用方案选单里的第一个。装了两个以上方案时，
/// 前端会把 `available` 摆成选择器让用户自己挑要调哪个 ——
/// 不然第二个方案的开关根本没入口。
#[tauri::command]
pub fn read_schema_options(config_dir: String, schema: Option<String>) -> AppResult<SchemaOptions> {
    let dir = Path::new(&config_dir);

    let enabled = crate::dict::enabled_schemas(&config_dir);
    let available: Vec<SchemaBrief> = enabled
        .iter()
        .map(|id| SchemaBrief {
            name: schema_display_name(dir, id),
            schema_id: id.clone(),
        })
        .collect();

    // 传进来的方案得真在选单里；没传就跟着「现在正在用的那个」走，
    // 而不是选单第一个 —— 用户在输入方案页切到粤拼，这边就该直接是粤拼
    let schema_id = schema
        .filter(|s| enabled.iter().any(|e| e == s))
        .or_else(|| previously_selected(dir).filter(|s| enabled.iter().any(|e| e == s)))
        .unwrap_or_else(|| detect_primary_schema(dir));

    let mut opts = SchemaOptions {
        schema_name: schema_id.clone(),
        supports_fuzzy: supports_fuzzy(&schema_id),
        schema_id: schema_id.clone(),
        missing: true,
        switches: vec![],
        fuzzy_pairs: vec![],
        available,
    };

    let Some(schema_file) = locate_schema_file(dir, &schema_id) else {
        return Ok(opts);
    };
    opts.missing = false;

    let content = std::fs::read_to_string(&schema_file)
        .map_err(|e| AppError::with(code::SCHEMA_READ_FAILED, e))?;
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| AppError::with(code::SCHEMA_PARSE_FAILED, e))?;

    if let Some(name) = doc
        .get("schema")
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
    {
        opts.schema_name = name.to_string();
    }

    let custom = read_patch(&dir.join(format!("{}.custom.yaml", schema_id))).unwrap_or_default();

    // 模糊音是写在这个方案自己的 custom 里的
    if let Some(seq) = custom
        .get(serde_yaml::Value::String("speller/algebra/+".into()))
        .and_then(|v| v.as_sequence())
    {
        let rules: Vec<String> = seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        opts.fuzzy_pairs = fuzzy_ids_from_rules(&rules);
    }

    let Some(switches) = doc.get("switches").and_then(|v| v.as_sequence()) else {
        return Ok(opts);
    };

    for (index, sw) in switches.iter().enumerate() {
        let states: Vec<String> = sw
            .get("states")
            .and_then(|v| v.as_sequence())
            .map(|s| {
                s.iter()
                    .map(|x| x.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();

        if states.len() < 2 {
            continue;
        }

        let name = sw.get("name").and_then(|v| v.as_str());
        let schema_default = sw.get("reset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let key = format!("switches/@{}/reset", index);
        let user_value = custom
            .get(serde_yaml::Value::String(key))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        opts.switches.push(SchemaSwitch {
            index,
            label_key: switch_label_key(name, &states).to_string(),
            raw_name: name.map(String::from),
            current: user_value.unwrap_or(schema_default).min(states.len() - 1),
            configured: user_value.is_some(),
            states,
        });
    }

    Ok(opts)
}

/// 存某个方案的模糊音
#[tauri::command]
pub fn save_fuzzy(config_dir: String, schema: String, pairs: Vec<String>) -> AppResult<()> {
    use serde_yaml::Value;

    let path = Path::new(&config_dir).join(format!("{}.custom.yaml", schema));

    let mut rules: Vec<Value> = vec![];
    let mut seen = std::collections::HashSet::new();
    if supports_fuzzy(&schema) {
        for id in &pairs {
            for rule in fuzzy_rules_for_id(id) {
                if seen.insert(rule) {
                    rules.push(Value::String(rule.to_string()));
                }
            }
        }
    }

    if rules.is_empty() {
        merge_patch(&path, [], &["speller/algebra/+"])
    } else {
        merge_patch(
            &path,
            [("speller/algebra/+".to_string(), Value::Sequence(rules))],
            &[],
        )
    }
}

#[tauri::command]
pub fn save_schema_switch(
    config_dir: String,
    schema: String,
    index: usize,
    value: usize,
) -> AppResult<()> {
    let path = Path::new(&config_dir).join(format!("{}.custom.yaml", schema));

    merge_patch(
        &path,
        [(
            format!("switches/@{}/reset", index),
            serde_yaml::Value::Number(serde_yaml::Number::from(value as u64)),
        )],
        &[],
    )
}

#[tauri::command]
pub fn save_input_settings(config_dir: String, settings: InputSettings) -> AppResult<()> {
    use serde_yaml::Value;

    let dir = Path::new(&config_dir);
    std::fs::create_dir_all(dir).map_err(|e| AppError::with(code::CONFIG_DIR_CREATE_FAILED, e))?;

    // ═══ 全局设置 → default.custom.yaml ═══
    {
        let mut set: Vec<(String, Value)> = vec![(
            "menu/page_size".into(),
            Value::Number(serde_yaml::Number::from(settings.page_size as u64)),
        )];

        let mut switch_key = serde_yaml::Mapping::new();
        switch_key.insert(
            Value::String("Shift_L".into()),
            Value::String(settings.shift_l_behavior.clone()),
        );
        switch_key.insert(
            Value::String("Shift_R".into()),
            Value::String(settings.shift_r_behavior.clone()),
        );
        switch_key.insert(Value::String("Control_L".into()), Value::String("noop".into()));
        switch_key.insert(Value::String("Control_R".into()), Value::String("noop".into()));
        switch_key.insert(Value::String("Caps_Lock".into()), Value::String("clear".into()));
        set.push(("ascii_composer/switch_key".into(), Value::Mapping(switch_key)));

        let bindings = page_key_bindings(&settings.page_keys);
        let mut remove: Vec<&str> = vec![];
        if bindings.is_empty() {
            remove.push("key_binder/bindings/+");
        } else {
            set.push(("key_binder/bindings/+".into(), Value::Sequence(bindings)));
        }

        // schema_list 等其他键由 merge_patch 原样保留
        merge_patch(&dir.join("default.custom.yaml"), set, &remove)?;
    }

    // ═══ 候选词个数：方案自己声明了就得往方案里写 ═══
    //
    // `menu/page_size` 的优先级是「方案 > default」。像雾凇这种自己写了
    // page_size 的方案，只改 default.custom.yaml 等于白改 —— 滑杆拉了没反应。
    {
        use serde_yaml::Value;
        let size = Value::Number(serde_yaml::Number::from(settings.page_size as u64));
        for schema in crate::dict::enabled_schemas(&config_dir) {
            let declares_own = std::fs::read_to_string(dir.join(format!("{}.schema.yaml", schema)))
                .ok()
                .and_then(|c| serde_yaml::from_str::<Value>(&c).ok())
                .and_then(|d| {
                    d.get("menu")
                        .and_then(|m| m.get("page_size"))
                        .map(|_| true)
                })
                .unwrap_or(false);
            if declares_own {
                merge_patch(
                    &dir.join(format!("{}.custom.yaml", schema)),
                    [("menu/page_size".to_string(), size.clone())],
                    &[],
                )?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn read_input_settings(config_dir: String) -> AppResult<InputSettings> {
    let dir = Path::new(&config_dir);
    let mut settings = InputSettings::default();

    if let Some(patch) = read_patch(&dir.join("default.custom.yaml")) {
        let get = |k: &str| patch.get(serde_yaml::Value::String(k.to_string()));

        if let Some(v) = get("menu/page_size").and_then(|v| v.as_u64()) {
            settings.page_size = v as u32;
        }
        if let Some(sk) = get("ascii_composer/switch_key") {
            if let Some(v) = sk.get("Shift_L").and_then(|v| v.as_str()) {
                settings.shift_l_behavior = v.to_string();
            }
            if let Some(v) = sk.get("Shift_R").and_then(|v| v.as_str()) {
                settings.shift_r_behavior = v.to_string();
            }
        }
        if let Some(id) = get("key_binder/bindings/+").and_then(page_keys_from_bindings) {
            settings.page_keys = id;
        }
    }

    // 用跟主题预览同一套优先级算，两处显示的数才对得上
    settings.page_size = crate::config::effective_page_size(dir);

    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_config(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("xgrime-test-{}-{}", tag, nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn schema_list_of(dir: &Path) -> Vec<String> {
        let content = std::fs::read_to_string(dir.join("default.custom.yaml")).unwrap_or_default();
        let doc: serde_yaml::Value = serde_yaml::from_str(&content).unwrap_or_default();
        doc.get("patch")
            .and_then(|p| p.get("schema_list"))
            .and_then(|v| v.as_sequence())
            .map(|s| {
                s.iter()
                    .filter_map(|i| i.get("schema").and_then(|v| v.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 回归测试：以前这里是整份覆盖写 default.custom.yaml，
    /// 结果用户一存输入设置，词库页装好的方案就从输入法选单里消失了。
    #[test]
    fn saving_input_settings_must_not_wipe_the_schema_list() {
        let dir = temp_config("keep-schema-list");
        let path = dir.to_string_lossy().to_string();

        crate::dict::set_schema_enabled(&path, "jyut6ping3", true).unwrap();
        crate::dict::set_schema_enabled(&path, "rime_ice", true).unwrap();
        assert_eq!(schema_list_of(&dir), ["jyut6ping3", "rime_ice"]);

        save_input_settings(
            path.clone(),
            InputSettings {
                page_size: 9,
                page_keys: "bracket".into(),
                ..Default::default()
            },
        )
        .unwrap();

        // 方案列表还在，设置也确实写进去了
        assert_eq!(schema_list_of(&dir), ["jyut6ping3", "rime_ice"]);
        assert_eq!(read_input_settings(path).unwrap().page_size, 9);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 反过来也要成立：装方案不能把已有的输入设置冲掉
    #[test]
    fn enabling_a_schema_must_not_wipe_input_settings() {
        let dir = temp_config("keep-settings");
        let path = dir.to_string_lossy().to_string();

        save_input_settings(
            path.clone(),
            InputSettings {
                page_size: 7,
                ..Default::default()
            },
        )
        .unwrap();
        crate::dict::set_schema_enabled(&path, "jyut6ping3", true).unwrap();

        assert_eq!(read_input_settings(path).unwrap().page_size, 7);
        assert_eq!(schema_list_of(&dir), ["jyut6ping3"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 以前存了读不回来，重开应用翻页键永远显示成「减号等号」
    #[test]
    fn global_settings_survive_a_round_trip() {
        let dir = temp_config("round-trip");
        let path = dir.to_string_lossy().to_string();

        let original = InputSettings {
            page_size: 6,
            page_keys: "tab".into(),
            shift_l_behavior: "inline_ascii".into(),
            shift_r_behavior: "commit_text".into(),
        };
        save_input_settings(path.clone(), original.clone()).unwrap();

        let back = read_input_settings(path).unwrap();
        assert_eq!(back.page_keys, "tab");
        assert_eq!(back.page_size, 6);
        assert_eq!(back.shift_l_behavior, "inline_ascii");
        assert_eq!(back.shift_r_behavior, "commit_text");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 模糊音是按方案存的：改甲方案不该动到乙方案
    #[test]
    fn fuzzy_is_stored_per_schema() {
        let dir = temp_config("fuzzy");
        let path = dir.to_string_lossy().to_string();

        save_fuzzy(
            path.clone(),
            "luna_pinyin".into(),
            vec!["n_l".into(), "in_ing".into()],
        )
        .unwrap();

        let read_back = |schema: &str| -> Vec<String> {
            read_patch(&dir.join(format!("{}.custom.yaml", schema)))
                .and_then(|p| {
                    p.get(serde_yaml::Value::String("speller/algebra/+".into()))
                        .and_then(|v| v.as_sequence())
                        .map(|seq| {
                            fuzzy_ids_from_rules(
                                &seq.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect::<Vec<_>>(),
                            )
                        })
                })
                .unwrap_or_default()
        };

        assert_eq!(read_back("luna_pinyin"), ["n_l", "in_ing"]);
        // 另一个方案完全没被碰过
        assert!(!dir.join("rime_ice.custom.yaml").exists());

        // 清空要能真的清掉
        save_fuzzy(path, "luna_pinyin".into(), vec![]).unwrap();
        assert!(read_back("luna_pinyin").is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 读不懂的 custom.yaml 绝不能被当成空的覆盖掉
    ///
    /// 用户手写坏了一行 YAML，我们再往里存个设置 —— 要是当成空的接着写，
    /// 他的 schema_list、按键绑定就全没了。宁可这次存不进去。
    #[test]
    fn broken_custom_yaml_is_never_silently_overwritten() {
        let dir = temp_config("broken-yaml");
        let path = dir.join("default.custom.yaml");
        let broken = "patch:
  schema_list:
    - schema: rime_ice
  bad: [unclosed
";
        std::fs::write(&path, broken).unwrap();

        let result = merge_patch(
            &path,
            [("menu/page_size".to_string(), serde_yaml::Value::from(9u64))],
            &[],
        );
        assert!(result.is_err(), "读不懂还照写，等于把用户的配置截断了");
        // 原文件一个字都不许动
        assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 方案自己声明了 page_size 的话，光改 default 是不生效的
    #[test]
    fn page_size_is_written_where_it_actually_wins() {
        let dir = temp_config("page-size");
        let path = dir.to_string_lossy().to_string();
        crate::dict::set_schema_enabled(&path, "rime_ice", true).unwrap();
        // 方案自带 page_size，优先级高于 default
        std::fs::write(
            dir.join("rime_ice.schema.yaml"),
            "schema:
  schema_id: rime_ice
menu:
  page_size: 9
",
        )
        .unwrap();

        save_input_settings(
            path.clone(),
            InputSettings { page_size: 6, ..Default::default() },
        )
        .unwrap();

        // 得写进方案自己的 custom 才压得住
        let custom = std::fs::read_to_string(dir.join("rime_ice.custom.yaml")).unwrap();
        assert!(custom.contains("menu/page_size"), "没写进方案：{}", custom);
        // 读回来也要是 6，不能又被方案的 9 盖回去
        assert_eq!(read_input_settings(path).unwrap().page_size, 6);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// 简繁 / 标点这些开关是从方案文件里读出来的，不是写死的
    #[test]
    fn schema_switches_are_read_from_the_schema_itself() {
        let dir = temp_config("switches");
        let path = dir.to_string_lossy().to_string();
        crate::dict::set_schema_enabled(&path, "jyut6ping3", true).unwrap();
        crate::dict::set_schema_enabled(&path, "luna_pinyin", true).unwrap();

        // 摘自 rime-cantonese 的真实 switches 段落
        std::fs::write(
            dir.join("jyut6ping3.schema.yaml"),
            r#"
schema:
  schema_id: jyut6ping3
  name: 粵語拼音
switches:
  - name: ascii_mode
    reset: 0
    states: [ 粵, 英 ]
  - name: full_shape
    states: [ 半角, 全角 ]
  - options: [ noop, variants_hk, trad_tw, simplification ]
    states: [ 傳統漢字, 香港傳統漢字, 臺灣傳統漢字, 大陆简化汉字 ]
    reset: 1
  - name: ascii_punct
    states: [ 。，, ．， ]
"#,
        )
        .unwrap();

        let opts = read_schema_options(path.clone(), None).unwrap();
        assert!(!opts.missing);
        // 默认读方案选单里的第一个
        assert_eq!(opts.schema_id, "jyut6ping3");
        // 装了两个方案，前端才有得选
        assert_eq!(opts.available.len(), 2);
        assert_eq!(opts.schema_name, "粵語拼音");
        // 粤拼不是汉语拼音，模糊音那套 derive 规则不该出现
        assert!(!opts.supports_fuzzy);
        assert_eq!(opts.switches.len(), 4);

        // 四选一的字形开关，默认停在方案自己写的 reset: 1
        let variants = &opts.switches[2];
        assert_eq!(variants.states.len(), 4);
        assert_eq!(variants.current, 1);
        assert!(!variants.configured);
        assert_eq!(variants.label_key, "variants");
        assert_eq!(variants.raw_name, None);
        assert_eq!(opts.switches[3].label_key, "ascii_punct");
        assert_eq!(opts.switches[0].label_key, "ascii_mode");

        // 改成「臺灣傳統漢字」后要能读回来，而且方案列表不受影响
        save_schema_switch(path.clone(), "jyut6ping3".into(), 2, 2).unwrap();
        let after = read_schema_options(path.clone(), Some("jyut6ping3".into())).unwrap();
        assert_eq!(after.switches[2].current, 2);
        assert!(after.switches[2].configured);
        assert_eq!(schema_list_of(&dir), ["jyut6ping3", "luna_pinyin"]);

        // 指定一个没启用的方案时，退回第一个而不是报错
        let fallback = read_schema_options(path, Some("wubi86".into())).unwrap();
        assert_eq!(fallback.schema_id, "jyut6ping3");

        std::fs::remove_dir_all(&dir).ok();
    }
}

// ═══════════════════════ 切换当前方案 ═══════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSchema {
    /// 现在正在用的那个
    pub current: String,
    /// 方案选单里全部启用中的
    pub available: Vec<SchemaBrief>,
}

/// 小狼毫把「上次用的是哪个方案」记在 user.yaml 里
fn user_yaml(config_dir: &Path) -> PathBuf {
    config_dir.join("user.yaml")
}

fn previously_selected(config_dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(user_yaml(config_dir)).ok()?;
    serde_yaml::from_str::<serde_yaml::Value>(&raw)
        .ok()?
        .get("var")?
        .get("previously_selected_schema")?
        .as_str()
        .map(String::from)
}

/// 现在正在用的那个方案；对不上就退回方案选单第一个
pub fn current_schema(config_dir: &Path) -> String {
    let enabled = crate::dict::enabled_schemas(&config_dir.to_string_lossy());
    previously_selected(config_dir)
        .filter(|id| enabled.contains(id))
        .unwrap_or_else(|| detect_primary_schema(config_dir))
}

#[tauri::command]
pub fn read_active_schema(config_dir: String) -> AppResult<ActiveSchema> {
    let dir = Path::new(&config_dir);
    let available: Vec<SchemaBrief> = crate::dict::enabled_schemas(&config_dir)
        .into_iter()
        .map(|id| SchemaBrief {
            name: schema_display_name(dir, &id),
            schema_id: id,
        })
        .collect();

    // user.yaml 里那个可能指向已经删掉的方案，对不上就退回选单里的第一个
    let current = previously_selected(dir)
        .filter(|id| available.iter().any(|s| &s.schema_id == id))
        .or_else(|| available.first().map(|s| s.schema_id.clone()))
        .unwrap_or_default();

    Ok(ActiveSchema { current, available })
}

/// 换成另一个输入方案
///
/// 只动 `user.yaml` 的 `previously_selected_schema` —— 那是小狼毫开新会话时
/// 读的那一项。**不碰 `schema_list`**：那是用户自己排的方案选单顺序，
/// 按 Ctrl+` 弹出来就是这个次序，切一次输入法就把人家的顺序打乱说不过去。
///
/// 调用顺序是死的：**停服 → 改文件 → 启动**，三步都不能少也不能换位置。
///
/// - 只改文件不重启：服务把当前方案缓在内存里，改了也不看，实测切不过去
/// - 先改文件再停服：服务退出时把内存里那份写回 `user.yaml`，刚写的直接被盖掉
#[tauri::command]
pub fn switch_active_schema(config_dir: String, schema: String) -> AppResult<ActiveSchema> {
    let dir = Path::new(&config_dir);
    let list = crate::dict::enabled_schemas(&config_dir);
    if !list.contains(&schema) {
        return Err(AppError::with(code::SCHEMA_NOT_ENABLED, schema));
    }

    // 只动这一个键，last_build_time / schema_access_time 要留着
    let path = user_yaml(dir);
    let mut doc = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_yaml::from_str::<serde_yaml::Value>(&raw).ok())
        .unwrap_or_else(|| serde_yaml::Value::Mapping(Default::default()));
    if !doc.is_mapping() {
        doc = serde_yaml::Value::Mapping(Default::default());
    }
    let map = doc.as_mapping_mut().expect("刚保证过是 mapping");
    let var_key = serde_yaml::Value::String("var".into());
    if !map.get(&var_key).map(|v| v.is_mapping()).unwrap_or(false) {
        map.insert(var_key.clone(), serde_yaml::Value::Mapping(Default::default()));
    }
    map.get_mut(&var_key)
        .and_then(|v| v.as_mapping_mut())
        .expect("刚建过")
        .insert(
            serde_yaml::Value::String("previously_selected_schema".into()),
            serde_yaml::Value::String(schema.clone()),
        );
    let text = serde_yaml::to_string(&doc).map_err(|e| AppError::with(code::YAML_SERIALIZE_FAILED, e))?;
    std::fs::write(&path, text).map_err(|e| AppError::with(code::FILE_WRITE_FAILED, e))?;

    read_active_schema(config_dir)
}

/// 停服 → 改文件 → 启动，一条命令走完
///
/// 拆成三条给前端调的话，顺序错了就是「看着切了、其实没切」这种最难查的毛病，
/// 所以在这边一次做完。
#[tauri::command]
pub async fn switch_schema_and_restart(
    config_dir: String,
    schema: String,
) -> AppResult<ActiveSchema> {
    tauri::async_runtime::spawn_blocking(move || {
        let platform = crate::platform::current_platform();
        let info = platform.detect();
        platform.stop_service(info.install_dir.as_deref())?;
        let result = switch_active_schema(config_dir, schema);
        // 写文件失败也要把服务拉回来，不然用户直接没输入法用了
        platform.start_service(info.install_dir.as_deref())?;
        result
    })
    .await
    .map_err(|e| AppError::with(code::DEPLOYER_LAUNCH_FAILED, e))?
}
