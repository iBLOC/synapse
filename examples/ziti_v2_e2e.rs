//! **v2 真网端到端探针** —— 同一个进程里当宿主 + 当客户端，跑完整条链。
//!
//! 用法：
//!   $env:ZITI_CLIENT_IDENTITY = "C:\...\shadow-brain-client.identity.json"
//!   $env:ZITI_HOST_IDENTITY   = "C:\...\shadow-brain-host.identity.json"
//!   $env:ZITI_ROUTER = "192.168.31.170:3022"
//!   $env:ZITI_SERVICE = "shadow-brain"
//!   cargo run --offline --features ziti --example ziti_v2_e2e
//!
//! 链路（全部来自 ziti v2.0.8 路由器源码 + 实测）：
//!   宿主：认证(cert) → 建 Bind 会话 → TLS → Hello(头1002=api-session token)
//!         → Bind(60790, body=Bind 会话 token, 头1000=connId) → 路由器自建 terminator
//!         → 收 Dial(60787) → 回 DialSuccess(60788, body=newConnId u32) → Data 回环
//!   客户端：认证 → 建 Dial 会话 → TLS → Hello → Connect(60783, body=Dial 会话 token)
//!         → StateConnected(60784) → Data(60786) → 收到回环
//!
//! ⚠️ **不需要手建 terminator** —— 路由器的 processBind 会自己建（binding=edge）。

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

struct Peer {
    stream: tokio_rustls::client::TlsStream<TcpStream>,
    buf: Vec<u8>,
}

impl Peer {
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
        s.push_str(&format!(" | h{k}={}", head(&printable, 40)));
    }
    let body_text = String::from_utf8_lossy(&f.body).into_owned();
    if !body_text.is_empty() {
        s.push_str(&format!(" | 文本={}", head(&body_text, 4000)));
    }
    s
}

struct Identity {
    zt_api: String,
    ca_pem: Vec<u8>,
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
}

fn load_identity(path: &str) -> Identity {
    let raw = std::fs::read_to_string(path).expect("读 identity.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("解析 identity.json");
    Identity {
        zt_api: v["ztAPI"]
            .as_str()
            .expect("缺 ztAPI")
            .trim_end_matches('/')
            .to_string(),
        ca_pem: std::fs::read(v["ca"].as_str().expect("ca")).expect("读 ca"),
        cert_pem: std::fs::read(v["cert"].as_str().expect("cert")).expect("读 cert"),
        key_pem: std::fs::read(v["key"].as_str().expect("key")).expect("读 key"),
    }
}

fn build_http(id: &Identity) -> reqwest::Client {
    let certs = reqwest::Certificate::from_pem_bundle(&id.ca_pem).expect("解析 CA");
    let mut bundle = id.cert_pem.clone();
    if !bundle.ends_with(b"\n") {
        bundle.push(b'\n');
    }
    bundle.extend_from_slice(&id.key_pem);
    let ident = reqwest::Identity::from_pem(&bundle).expect("解析身份");
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .identity(ident)
        .tls_certs_only(certs)
        .no_proxy()
        .build()
        .expect("建客户端")
}

/// 认证 + 建服务会话，返回 (api-session token, 服务会话 token, 服务 id)
async fn authenticate_and_session(
    id: &Identity,
    client: &reqwest::Client,
    service_name: &str,
    session_type: &str,
) -> Result<(String, String, String), String> {
    let resp = client
        .post(format!("{}/authenticate?method=cert", id.zt_api))
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
        .map_err(|e| format!("认证失败: {e}"))?;
    let text = resp.text().await.unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let api_token = v["data"]["token"].as_str().unwrap_or("").to_string();
    if api_token.is_empty() {
        return Err(format!("没拿到 api-session token: {}", head(&text, 300)));
    }
    let resp = client
        .get(format!("{}/services", id.zt_api))
        .header("zt-session", &api_token)
        .send()
        .await
        .map_err(|e| format!("列服务失败: {e}"))?;
    let text = resp.text().await.unwrap_or_default();
    let sv: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let mut service_id = String::new();
    if let Some(items) = sv["data"].as_array() {
        for it in items {
            if it["name"].as_str() == Some(service_name) {
                service_id = it["id"].as_str().unwrap_or("").to_string();
            }
        }
    }
    if service_id.is_empty() {
        return Err(format!("找不到服务 {service_name}"));
    }
    let body = serde_json::json!({ "serviceId": service_id, "type": session_type }).to_string();
    let resp = client
        .post(format!("{}/sessions", id.zt_api))
        .header("zt-session", &api_token)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("建会话失败: {e}"))?;
    let text = resp.text().await.unwrap_or_default();
    let sess: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let session_token = sess["data"]["token"].as_str().unwrap_or("").to_string();
    if session_token.is_empty() {
        return Err(format!(
            "没拿到 {session_type} 会话 token: {}",
            head(&text, 400)
        ));
    }
    Ok((api_token, session_token, service_id))
}

