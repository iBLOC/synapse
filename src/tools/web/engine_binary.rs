//! 引擎二进制的**分发链校验** —— 我们跑的是**别人家的**可执行文件。
//!
//! ⚠️ **为什么要有这一层**：引擎是**外进程**（本仓不 vendor 它的 crates）
//! ⇒ ⇒ 装到机器上的那个发行包是**下载来的** ⇒ ⇒
//! 「跑的是不是官方那一份」是**分发链**的问题，不是代码的问题。
//!
//! 官方 release 页面（GitHub 的 `expanded_assets`）**逐资产带 sha256 摘要**
//! ⇒ ⇒ 这里把 v0.2.7 的 **20 个资产**摘要**钉死**，并给出可复核的校验函数。
//!
//! ⚠️ **摘要只覆盖「发行包」**（zip／tar.gz）⇒ ⇒ 解压出来的 `obscura.exe`
//! **官方不给摘要** ⇒ ⇒ 能核对的只有发行包那一格 —— 说清这一点，别含糊。
//!
//! 实测（2026-11-16，Windows x86_64）：官方 `obscura-x86_64-windows.zip` 的摘要
//! `be983b663552e11faf8c1e07696578bad622dbeb48afb9f59036032ca8e97fb3`
//! ＝ 本机下载的那一份的 sha256 ⇒ ⇒ **跑的就是官方那一份**。

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// 引擎名。
pub const ENGINE_NAME: &str = "obscura";

/// 引擎版本（摘要表对应的是这一版）。
pub const ENGINE_VERSION: &str = "0.2.7";

/// 官方 release 的资产页（**逐资产带 sha256 摘要**）。
pub const RELEASE_ASSETS_URL: &str =
    "https://github.com/h4ckf0r0day/obscura/releases/expanded_assets/v0.2.7";

/// 官方发行包 → sha256（**逐字抄自官方 release 资产页**）。
///
/// ⚠️ 换版本时**整表换**，别只改一行 —— 半新半旧的摘要表 = 没有摘要表。
pub const OFFICIAL_DIGESTS: &[(&str, &str)] = &[
    ("obscura-aarch64-linux-no-render-stealth.tar.gz", "9ab81a83db31be4da8f6690ee1b2e564092ca5ce28033f00eea2d6e0074902e9"),
    ("obscura-aarch64-linux-no-render.tar.gz", "f3cc7010c1f9ffb0d374a12cb0cbfb4744bf05232352a76a3845d98019160601"),
    ("obscura-aarch64-linux-stealth.tar.gz", "f51167d7ae98be6c6327d3e7e015e74430e190ea6846589e3c131e10641a52b7"),
    ("obscura-aarch64-linux.tar.gz", "4ee0aeeb38fa8bb2cb004ba104abe1c79f1bdfd02f51672c694e8234a5ae0e5c"),
    ("obscura-aarch64-macos-no-render-stealth.tar.gz", "0983cdb5ff0c90d8a613f2541cb0991571e00076dfa00c93a7f32fbf63cafaed"),
    ("obscura-aarch64-macos-no-render.tar.gz", "da393a29dcb94c55a1fe834efafc9dbe8fe87059a3861cc60eee0be2666ad061"),
    ("obscura-aarch64-macos-stealth.tar.gz", "3b05ebcd4755a325be224992fad6a038d86f7e5ab483e4c781f14de06cab973f"),
    ("obscura-aarch64-macos.tar.gz", "fc7e73f23058f18d6d5be983d83b24ba374d12167677657a307a9476c0f5a7c1"),
    ("obscura-x86_64-linux-no-render-stealth.tar.gz", "d6137f391e0bb5332dc06489eb6df37da5efa25870a6bcc90a89ecf3abfa85cb"),
    ("obscura-x86_64-linux-no-render.tar.gz", "d64c8521b95bb7cd0efb479b29c88bb5cd297949f6ccf2b7cca82f40e8d26948"),
    ("obscura-x86_64-linux-stealth.tar.gz", "1d6514beacef9fe75671f4bf7a66d3a7bceef9bde39f8156d1ca4a701eb1e216"),
    ("obscura-x86_64-linux.tar.gz", "11b53d27cd743e00e196e216920e7f3f794cf378e3f1792b8d25d3beabaf9cad"),
    ("obscura-x86_64-macos-no-render-stealth.tar.gz", "65df721d4116f523c837f2318198336ae44a5fbb8ca818567a62221fe4fa7eb2"),
    ("obscura-x86_64-macos-no-render.tar.gz", "0867f07059db715fc096675276a29ab755bd82d1fdf51c8fd7957fbe19c1a81f"),
    ("obscura-x86_64-macos-stealth.tar.gz", "62bd094388a1f97480fd06be349299e721a3488c216c86a2fce9367310594ec3"),
    ("obscura-x86_64-macos.tar.gz", "c47383e5b5551e09d2055ee7bdcce788af409fb4394e39bdaf179734df18e829"),
    ("obscura-x86_64-windows-no-render-stealth.zip", "5368700eb78f07124c6ddfdc052f257e1ce92cfbc3f3a6c4330ee3558923c88a"),
    ("obscura-x86_64-windows-no-render.zip", "a2ac7d8b08e93e3ed31f6f14cc7057d822725d3f55d91fc7992a6f2d3e0ad1b6"),
    ("obscura-x86_64-windows-stealth.zip", "ffae5e71cfca0108709b57ad83ce7d6b588eff6a15a43093b0cf867dbd18c867"),
    ("obscura-x86_64-windows.zip", "be983b663552e11faf8c1e07696578bad622dbeb48afb9f59036032ca8e97fb3"),
];

