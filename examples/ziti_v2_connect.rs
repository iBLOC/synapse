//! **v2 全链路探针** —— 认证 → 建服务会话 → Hello(1002) → Connect(60783) → Data。
//!
//! 依据（openziti/ziti v2.0.8 router/xgress_edge/listener.go:852-923）：
//!   serviceSessionTokenStr := string(req.Body)          ← Connect 的 body 是**服务会话 token**
//!   useXgToSdk, _ := req.GetBoolHeader(UseXgressToSdk)  ← 不带这个头 ⇒ legacy 数据流
//! 以及 connections.go:646-655：
//!   byteToken, ok := ch.Headers()[edgeSdk.SessionTokenHeader]  ← hello 头 1002 = api-session token

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

struct FrameReader {
    stream: tokio_rustls::client::TlsStream<TcpStream>,
    buf: Vec<u8>,
}

impl FrameReader {
    fn try_take(&mut self) -> Result<Option<wire::Frame>, String> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        match wire::decode(&self.buf) {
            Ok((wire::Decoded::Frame(f), used)) => {
                self.buf.drain(..used);
                Ok(Some(f))
            }
            Ok((other, used)) => {
                self.buf.drain(..used);
                Err(format!("非普通帧: {other:?}"))
            }
            Err(wire::WireError::TooShort { .. }) => Ok(None),
            Err(e) => Err(format!("解帧失败: {e}")),
        }
    }

    async fn next(&mut self, wait: Duration) -> Result<wire::Frame, String> {
        let deadline = tokio::time::Instant::now() + wait;
        loop {
            if let Some(f) = self.try_take()? {
                return Ok(f);
            }
            let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remain.is_zero() {
                return Err("超时".to_string());
            }
            let mut tmp = vec![0u8; 8192];
            let n = match tokio::time::timeout(remain, self.stream.read(&mut tmp)).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(format!("读失败: {e}")),
                Err(_) => return Err("超时".to_string()),
            };
            if n == 0 {
                return Err("对端关闭了连接".to_string());
            }
            self.buf.extend_from_slice(&tmp[..n]);
        }
    }

    async fn send(&mut self, f: &wire::Frame) -> Result<usize, String> {
        let bytes = f.encode(&wire::MAGIC_V2);
        self.stream
            .write_all(&bytes)
            .await
            .map_err(|e| format!("写失败: {e}"))?;
        let _ = self.stream.flush().await;
        Ok(bytes.len())
    }
}

fn describe(f: &wire::Frame) -> String {
    let mut s = format!("ct={} seq={} body={}字节", f.content_type, f.sequence, f.body.len());
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
        s.push_str(&format!(" | h{k}={}", head(&printable, 48)));
    }
    let body_text = String::from_utf8_lossy(&f.body).into_owned();
    if !body_text.is_empty() {
        s.push_str(&format!(" | 文本={}", head(&body_text, 220)));
    }
    s
}

