//! 生产路径的逐段体检：ZitiGateway::listen / dial / accept / 数据回环。
//!
//! ⚠️ **时序要点** —— 路由器的 Dial 只等 **5 秒**（dialLegacy 的 WithTimeout）
//! ⇒ ⇒ 宿主必须**并发**地跑 accept 循环 ⇒ ⇒ 不能「先拨号再 accept」。
//!
//! 用法：
//!   $env:ZITI_CLIENT_IDENTITY = "...client.identity.json"
//!   $env:ZITI_HOST_IDENTITY   = "...host.identity.json"
//!   $env:ZITI_SERVICE         = "shadow-brain"
//!   cargo run --offline --features ziti --example ziti_v2_prod

use std::time::Duration;

use synapse::transport::ziti::ZitiGateway;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

#[tokio::main]
async fn main() {
    let client_path = env_or("ZITI_CLIENT_IDENTITY", "");
    let host_path = env_or("ZITI_HOST_IDENTITY", "");
    let service = env_or("ZITI_SERVICE", "shadow-brain");
    assert!(!client_path.is_empty(), "需要 ZITI_CLIENT_IDENTITY");
    assert!(!host_path.is_empty(), "需要 ZITI_HOST_IDENTITY");

    println!("[1] 建宿主 gateway ...");
    let host = ZitiGateway::new(&host_path).await.expect("宿主 gateway");
    println!("[2] 宿主 listen({service}) ...");
    let mut listener = tokio::time::timeout(Duration::from_secs(30), host.listen(&service))
        .await
        .expect("listen 超时")
        .expect("listen 失败");
    println!("[3] ✅ 宿主已 listen");

    // ⚠️ 宿主在**后台**跑 accept —— 路由器的 Dial 只等 5 秒。
    let host_task = tokio::spawn(async move {
        let mut h = listener.accept().await?;
        println!("[H1] ✅ 宿主已 accept（conn={}）", h.conn_id());
        let mut buf = [0u8; 64];
        let n = tokio::time::timeout(Duration::from_secs(15), h.read(&mut buf))
            .await
            .map_err(|_| anyhow::anyhow!("宿主读超时"))??;
        println!("[H2] ✅ 宿主收到 {:?}（{n} 字节）", &buf[..n]);
        h.write_all(b"pong").await?;
        println!("[H3] ✅ 宿主已回 pong");
        Ok::<(), anyhow::Error>(())
    });

    println!("[4] 建客户端 gateway ...");
    let client = ZitiGateway::new(&client_path).await.expect("客户端 gateway");
    println!("[5] 客户端 dial({service}) ...");
    let mut cstream = match tokio::time::timeout(Duration::from_secs(30), client.dial(&service)).await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            println!("❌ dial 失败: {e}");
            let _ = host_task.await;
            return;
        }
        Err(_) => {
            println!("❌ dial 超时");
            let _ = host_task.await;
            return;
        }
    };
    println!("[6] ✅ 客户端已 dial（conn={}）", cstream.conn_id());

    let mut buf = [0u8; 64];
    cstream.write_all(b"ping").await.expect("客户端写");
    println!("[7] 客户端已写 ping");
    let n = match tokio::time::timeout(Duration::from_secs(15), cstream.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            println!("❌ 客户端读失败: {e}");
            let _ = host_task.await;
            return;
        }
        Err(_) => {
            println!("❌ 客户端读超时");
            let _ = host_task.await;
            return;
        }
    };
    println!("[8] ✅ 客户端收到 {:?}（{n} 字节）", &buf[..n]);

    match host_task.await {
        Ok(Ok(())) => println!("✅ 全链路通（生产 API：listen/dial/accept/数据回环）"),
        Ok(Err(e)) => println!("❌ 宿主侧出错: {e}"),
        Err(e) => println!("❌ 宿主任务异常: {e}"),
    }
}