/// 校验这一格出的错。
#[derive(Debug, Error)]
pub enum DigestError {
    #[error("cannot read '{path}': {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no official digest for asset '{0}' ({1} assets are pinned)")]
    UnknownAsset(String, usize),
    #[error("digest mismatch for '{path}': expected {expected}, got {actual}")]
    Mismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error("cannot read manifest '{path}': {source}")]
    ManifestIo {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("manifest '{path}' is not a usable manifest: {message}")]
    ManifestParse { path: String, message: String },
    #[error("manifest entry '{asset}' has a malformed sha256: '{value}'")]
    ManifestBadDigest { asset: String, value: String },
    #[error("digest conflict for '{asset}': the pinned anchor says {anchor}, the manifest says {manifest}")]
    ManifestConflict {
        asset: String,
        anchor: String,
        manifest: String,
    },
}

/// 这个资产的官方摘要（按**资产名**查 —— 文件名就是资产名）。
pub fn official_digest(asset: &str) -> Option<&'static str> {
    OFFICIAL_DIGESTS
        .iter()
        .find(|(name, _)| *name == asset)
        .map(|(_, digest)| *digest)
}

/// 从**文件名**猜资产名（官方资产名是「架构 ＋ 平台 ＋ 变体」的确定形状）。
///
/// ⚠️ 为什么要有它：下载下来的文件**常被改名**（实测本机那份叫
/// `obscura-windows.zip`，官方资产名却是 `obscura-x86_64-windows.zip`）
/// ⇒ ⇒ 按文件名直接查表会「查不到」⇒ ⇒ 猜出来才核得上。
///
/// ⚠️ 猜出来的名字**必须**能在摘要表里查到 ⇒ ⇒ 对不上就返回 `None`
/// ⇒ ⇒ 宁可说「不知道」，也不硬套一个摘要去比（比错资产 = 比了个寂寞）。
pub fn guess_asset(file_name: &str) -> Option<&'static str> {
    let lower = file_name.to_ascii_lowercase();
    let stealth = lower.contains("stealth");
    let no_render = lower.contains("no-render") || lower.contains("norender");
    let (platform, ext) = if lower.contains("windows") {
        ("windows", "zip")
    } else if lower.contains("macos") || lower.contains("darwin") {
        ("macos", "tar.gz")
    } else if lower.contains("linux") {
        ("linux", "tar.gz")
    } else {
        return None;
    };
    let arch = if lower.contains("aarch64") || lower.contains("arm64") {
        "aarch64"
    } else {
        "x86_64"
    };
    let mut name = format!("obscura-{arch}-{platform}");
    if no_render {
        name.push_str("-no-render");
    }
    if stealth {
        name.push_str("-stealth");
    }
    name.push('.');
    name.push_str(ext);
    OFFICIAL_DIGESTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, _)| *n)
}