async fn connect_router(id: &Identity, router: &str) -> Result<Peer, String> {
    let mut roots = RootCertStore::empty();
    for c in load_certs(&id.ca_pem) {
        let _ = roots.add(c);
    }
    let chain = load_certs(&id.cert_pem);
    let key = load_key(&id.key_pem).ok_or("私钥解析失败")?;
    let mut cfg = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(chain, key)
        .map_err(|e| format!("TLS 配置: {e}"))?;
    cfg.alpn_protocols = Vec::new();
    let connector = TlsConnector::from(Arc::new(cfg));
    let addr: std::net::SocketAddr = router.parse().map_err(|_| format!("地址错: {router}"))?;
    let tcp = TcpStream::connect(addr).await.map_err(|e| format!("TCP: {e}"))?;
    let sn = ServerName::try_from(addr.ip().to_string()).map_err(|e| format!("ServerName: {e}"))?;
    let stream = connector
        .connect(sn, tcp)
        .await
        .map_err(|e| format!("TLS: {e}"))?;
    Ok(Peer {
        stream,
        buf: Vec::new(),
    })
}

async fn hello(peer: &mut Peer, api_token: &str) -> Result<(), String> {
    let mut h = wire::Frame::new(proto::content_type::HELLO, -1);
    h.put_bytes(proto::header_id::SESSION_TOKEN, api_token.as_bytes());
    peer.send(&h).await?;
    let f = peer.next(Duration::from_secs(6)).await?;
    if f.content_type != proto::content_type::RESULT {
        return Err(format!("Hello 回应不是 Result: {}", describe(&f)));
    }
    if f.get_bool(proto::channel_header::RESULT_SUCCESS) != Some(true) {
        return Err(format!("Hello 被拒: {}", describe(&f)));
    }
    Ok(())
}

/// 宿主：Bind → 收 Dial → 回 DialSuccess → Data 回环
async fn run_host(
    id: Identity,
    client: reqwest::Client,
    router: String,
    service_name: String,
) -> Result<(), String> {
    let (api_token, session_token, service_id) =
        authenticate_and_session(&id, &client, &service_name, "Bind").await?;
    println!("[宿主] api-session={}", head(&api_token, 12));
    println!("[宿主] 服务={service_id} Bind 会话={}", head(&session_token, 24));
    let mut peer = connect_router(&id, &router).await?;
    hello(&mut peer, &api_token).await?;
    println!("[宿主] Hello 成功");

    let mut bind = wire::Frame::new(proto::content_type::BIND, 1);
    bind.put_u32(proto::header_id::CONNECTION_ID, 1);
    bind.body = session_token.as_bytes().to_vec();
    peer.send(&bind).await?;
    println!("[宿主] 已发 Bind");

    let mut data_conn_id: Option<u32> = None;
    let mut next_conn_id: u32 = 1000;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(40);
    loop {
        let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remain.is_zero() {
            return Err("宿主等待超时".to_string());
        }
        let f = match peer.next(remain).await {
            Ok(f) => f,
            Err(e) => return Err(format!("宿主读取: {e}")),
        };
        println!("[宿主] 收到 {}", describe(&f));
        match f.content_type {
            proto::content_type::DIAL => {
                // ⚠️ Dial 的 body 是**宿主自己的 Bind 会话 token**（路由器回传让我们自证身份），
                //    不是 newConnId。SDK 官方也是拿它跟自己的 token 比（hosting_conn.go:278）。
                let dial_token = String::from_utf8_lossy(&f.body).into_owned();
                if dial_token != session_token {
                    println!("[宿主] ⚠️ Dial token 与本地 Bind 会话不一致，路由器发错了？");
                }
                // newConnId：legacy 路径下路由器不填 RouterProvidedConnId(1012)，
                // 所以由宿主自己分配（SDK 用 msgMux.GetNextId()，同理）。
                let new_conn_id = f
                    .get_u32(proto::header_id::ROUTER_PROVIDED_CONN_ID)
                    .unwrap_or_else(|| {
                        next_conn_id += 1;
                        next_conn_id
                    });
                let mut ok = wire::Frame::new(proto::content_type::DIAL_SUCCESS, 2);
                ok.put_u32(
                    proto::header_id::CONNECTION_ID,
                    f.get_u32(proto::header_id::CONNECTION_ID).unwrap_or(1),
                );
                // ⚠️⚠️ **必须带 ReplyFor（channel 头 1）** —— 路由器用 SendForReply 等这条回复，
                //    靠的就是 ReplyFor == 被回应那条 Dial 的 sequence。
                //    少了它 ⇒ 路由器把回复当普通消息丢掉 ⇒ 5 秒后
                //    "timeout waiting for message reply" ⇒ 客户端收到 StateClosed。
                if f.sequence >= 0 {
                    ok.put_u32(proto::channel_header::REPLY_FOR, f.sequence as u32);
                }
                ok.body = new_conn_id.to_le_bytes().to_vec();
                peer.send(&ok).await?;
                data_conn_id = Some(new_conn_id);
                println!(
                    "[宿主] ✅ 已回 DialSuccess，newConnId={new_conn_id}（ReplyFor={}）",
                    f.sequence
                );
            }
            proto::content_type::DATA => {
                let cid = f.get_u32(proto::header_id::CONNECTION_ID).unwrap_or(1);
                let text = String::from_utf8_lossy(&f.body).into_owned();
                println!("[宿主] Data: {text}");
                let use_cid = data_conn_id.unwrap_or(cid);
                let mut d = wire::Frame::new(proto::content_type::DATA, 3);
                d.put_u32(proto::header_id::CONNECTION_ID, use_cid);
                d.body = format!("回声:{text}").into_bytes();
                peer.send(&d).await?;
                println!("[宿主] 已回声（connId={use_cid}）");
            }
            proto::content_type::STATE_CLOSED => {
                return Err(format!("宿主被关: {}", describe(&f)));
            }
            _ => {}
        }
    }
}

