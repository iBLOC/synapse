//! OpenZiti **v2** 的线路层 —— **实现在另一个仓**。
//!
//! ⚠️ **2026-11-05：v2 被抽成 `iBLOC/ziti-rust` 的第二个 crate（`ziti-v2`）**
//! —— 实现在 `ziti-rust/crates/ziti-v2/`（OpenZiti 官方没有 Rust SDK ⇒ ⇒
//! 那套 v1 的 `ziti-sdk` 说的是 WebSocket + 无 magic 的自制帧 ⇒ ⇒ 对着
//! OpenZiti v2 控制器直接用不了 ⇒ ⇒ 线路层是**我们自己写的**）。
//!
//! ⇒ ⇒ 本模块**只是 re-export** ⇒ ⇒ `synapse::transport::ziti::v2::…`
//! 的路径**对调用方零变化**（仓内 7 个 example 直接用这条路径）。
//!
//! # 为什么用 path 依赖而不是 git 依赖
//!
//! iBLOC 全栈是**多仓 path 工作区**（每个 crate 用 `path = "../<repo>"` 找
//! 兄弟仓）⇒ ⇒ 这里跟着同一套约定 ⇒ ⇒ CI 把兄弟仓 checkout 到工作区的
//! **子目录**（`actions/checkout` 不许 checkout 到工作区之外）⇒ ⇒
//! `../ziti-rust/crates/ziti-v2` 在 CI 里同样解析得到。
//!
//! # 每一层的职责
//!
//! | 模块 | 干什么 |
//! |---|---|
//! | `wire` | 帧编解码（小端、20 字节帧头） |
//! | `proto` | 内容类型 / 头 id / 错误码常量表 |
//! | `identity` | 身份文件（ztAPI / cert / key / ca） |
//! | `controller` | v2 控制器 API（认证 / 列服务 / 建服务会话） |
//! | `channel` | 一条到 edge router 的 TLS 连接 ＋ 帧收发 ＋ 分发 |
//! | `stream` | 一条类 TCP 的双向流（AsyncRead + AsyncWrite） |
//! | `context` | dial / listen 两个动作 |

pub use ziti_v2::*;
