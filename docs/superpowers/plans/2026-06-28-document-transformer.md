# Synapse Document Transformer (Chunking) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]` syntax) for tracking.

**Goal:** 实现 Document Transformer 抽象 + 3 个常用 chunker (Character / Recursive / Markdown)，让 RAG 实际可用。

**Architecture:** 引入 `DocumentTransformer` trait（`async fn transform` + `name`）。3 个 chunker 都实现这个 trait，操作 `retriever::Document`（与现有 Indexer 兼容）。每个 chunk 自动加 `source_id` + `chunk_index` metadata。

**Tech Stack:** Rust, async-trait, serde, serde_json, std

**Spec Reference:** `docs/superpowers/specs/2026-06-28-document-transformer-design.md`

---

## File Structure

**New Files:**
- `src/components/transformer/mod.rs` — DocumentTransformer trait + re-exports
- `src/components/transformer/character_splitter.rs` — CharacterTextSplitter
- `src/components/transformer/recursive_character_splitter.rs` — RecursiveCharacterTextSplitter
- `src/components/transformer/markdown_header_splitter.rs` — MarkdownHeaderSplitter
- `tests/transformer_test.rs` — 10 integration tests

**Modified Files:**
- `src/components/mod.rs` — 添加 `pub mod transformer;`

---

## Chunk 1: Trait + CharacterTextSplitter

**目标：** DocumentTransformer trait + 第一个 chunker（最简单）+ 4 unit tests。

### Task 1.1: 创建 transformer 模块骨架

**Files:**
- Create: `src/components/transformer/mod.rs`

- [ ] **Step 1: 写最小骨架**

```rust
// src/components/transformer/mod.rs
//! Document transformers for RAG pre-processing.
//!
//! A `DocumentTransformer` takes documents and produces documents. The most
//! common use case is chunking long documents into smaller pieces suitable
//! for embedding and retrieval.

pub mod character_splitter;
pub mod markdown_header_splitter;
pub mod recursive_character_splitter;

pub use character_splitter::CharacterTextSplitter;
pub use markdown_header_splitter::MarkdownHeaderSplitter;
pub use recursive_character_splitter::RecursiveCharacterTextSplitter;

use async_trait::async_trait;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;

#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>>;
    fn name(&self) -> &str;
}
```

- [ ] **Step 2: 创建占位子模块文件**

Create `src/components/transformer/character_splitter.rs`, `recursive_character_splitter.rs`, `markdown_header_splitter.rs` with single line `// placeholder` each.

- [ ] **Step 3: 编译验证**

Run: `cargo build 2>&1 | tail -3`
Expected: success

- [ ] **Step 4: 添加到 src/components/mod.rs**

Add `pub mod transformer;` to `src/components/mod.rs`.

- [ ] **Step 5: 编译**

Run: `cargo build 2>&1 | tail -3`
Expected: success

- [ ] **Step 6: 提交**

```bash
git add src/components/transformer/ src/components/mod.rs
git commit -m "feat(transformer): scaffold DocumentTransformer trait module"
```

### Task 1.2: CharacterTextSplitter 实现 + 4 unit tests

**Files:**
- Modify: `src/components/transformer/character_splitter.rs`
- Create: `tests/character_splitter_test.rs`

- [ ] **Step 1: 写测试 — 基本 case**

```rust
// tests/character_splitter_test.rs
use std::collections::HashMap;
use synapse::components::retriever::Document;
use synapse::components::transformer::{DocumentTransformer, CharacterTextSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_character_splitter_short_text_no_split() {
    let splitter = CharacterTextSplitter::new(1000, 200);
    let doc = make_doc("doc1", "Short text");
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Short text");
}

#[tokio::test]
async fn test_character_splitter_long_text_splits() {
    let splitter = CharacterTextSplitter::new(50, 10);
    let long = "a".repeat(200);
    let doc = make_doc("doc1", &long);
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert!(chunks.len() > 1);
    for chunk in &chunks {
        assert!(chunk.content.len() <= 50);
    }
}

#[tokio::test]
async fn test_character_splitter_metadata_preserved() {
    let splitter = CharacterTextSplitter::new(50, 10);
    let mut metadata = HashMap::new();
    metadata.insert("source".into(), json!("test"));
    let doc = Document {
        id: "doc1".into(),
        content: "a".repeat(200),
        metadata,
    };
    let chunks = splitter.transform(vec![doc]).await.unwrap();
    assert!(chunks.len() > 1);
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.metadata.get("source").and_then(|v| v.as_str()), Some("test"));
        assert_eq!(chunk.metadata.get("source_id").and_then(|v| v.as_str()), Some("doc1"));
        assert_eq!(chunk.metadata.get("chunk_index").and_then(|v| v.as_u64()), Some(i as u64));
        assert_eq!(chunk.id, format!("doc1_{}", i));
    }
}

#[tokio::test]
async fn test_character_splitter_name() {
    let splitter = CharacterTextSplitter::new(100, 10);
    assert_eq!(splitter.name(), "CharacterTextSplitter");
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test character_splitter_test 2>&1 | tail -5`
Expected: FAIL — CharacterTextSplitter not found

