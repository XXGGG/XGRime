use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 前端传入的主题配置（CSS #RRGGBB 格式）
// Default + serde(default)：以后往里加字段时，用户存过的旧预设还能读出来，
// 不会因为少一个字段整份解析失败
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeConfig {
    pub name: String,
    pub back_color: String,
    pub border_color: String,
    pub text_color: String,
    pub hilited_text_color: String,
    pub hilited_back_color: String,
    pub candidate_text_color: String,
    pub comment_text_color: String,
    pub label_color: String,
    pub hilited_candidate_text_color: String,
    pub hilited_candidate_back_color: String,
    pub hilited_candidate_label_color: String,
    pub hilited_comment_text_color: String,
    /// 选中项前面那个标记的颜色。空 = 不显示。
    /// 小狼毫默认透明（不显示）；设成不透明色就会画出 Windows 11 那种竖条标记。
    pub hilited_mark_color: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LayoutConfig {
    pub horizontal: bool,
    pub inline_preedit: bool,
    pub font_face: String,
    pub font_size: u32,
    pub corner_radius: u32,
    pub border_width: u32,
    // 高级
    pub margin_x: u32,
    pub margin_y: u32,
    /// 高亮块的左右内边距。小狼毫是分开的两个键，之前我们只给了一个，
    /// 想让高亮块横向宽一点、纵向薄一点做不到。
    pub hilite_padding_x: u32,
    pub hilite_padding_y: u32,
    pub candidate_spacing: u32,
    pub hilite_spacing: u32,
    pub spacing: u32,
    pub round_corner: u32,
    pub shadow_radius: u32,
    pub label_font_face: String,
    pub label_font_size: u32,
    /// 候选框最小宽度。小狼毫默认 160，**这就是候选框看起来比系统输入法宽一大截的原因**，
    /// 之前没暴露出来，用户怎么调边距都没用。0 = 不设下限，跟着内容走。
    pub min_width: u32,
    /// 0 = 不限
    pub max_width: u32,
    /// 序号怎么显示，小狼毫默认 "%s."（出来是 1. 2. 3.）
    pub label_format: String,
    /// 标记用什么字符。空字符串 = 画成一条竖杠（Win11 风格）
    pub mark_text: String,
}

/// 切换中英文、切换方案时弹的那个提示
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyConfig {
    /// always = 都弹，never = 都不弹
    pub mode: String,
    /// 弹多久（毫秒），小狼毫默认 1200
    pub duration_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleConfig {
    pub theme: ThemeConfig,
    pub layout: LayoutConfig,
    pub notify: NotifyConfig,
    /// 一页几个候选。预览要照这个数画，不然跟真的对不上
    pub page_size: u32,
}

/// CSS #RRGGBB → RIME BGR 0xBBGGRR 数字
fn hex_to_bgr(hex: &str) -> u64 {
    let clean = hex.trim_start_matches('#');
    let r = u64::from_str_radix(&clean[0..2], 16).unwrap_or(0);
    let g = u64::from_str_radix(&clean[2..4], 16).unwrap_or(0);
    let b = u64::from_str_radix(&clean[4..6], 16).unwrap_or(0);
    (b << 16) | (g << 8) | r
}

/// CSS #RRGGBB → 配色方案里那种 `0xAABBGGRR` 写法
///
/// 千万别写成十进制数字。`_RimeGetColor` 只有认出十六进制串才走字符串分支，
/// 否则退回 `config_get_int` —— 那是个 32 位有符号 int，带 alpha 的颜色
/// （0xff…… 一律大于 INT_MAX）会解析失败，整项静默退回默认色。
/// 表现就是：主题页调什么都不生效，选中标记因为默认值是 0 直接不画。
fn bgr_literal(hex: &str) -> serde_yaml::Value {
    serde_yaml::Value::String(format!("0x{:08X}", hex_to_bgr(hex) | 0xff00_0000))
}

/// BGR 数字 → CSS #RRGGBB
fn bgr_to_hex(bgr: u64) -> String {
    let r = bgr & 0xff;
    let g = (bgr >> 8) & 0xff;
    let b = (bgr >> 16) & 0xff;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// 生成 squirrel.custom.yaml 或 weasel.custom.yaml 的 patch 内容
fn build_yaml_patch(config: &StyleConfig) -> BTreeMap<String, serde_yaml::Value> {
    use serde_yaml::Value;

    let t = &config.theme;
    let l = &config.layout;

    let mut color_scheme = BTreeMap::new();
    color_scheme.insert("name".to_string(), Value::String("XGRime 自定义主题".to_string()));
    color_scheme.insert("back_color".to_string(), bgr_literal(&t.back_color));
    color_scheme.insert("border_color".to_string(), bgr_literal(&t.border_color));
    color_scheme.insert("text_color".to_string(), bgr_literal(&t.text_color));
    color_scheme.insert("hilited_text_color".to_string(), bgr_literal(&t.hilited_text_color));
    color_scheme.insert("hilited_back_color".to_string(), bgr_literal(&t.hilited_back_color));
    color_scheme.insert("candidate_text_color".to_string(), bgr_literal(&t.candidate_text_color));
    color_scheme.insert("comment_text_color".to_string(), bgr_literal(&t.comment_text_color));
    color_scheme.insert("label_color".to_string(), bgr_literal(&t.label_color));
    color_scheme.insert("hilited_candidate_text_color".to_string(), bgr_literal(&t.hilited_candidate_text_color));
    color_scheme.insert("hilited_candidate_back_color".to_string(), bgr_literal(&t.hilited_candidate_back_color));
    // 两个键都写：小狼毫读的是 `hilited_label_color`，鼠须管读的是
    // `hilited_candidate_label_color`。只写后者的话，Windows 上这一项形同虚设。
    let hl_label = bgr_literal(&t.hilited_candidate_label_color);
    color_scheme.insert("hilited_label_color".to_string(), hl_label.clone());
    color_scheme.insert("hilited_candidate_label_color".to_string(), hl_label);
    color_scheme.insert("hilited_comment_text_color".to_string(), bgr_literal(&t.hilited_comment_text_color));

    let mut patch = BTreeMap::new();
    // 系统在深色模式下，小狼毫读的是 `style/color_scheme_dark`，只写 `style/color_scheme`
    // 等于白配 —— 界面上调的颜色全被词库自带的深色方案盖掉，连选中标记都跟着不见。
    // 两个键都指向同一套，用户调什么就是什么，跟系统模式无关。
    patch.insert("style/color_scheme".to_string(), Value::String("xgrime_custom".to_string()));
    patch.insert("style/color_scheme_dark".to_string(), Value::String("xgrime_custom".to_string()));
    patch.insert(
        "preset_color_schemes/xgrime_custom".to_string(),
        Value::Mapping(color_scheme.into_iter().map(|(k, v)| (Value::String(k), v)).collect()),
    );

    // style 直属设置
    patch.insert("style/horizontal".into(), Value::Bool(l.horizontal));
    patch.insert("style/inline_preedit".into(), Value::Bool(l.inline_preedit));
    patch.insert("style/font_point".into(), Value::Number(serde_yaml::Number::from(l.font_size as u64)));
    if !l.font_face.is_empty() {
        patch.insert("style/font_face".into(), Value::String(l.font_face.clone()));
    }
    if !l.label_font_face.is_empty() {
        patch.insert("style/label_font_face".into(), Value::String(l.label_font_face.clone()));
    }
    if l.label_font_size > 0 {
        patch.insert("style/label_font_point".into(), Value::Number(serde_yaml::Number::from(l.label_font_size as u64)));
    }

    // style/layout/ 空间属性
    let n = |v: u32| Value::Number(serde_yaml::Number::from(v as u64));
    patch.insert("style/layout/border_width".into(), n(l.border_width));
    patch.insert("style/layout/corner_radius".into(), n(l.corner_radius));
    patch.insert("style/layout/margin_x".into(), n(l.margin_x));
    patch.insert("style/layout/margin_y".into(), n(l.margin_y));
    patch.insert("style/layout/hilite_padding_x".into(), n(l.hilite_padding_x));
    patch.insert("style/layout/hilite_padding_y".into(), n(l.hilite_padding_y));
    patch.insert("style/layout/candidate_spacing".into(), n(l.candidate_spacing));
    patch.insert("style/layout/hilite_spacing".into(), n(l.hilite_spacing));
    patch.insert("style/layout/spacing".into(), n(l.spacing));
    patch.insert("style/layout/round_corner".into(), n(l.round_corner));
    patch.insert("style/layout/shadow_radius".into(), n(l.shadow_radius));
    if !l.label_format.is_empty() {
        patch.insert("style/label_format".into(), Value::String(l.label_format.clone()));
    }
    if !config.theme.hilited_mark_color.is_empty() {
        color_scheme_mark(&mut patch, &config.theme.hilited_mark_color, &l.mark_text);
    }
    patch.insert("style/layout/min_width".into(), n(l.min_width));
    patch.insert("style/layout/max_width".into(), n(l.max_width));

    // ── 中英切换那个提示 ──
    let notify_on = config.notify.mode != "never";
    #[cfg(target_os = "macos")]
    {
        // 鼠须管用的是另一个键，取值也不同
        patch.insert(
            "show_notifications_when".into(),
            Value::String(if notify_on { "always".into() } else { "never".into() }),
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        patch.insert("show_notifications".into(), Value::Bool(notify_on));
        patch.insert(
            "show_notifications_time".into(),
            Value::Number(serde_yaml::Number::from(config.notify.duration_ms as u64)),
        );
    }

    patch
}

/// 获取平台对应的 custom.yaml 文件名
fn platform_custom_yaml() -> &'static str {
    #[cfg(target_os = "windows")]
    { "weasel.custom.yaml" }
    #[cfg(target_os = "macos")]
    { "squirrel.custom.yaml" }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    { "rime.custom.yaml" }
}

#[tauri::command]
pub fn save_theme_config(config_dir: String, config: StyleConfig) -> AppResult<()> {
    let dir = Path::new(&config_dir);
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|e| AppError::with(code::CONFIG_DIR_CREATE_FAILED, e))?;
    }

    let patch = build_yaml_patch(&config);
    let file_path = dir.join(platform_custom_yaml());

    // 合并而不是整份覆盖：这个文件里可能还有用户自己写的 app_options 之类，
    // 覆盖会把它们一并抹掉 —— 跟 settings.rs 当初那个坑是同一种。
    crate::settings::merge_patch(&file_path, patch, &[])?;

    Ok(())
}

// ═══════════════════ 读「实际生效」的样式 ═══════════════════

/// RIME 的样式不是只看我们写的那份 patch
///
/// 真正生效的是一条链：小狼毫自带的 `data/weasel.yaml` → 用户目录里发行版铺的
/// `weasel.yaml`（雾凇就带了一份，还指定了 `color_scheme: purity_of_form_custom`）
/// → 最后才是我们的 `weasel.custom.yaml`。
/// 之前只读最后一环，所以界面上的预览跟屏幕上真正弹出来的候选框对不上。
fn load_yaml(path: &Path) -> Option<serde_yaml::Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&raw).ok()
}

