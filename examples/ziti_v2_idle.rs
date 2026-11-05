//! 长连接体检 —— **空闲保持 / 心跳保活 / 稳定性循环**。
//!
//! ⚠️ 为什么要有这个探针：真网那 5 条测试跑的是**短连接** ⇒ 「长连接会不会被
//!    路由器悄悄回收」「空闲多久会断」「连一万次会不会漏」这三件事**从来没被量过**
//!    ⇒ ⇒ 手册与架构图页只能把它们标成「待办」⇒ ⇒ 本探针把它们变成**读数**。
//!
//! 三段：
//!   A1 **纯空闲**：dial 之后两端**都不发任何数据** ⇒ 宿主侧的读何时返回 ⇒
//!      那就是**空闲超时**（路由器的会话回收窗口）。
//!   A2 **心跳保活**：每 30 秒 ping/pong 一次 ⇒ 看能不能一直撑住 ⇒
//!      ⇒ 若 A1 被回收而 A2 活着 ⇒ **心跳是必须的**（这是给上层的心跳间隔下界）。
//!   A3 **稳定性循环**：连续 N 次 dial → echo → 关闭 ⇒ 成功率 ＋ 延迟分位。
//!
//! 用法：
//!   $env:ZITI_CLIENT_IDENTITY = "...client.identity.json"
//!   $env:ZITI_HOST_IDENTITY   = "...host.identity.json"
//!   $env:ZITI_SERVICE         = "shadow-brain"
//!   $env:IDLE_SECS            = "900"    # A1 最长等多久（默认 15 分钟）
//!   $env:HEARTBEAT_SECS       = "600"    # A2 跑多久（默认 10 分钟）
//!   $env:CYCLES               = "50"     # A3 循环几次
//!   cargo run --offline --features ziti --example ziti_v2_idle
//!
//! ⚠️ 宿主必须**并发**跑 accept —— 路由器的 Dial 只等 5 秒（见 ziti_v2_prod.rs）。

use std::time::{Duration, Instant};

use synapse::transport::ziti::ZitiGateway;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn env_or(key: &str, fallback: &str) -> String {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn env_num(key: &str, fallback: u64) -> u64 {
    env_or(key, "")
        .parse::<u64>()
        .unwrap_or(fallback)
}

/// 宿主侧的一条连接断了。
struct Break {
    conn: u32,
    /// 从 accept 到断开过了多久（毫秒）。
    alive_ms: u64,
    why: String,
}

/// 等某条连接断开（超时 = 还没断）。
async fn wait_break(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<Break>,
    conn: u32,
    timeout: Duration,
) -> Option<Break> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        match tokio::time::timeout(left, rx.recv()).await {
            Ok(Some(b)) if b.conn == conn => return Some(b),
            Ok(Some(_)) => continue, // 别的连接的断开事件
            Ok(None) => return None,
            Err(_) => return None,
        }
    }
}