- [ ] **Step 3: 实现 CharacterTextSplitter**

```rust
// src/components/transformer/character_splitter.rs
use async_trait::async_trait;
use std::collections::HashMap;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;
use super::DocumentTransformer;

pub struct CharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separator: String,
}

impl CharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separator: "\n".to_string(),
        }
    }
}

#[async_trait]
impl DocumentTransformer for CharacterTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = split_by_char(&doc.content, self.chunk_size, self.chunk_overlap, &self.separator);
            for (i, chunk_text) in chunks.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                metadata.insert("source_id".into(), serde_json::json!(doc.id));
                metadata.insert("chunk_index".into(), serde_json::json!(i));
                result.push(Document {
                    id: format!("{}_{}", doc.id, i),
                    content: chunk_text,
                    metadata,
                });
            }
        }
        Ok(result)
    }

    fn name(&self) -> &str { "CharacterTextSplitter" }
}

fn split_by_char(text: &str, chunk_size: usize, chunk_overlap: usize, sep: &str) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let char_len = chars.len();
    let mut start = 0;

    while start < char_len {
        let end = (start + chunk_size).min(char_len);
        let mut chunk: String = chars[start..end].iter().collect();

        if end < char_len {
            if let Some(last_sep_pos) = chunk.rfind(sep) {
                if last_sep_pos > chunk_size / 2 {
                    chunk.truncate(last_sep_pos + sep.len());
                }
            }
        }

        chunks.push(chunk);

        if end == char_len { break; }
        start = end.saturating_sub(chunk_overlap);
    }

    chunks
}
```

- [ ] **Step 4: 编译 + 运行测试**

Run: `cargo test --test character_splitter_test 2>&1 | tail -10`
Expected: 4 tests pass

- [ ] **Step 5: 提交**

```bash
git add src/components/transformer/character_splitter.rs tests/character_splitter_test.rs
git commit -m "feat(transformer): CharacterTextSplitter with overlap"
```

---

## Chunk 2: RecursiveCharacterTextSplitter

**目标：** 通用 chunker，按分隔符递归切分。+ 3 unit tests。

### Task 2.1: RecursiveCharacterTextSplitter 实现

**Files:**
- Modify: `src/components/transformer/recursive_character_splitter.rs`
- Modify: `tests/character_splitter_test.rs` (add new tests) or create `tests/recursive_character_splitter_test.rs`

- [ ] **Step 1: 写测试**

```rust
// tests/recursive_character_splitter_test.rs
use std::collections::HashMap;
use synapse::components::retriever::Document;
use synapse::components::transformer::{DocumentTransformer, RecursiveCharacterTextSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_recursive_splitter_splits_by_paragraph() {
    let splitter = RecursiveCharacterTextSplitter::new(100, 20);
    let text = "Paragraph 1.\n\nParagraph 2.\n\nParagraph 3.";
    let chunks = splitter.transform(vec![make_doc("doc1", text)]).await.unwrap();
    assert!(chunks.len() >= 2, "expected multiple chunks, got {}", chunks.len());
    for chunk in &chunks {
        assert!(chunk.content.len() <= 100);
    }
}

#[tokio::test]
async fn test_recursive_splitter_recursive_fallback() {
    let splitter = RecursiveCharacterTextSplitter::new(20, 5);
    let text = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj kkkk llll";
    let chunks = splitter.transform(vec![make_doc("doc1", text)]).await.unwrap();
    assert!(chunks.len() > 1, "expected multiple chunks for one paragraph");
    for chunk in &chunks {
        assert!(chunk.content.len() <= 20);
    }
}

#[tokio::test]
async fn test_recursive_splitter_name() {
    let splitter = RecursiveCharacterTextSplitter::new(100, 10);
    assert_eq!(splitter.name(), "RecursiveCharacterTextSplitter");
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test recursive_character_splitter_test 2>&1 | tail -5`
Expected: FAIL — RecursiveCharacterTextSplitter not found

- [ ] **Step 3: 实现**

