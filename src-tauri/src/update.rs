use crate::error::{code, AppError, AppResult};
use serde::{Deserialize, Serialize};

/// 小狼毫 / 鼠须管本身的版本情况
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RimeUpdate {
    /// 装着的版本（注册表 / Info.plist 读出来的）
    pub installed: Option<String>,
    /// 官方最新稳定版
    pub latest: Option<String>,
    pub update_available: bool,
    /// 发布页，让用户自己看改了啥
    pub release_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    prerelease: bool,
}

#[derive(Deserialize)]
struct GithubCommit {
    sha: String,
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("XGRime/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::with(code::HTTP_CLIENT_FAILED, e))
}

/// 版本号按段比大小
///
/// 不能直接比字符串：小狼毫注册表里写的是 `0.17.4.0`，
/// release 的 tag 是 `0.17.4`，字符串比会得出「装着的更新」这种荒唐结论。
/// 段数不齐的用 0 补齐再比。
fn is_newer(latest: &str, installed: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-'])
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parse(latest), parse(installed));
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

fn rime_repo() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "rime/squirrel"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "rime/weasel"
    }
}

#[tauri::command]
pub async fn check_rime_update(installed: Option<String>) -> AppResult<RimeUpdate> {
    let repo = rime_repo();
    let fallback = format!("https://github.com/{}/releases", repo);

    // 只认稳定版。官方还挂着一个滚动的 latest tag，那是 nightly，
    // 引导普通用户去装 nightly 是给自己找麻烦。
    let releases: Vec<GithubRelease> = client()?
        .get(format!(
            "https://api.github.com/repos/{}/releases?per_page=20",
            repo
        ))
        .send()
        .await
        .map_err(|e| AppError::with(code::UPDATE_CHECK_FAILED, e))?
        .json()
        .await
        .map_err(|e| AppError::with(code::GITHUB_PARSE_FAILED, e))?;

    let stable = releases
        .iter()
        .find(|r| !r.prerelease && r.tag_name.to_lowercase() != "latest");

    let latest = stable.map(|r| r.tag_name.trim_start_matches(['v', 'V']).to_string());
    let release_url = stable.map(|r| r.html_url.clone()).unwrap_or(fallback);

    let update_available = match (&latest, &installed) {
        (Some(l), Some(i)) => is_newer(l, i),
        // 版本号读不出来时不瞎报「有更新」，免得天天骚扰用户
        _ => false,
    };

    Ok(RimeUpdate {
        installed,
        latest,
        update_available,
        release_url,
    })
}

/// 取某个仓库分支当前的提交号，用来判断词库有没有更新
pub async fn head_sha(repo: &str, branch: &str) -> Option<String> {
    let commit: GithubCommit = client()
        .ok()?
        .get(format!(
            "https://api.github.com/repos/{}/commits/{}",
            repo, branch
        ))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    Some(commit.sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_handles_uneven_segments() {
        // 注册表写四段，tag 只有三段 —— 直接比字符串会翻车
        assert!(!is_newer("0.17.4", "0.17.4.0"));
        assert!(!is_newer("0.17.4", "0.17.4"));
        assert!(is_newer("0.17.5", "0.17.4.0"));
        assert!(is_newer("0.18.0", "0.17.4.0"));
        assert!(!is_newer("0.9.9", "0.17.0"));
        // 字符串比较会认为 "0.9" > "0.17"，这条就是防它
        assert!(!is_newer("0.9", "0.17"));
        assert!(is_newer("v1.1.2", "1.1.1"));
    }
}
