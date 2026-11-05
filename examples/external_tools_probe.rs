//! **真机探针** —— 读本机上**真实**的外部工具配置。
//!
//! ⚠️⚠️ **绝不打印材料** ⇒ ⇒ 只打印 **id / 来源文件 / 材料长度**。
//!
//! ⚠️ **它证明什么**：这族能力在**真实机器的真实文件**上跑得通
//! ⇒ ⇒ 而不只是「单测里的合成夹具跑得通」。
//!
//! ⚠️ **它不证明什么**：Claude Code / Codex 在没装的机器上**读不到**
//! ⇒ ⇒ 那一族只有合成夹具的证据（见各适配器模块的「证据边界」）。

use synapse::providers::import::{self, ExternalTool};

fn main() {
    let mut found = 0usize;

    for tool in ExternalTool::all() {
        let installed = import::is_installed(tool);
        println!("== {} installed={installed}", tool.display_name());

        if !installed {
            continue;
        }

        match import::read(tool) {
            Ok(cfg) => {
                for f in &cfg.files {
                    println!("   file: {}", f.display());
                }
                for p in &cfg.providers {
                    found += 1;
                    // ⚠️⚠️ **只报长度** —— 材料一个字都不打。
                    println!(
                        "   - id={} key_len={:?} base_url={:?}",
                        p.source_id,
                        p.config.api_key.as_ref().map(|k| k.len()),
                        p.config.base_url
                    );
                }
            }
            Err(e) => println!("   ERROR: {e}"),
        }
    }

    println!("-- providers found: {found}");
    if found == 0 {
        println!("(nothing to import on this machine)");
    }
}