```rust
// src/components/transformer/recursive_character_splitter.rs
use async_trait::async_trait;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;
use super::DocumentTransformer;

pub struct RecursiveCharacterTextSplitter {
    chunk_size: usize,
    chunk_overlap: usize,
    separators: Vec<String>,
}

impl RecursiveCharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            separators: vec!["\n\n".into(), "\n".into(), " ".into(), "".into()],
        }
    }
}

#[async_trait]
impl DocumentTransformer for RecursiveCharacterTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = recursive_split(&doc.content, self.chunk_size, self.chunk_overlap, &self.separators);
            for (i, chunk_text) in chunks.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                metadata.insert("source_id".into(), serde_json::json!(doc.id));
                metadata.insert("chunk_index".into(), serde_json::json!(i));
                result.push(Document {
                    id: format!("{}_{}", doc.id, i),
                    content: chunk_text,
                    metadata,
                });
            }
        }
        Ok(result)
    }

    fn name(&self) -> &str { "RecursiveCharacterTextSplitter" }
}

fn recursive_split(
    text: &str,
    chunk_size: usize,
    _chunk_overlap: usize,
    separators: &[String],
) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }

    let sep = &separators[0];
    let remaining = &separators[1..];

    if sep.is_empty() {
        return hard_split(text, chunk_size);
    }

    let pieces: Vec<String> = text.split(sep).enumerate().map(|(i, p)| {
        if i == 0 { p.to_string() } else { format!("{}{}", sep, p) }
    }).collect();

    let mut chunks = Vec::new();
    let mut current = String::new();

    for piece in pieces {
        if current.len() + piece.len() > chunk_size && !current.is_empty() {
            chunks.push(current);
            current = piece;
        } else {
            current.push_str(&piece);
        }
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    // Recursively re-split any chunks that are still too large
    let final_chunks: Vec<String> = chunks
        .into_iter()
        .flat_map(|c| {
            if c.len() > chunk_size && !remaining.is_empty() {
                recursive_split(&c, chunk_size, _chunk_overlap, remaining)
            } else if c.len() > chunk_size {
                hard_split(&c, chunk_size)
            } else {
                vec![c]
            }
        })
        .collect();

    final_chunks
}

fn hard_split(text: &str, chunk_size: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(chunk_size)
        .map(|c| c.iter().collect())
        .collect()
}
```

- [ ] **Step 4: 编译 + 运行测试**

Run: `cargo test --test recursive_character_splitter_test 2>&1 | tail -10`
Expected: 3 tests pass

- [ ] **Step 5: 提交**

```bash
git add src/components/transformer/recursive_character_splitter.rs tests/recursive_character_splitter_test.rs
git commit -m "feat(transformer): RecursiveCharacterTextSplitter with fallback"
```

---

## Chunk 3: MarkdownHeaderSplitter

**目标：** 按 markdown header 切分，header 进入 metadata。+ 2 unit tests。

### Task 3.1: MarkdownHeaderSplitter 实现

**Files:**
- Modify: `src/components/transformer/markdown_header_splitter.rs`
- Create: `tests/markdown_header_splitter_test.rs`

- [ ] **Step 1: 写测试**

```rust
// tests/markdown_header_splitter_test.rs
use std::collections::HashMap;
use synapse::components::retriever::Document;
use synapse::components::transformer::{DocumentTransformer, MarkdownHeaderSplitter};

fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn test_markdown_splitter_basic() {
    let splitter = MarkdownHeaderSplitter::new();
    let md = "# Header 1\nFirst content.\n## Header 2\nSecond content.\n# Header 3\nThird content.";
    let chunks = splitter.transform(vec![make_doc("doc1", md)]).await.unwrap();
    assert_eq!(chunks.len(), 3, "expected 3 chunks, got {}: {:?}", chunks.len(), chunks);

    assert_eq!(chunks[0].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 1"));
    assert!(chunks[0].content.contains("First content"));

    assert_eq!(chunks[1].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 1"));
    assert_eq!(chunks[1].metadata.get("h2").and_then(|v| v.as_str()), Some("Header 2"));

    assert_eq!(chunks[2].metadata.get("h1").and_then(|v| v.as_str()), Some("Header 3"));
}

#[tokio::test]
async fn test_markdown_splitter_name() {
    let splitter = MarkdownHeaderSplitter::new();
    assert_eq!(splitter.name(), "MarkdownHeaderSplitter");
}
```

- [ ] **Step 2: 验证测试失败**

Run: `cargo test --test markdown_header_splitter_test 2>&1 | tail -5`
Expected: FAIL — MarkdownHeaderSplitter not found

- [ ] **Step 3: 实现**