/// 客户端：Connect → Data → 收回声
async fn run_client(
    id: Identity,
    client: reqwest::Client,
    router: String,
    service_name: String,
) -> Result<(), String> {
    let (api_token, session_token, service_id) =
        authenticate_and_session(&id, &client, &service_name, "Dial").await?;
    println!("[客户端] api-session={}", head(&api_token, 12));
    println!("[客户端] 服务={service_id} Dial 会话={}", head(&session_token, 24));
    let mut peer = connect_router(&id, &router).await?;
    hello(&mut peer, &api_token).await?;
    println!("[客户端] Hello 成功");

    let mut req = wire::Frame::new(proto::content_type::CONNECT, 1);
    req.put_u32(proto::header_id::CONNECTION_ID, 7);
    req.body = session_token.as_bytes().to_vec();
    peer.send(&req).await?;
    println!("[客户端] 已发 Connect");
    let f = peer.next(Duration::from_secs(15)).await?;
    println!("[客户端] 收到 {}", describe(&f));
    if f.content_type != proto::content_type::STATE_CONNECTED {
        return Err(format!("没连上: {}", describe(&f)));
    }
    let cid = f.get_u32(proto::header_id::CONNECTION_ID).unwrap_or(7);
    println!("[客户端] ✅ StateConnected connId={cid}");

    let mut d = wire::Frame::new(proto::content_type::DATA, 2);
    d.put_u32(proto::header_id::CONNECTION_ID, cid);
    d.body = "你好，shadow-brain".as_bytes().to_vec();
    peer.send(&d).await?;
    println!("[客户端] 已发 Data");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let remain = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remain.is_zero() {
            return Err("客户端等回声超时".to_string());
        }
        let f = match peer.next(remain).await {
            Ok(f) => f,
            Err(e) => return Err(format!("客户端读取: {e}")),
        };
        println!("[客户端] 收到 {}", describe(&f));
        if f.content_type == proto::content_type::DATA {
            let text = String::from_utf8_lossy(&f.body).into_owned();
            println!("[客户端] ✅ 收到回声：{text}");
            return Ok(());
        }
        if f.content_type == proto::content_type::STATE_CLOSED {
            return Err(format!("连接被关: {}", describe(&f)));
        }
    }
}

#[tokio::main]
async fn main() {
    let client_path = std::env::var("ZITI_CLIENT_IDENTITY")
        .expect("需要 ZITI_CLIENT_IDENTITY");
    let host_path = std::env::var("ZITI_HOST_IDENTITY").expect("需要 ZITI_HOST_IDENTITY");
    let router = std::env::var("ZITI_ROUTER").unwrap_or_else(|_| "192.168.31.170:3022".into());
    let service = std::env::var("ZITI_SERVICE").unwrap_or_else(|_| "shadow-brain".into());

    println!("== v2 真网端到端 ==");
    println!("  router={router} service={service}");

    let cid = load_identity(&client_path);
    let hid = load_identity(&host_path);
    let cclient = build_http(&cid);
    let hclient = build_http(&hid);

    let host = tokio::spawn(run_host(hid, hclient, router.clone(), service.clone()));
    tokio::time::sleep(Duration::from_millis(800)).await;
    let client_res = run_client(cid, cclient, router.clone(), service.clone()).await;

    let host_res = host.await;

    println!();
    println!("== 结果 ==");
    match client_res {
        Ok(()) => println!("  客户端: ✅ 成功"),
        Err(e) => println!("  客户端: ❌ {e}"),
    }
    match host_res {
        Ok(Ok(())) => println!("  宿主: ✅ 成功"),
        Ok(Err(e)) => println!("  宿主: ❌ {e}"),
        Err(e) => println!("  宿主: ❌ 任务异常 {e}"),
    }
}