/// 按优先级从低到高排好的配置源
fn style_sources(config_dir: &Path) -> Vec<serde_yaml::Value> {
    let mut out = Vec::new();
    let name = platform_custom_yaml().trim_end_matches(".custom.yaml");

    // 1. 程序自带的
    if let Some(shared) = crate::dict::shared_data_dir() {
        if let Some(v) = load_yaml(&shared.join(format!("{}.yaml", name))) {
            out.push(v);
        }
    }
    // 2. 用户目录里发行版铺的
    if let Some(v) = load_yaml(&config_dir.join(format!("{}.yaml", name))) {
        out.push(v);
    }
    // 3. 我们自己的 patch（结构不同，键是扁平的，单独处理）
    out
}

/// 从 `style:` 这一层取值，后面的源覆盖前面的
fn style_get<'a>(sources: &'a [serde_yaml::Value], key: &str) -> Option<&'a serde_yaml::Value> {
    sources
        .iter()
        .rev()
        .find_map(|doc| doc.get("style").and_then(|s| s.get(key)))
}

fn layout_get<'a>(sources: &'a [serde_yaml::Value], key: &str) -> Option<&'a serde_yaml::Value> {
    sources.iter().rev().find_map(|doc| {
        doc.get("style")
            .and_then(|s| s.get("layout"))
            .and_then(|l| l.get(key))
    })
}

