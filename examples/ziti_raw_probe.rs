//! **裸 TLS 探针（v2）** —— 直接抓 edge router 在 TLS 之后发/收的原始字节。
//!
//! 用法：
//!   $env:ZITI_IDENTITY = "C:\path\to\client.identity.json"
//!   $env:ZITI_ROUTER   = "192.168.31.170:3022"
//!   $env:ZITI_SERVICE  = "shadow-brain"
//!   cargo run --offline --features ziti --example ziti_raw_probe
//!
//! ⚠️ **2026-11-04：v2 迁移后重写** —— 旧版发的是 v1 的 ZitiMessage（无 magic）
//!    ⇒ ⇒ 现在发的是 **channel v4 的 magicV2 帧（＋ 头 1002）**。
//!
//! ⚠️ **为什么需要它** ——
//! 线路探针（ziti_wire_probe）只看「有没有回应」；这个探针把**字节**摊开：
//!   ① 读路由器主动发来的字节（如果有）；
//!   ② 主动发 v2 的 Hello 帧，看路由器认不认；
//!   ③ 再试一次 HTTP/1.1 GET，看它是不是只认 HTTP。
//!
//! ⚠️ **它还回答一个关键问题：ALPN 报不报** —— 逐个 ALPN 候选各开一条连接：
//!   路由器**只认不报 ALPN**（报了 ⇒ ⇒ 路由器回 TLS fatal InternalError）。
//!   ⇒ ⇒ 这就是「三种结果 ⇒ 三条不同路线」里最省事的一次性对照。

use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

use synapse::transport::ziti::v2::{proto, wire, Controller, Identity};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

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

fn dump(label: &str, data: &[u8]) {
    println!("  --- {label}：{} 字节 ---", data.len());
    let n = data.len().min(96);
    let hex: Vec<String> = data[..n].iter().map(|b| format!("{:02x}", b)).collect();
    println!("  hex  : {}", hex.join(" "));
    let ascii: String = data[..n]
        .iter()
        .map(|b| {
            if (0x20..0x7f).contains(b) {
                *b as char
            } else {
                '.'
            }
        })
        .collect();
    println!("  ascii: {ascii}");
}

async fn read_some(
    stream: &mut tokio_rustls::client::TlsStream<TcpStream>,
    label: &str,
    secs: u64,
) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; 2048];
    match tokio::time::timeout(Duration::from_secs(secs), stream.read(&mut buf)).await {
        Ok(Ok(0)) => {
            println!("  --- {label}：对端关闭了连接 ---");
            None
        }
        Ok(Ok(n)) => {
            buf.truncate(n);
            dump(label, &buf);
            Some(buf)
        }
        Ok(Err(e)) => {
            println!("  --- {label}：读失败 {e} ---");
            None
        }
        Err(_) => {
            println!("  --- {label}：{secs}s 内没有数据 ---");
            None
        }
    }
}