/// 算一个文件的 sha256（**流式** ⇒ ⇒ 72MB 的发行包不会整个进内存）。
pub fn sha256_file(path: &Path) -> Result<String, DigestError> {
    let display = path.display().to_string();
    let file =
        File::open(path).map_err(|source| DigestError::Io { path: display.clone(), source })?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|source| DigestError::Io { path: display.clone(), source })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 校验一个发行包（**对不上就报错** ⇒ ⇒ 别含糊）。
///
/// 成功返回**算出来的**摘要（让人能自己再核一遍）。
pub fn verify_artifact(path: &Path, asset: &str) -> Result<String, DigestError> {
    let expected = official_digest(asset).ok_or_else(|| {
        DigestError::UnknownAsset(asset.to_string(), OFFICIAL_DIGESTS.len())
    })?;
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(DigestError::Mismatch {
            path: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(actual)
}

// ── 清单机制：「换版本要改代码」的那个洞的出口 ──────────────────────────

/// 一份**发行包摘要清单** —— 摘要的**数据形态**（不在代码里）。
///
/// ⚠️ **为什么需要它**：内置的 [`OFFICIAL_DIGESTS`] 是**代码** ⇒ ⇒ 换版本
/// 就得**改代码** ⇒ ⇒ 运维想升级引擎时**得等一个开发者**。⇒ ⇒ 这份清单
/// 让摘要变成**可更新的数据**：
/// - 清单从**官方 release API**取回（逐资产带 `digest`；取回的活儿在父仓
///   那个脚本里，**校验**的活儿在这儿 ⇒ ⇒ 摘要逻辑只有这一份）；
/// - 校验时**锚与清单同时在场** ⇒ ⇒ 同版本必须一致（不一致 ⇒ ⇒ 有一方
///   被改过 ⇒ ⇒ 响亮报错，不静默取其一）；
/// - 锚**不认识**这一版时 ⇒ ⇒ 清单接管（这正是「升级」那一格）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestManifest {
    /// 这份清单说的是**哪一版**（`"0.2.7"`／`"0.2.6"` …）。
    pub version: String,
    /// 这份清单的**出处**（官方 API 的 URL／release 页）。
    #[serde(default)]
    pub source_url: String,
    /// 取回时刻（ISO-8601）—— ⇒ 「这份数据是什么时候的」可查。
    #[serde(default)]
    pub retrieved_at: String,
    /// 逐资产的摘要。
    pub assets: Vec<AssetDigest>,
}

/// 清单里的一条：资产名 → sha256。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDigest {
    /// 官方资产名（**文件名就是资产名**）。
    pub name: String,
    /// sha256（**小写十六进制**）。
    pub sha256: String,
}

/// 摘要的**出处** —— **出声**用（运维要知道这份摘要是谁给的）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestSource {
    /// 来自**内置锚**（代码里的官方表 ⇒ ⇒ 离线可验）。
    Anchor,
    /// 来自**清单**（升级路径：锚不认识这一版）。
    Manifest,
}

impl DigestSource {
    /// 人话。
    pub fn label(self) -> &'static str {
        match self {
            DigestSource::Anchor => "内置锚（代码里的官方表）",
            DigestSource::Manifest => "清单（升级路径的数据）",
        }
    }
}

/// 校验的结果：摘要 **＋ 它从哪儿来**。
#[derive(Debug, Clone)]
pub struct Verified {
    /// 算出来的摘要（**等于**期望值）。
    pub digest: String,
    /// 这份期望值是从哪儿来的。
    pub source: DigestSource,
}

impl DigestManifest {
    /// 从**内置锚**导出一份清单（⇒ ⇒ 锚也能当数据用）。
    pub fn from_builtin() -> Self {
        Self {
            version: ENGINE_VERSION.to_string(),
            source_url: RELEASE_ASSETS_URL.to_string(),
            retrieved_at: String::new(),
            assets: OFFICIAL_DIGESTS
                .iter()
                .map(|(name, sha256)| AssetDigest {
                    name: (*name).to_string(),
                    sha256: (*sha256).to_string(),
                })
                .collect(),
        }
    }