```rust
// src/components/transformer/markdown_header_splitter.rs
use async_trait::async_trait;
use std::collections::HashMap;

use crate::components::document::DocumentResult;
use crate::components::retriever::Document;
use super::DocumentTransformer;

pub struct MarkdownHeaderSplitter {
    header_levels: Vec<String>,
}

impl MarkdownHeaderSplitter {
    pub fn new() -> Self {
        Self {
            header_levels: vec![
                "#".into(),
                "##".into(),
                "###".into(),
                "####".into(),
                "#####".into(),
                "######".into(),
            ],
        }
    }
}

impl Default for MarkdownHeaderSplitter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentTransformer for MarkdownHeaderSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let segments = split_markdown(&doc.content, &self.header_levels);
            for (i, (headers, content)) in segments.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                for (level, text) in &headers {
                    metadata.insert(level.clone(), serde_json::json!(text));
                }
                metadata.insert("source_id".into(), serde_json::json!(doc.id));
                metadata.insert("chunk_index".into(), serde_json::json!(i));
                result.push(Document {
                    id: format!("{}_{}", doc.id, i),
                    content,
                    metadata,
                });
            }
        }
        Ok(result)
    }

    fn name(&self) -> &str { "MarkdownHeaderSplitter" }
}

fn split_markdown(text: &str, header_levels: &[String]) -> Vec<(HashMap<String, String>, String)> {
    let mut chunks: Vec<(HashMap<String, String>, String)> = Vec::new();
    let mut current_headers: HashMap<String, String> = HashMap::new();
    let mut current_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        let matched_level = header_levels.iter().find(|h| {
            let prefix = format!("{} ", h);
            trimmed.starts_with(&prefix)
        });

        match matched_level {
            Some(level) => {
                if !current_content.is_empty() || !current_headers.is_empty() {
                    chunks.push((current_headers.clone(), current_content.clone()));
                    current_content.clear();
                }
                let header_text = trimmed
                    .trim_start_matches(&format!("{} ", level))
                    .trim()
                    .to_string();
                current_headers.insert(level.clone(), header_text);
            }
            None => {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
    }

    if !current_content.is_empty() || !current_headers.is_empty() {
        chunks.push((current_headers, current_content));
    }

    chunks
}
```

- [ ] **Step 4: 编译 + 运行测试**

Run: `cargo test --test markdown_header_splitter_test 2>&1 | tail -10`
Expected: 2 tests pass

- [ ] **Step 5: 提交**

```bash
git add src/components/transformer/markdown_header_splitter.rs tests/markdown_header_splitter_test.rs
git commit -m "feat(transformer): MarkdownHeaderSplitter with header path metadata"
```

---

## Chunk 4: 整合测试 + 合并

**目标：** 跑全套测试，整合到 components mod，合并到 master。

### Task 4.1: 整合测试

- [ ] **Step 1: 跑所有测试**

Run: `cargo test 2>&1 | grep "test result" | awk '{p+=$4; f+=$6} END {print "Total passed:", p, "failed:", f}'`
Expected: 99 baseline + 9 new = 108 tests

- [ ] **Step 2: 修复任何 regression**

### Task 4.2: 验证导出

- [ ] **Step 1: 检查 src/components/mod.rs 已包含 transformer**

```bash
grep transformer https://github.com/iBLOC/synapse/src/components/mod.rs
```
Expected: `pub mod transformer;` 存在

- [ ] **Step 2: 编译 release 模式**

Run: `cargo build --release 2>&1 | tail -3`
Expected: success

### Task 4.3: 合并到 master

- [ ] **Step 1: 切换 master 并 merge**

```bash
git checkout master
git merge feature/document-transformer --no-ff -m "Merge feature/document-transformer: chunking transformers

DocumentTransformer trait + 3 chunkers:
- CharacterTextSplitter (fixed-size + overlap)
- RecursiveCharacterTextSplitter (recursive separator fallback)
- MarkdownHeaderSplitter (header-based, headers in metadata)

Each chunker produces chunks with:
- source_id metadata (traces back to source doc)
- chunk_index metadata (sequence in source)
- id: {source_id}_{chunk_index}

9 new tests. 108 total (no regression)."
```

- [ ] **Step 2: 最终验证**

Run: `cargo test 2>&1 | tail -5`
Expected: 108 tests pass

- [ ] **Step 3: 清理 worktree**

```bash
git worktree remove .worktrees/transformer
git branch -d feature/document-transformer
```

---

## 风险与回退

| 风险 | 缓解 |
|------|------|
| Recursive splitter 算法 bug（边界条件） | 3 个 test cases 覆盖主要路径 |
| Markdown header 解析不健壮 | 标准化 `"{level} "` 前缀匹配 |
| Unicode char 计数（按 char 不 byte） | 显式 `.chars().collect()` |
| 两个 Document 类型并存造成混淆 | spec 文档说明 + doc comment |

## 完成标准

- [ ] DocumentTransformer trait 定义清晰
- [ ] CharacterTextSplitter 4 unit tests pass
- [ ] RecursiveCharacterTextSplitter 3 unit tests pass
- [ ] MarkdownHeaderSplitter 2 unit tests pass
- [ ] 全部 108 个测试 pass
- [ ] cargo build --release 成功
- [ ] 合并到 master
