//! **channel v5 握手探针** —— 对着真路由器发第一帧，看它怎么答。
//!
//! 用法：
//!   $env:ZITI_IDENTITY = "C:\path\to\client.identity.json"
//!   cargo run --offline --features ziti --example ziti_v2_hello
//!
//! 为什么需要它 —— 前面已经确认：
//! - 路由器说它是 channel v5（回 magicUnknownVersion + 版本列表 [1,2]）
//! - channel v5 的 readHello **只认 magicV2**（03 06 09 0c）
//! - Hello 的 body 就是 idToken（channel.NewHello 把 idToken 放 body）
//! ⇒ ⇒ 现在要确认的是：**body 里放什么、要不要带头、路由器认不认**。
//!
//! 一次试四种组合，每种都开一条新的 TLS 连接，避免互相污染。

use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

use synapse::transport::ziti::v2::{proto, wire};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

fn head(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn load_certs(pem: &[u8]) -> Vec<CertificateDer<'static>> {
    let mut reader = BufReader::new(pem);
    rustls_pemfile::certs(&mut reader)
        .filter_map(|c| c.ok())
        .collect()
}

fn load_key(pem: &[u8]) -> Option<PrivateKeyDer<'static>> {
    let mut reader = BufReader::new(pem);
    rustls_pemfile::private_key(&mut reader).ok().flatten()
}

struct Variant {
    name: &'static str,
    magic: [u8; 4],
    body: Vec<u8>,
    headers: Vec<(i32, Vec<u8>)>,
}

async fn read_frame(
    stream: &mut tokio_rustls::client::TlsStream<TcpStream>,
) -> Result<wire::Decoded, String> {
    let mut acc: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 4096];
    loop {
        let n = match tokio::time::timeout(Duration::from_secs(6), stream.read(&mut tmp)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("读失败: {e}")),
            Err(_) => return Err("6 秒内没有数据".to_string()),
        };
        if n == 0 {
            return Err("对端关闭了连接".to_string());
        }
        acc.extend_from_slice(&tmp[..n]);
        match wire::decode(&acc) {
            Ok((d, _used)) => return Ok(d),
            Err(wire::WireError::TooShort { .. }) => continue,
            Err(e) => return Err(format!("解帧失败: {e}")),
        }
    }
}