#[tokio::main]
async fn main() {
    let client_path = env_or("ZITI_CLIENT_IDENTITY", "");
    let host_path = env_or("ZITI_HOST_IDENTITY", "");
    let service = env_or("ZITI_SERVICE", "shadow-brain");
    let idle_secs = env_num("IDLE_SECS", 900);
    let hb_secs = env_num("HEARTBEAT_SECS", 600);
    let cycles = env_num("CYCLES", 50) as usize;
    assert!(!client_path.is_empty(), "需要 ZITI_CLIENT_IDENTITY");
    assert!(!host_path.is_empty(), "需要 ZITI_HOST_IDENTITY");

    println!("=== 长连接体检 ===");
    println!("service={service}  A1上限={idle_secs}s  A2={hb_secs}s  A3={cycles} 次");

    // ── 宿主：listen + 后台 accept 循环 ──────────────────────────────
    let host = ZitiGateway::new(&host_path).await.expect("宿主 gateway");
    let mut listener = host.listen(&service).await.expect("listen 失败");
    println!("[host] listen({service}) ✅");
    let (break_tx, mut break_rx) = tokio::sync::mpsc::unbounded_channel::<Break>();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok(mut stream) => {
                    let conn = stream.conn_id();
                    let at = Instant::now();
                    let tx = break_tx.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 256];
                        loop {
                            match stream.read(&mut buf).await {
                                Ok(0) => {
                                    let _ = tx.send(Break { conn, alive_ms: at.elapsed().as_millis() as u64, why: "对端 FIN（读到 EOF）".into() });
                                    break;
                                }
                                Ok(n) => {
                                    if stream.write_all(&buf[..n]).await.is_err() {
                                        let _ = tx.send(Break { conn, alive_ms: at.elapsed().as_millis() as u64, why: "回写失败（通道已关）".into() });
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Break { conn, alive_ms: at.elapsed().as_millis() as u64, why: format!("读错误: {e}") });
                                    break;
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    println!("[host] accept 失败: {e}");
                    break;
                }
            }
        }
    });

    // ── 客户端 gateway ─────────────────────────────────────────────
    let client = ZitiGateway::new(&client_path).await.expect("客户端 gateway");

    // ── A1：纯空闲（两端都不发数据）────────────────────────────────
    let t1 = Instant::now();
    let mut idle_stream = client.dial(&service).await.expect("A1 dial 失败");
    let idle_conn = idle_stream.conn_id();
    println!("[A1] dial ✅ conn={idle_conn} —— 两端都不发任何数据 ...");
    let b = wait_break(&mut break_rx, idle_conn, Duration::from_secs(idle_secs)).await;
    let idle_result = match b {
        Some(b) => format!("断开：{} 秒（{}）", b.alive_ms / 1000, b.why),
        None => format!("**撑满 {idle_secs} 秒仍未断**（本窗口内路由器没回收空闲会话）"),
    };
    println!("[A1] 结果 —— {idle_result}");
    // 客户端自己也看一眼：这条连接现在还能不能写。
    let idle_write = match idle_stream.write_all(b"x").await {
        Ok(()) => "还能写（客户端侧没察觉）".to_string(),
        Err(e) => format!("写失败（客户端侧也断了）: {e}"),
    };
    println!("[A1] 客户端视角 —— {idle_write}（t={}s）", t1.elapsed().as_secs());
    drop(idle_stream);

    // ── A2：心跳保活 ──────────────────────────────────────────────
    let mut hb = client.dial(&service).await.expect("A2 dial 失败");
    let hb_conn = hb.conn_id();
    println!("[A2] dial ✅ conn={hb_conn} —— 每 30 秒 ping/pong 一次 ...");
    let t2 = Instant::now();
    let mut hb_ok = 0u64;
    let mut hb_fail = 0u64;
    let mut hb_err = String::new();
    while t2.elapsed().as_secs() < hb_secs {
        tokio::time::sleep(Duration::from_secs(30)).await;
        if let Err(e) = hb.write_all(b"ping").await {
            hb_fail += 1;
            hb_err = format!("写失败: {e}");
            break;
        }
        let mut buf = [0u8; 16];
        match tokio::time::timeout(Duration::from_secs(10), hb.read(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => hb_ok += 1,
            Ok(Ok(_)) => {
                hb_fail += 1;
                hb_err = "读到 EOF".into();
                break;
            }
            Ok(Err(e)) => {
                hb_fail += 1;
                hb_err = format!("读错误: {e}");
                break;
            }
            Err(_) => {
                hb_fail += 1;
                hb_err = "读超时（10 秒没回 pong）".into();
                break;
            }
        }
    }
    println!("[A2] 结果 —— 撑了 {} 秒，心跳成功 {hb_ok} 次，失败 {hb_fail} 次{}", t2.elapsed().as_secs(), if hb_err.is_empty() { String::new() } else { format!("（{hb_err}）") });
    drop(hb);

    // ── A3：稳定性循环 ────────────────────────────────────────────
    println!("[A3] 连续 {cycles} 次 dial → echo → 关闭 ...");
    let mut ok = 0u64;
    let mut fail = 0u64;
    let mut lat: Vec<u64> = Vec::new();
    let mut first_err = String::new();
    for i in 0..cycles {
        let t = Instant::now();
        match client.dial(&service).await {
            Ok(mut st) => {
                match st.write_all(b"ping").await {
                    Ok(()) => {
                        let mut buf = [0u8; 16];
                        match tokio::time::timeout(Duration::from_secs(10), st.read(&mut buf)).await {
                            Ok(Ok(n)) if n > 0 => {
                                ok += 1;
                                lat.push(t.elapsed().as_millis() as u64);
                            }
                            Ok(Ok(_)) => {
                                fail += 1;
                                if first_err.is_empty() { first_err = format!("第 {} 次：读到 EOF", i + 1); }
                            }
                            Ok(Err(e)) => {
                                fail += 1;
                                if first_err.is_empty() { first_err = format!("第 {} 次：读错误 {e}", i + 1); }
                            }
                            Err(_) => {
                                fail += 1;
                                if first_err.is_empty() { first_err = format!("第 {} 次：读超时", i + 1); }
                            }
                        }
                    }
                    Err(e) => {
                        fail += 1;
                        if first_err.is_empty() { first_err = format!("第 {} 次：写错误 {e}", i + 1); }
                    }
                }
            }
            Err(e) => {
                fail += 1;
                if first_err.is_empty() { first_err = format!("第 {} 次：dial 失败 {e}", i + 1); }
            }
        }
    }
    lat.sort_unstable();
    let p = |q: f64| -> u64 {
        if lat.is_empty() {
            return 0;
        }
        let idx = ((lat.len() as f64 - 1.0) * q).round() as usize;
        lat[idx.min(lat.len() - 1)]
    };
    println!("[A3] 结果 —— 成功 {ok} / 失败 {fail}（共 {cycles} 次）");
    println!("[A3] 延迟 —— p50={}ms  p95={}ms  max={}ms", p(0.5), p(0.95), lat.last().copied().unwrap_or(0));
    if !first_err.is_empty() {
        println!("[A3] 首个失败 —— {first_err}");
    }

    println!();
    if fail == 0 {
        println!("✅ A3 稳定性：{cycles}/{cycles} 全通");
    } else {
        println!("❌ A3 稳定性：失败 {fail} 次");
    }
    std::process::exit(if fail == 0 { 0 } else { 1 });
}