async fn probe(
    alpn: Option<&[&str]>,
    label: &str,
    host: &str,
    api_session: &str,
    identity: &Identity,
) {
    println!();
    println!("=== ALPN = {label} ===");

    let mut roots = RootCertStore::empty();
    for c in load_certs(&identity.ca_pem) {
        if let Err(e) = roots.add(c) {
            println!("  [WARN] 根证书加不进去: {e}");
        }
    }
    let chain = load_certs(&identity.cert_pem);
    let key = match load_key(&identity.key_pem) {
        Some(k) => k,
        None => {
            println!("  [FAIL] 私钥解析失败");
            return;
        }
    };
    let mut cfg = match ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(chain, key)
    {
        Ok(c) => c,
        Err(e) => {
            println!("  [FAIL] TLS 配置: {e}");
            return;
        }
    };
    cfg.alpn_protocols = alpn
        .map(|a| a.iter().map(|s| s.as_bytes().to_vec()).collect())
        .unwrap_or_default();

    let addr: std::net::SocketAddr = match host.parse() {
        Ok(a) => a,
        Err(_) => {
            let host_only = host.split(':').next().unwrap_or(host);
            let port: u16 = host
                .split(':')
                .nth(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(3022);
            match format!("{host_only}:{port}").parse() {
                Ok(a) => a,
                Err(e) => {
                    println!("  [FAIL] 地址解析失败: {e}");
                    return;
                }
            }
        }
    };
    let tcp = match TcpStream::connect(addr).await {
        Ok(t) => t,
        Err(e) => {
            println!("  [FAIL] TCP 连接失败: {e}");
            return;
        }
    };
    let server_name = match ServerName::try_from(addr.ip().to_string()) {
        Ok(s) => s,
        Err(e) => {
            println!("  [FAIL] ServerName: {e}");
            return;
        }
    };
    let connector = TlsConnector::from(Arc::new(cfg));
    let mut stream = match connector.connect(server_name, tcp).await {
        Ok(s) => s,
        Err(e) => {
            println!("  [FAIL] TLS 握手失败: {e}");
            return;
        }
    };
    println!("  [OK] TLS 握手成功");

    // ① 路由器主动发来的字节
    read_some(&mut stream, "路由器先发", 3).await;

    // ② 发 v2 的 Hello（magicV2 ＋ 头 1002 ＝ api-session token）
    let mut hello = wire::Frame::new(proto::content_type::HELLO, -1);
    hello.put_bytes(proto::header_id::SESSION_TOKEN, api_session.as_bytes());
    let bytes = hello.encode(&wire::MAGIC_V2);
    println!("  发 Hello 帧（magicV2 ＋ 头 1002，{} 字节）", bytes.len());
    if let Err(e) = stream.write_all(&bytes).await {
        println!("  [FAIL] 发 Hello 失败: {e}");
        return;
    }
    let _ = stream.flush().await;
    if let Some(reply) = read_some(&mut stream, "Hello 的回应", 6).await {
        match wire::decode(&reply) {
            Ok((wire::Decoded::Frame(f), _)) => {
                println!("  ✅ 路由器回的是 v2 帧！content_type = {}", f.content_type);
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
                    println!("       header {k} = {printable}");
                }
            }
            Ok((other, _)) => println!("  ⇒ 路由器回的是: {other:?}"),
            Err(e) => println!("  ⚠️  回应不是 v2 帧：{e}"),
        }
    }

    // ③ 再试一次 HTTP/1.1 GET（看它是不是只认 HTTP）
    let req = format!(
        "GET /edge/client/v1 HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    if stream.write_all(req.as_bytes()).await.is_ok() {
        let _ = stream.flush().await;
        read_some(&mut stream, "HTTP GET 的回应", 3).await;
    }
}

#[tokio::main]
async fn main() {
    let identity_path =
        std::env::var("ZITI_IDENTITY").expect("需要 ZITI_IDENTITY（指向 identity.json）");
    let router = std::env::var("ZITI_ROUTER").unwrap_or_else(|_| "192.168.31.170:3022".to_string());
    let service = std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "shadow-brain".to_string());

    println!("== 裸 TLS 探针（v2）==");
    println!("  router  = {router}");
    println!("  service = {service}");

    let identity = match Identity::load(&identity_path) {
        Ok(i) => i,
        Err(e) => {
            println!("  [FAIL] 加载身份: {e}");
            return;
        }
    };
    let ctrl = match Controller::authenticate(&identity).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [FAIL] 控制器认证: {e}");
            return;
        }
    };
    let api_session = match ctrl.api_session().await {
        Ok(t) => t,
        Err(e) => {
            println!("  [FAIL] 拿 api-session: {e}");
            return;
        }
    };
    println!("  ✅ 控制器认证成功（api-session 已拿到）");

    // ALPN 候选：v2 的 edge 通道**不报** ALPN ⇒ ⇒ 「不提供」才是对的那一个。
    let candidates: Vec<(Option<&[&str]>, &str)> = vec![
        (None, "(不提供)"),
        (Some(&["ziti"]), "ziti"),
        (Some(&["ziti-ctrl"]), "ziti-ctrl"),
        (Some(&["h2"]), "h2"),
        (Some(&["http/1.1"]), "http/1.1"),
    ];
    for (alpn, label) in candidates {
        probe(alpn, label, &router, &api_session, &identity).await;
    }
}