/// 把标记色和标记字符写进配色方案
fn color_scheme_mark(
    patch: &mut BTreeMap<String, serde_yaml::Value>,
    mark_color: &str,
    mark_text: &str,
) {
    use serde_yaml::Value;
    // 必须是不透明的才会被画出来，bgr_literal 已经把 alpha 补成 ff
    if let Some(Value::Mapping(scheme)) = patch.get_mut("preset_color_schemes/xgrime_custom") {
        scheme.insert(
            Value::String("hilited_mark_color".into()),
            bgr_literal(mark_color),
        );
    }
    patch.insert("style/mark_text".into(), Value::String(mark_text.to_string()));
}

/// 只有 alpha 不为 0 的颜色才算「开着」，为 0 表示这个东西不画
fn opaque_color(node: Option<&serde_yaml::Value>) -> Option<String> {
    let v = node?;
    // 位数是关键：`0x00ffffff` 和 `0xffffff` 数值相同，但前者 alpha 是 0（透明），
    // 后者是六位写法（librime 会补成不透明）。只看数值区分不出来。
    let (n, explicit_alpha) = match v {
        serde_yaml::Value::Number(num) => (num.as_u64()?, false),
        serde_yaml::Value::String(s) => {
            let t = s.trim();
            let hex = t
                .strip_prefix("0x")
                .or_else(|| t.strip_prefix("0X"))
                .or_else(|| t.strip_prefix('#'))?;
            (u64::from_str_radix(hex, 16).ok()?, hex.len() >= 8)
        }
        _ => return None,
    };

    let alpha = if explicit_alpha || n > 0xff_ffff {
        (n >> 24) & 0xff
    } else {
        0xff // 六位写法按不透明算，这是 librime 的规则
    };
    if alpha == 0 {
        None
    } else {
        Some(bgr_to_hex(n & 0x00FF_FFFF))
    }
}

