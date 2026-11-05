//! OpenZiti **v2 线路协议**探针 —— 只测「能不能和 edge router 说上话」。
//!
//! 用法：
//!   $env:ZITI_IDENTITY = "C:\path\to\client.identity.json"
//!   $env:ZITI_SERVICE  = "shadow-brain"
//!   cargo run --offline --features ziti --example ziti_wire_probe
//!
//! ⚠️ **2026-11-04：v2 迁移后重写** —— 旧版探针发的是 **v1** 的 ZitiMessage
//!    （没有 magic 的 WebSocket 帧）⇒ ⇒ 在 v2 路由器上**必然**失败
//!    ⇒ ⇒ 现在发的是 **channel v4 的 magicV2 二进制帧**。
//!
//! **它测的是什么、不是什么**
//! - ✅ 测：控制器能不能 mTLS 认证 ＋ 服务会话能不能建 ＋ 客户端面能不能
//!   TLS 握手 ＋ **Hello 帧（magicV2 ＋ 头 1002）能不能被路由器认出来**。
//! - ❌ 不测：Connect / Bind / 数据面 —— 那是 ziti_v2_e2e 与 ziti_v2_prod 的活。
//!
//! ⇒ ⇒ 配合 ziti_doctor（控制器 API 层）＝ 两层各测各的，
//!    分开就能定位到底是哪一层坏了。

use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

use synapse::transport::ziti::v2::{proto, wire, Controller, Identity, SessionKind};
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

async fn read_frame(
    stream: &mut tokio_rustls::client::TlsStream<TcpStream>,
) -> Result<wire::Decoded, String> {
    let mut acc: Vec<u8> = Vec::new();
    let mut tmp = vec![0u8; 4096];
    loop {
        let n = match tokio::time::timeout(Duration::from_secs(8), stream.read(&mut tmp)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("读失败: {e}")),
            Err(_) => return Err("8 秒内没有数据".to_string()),
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
                println!("    header {k} = ascii={printable} ({} 字节)", v.len());
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
    let service = std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "shadow-brain".to_string());
    let router_override = std::env::var("ZITI_ROUTER").ok().filter(|s| !s.trim().is_empty());

    println!("== OpenZiti v2 线路探针 ==");
    println!("  identity = {identity_path}");
    println!("  service  = {service}");

    // ① 身份文件
    let identity = match Identity::load(&identity_path) {
        Ok(i) => i,
        Err(e) => {
            println!("  [FAIL] 加载身份: {e}");
            return;
        }
    };
    println!("  ✅ 身份文件读到了：ztAPI = {}", identity.zt_api);

    // ② 控制器 mTLS 认证（私有 CA ⇒ 只能用 tls_certs_only）
    let ctrl = match Controller::authenticate(&identity).await {
        Ok(c) => c,
        Err(e) => {
            println!("  [FAIL] 控制器认证: {e}");
            return;
        }
    };
    println!("  ✅ 控制器 mTLS 认证成功");

    // ③ 建服务会话 ⇒ 顺带拿到**客户端面**地址
    let (api_session, session) = match ctrl.create_session(&service, SessionKind::Dial).await {
        Ok(s) => s,
        Err(e) => {
            println!("  [FAIL] 建服务会话: {e}");
            return;
        }
    };
    println!("  ✅ 服务会话建好：service_id = {}", session.service_id);
    println!("     edge routers（客户端面）= {:?}", session.edge_routers);

    let addr_str = match router_override.or_else(|| session.edge_routers.first().cloned()) {
        Some(a) => a,
        None => {
            println!("  [FAIL] 会话里没有 edge router 地址 ⇒ ⇒ 客户端面拿不到");
            return;
        }
    };
    println!("  客户端面 = {addr_str}");

    // ④ TLS 握手 —— ⚠️ **不许报 ALPN**（报了 ⇒ 路由器回 TLS fatal InternalError）
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
    cfg.alpn_protocols = Vec::new();

    let addr: std::net::SocketAddr = match addr_str.parse() {
        Ok(a) => a,
        Err(_) => {
            println!("  [FAIL] 客户端面地址不是 host:port —— {addr_str}");
            return;
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
    println!("  ✅ TLS 握手成功（未报 ALPN）");

    // ⑤ Hello 帧：magicV2 ＋ **头 1002 ＝ api-session token**
    let mut hello = wire::Frame::new(proto::content_type::HELLO, -1);
    hello.put_bytes(proto::header_id::SESSION_TOKEN, api_session.as_bytes());
    let bytes = hello.encode(&wire::MAGIC_V2);
    println!("  发出 Hello 帧：{} 字节（magicV2 ＋ 头 1002）", bytes.len());
    if let Err(e) = stream.write_all(&bytes).await {
        println!("  [FAIL] 写 Hello 失败: {e}");
        return;
    }
    let _ = stream.flush().await;

    match read_frame(&mut stream).await {
        Ok(d) => {
            print_frame("路由器的回应", &d);
            println!();
            println!("==> 线路层通了（v2 magicV2 ＋ 头 1002 被路由器认出来）");
        }
        Err(e) => {
            println!("  没拿到回应: {e}");
            println!();
            println!("!! 线路层没通 ⇒ ⇒ 查：客户端面端口（3022，不是 10080）／ 头 1002 有没有带上 ／ ALPN 有没有报");
        }
    }
}
