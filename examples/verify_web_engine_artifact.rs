//! 校验引擎发行包的 sha256 —— 对**官方 release 的摘要**。
//!
//! 用法：
//!
//! ```text
//! cargo run --example verify_web_engine_artifact -- <发行包路径> [官方资产名]
//! cargo run --example verify_web_engine_artifact -- --manifest <清单.json> <发行包路径> [--asset <资产名>]
//! cargo run --example verify_web_engine_artifact -- --emit-manifest <清单.json>
//! ```
//!
//! ⚠️ **两种摘要来源**：
//! - **锚**＝代码里的官方表（内置、离线可验、随仓走）；
//! - **清单**＝可更新的数据（官方 release API 逐资产带 `digest` ⇒ ⇒ 换版本
//!   **不必改代码** ⇒ ⇒ 运维自己能升级）。
//!
//! ⚠️ 两者同时在场时：**同一版**必须一致（不一致 ⇒ ⇒ 响亮报错 ⇒ ⇒ 有一方
//! 被改过）；**不同版**⇒ ⇒ 用清单（清单说的就是它自己那一版）。
//!
//! ⚠️ 官方只给**发行包**的摘要 ⇒ ⇒ 解压出来的 `obscura.exe` 没有可比对的值。

use std::path::{Path, PathBuf};

use synapse::tools::web::engine_binary::{self, DigestManifest};

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  verify_web_engine_artifact <asset-file> [official-asset-name]");
    eprintln!("  verify_web_engine_artifact --manifest <manifest.json> <asset-file> [--asset <name>]");
    eprintln!("  verify_web_engine_artifact --emit-manifest <manifest.json> [--manifest <manifest.json>]");
    eprintln!("known assets: {}", engine_binary::OFFICIAL_DIGESTS.len());
    eprintln!("digests come from: {}", engine_binary::RELEASE_ASSETS_URL);
    std::process::exit(2);
}

fn next_arg<'a>(args: &'a [String], i: usize, flag: &str) -> &'a str {
    match args.get(i) {
        Some(v) => v.as_str(),
        None => {
            eprintln!("{flag} needs a value");
            usage()
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut manifest_path: Option<PathBuf> = None;
    let mut asset_override: Option<String> = None;
    let mut emit_manifest: Option<PathBuf> = None;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest" => {
                i += 1;
                manifest_path = Some(PathBuf::from(next_arg(&args, i, "--manifest")));
            }
            "--asset" => {
                i += 1;
                asset_override = Some(next_arg(&args, i, "--asset").to_string());
            }
            "--emit-manifest" => {
                i += 1;
                emit_manifest = Some(PathBuf::from(next_arg(&args, i, "--emit-manifest")));
            }
            other if other.starts_with("--") => {
                eprintln!("unknown flag: {other}");
                usage();
            }
            other => positional.push(other.to_string()),
        }
        i += 1;
    }

    // ---- 清单：读不到就**响亮地失败**（不许当它不存在） ----
    let manifest = match manifest_path.as_ref() {
        Some(p) => match DigestManifest::load(p) {
            Ok(m) => {
                println!(
                    "manifest: {} (version {}, {} assets, source {})",
                    p.display(),
                    m.version,
                    m.assets.len(),
                    if m.source_url.is_empty() {
                        "unspecified"
                    } else {
                        m.source_url.as_str()
                    }
                );
                Some(m)
            }
            Err(e) => {
                eprintln!("FAIL cannot use manifest {}: {e}", p.display());
                std::process::exit(1);
            }
        },
        None => None,
    };

    // ---- 导出清单（锚 ⇒ ⇒ 数据） ----
    if let Some(out) = emit_manifest.as_ref() {
        let to_write = manifest.clone().unwrap_or_else(DigestManifest::from_builtin);
        let json = to_write.to_json();
        if let Err(e) = std::fs::write(out, json.as_bytes()) {
            eprintln!("FAIL cannot write manifest {}: {e}", out.display());
            std::process::exit(1);
        }
        println!(
            "OK wrote manifest {} (version {}, {} assets)",
            out.display(),
            to_write.version,
            to_write.assets.len()
        );
        std::process::exit(0);
    }

    let Some(path_arg) = positional.first() else {
        usage();
    };
    let path = Path::new(path_arg);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path_arg);
    let asset = match asset_override {
        Some(explicit) => explicit,
        None => match positional.get(1) {
            Some(explicit) => explicit.clone(),
            None => match engine_binary::official_digest(file_name) {
                Some(_) => file_name.to_string(),
                None => engine_binary::guess_asset(file_name)
                    .map(str::to_string)
                    .unwrap_or_else(|| file_name.to_string()),
            },
        },
    };

    match engine_binary::verify_artifact_with(path, &asset, manifest.as_ref()) {
        Ok(verified) => {
            println!(
                "OK {asset} sha256={} (source: {})",
                verified.digest,
                verified.source.label()
            );
            if let Some(m) = manifest.as_ref() {
                if m.version.trim() != engine_binary::ENGINE_VERSION {
                    println!(
                        "NOTE the pinned anchor is for version {}, this manifest is for version {} ⇒ ⇒ the manifest ruled",
                        engine_binary::ENGINE_VERSION,
                        m.version
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("FAIL {file_name} (as {asset}): {e}");
            eprintln!("known assets:");
            for (name, _) in engine_binary::OFFICIAL_DIGESTS {
                eprintln!("  {name}");
            }
            std::process::exit(1);
        }
    }
}