#[tokio::main]
async fn main() {
    let identity_path =
        std::env::var("ZITI_IDENTITY").expect("需要 ZITI_IDENTITY（指向 identity.json）");
    let router = std::env::var("ZITI_ROUTER").unwrap_or_else(|_| "192.168.31.170:3022".into());
    let want_service = std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "shadow-brain".into());

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

    println!("== v2 全链路 ==");
    let resp = client
        .post(format!("{zt_api}/authenticate?method=cert"))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .expect("认证");
    let text = resp.text().await.unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let api_token = v["data"]["token"].as_str().unwrap_or("").to_string();
    println!("  api-session token = {api_token}");

    let resp = client
        .get(format!("{zt_api}/services"))
        .header("zt-session", &api_token)
        .send()
        .await
        .expect("列服务");
    let text = resp.text().await.unwrap_or_default();
    let sv: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let mut service_id = String::new();
    if let Some(items) = sv["data"].as_array() {
        for it in items {
            if it["name"].as_str() == Some(want_service.as_str()) {
                service_id = it["id"].as_str().unwrap_or("").to_string();
            }
        }
    }
    println!("  服务 id = {service_id}");

    // 建服务会话
    let resp = client
        .post(format!("{zt_api}/sessions"))
        .header("zt-session", &api_token)
        .header("Content-Type", "application/json")
        .body(
            serde_json::json!({ "serviceId": service_id, "type": "Dial" }).to_string(),
        )
        .send()
        .await
        .expect("建会话");
    let sstatus = resp.status();
    let stext = resp.text().await.unwrap_or_default();
    println!("  POST /sessions -> {sstatus}");
    println!("  {}", head(&stext, 1200));
    let sess: serde_json::Value = serde_json::from_str(&stext).unwrap_or(serde_json::Value::Null);
    let session_token = sess["data"]["token"].as_str().unwrap_or("").to_string();
    if session_token.is_empty() {
        println!("  !! 没拿到会话 token，后面没法连");
        return;
    }
    println!("  服务会话 token = {}", head(&session_token, 60));

    // TLS
    let mut roots = RootCertStore::empty();
    for c in load_certs(&ca_pem) {
        let _ = roots.add(c);
    }
    let chain = load_certs(&cert_pem);
    let key = load_key(&key_pem).expect("私钥解析失败");
    let mut cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(chain, key)
        .expect("TLS 配置");
    cfg.alpn_protocols = Vec::new();
    let connector = TlsConnector::from(Arc::new(cfg.clone()));
    let addr: std::net::SocketAddr = router.parse().expect("地址");
    let tcp = TcpStream::connect(addr).await.expect("TCP");
    let server_name = ServerName::try_from(addr.ip().to_string()).expect("ServerName");
    let stream = connector
        .connect(server_name, tcp)
        .await
        .expect("TLS 握手失败");
    let mut rd = FrameReader {
        stream,
        buf: Vec::new(),
    };
    println!("  TLS 握手成功 -> {router}");

    // Hello：头 1002 = api-session token
    let mut hello = wire::Frame::new(proto::content_type::HELLO, -1);
    hello.body = Vec::new();
    hello.put_bytes(proto::header_id::SESSION_TOKEN, api_token.as_bytes());
    let n = rd.send(&hello).await.expect("发 Hello");
    println!("  发出 Hello {} 字节", n);
    match rd.next(Duration::from_secs(5)).await {
        Ok(f) => println!("  Hello 回应: {}", describe(&f)),
        Err(e) => {
            println!("  Hello 没回应: {e}");
            return;
        }
    }

    // Connect：body = 服务会话 token
    let mut req = wire::Frame::new(proto::content_type::CONNECT, 1);
    req.put_u32(proto::header_id::CONNECTION_ID, 1);
    req.body = session_token.as_bytes().to_vec();
    let n = rd.send(&req).await.expect("发 Connect");
    println!("  发出 Connect {} 字节", n);
    match rd.next(Duration::from_secs(10)).await {
        Ok(f) => {
            println!("  Connect 回应: {}", describe(&f));
            if f.content_type == proto::content_type::STATE_CONNECTED {
                let cid = f.get_u32(proto::header_id::CONNECTION_ID).unwrap_or(1);
                println!("  ✅ StateConnected！connId={cid}");
                // 发一帧 Data 看看路由器认不认
                let mut data = wire::Frame::new(proto::content_type::DATA, 2);
                data.put_u32(proto::header_id::CONNECTION_ID, cid);
                data.body = b"ping-from-rust".to_vec();
                let dn = rd.send(&data).await.expect("发 Data");
                println!("  发出 Data {} 字节", dn);
                match rd.next(Duration::from_secs(5)).await {
                    Ok(f2) => println!("  Data 回应: {}", describe(&f2)),
                    Err(e) => println!("  Data 没回应: {e}"),
                }
            }
            if f.content_type == proto::content_type::STATE_CLOSED {
                println!("  ❌ StateClosed：被拒了");
            }
        }
        Err(e) => println!("  Connect 没回应: {e}"),
    }
}