    /// 读一份清单文件 —— **坏文件就报错**（不许静默忽略）。
    ///
    /// ⚠️ 清单里的 sha256 必须是**合法的小写 64 位十六进制** ⇒ ⇒ 手滑写错
    /// 会在**读的时候**就响亮地失败，而不是等到比对了才「不匹配」。
    pub fn load(path: &Path) -> Result<Self, DigestError> {
        let display = path.display().to_string();
        let raw =
            std::fs::read_to_string(path).map_err(|source| DigestError::ManifestIo {
                path: display.clone(),
                source,
            })?;
        let mut manifest: DigestManifest =
            serde_json::from_str(&raw).map_err(|e| DigestError::ManifestParse {
                path: display.clone(),
                message: e.to_string(),
            })?;
        if manifest.version.trim().is_empty() {
            return Err(DigestError::ManifestParse {
                path: display.clone(),
                message: "the version field is empty".into(),
            });
        }
        for entry in &mut manifest.assets {
            entry.sha256 = entry.sha256.trim().to_ascii_lowercase();
            if !is_sha256_hex(&entry.sha256) {
                return Err(DigestError::ManifestBadDigest {
                    asset: entry.name.clone(),
                    value: entry.sha256.clone(),
                });
            }
        }
        Ok(manifest)
    }

    /// 清单里某个资产的摘要。
    pub fn digest_of(&self, asset: &str) -> Option<&str> {
        self.assets
            .iter()
            .find(|a| a.name == asset)
            .map(|a| a.sha256.as_str())
    }

    /// 清单的 JSON（**给人看／给人存**）。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// 是不是一个**合法**的 sha256（64 位十六进制）。
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// 挑出「这个资产该是什么摘要」—— **纯函数**（不碰文件、不碰网络）。
///
/// ⚠️ **口径（三条，别含糊）**：
/// - 锚有、清单没有 ⇒ ⇒ 用**锚**；
/// - 锚没有、清单有 ⇒ ⇒ 用**清单**（**升级**那一格）；
/// - 两边都有 ⇒ ⇒ **看版本**：同一版 ⇒ 必须一致（不一致 ⇒ ⇒ 响亮报错，
///   因为有一方被改过）；不同版 ⇒ ⇒ 用**清单**（清单说的就是它自己那一版，
///   锚说的是另一版 ⇒ ⇒ 混用会拿错摘要）。
pub fn resolve_expected_digest(
    asset: &str,
    manifest: Option<&DigestManifest>,
) -> Result<(String, DigestSource), DigestError> {
    let anchor = official_digest(asset).map(str::to_string);
    let listed = manifest
        .and_then(|m| m.digest_of(asset))
        .map(str::to_string);
    match (anchor, listed) {
        (None, None) => Err(DigestError::UnknownAsset(
            asset.to_string(),
            OFFICIAL_DIGESTS.len(),
        )),
        (Some(a), None) => Ok((a, DigestSource::Anchor)),
        (None, Some(m)) => Ok((m, DigestSource::Manifest)),
        (Some(a), Some(m)) => {
            let same_release = manifest
                .map(|m| m.version.trim() == ENGINE_VERSION)
                .unwrap_or(false);
            if same_release {
                if a == m {
                    Ok((a, DigestSource::Anchor))
                } else {
                    Err(DigestError::ManifestConflict {
                        asset: asset.to_string(),
                        anchor: a,
                        manifest: m,
                    })
                }
            } else {
                Ok((m, DigestSource::Manifest))
            }
        }
    }
}

/// 校验一个发行包（**带清单**）—— 返回摘要 **和它的出处**。
pub fn verify_artifact_with(
    path: &Path,
    asset: &str,
    manifest: Option<&DigestManifest>,
) -> Result<Verified, DigestError> {
    let (expected, source) = resolve_expected_digest(asset, manifest)?;
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(DigestError::Mismatch {
            path: path.display().to_string(),
            expected,
            actual,
        });
    }
    Ok(Verified {
        digest: actual,
        source,
    })
}