/// 颜色在 YAML 里是 `0xBBGGRR`，也可能带一字节 alpha（`0xAABBGGRR`）
fn color_of(node: Option<&serde_yaml::Value>) -> Option<String> {
    let v = node?;
    let n = match v {
        serde_yaml::Value::Number(num) => num.as_u64()?,
        serde_yaml::Value::String(s) => {
            let t = s.trim();
            u64::from_str_radix(t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))?, 16).ok()?
        }
        _ => return None,
    };
    Some(bgr_to_hex(n & 0x00FF_FFFF))
}

/// 找出当前配色方案的那一组颜色
fn active_scheme(
    sources: &[serde_yaml::Value],
    patch: Option<&serde_yaml::Value>,
) -> (String, Option<serde_yaml::Value>) {
    // 深色模式下小狼毫认的是另一个键，读的时候也得跟着它走，
    // 否则「读取当前配置」拿回来的是一套根本没在用的颜色。
    let key = if crate::platform::system_dark_mode() {
        "color_scheme_dark"
    } else {
        "color_scheme"
    };
    // patch 里的键是扁平的 `style/xxx`
    let name = patch
        .and_then(|p| p.get(format!("style/{}", key)))
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| style_get(sources, key).and_then(|v| v.as_str().map(String::from)))
        .or_else(|| style_get(sources, "color_scheme").and_then(|v| v.as_str().map(String::from)))
        .unwrap_or_else(|| "aqua".into());

    // patch 自带的方案优先（就是我们上次保存的那套）
    if let Some(scheme) = patch.and_then(|p| p.get(format!("preset_color_schemes/{}", name))) {
        return (name, Some(scheme.clone()));
    }
    let found = sources.iter().rev().find_map(|doc| {
        doc.get("preset_color_schemes")
            .and_then(|m| m.get(&name))
            .cloned()
    });
    (name, found)
}

