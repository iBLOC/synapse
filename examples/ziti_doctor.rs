//! 零信任网络的「体检工具」—— 把每一步的**真实结果**打出来。
//!
//! 用法：
//!   $env:ZITI_IDENTITY = "C:\path\to\client.identity.json"
//!   cargo run --offline --features ziti --example ziti_doctor
//!
//! 它回答一个问题：**「我的零信任网络到底配好了没有，卡在哪一步？」**
//! => => 每一步都打印 HTTP 状态码 / 完整错误链 => => 不靠猜。
//!
//! ⚠️ **为什么要有它** —— 旧的 ziti-sdk 把错误包成了
//! "Failed to send authentication request: error sending request for url (...)"
//! => => 真正的根因（证书链？代理？名字？）被吞掉了
//! => => 症状长得都一样 => => 不体检就只能猜。
//!
//! ⚠️ **它会对比两种校验器** ——
//! - 平台校验器 = reqwest 默认的 rustls-platform-verifier（Windows 上查**系统证书库**）
//! - 纯 rustls   = tls_certs_only()，只用身份自带的 CA bundle
//! => => 私有 CA 的场景下这两者结论可能不同 => => 体检工具把差异摆出来。

use std::time::Duration;

fn show_err(context: &str, e: &(dyn std::error::Error + 'static)) {
    println!("  [FAIL] {context}");
    let mut src: Option<&(dyn std::error::Error + 'static)> = Some(e);
    let mut depth = 0;
    while let Some(s) = src {
        println!("        {}. {}", depth, s);
        src = s.source();
        depth += 1;
        if depth > 8 {
            break;
        }
    }
}

/// 校验器模式。
#[derive(Clone, Copy, Debug)]
enum Verifier {
    /// reqwest 默认：平台校验器（Windows 上 = 系统证书库）＋ 额外根
    Platform,
    /// tls_certs_only()：纯 rustls/webpki，只认身份自带的 CA bundle
    WebpkiOnly,
}

/// 造一个与 SDK 的 controller_client **同形**的 reqwest 客户端。
fn build_client(
    ca_pem: &[u8],
    cert_pem: &[u8],
    key_pem: &[u8],
    use_proxy: bool,
    verifier: Verifier,
) -> Result<reqwest::Client, reqwest::Error> {
    // ⚠️ 用 reqwest 自带的 PEM bundle 解析 —— **不要**手写 base64 解码。
    let certs = reqwest::Certificate::from_pem_bundle(ca_pem)?;

    let mut bundle = cert_pem.to_vec();
    if !cert_pem.ends_with(b"\n") {
        bundle.push(b'\n');
    }
    bundle.extend_from_slice(key_pem);
    let identity = reqwest::Identity::from_pem(&bundle)?;

    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .identity(identity);
    builder = match verifier {
        Verifier::Platform => builder.tls_certs_merge(certs),
        Verifier::WebpkiOnly => builder.tls_certs_only(certs),
    };
    if !use_proxy {
        builder = builder.no_proxy();
    }
    builder.build()
}

#[tokio::main]
async fn main() {
    let identity_path = std::env::var("ZITI_IDENTITY")
        .expect("需要 ZITI_IDENTITY（指向 identity.json）");

    println!("== 零信任体检 ==");
    println!("  identity = {identity_path}");

    let raw = std::fs::read_to_string(&identity_path).expect("读 identity.json");
    let id: serde_json::Value = serde_json::from_str(&raw).expect("解析 identity.json");
    let zt_api = id["ztAPI"]
        .as_str()
        .expect("identity.json 缺 ztAPI")
        .trim_end_matches('/')
        .to_string();
    let cert_path = id["cert"].as_str().expect("identity.json 缺 cert");
    let key_path = id["key"].as_str().expect("identity.json 缺 key");
    let ca_path = id["ca"].as_str().expect("identity.json 缺 ca");
    println!("  ztAPI    = {zt_api}");

    let ca_pem = std::fs::read(ca_path).expect("读 ca.pem");
    let cert_pem = std::fs::read(cert_path).expect("读 cert.pem");
    let key_pem = std::fs::read(key_path).expect("读 key.pem");
    let ca_count = String::from_utf8_lossy(&ca_pem)
        .matches("-----BEGIN CERTIFICATE-----")
        .count();
    println!(
        "  ca={} bytes（{ca_count} 张）/ cert={} bytes / key={} bytes",
        ca_pem.len(),
        cert_pem.len(),
        key_pem.len()
    );

    let mut done = false;
    'outer: for (vlabel, verifier) in [
        ("平台校验器（系统证书库 + 额外根）", Verifier::Platform),
        ("纯 rustls（只用身份自带的 CA）", Verifier::WebpkiOnly),
    ] {
        for (plabel, use_proxy) in [("直连", false), ("走系统代理", true)] {
            println!();
            println!("-- 控制器 mTLS 认证：{vlabel} · {plabel} --");
            let client = match build_client(&ca_pem, &cert_pem, &key_pem, use_proxy, verifier) {
                Ok(c) => c,
                Err(e) => {
                    show_err("建客户端", &e);
                    continue;
                }
            };
            let url = format!("{zt_api}/authenticate?method=cert");
            let resp = match client
                .post(&url)
                .header("Content-Type", "application/json")
                .body("{}")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    show_err("控制器认证", &e);
                    continue;
                }
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            println!("  [OK] {url} -> {status}");
            println!("       {}", &text.chars().take(400).collect::<String>());
            if !status.is_success() {
                continue;
            }

            let v: serde_json::Value =
                serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
            let token = v["data"]["token"].as_str().unwrap_or("").to_string();
            let session = v["data"]["id"].as_str().unwrap_or("").to_string();
            println!("  api-session id = {session}");

            println!();
            println!("-- 这张身份能看到的 edge router --");
            match client
                .get(format!("{zt_api}/current-identity/edge-routers"))
                .header("zt-session", &token)
                .send()
                .await
            {
                Ok(r) => {
                    let status = r.status();
                    let t = r.text().await.unwrap_or_default();
                    println!("  status = {status}");
                    println!("  {}", &t.chars().take(900).collect::<String>());
                }
                Err(e) => show_err("edge-routers", &e),
            }

            println!();
            println!("-- 这张身份能看到的服务 --");
            match client
                .get(format!("{zt_api}/services"))
                .header("zt-session", &token)
                .send()
                .await
            {
                Ok(r) => {
                    let status = r.status();
                    let t = r.text().await.unwrap_or_default();
                    println!("  status = {status}");
                    println!("  {}", &t.chars().take(900).collect::<String>());
                }
                Err(e) => show_err("services", &e),
            }
            done = true;
            break 'outer;
        }
    }

    if !done {
        println!();
        println!("!! 所有组合都失败了 —— 上面的错误链就是根因。");
    }
}
