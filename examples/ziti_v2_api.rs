//! **v2 控制器 API 面探测** —— 把「客户端身份到底能问控制器要什么」一次性打出来。
//!
//! 用法：
//!   $env:ZITI_IDENTITY = "C:\path\to\client.identity.json"
//!   cargo run --offline --features ziti --example ziti_v2_api
//!
//! 为什么需要它：旧的 ziti-sdk 0.3 用的全是 v1 的 API 形状：
//!   ① 拨号取 terminator 走 GET {ztAPI}/services/{id}/terminators ⇒ v2 实测 404
//!   ② 绑定取 router 走 GET {ztAPI}/current-identity/edge-routers ⇒ v2 回 200
//!      但 hostname 为空且没有 port ⇒ SDK 的 EdgeRouter 结构体反序列化不了
//! 不知道 v2 正确的形状就只能继续猜 ⇒ 这里把候选端点全打一遍。

use std::time::Duration;

fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn build_client(
    ca_pem: &[u8],
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<reqwest::Client, reqwest::Error> {
    let certs = reqwest::Certificate::from_pem_bundle(ca_pem)?;
    let mut bundle = cert_pem.to_vec();
    if !cert_pem.ends_with(b"\n") {
        bundle.push(b'\n');
    }
    bundle.extend_from_slice(key_pem);
    let identity = reqwest::Identity::from_pem(&bundle)?;
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .identity(identity)
        .tls_certs_only(certs)
        .no_proxy()
        .build()
}

#[tokio::main]
async fn main() {
    let identity_path =
        std::env::var("ZITI_IDENTITY").expect("需要 ZITI_IDENTITY（指向 identity.json）");
    let raw = std::fs::read_to_string(&identity_path).expect("读 identity.json");
    let id: serde_json::Value = serde_json::from_str(&raw).expect("解析 identity.json");
    let zt_api = id["ztAPI"]
        .as_str()
        .expect("identity.json 缺 ztAPI")
        .trim_end_matches('/')
        .to_string();
    let ca_pem = std::fs::read(id["ca"].as_str().expect("ca")).expect("读 ca");
    let cert_pem = std::fs::read(id["cert"].as_str().expect("cert")).expect("读 cert");
    let key_pem = std::fs::read(id["key"].as_str().expect("key")).expect("读 key");

    println!("== v2 控制器 API 面 ==");
    println!("  ztAPI = {zt_api}");

    let client = build_client(&ca_pem, &cert_pem, &key_pem).expect("建客户端");

    let resp = client
        .post(format!("{zt_api}/authenticate?method=cert"))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .expect("认证请求");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    println!();
    println!("-- 认证 POST /authenticate?method=cert -> {status} --");
    println!("  {}", head(&text, 300));
    if !status.is_success() {
        return;
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let token = v["data"]["token"].as_str().unwrap_or("").to_string();
    let session_id = v["data"]["id"].as_str().unwrap_or("").to_string();
    println!("  api-session id = {session_id}");
    println!("  token 前 40 字符 = {}", head(&token, 40));

    let resp = client
        .get(format!("{zt_api}/services"))
        .header("zt-session", &token)
        .send()
        .await
        .expect("列服务");
    let sv_status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    println!();
    println!("-- GET /services -> {sv_status} --");
    println!("  {}", head(&text, 900));
    let sv: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let mut service_id = String::new();
    let mut service_name = String::new();
    if let Some(items) = sv["data"].as_array() {
        for it in items {
            let perms = it["permissions"].as_array().map(|a| {
                a.iter()
                    .filter_map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            });
            println!(
                "  服务 {} (id={}) permissions=[{}]",
                it["name"].as_str().unwrap_or("?"),
                it["id"].as_str().unwrap_or("?"),
                perms.unwrap_or_default()
            );
            if service_id.is_empty() {
                service_id = it["id"].as_str().unwrap_or("").to_string();
                service_name = it["name"].as_str().unwrap_or("").to_string();
            }
        }
    }
    if service_id.is_empty() {
        println!("  !! 没有可见的服务");
        return;
    }

    let candidates: Vec<String> = vec![
        format!("{zt_api}/services/{service_id}"),
        format!("{zt_api}/services/{service_id}/terminators"),
        format!("{zt_api}/current-identity"),
        format!("{zt_api}/current-identity/edge-routers"),
        format!("{zt_api}/edge-routers"),
        format!("{zt_api}/sessions"),
        format!("{zt_api}/service-edge-routers"),
    ];
    for url in candidates {
        let resp = match client.get(&url).header("zt-session", &token).send().await {
            Ok(r) => r,
            Err(e) => {
                println!();
                println!("-- GET {url} -> 请求失败: {e}");
                continue;
            }
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        println!();
        println!("-- GET {url} -> {status} --");
        println!("  {}", head(&text, 1200));
    }

    let mgmt = std::env::var("ZITI_MGMT").unwrap_or_else(|_| {
        let base = zt_api
            .rsplit_once("/edge/client/v1")
            .map(|(a, _)| a.to_string())
            .unwrap_or_else(|| zt_api.clone());
        format!("{base}/edge/management/v1")
    });
    let url = format!("{mgmt}/services/{service_id}/terminators");
    println!();
    println!("-- GET {url} --");
    match client.get(&url).header("zt-session", &token).send().await {
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            println!("  status = {status}");
            println!("  {}", head(&text, 1500));
        }
        Err(e) => println!("  请求失败: {e}"),
    }

    println!();
    println!("(服务 {service_name} / id {service_id})");
}