/// 一页显示几个候选
pub fn effective_page_size(config_dir: &Path) -> u32 {
    let from = |v: Option<&serde_yaml::Value>| v.and_then(|x| x.as_u64()).map(|x| x as u32);

    // 方案自己的设定优先，其次 default.custom.yaml，最后 default.yaml
    let schema = crate::settings::detect_primary_schema(config_dir);
    if let Some(doc) = load_yaml(&config_dir.join(format!("{}.custom.yaml", schema))) {
        if let Some(v) = from(doc.get("patch").and_then(|p| p.get("menu/page_size"))) {
            return v;
        }
    }
    if let Some(doc) = load_yaml(&config_dir.join(format!("{}.schema.yaml", schema))) {
        if let Some(v) = from(doc.get("menu").and_then(|m| m.get("page_size"))) {
            return v;
        }
    }
    if let Some(doc) = load_yaml(&config_dir.join("default.custom.yaml")) {
        if let Some(v) = from(doc.get("patch").and_then(|p| p.get("menu/page_size"))) {
            return v;
        }
    }
    if let Some(doc) = load_yaml(&config_dir.join("default.yaml")) {
        if let Some(v) = from(doc.get("menu").and_then(|m| m.get("page_size"))) {
            return v;
        }
    }
    5
}

#[tauri::command]
pub fn read_theme_config(config_dir: String) -> AppResult<Option<StyleConfig>> {
    let dir = Path::new(&config_dir);
    let sources = style_sources(dir);
    let custom = load_yaml(&dir.join(platform_custom_yaml()));
    let patch = custom.as_ref().and_then(|d| d.get("patch"));

    // 一份配置都没有 = 输入法还没装好，让前端保持自己的默认值
    if sources.is_empty() && patch.is_none() {
        return Ok(None);
    }

    let (scheme_name, scheme) = active_scheme(&sources, patch);
    let color = |key: &str, fallback: &str| -> String {
        scheme
            .as_ref()
            .and_then(|s| color_of(s.get(key)))
            .unwrap_or_else(|| fallback.to_string())
    };

    // 配色方案常常只写必需的几项，剩下的靠 fallback。这条链抄自
    // weasel 的 `_UpdateUIStyleColor`，顺序不能想当然 ——
    // 比如 border_color 回落到的是**文字色**，不是背景色。
    let back = color("back_color", "#ffffff");
    let text = color("text_color", "#000000");
    let candidate = color("candidate_text_color", &text);
    let hilited_text = color("hilited_text_color", &text);
    let hilited_back = color("hilited_back_color", &back);
    let hilited_candidate = color("hilited_candidate_text_color", &hilited_text);
    let hilited_candidate_back = color("hilited_candidate_back_color", &hilited_back);
    let label = color("label_color", &candidate);
    // 小狼毫读的是 hilited_label_color，鼠须管读的是 hilited_candidate_label_color，
    // 两个都认一下
    let hilited_label = scheme
        .as_ref()
        .and_then(|s| {
            color_of(s.get("hilited_label_color"))
                .or_else(|| color_of(s.get("hilited_candidate_label_color")))
        })
        .unwrap_or_else(|| hilited_candidate.clone());

    let theme = ThemeConfig {
        name: scheme_name,
        back_color: back.clone(),
        border_color: color("border_color", &text),
        text_color: text.clone(),
        hilited_text_color: hilited_text,
        hilited_back_color: hilited_back,
        candidate_text_color: candidate,
        comment_text_color: color("comment_text_color", &label),
        label_color: label,
        hilited_candidate_text_color: hilited_candidate.clone(),
        hilited_candidate_back_color: hilited_candidate_back,
        hilited_candidate_label_color: hilited_label.clone(),
        hilited_comment_text_color: color("hilited_comment_text_color", &hilited_label),
        // 默认 0 = 全透明 = 不画标记
        hilited_mark_color: scheme
            .as_ref()
            .and_then(|s| opaque_color(s.get("hilited_mark_color")))
            .unwrap_or_default(),
    };

    // 数值：patch 的扁平键优先，其次 weasel.yaml 的嵌套结构，最后小狼毫出厂默认
    let num = |flat: &str, nested: &str, default: u32| -> u32 {
        patch
            .and_then(|p| p.get(flat))
            .and_then(|v| v.as_u64())
            .or_else(|| layout_get(&sources, nested).and_then(|v| v.as_u64()))
            .map(|v| v as u32)
            .unwrap_or(default)
    };
    let top_num = |flat: &str, key: &str, default: u32| -> u32 {
        patch
            .and_then(|p| p.get(flat))
            .and_then(|v| v.as_u64())
            .or_else(|| style_get(&sources, key).and_then(|v| v.as_u64()))
            .map(|v| v as u32)
            .unwrap_or(default)
    };
    let flag = |flat: &str, key: &str, default: bool| -> bool {
        patch
            .and_then(|p| p.get(flat))
            .and_then(|v| v.as_bool())
            .or_else(|| style_get(&sources, key).and_then(|v| v.as_bool()))
            .unwrap_or(default)
    };
    let text_of = |flat: &str, key: &str, default: &str| -> String {
        patch
            .and_then(|p| p.get(flat))
            .and_then(|v| v.as_str())
            .or_else(|| style_get(&sources, key).and_then(|v| v.as_str()))
            .unwrap_or(default)
            .to_string()
    };

    let layout = LayoutConfig {
        horizontal: flag("style/horizontal", "horizontal", false),
        inline_preedit: flag("style/inline_preedit", "inline_preedit", false),
        font_face: text_of("style/font_face", "font_face", ""),
        font_size: top_num("style/font_point", "font_point", 14),
        label_font_face: text_of("style/label_font_face", "label_font_face", ""),
        label_font_size: top_num("style/label_font_point", "label_font_point", 14),
        label_format: text_of("style/label_format", "label_format", "%s."),
        mark_text: text_of("style/mark_text", "mark_text", ""),
        // 下面这些默认值抄自小狼毫出厂的 weasel.yaml，别乱改
        corner_radius: num("style/layout/corner_radius", "corner_radius", 4),
        border_width: num("style/layout/border_width", "border_width", 3),
        margin_x: num("style/layout/margin_x", "margin_x", 12),
        margin_y: num("style/layout/margin_y", "margin_y", 12),
        // 小狼毫自己也是先看 _x / _y，没有才回落到 hilite_padding
        hilite_padding_x: num("style/layout/hilite_padding_x", "hilite_padding_x", 0)
            .max(num("style/layout/hilite_padding", "hilite_padding", 2)),
        hilite_padding_y: num("style/layout/hilite_padding_y", "hilite_padding_y", 0)
            .max(num("style/layout/hilite_padding", "hilite_padding", 2)),
        candidate_spacing: num("style/layout/candidate_spacing", "candidate_spacing", 5),
        hilite_spacing: num("style/layout/hilite_spacing", "hilite_spacing", 4),
        spacing: num("style/layout/spacing", "spacing", 10),
        round_corner: num("style/layout/round_corner", "round_corner", 4),
        shadow_radius: num("style/layout/shadow_radius", "shadow_radius", 0),
        min_width: num("style/layout/min_width", "min_width", 160),
        max_width: num("style/layout/max_width", "max_width", 0),
    };

    // 两个平台的键不一样：小狼毫是布尔的 show_notifications，
    // 鼠须管是字符串的 show_notifications_when。存什么就得读什么，
    // 不然设置存进去了、下次打开又显示成开着。
    let notify_when = |doc: &serde_yaml::Value| -> Option<bool> {
        doc.get("show_notifications_when")
            .and_then(|v| v.as_str())
            .map(|s| s != "never")
    };
    let notify_on = patch
        .and_then(|p| p.get("show_notifications").and_then(|v| v.as_bool()))
        .or_else(|| patch.and_then(notify_when))
        .or_else(|| {
            sources.iter().rev().find_map(|d| {
                d.get("show_notifications")
                    .and_then(|v| v.as_bool())
                    .or_else(|| notify_when(d))
            })
        })
        .unwrap_or(true);
    let notify_ms = patch
        .and_then(|p| p.get("show_notifications_time"))
        .and_then(|v| v.as_u64())
        .or_else(|| {
            sources
                .iter()
                .rev()
                .find_map(|d| d.get("show_notifications_time").and_then(|v| v.as_u64()))
        })
        .unwrap_or(1200) as u32;

    Ok(Some(StyleConfig {
        theme,
        layout,
        notify: NotifyConfig {
            mode: if notify_on { "always".into() } else { "never".into() },
            duration_ms: notify_ms,
        },
        page_size: effective_page_size(dir),
    }))
}