fn print_frame(label: &str, d: &wire::Decoded) {
    println!("  [{label}]");
    match d {
        wire::Decoded::Frame(f) => {
            println!("    content_type = {}", f.content_type);
            println!("    sequence     = {}", f.sequence);
            for (k, v) in &f.headers {
                let printable: String = v
                    .iter()
                    .map(|b| {
                        if (0x20..0x7f).contains(b) {
                            *b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();
                println!(
                    "    header {k} = [{}] len={} ascii={}",
                    v.iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" "),
                    v.len(),
                    printable
                );
            }
            let body_text = String::from_utf8_lossy(&f.body).into_owned();
            println!("    body ({} 字节) = {}", f.body.len(), head(&body_text, 300));
        }
        wire::Decoded::UnknownVersion { versions } => {
            println!("    ⇒ 路由器说：不认识这个 magic，它支持版本 {versions:?}");
        }
        wire::Decoded::UnsupportedMagic(m) => {
            println!("    ⇒ 完全不认识的 magic: {:02x?}", m);
        }
    }
}

#[tokio::main]
async fn main() {
    let identity_path =
        std::env::var("ZITI_IDENTITY").expect("需要 ZITI_IDENTITY（指向 identity.json）");
    let router = std::env::var("ZITI_ROUTER").unwrap_or_else(|_| "192.168.31.170:3022".into());

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

    // ① 向控制器换一个 api-session token（当 hello 的 idToken 用）
    let certs = reqwest::Certificate::from_pem_bundle(&ca_pem).expect("解析 CA");
    let mut bundle = cert_pem.clone();
    if !bundle.ends_with(b"\n") {
        bundle.push(b'\n');
    }
    bundle.extend_from_slice(&key_pem);
    let ident = reqwest::Identity::from_pem(&bundle).expect("解析身份");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .identity(ident)
        .tls_certs_only(certs)
        .no_proxy()
        .build()
        .expect("建客户端");
    let resp = client
        .post(format!("{zt_api}/authenticate?method=cert"))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .expect("认证");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    println!("== channel v5 握手探针 ==");
    println!("  router = {router}");
    println!("  控制器认证 -> {status}");
    if !status.is_success() {
        println!("  认证失败：{}", head(&text, 300));
        return;
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let token = v["data"]["token"].as_str().unwrap_or("").to_string();
    println!("  api-session token 前 60 字符 = {}", head(&token, 60));
    println!(
        "  看起来像 JWT 吗 = {}",
        token.starts_with("eyJ") || token.split('.').count() == 3
    );

    // ② 准备 TLS
    let mut roots = RootCertStore::empty();
    for c in load_certs(&ca_pem) {
        if let Err(e) = roots.add(c) {
            println!("  [WARN] 根证书加不进去: {e}");
        }
    }
    let chain = load_certs(&cert_pem);
    let key = load_key(&key_pem).expect("私钥解析失败");
    let mut cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(chain, key)
        .expect("TLS 配置");
    cfg.alpn_protocols = Vec::new();

    let variants = vec![
        Variant {
            name: "A: magicV2 + body=api-session token",
            magic: wire::MAGIC_V2,
            body: token.as_bytes().to_vec(),
            headers: Vec::new(),
        },
        Variant {
            name: "B: magicV2 + body=空（无会话）",
            magic: wire::MAGIC_V2,
            body: Vec::new(),
            headers: Vec::new(),
        },
        Variant {
            name: "C: magicV2 + token + HelloVersion 头",
            magic: wire::MAGIC_V2,
            body: token.as_bytes().to_vec(),
            headers: vec![(
                proto::channel_header::HELLO_VERSION,
                1u32.to_le_bytes().to_vec(),
            )],
        },
        Variant {
            name: "D: magicV3 + body=api-session token",
            magic: wire::MAGIC_V3,
            body: token.as_bytes().to_vec(),
            headers: Vec::new(),
        },
    ];

    for variant in variants {
        println!();
        println!("=== {} ===", variant.name);
        let addr: std::net::SocketAddr = match router.parse() {
            Ok(a) => a,
            Err(_) => {
                println!("  地址解析失败: {router}");
                continue;
            }
        };
        let tcp = match TcpStream::connect(addr).await {
            Ok(t) => t,
            Err(e) => {
                println!("  TCP 连接失败: {e}");
                continue;
            }
        };
        let server_name = match ServerName::try_from(addr.ip().to_string()) {
            Ok(s) => s,
            Err(e) => {
                println!("  ServerName: {e}");
                continue;
            }
        };
        let connector = TlsConnector::from(Arc::new(cfg.clone()));
        let mut stream = match connector.connect(server_name, tcp).await {
            Ok(s) => s,
            Err(e) => {
                println!("  TLS 握手失败: {e}");
                continue;
            }
        };
        println!("  TLS 握手成功");

        let mut frame = wire::Frame::new(proto::content_type::HELLO, -1);
        frame.body = variant.body.clone();
        for (k, val) in &variant.headers {
            frame.headers.push((*k, val.clone()));
        }
        let bytes = frame.encode(&variant.magic);
        println!("  发出 {} 字节：magic {:02x?}", bytes.len(), variant.magic);
        if let Err(e) = stream.write_all(&bytes).await {
            println!("  写失败: {e}");
            continue;
        }
        let _ = stream.flush().await;
        match read_frame(&mut stream).await {
            Ok(d) => print_frame("路由器的回应", &d),
            Err(e) => println!("  没拿到回应: {e}"),
        }
    }
}