#[tauri::command]
pub fn open_config_dir(config_dir: String) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&config_dir)
            .spawn()
            .map_err(|e| AppError::with(code::LAUNCH_FAILED, e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&config_dir)
            .spawn()
            .map_err(|e| AppError::with(code::LAUNCH_FAILED, e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LayoutConfig, NotifyConfig, StyleConfig, ThemeConfig};

    fn sample() -> StyleConfig {
        StyleConfig {
            theme: ThemeConfig {
                name: "xgrime_custom".into(),
                back_color: "#ffffff".into(),
                border_color: "#f2f2f2".into(),
                text_color: "#000000".into(),
                hilited_text_color: "#000000".into(),
                hilited_back_color: "#e8e8e8".into(),
                candidate_text_color: "#000000".into(),
                comment_text_color: "#888888".into(),
                label_color: "#666666".into(),
                hilited_candidate_text_color: "#000000".into(),
                hilited_candidate_back_color: "#ffffff".into(),
                hilited_candidate_label_color: "#000000".into(),
                hilited_comment_text_color: "#555555".into(),
                hilited_mark_color: "#1884e2".into(),
            },
            layout: LayoutConfig::default(),
            notify: NotifyConfig { mode: "always".into(), duration_ms: 1200 },
            page_size: 5,
        }
    }

    /// 颜色必须写成十六进制串。写十进制数字的话 `_RimeGetColor` 会退回
    /// `config_get_int`，而带 alpha 的颜色一律大于 INT_MAX，librime 解析失败
    /// 就悄悄用默认色 —— 主题页整页白调，选中标记默认值是 0 更是直接不画。
    #[test]
    fn colors_are_written_as_hex_literals_not_decimals() {
        let patch = build_yaml_patch(&sample());
        let scheme = patch
            .get("preset_color_schemes/xgrime_custom")
            .and_then(|v| v.as_mapping())
            .expect("配色方案不在 patch 里");

        for (k, v) in scheme {
            let key = k.as_str().unwrap_or_default();
            if key == "name" {
                continue;
            }
            let text = v.as_str().unwrap_or_else(|| panic!("{key} 不是字符串，八成又写成数字了"));
            assert!(
                text.starts_with("0x") && text.len() == 10,
                "{key} = {text}，应该是 0xAABBGGRR 十位写法"
            );
            u32::from_str_radix(&text[2..], 16).unwrap_or_else(|_| panic!("{key} 不是合法十六进制"));
        }
        assert_eq!(
            scheme.get(serde_yaml::Value::String("hilited_mark_color".into())).and_then(|v| v.as_str()),
            Some("0xFFE28418"),
            "标记色要带上不透明的 alpha，否则小狼毫不画那条竖杠"
        );
    }

    /// 系统开着深色模式时，小狼毫只认 `style/color_scheme_dark`。
    /// 少写这一个键，整个主题页在深色系统上等于没作用 —— 用户调半天颜色，
    /// 生效的还是词库自带的深色配色，连选中标记都跟着不见。
    #[test]
    fn both_light_and_dark_scheme_keys_are_written() {
        let patch = build_yaml_patch(&sample());
        for key in ["style/color_scheme", "style/color_scheme_dark"] {
            assert_eq!(
                patch.get(key).and_then(|v| v.as_str()),
                Some("xgrime_custom"),
                "{key} 没指到我们自己的配色方案"
            );
        }
    }
}
