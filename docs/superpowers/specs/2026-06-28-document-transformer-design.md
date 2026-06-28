# Synapse Document Transformer (Chunking) 设计 Spec

> **作者：** Synapse 架构组
> **状态：** 草案
> **日期：** 2026-06-28
> **关联项目：** 神经突触 Synapse（本项目）

## 背景与目标

### 现状

Synapse 已有以下 RAG 基础设施：
- `Document` (src/components/retriever/mod.rs) — RAG 文档模型 (id, content, metadata)
- `Document` (src/components/document/mod.rs) — 原始文档模型 (id, content, mime_type, metadata) + `DocumentParser` trait
- `Embedding` (OpenAI) + `Indexer` (InMemory) + `Retriever` (InMemory)
- `InMemoryIndexer::index(documents)` + `index_from_text(texts)`

### 缺失

1. **没有 DocumentTransformer 抽象** — 用户拿到 parsed document 后无法预处理
2. **没有 chunker** — 长文档（如整本书）不能被切分成 embedding 大小合适的片段
3. **RAG 实战不可用** — `index_from_text` 把整段 text 作为单个 embedding 输入，会导致：
   - 超出 embedding 模型 token 限制
   - 检索粒度太粗，召回不精准
4. **现有架构隐性问题** — `document::Document` 和 `retriever::Document` 两个 Document 类型并存

### 目标

实现 Document Transformer 抽象 + 3 个常用 chunker：
- **CharacterTextSplitter** — 固定大小 + overlap（最简单）
- **RecursiveCharacterTextSplitter** — 递归分隔符切分（最通用）
- **MarkdownHeaderSplitter** — 按 markdown 标题切分（适合文档 RAG）

## 设计

### 架构

```
┌────────────────────────────────────────────────────────────┐
│  bytes + mime_type                                         │
│     │                                                      │
│     ▼                                                      │
│  DocumentParser (existing)                                 │
│     │ returns ParsedDocument (content + metadata)          │
│     ▼                                                      │
│  User: wrap as retriever::Document (id, content, metadata) │
│     │                                                      │
│     ▼                                                      │
│  ┌──────────────────────────────────────────────┐          │
│  │ DocumentTransformer trait (NEW)              │          │
│  │   async fn transform(docs) -> Vec<Document>  │          │
│  │   fn name() -> &str                          │          │
│  └──────────────────────────────────────────────┘          │
│     │                                                      │
│     ├── CharacterTextSplitter (NEW)                        │
│     │      chunk_size: usize, chunk_overlap: usize         │
│     ├── RecursiveCharacterTextSplitter (NEW)              │
│     │      chunk_size, chunk_overlap, separators: Vec<Str> │
│     └── MarkdownHeaderSplitter (NEW)                       │
│            splits by # / ## / ###, header becomes metadata │
│                                                              │
│     ▼                                                      │
│  Vec<Document> (chunks)                                    │
│     │                                                      │
│     ▼                                                      │
│  Indexer.index(documents) (existing)                       │
└────────────────────────────────────────────────────────────┘
```

### 核心设计决策

#### 1. 用哪个 Document 类型？

**决策**：transformer 用 `retriever::Document`（id, content, metadata）。

**理由**：
- `indexer::index(documents: Vec<Document>)` 已经接受 `retriever::Document`
- transformer 输出直接喂给 indexer，**类型一致无需转换**
- mime_type 信息可以在 parse → Document 转换时存到 metadata（如 `metadata["source_mime_type"]`）

#### 2. DocumentTransformer trait 设计

```rust
#[async_trait]
pub trait DocumentTransformer: Send + Sync {
    /// Transform a batch of documents into a new batch.
    /// Common case: 1 doc → N chunks. But transformers can also be 1:1
    /// (cleaning, translation) or N:M (merging).
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>>;

    /// Human-readable name (e.g., "CharacterTextSplitter").
    fn name(&self) -> &str;
}
```

**理由**：
- `async` 因为未来可能加 LLM-based transformer（summarization-based chunking）
- `Send + Sync` 让 transformer 可在 Arc 中共享（与 Embedding 一致）
- `Vec<Document>` 输入输出对齐 RAG pipeline 风格

#### 3. Chunk 命名与元数据

每个 chunk 自动加 metadata：

| 字段 | 类型 | 说明 |
|------|------|------|
| `source_id` | String | 原始 Document 的 id（用于追溯） |
| `chunk_index` | u64 | chunk 在源文档中的序号（从 0 开始） |
| `total_chunks` | u64 | 源文档总 chunk 数（split 完成后回填，详见实现） |

**注**：`total_chunks` 需要先 transform 完才知道，所以实现策略：
- 第一次 transform 完拿到所有 chunks，再回填 `total_chunks` 给每个 chunk

**或简化**：只填 `source_id` + `chunk_index`，不填 `total_chunks`。用户需要时可自己加。

**决策**：先实现 `source_id` + `chunk_index` 即可，`total_chunks` 作为后续增强。

### 组件

#### CharacterTextSplitter

**行为**：按字符数切分，固定大小 + overlap。

```rust
pub struct CharacterTextSplitter {
    chunk_size: usize,       // 默认 1000
    chunk_overlap: usize,    // 默认 200
    separator: String,       // 默认 "\n"
}

impl CharacterTextSplitter {
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self;
}

#[async_trait]
impl DocumentTransformer for CharacterTextSplitter {
    async fn transform(&self, documents: Vec<Document>) -> DocumentResult<Vec<Document>> {
        let mut result = Vec::new();
        for doc in documents {
            let chunks = split_by_char(&doc.content, self.chunk_size, self.chunk_overlap, &self.separator);
            for (i, chunk_text) in chunks.into_iter().enumerate() {
                let mut metadata = doc.metadata.clone();
                metadata.insert("source_id".into(), json!(doc.id));
                metadata.insert("chunk_index".into(), json!(i));
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
```

**核心算法** `split_by_char(text, size, overlap, sep)`:
1. 找所有 separator 位置
2. 累积 separator-delimited segments 直到累计长度 ≥ size
3. 输出 segment + 保留最后 overlap 字符
4. 循环

简化版（先实现最直接版本）：
1. 从 `text.start` 开始，截取 `chunk_size` 字符
2. 如包含 separator：在最后一个 separator 处断
3. 否则硬切
4. 下一 chunk 从 `chunk_size - overlap` 开始

#### RecursiveCharacterTextSplitter

**行为**：先按 `\n\n` 切，得到的 segment 如果还太长，再按 `\n` 切，依次递归到 `""`（单字符）。

```rust
pub struct RecursiveCharacterTextSplitter {
    chunk_size: usize,                // 默认 1000
    chunk_overlap: usize,             // 默认 200
    separators: Vec<String>,          // 默认 ["\n\n", "\n", " ", ""]
}

impl Default for RecursiveCharacterTextSplitter {
    fn default() -> Self {
        Self::new(1000, 200)
    }
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
```

**核心算法** `recursive_split(text, chunk_size, separators)`:
1. 选第一个 separator，找所有出现位置
2. 累积 separator-delimited segments 直到累计 ≥ chunk_size
3. 输出 segments
4. 对每个还 > chunk_size 的 segment，用下一个 separator 递归
5. 直到所有 segments ≤ chunk_size 或 separator 列表用完（最后硬切）

**简化版实现**：用循环 + 优先匹配大粒度 separator：
- 优先级：`\n\n` > `\n` > ` ` > `""`
- 一次扫描找到所有匹配的最大粒度
- 然后每个 segment 检查是否 ≤ chunk_size
- 对 > chunk_size 的，递归用下一粒度切

#### MarkdownHeaderSplitter

**行为**：按 markdown header（`# `, `## `, `### ` 等）切分，每个 header 成为 chunk 的开始。Header 文本作为 metadata。

```rust
pub struct MarkdownHeaderSplitter {
    headers_to_split_on: Vec<String>,  // 默认 ["#", "##", "###", "####", "#####", "######"]
}
```

**输出 metadata**：
- `h1`, `h2`, `h3` ... 当前 chunk 路径上各级 header 文本
- `source_id` + `chunk_index` (与上面一致)

**核心算法** `split_by_markdown(text)`:
1. 找所有 header 行（以 `# `, `## ` 等开头）
2. 把 text 切分成 (header_text, content) pairs
3. 维护当前 header 路径（h1, h2, h3...）
4. 每个 chunk 携带 header 路径作为 metadata

### 文件结构

```
src/components/transformer/
├── mod.rs                              # DocumentTransformer trait + DocumentResult + DocumentError
├── character_splitter.rs               # CharacterTextSplitter
├── recursive_character_splitter.rs     # RecursiveCharacterTextSplitter
└── markdown_header_splitter.rs         # MarkdownHeaderSplitter

tests/transformer_test.rs               # 集成测试
```

**修改文件**：
- `src/components/mod.rs` — 添加 `pub mod transformer;`
- `src/lib.rs` (如果有) — export transformer module

### 公开 API

```rust
// src/components/transformer/mod.rs
pub use character_splitter::CharacterTextSplitter;
pub use recursive_character_splitter::RecursiveCharacterTextSplitter;
pub use markdown_header_splitter::MarkdownHeaderSplitter;

// 用户使用
use synapse::components::transformer::{DocumentTransformer, CharacterTextSplitter};

let splitter = CharacterTextSplitter::new(1000, 200);
let chunks: Vec<Document> = splitter.transform(parsed_docs).await?;
indexer.index(chunks).await?;

// 链式
let splitter = RecursiveCharacterTextSplitter::default();
let chunks = splitter.transform(docs).await?;
let indexer = InMemoryIndexer::new(store, embedder);
indexer.index(chunks).await?;
```

### 测试

#### 测试矩阵

| # | 测试名 | 场景 | 验证点 |
|---|--------|------|--------|
| 1 | `test_character_splitter_basic` | 短文本，不需切分 | 1 个 chunk |
| 2 | `test_character_splitter_long_text` | 长文本（> chunk_size） | 多个 chunk，size ≤ chunk_size |
| 3 | `test_character_splitter_overlap` | 长文本带 overlap | 相邻 chunk 有 overlap 字符 |
| 4 | `test_character_splitter_metadata` | 多 chunk 同一 source | metadata 正确（source_id, chunk_index） |
| 5 | `test_recursive_splitter_paragraphs` | 段落分隔优先 | 按 `\n\n` 切，保留段落 |
| 6 | `test_recursive_splitter_recursive` | 一个超大段落 | 递归到 `\n` / ` ` 切 |
| 7 | `test_recursive_splitter_preserves_separators` | 验证分隔符保留 | 切完的 chunk 包含原始 `\n\n` 边界 |
| 8 | `test_markdown_splitter_basic` | 简单 markdown | 按 `#` / `##` 切，header 进入 metadata |
| 9 | `test_markdown_splitter_header_path` | 多级 header | metadata 包含 h1, h2, h3 路径 |
| 10 | `test_transformer_name` | name() 方法 | 每个 splitter 返回正确的 name |

#### Mock 文档 helper

```rust
fn make_doc(id: &str, content: &str) -> Document {
    Document {
        id: id.into(),
        content: content.into(),
        metadata: HashMap::new(),
    }
}
```

### 关键算法细节

#### CharacterTextSplitter.split_by_char

```rust
fn split_by_char(text: &str, chunk_size: usize, chunk_overlap: usize, sep: &str) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }
    
    let mut chunks = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    let char_len = chars.len();
    
    while start < char_len {
        let end = (start + chunk_size).min(char_len);
        let mut chunk: String = chars[start..end].iter().collect();
        
        // 如果不是最后一块，尝试在 separator 处断
        if end < char_len {
            if let Some(last_sep_pos) = chunk.rfind(sep) {
                if last_sep_pos > chunk_size / 2 {  // 至少保留一半
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

#### RecursiveCharacterTextSplitter 简化实现

```rust
fn recursive_split(text: &str, chunk_size: usize, separators: &[String]) -> Vec<String> {
    if text.len() <= chunk_size {
        return vec![text.to_string()];
    }
    
    let sep = &separators[0];
    let remaining_seps = &separators[1..];
    
    if sep.is_empty() {
        // 最后 fallback：硬切
        return hard_split(text, chunk_size);
    }
    
    // 按 separator 切分
    let parts: Vec<&str> = text.split(sep).collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    
    for (i, part) in parts.iter().enumerate() {
        let piece = if i < parts.len() - 1 {
            format!("{}{}", part, sep)
        } else {
            part.to_string()
        };
        
        if current.len() + piece.len() > chunk_size && !current.is_empty() {
            // current 满了，输出
            if current.len() > chunk_size && !remaining_seps.is_empty() {
                // current 仍超长，递归
                chunks.extend(recursive_split(&current, chunk_size, remaining_seps));
            } else {
                chunks.push(current);
            }
            current = piece;
        } else {
            current.push_str(&piece);
        }
    }
    
    if !current.is_empty() {
        chunks.push(current);
    }
    
    chunks
}

fn hard_split(text: &str, chunk_size: usize) -> Vec<String> {
    text.chars()
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .map(|c| c.iter().collect::<String>())
        .collect()
}
```

#### MarkdownHeaderSplitter 简化实现

```rust
fn split_markdown(text: &str, header_levels: &[String]) -> Vec<(HashMap<String, String>, String)> {
    let mut chunks = Vec::new();
    let mut current_headers: HashMap<String, String> = HashMap::new();
    let mut current_content = String::new();
    
    for line in text.lines() {
        let trimmed = line.trim_start();
        let level = header_levels.iter().find(|h| trimmed.starts_with(&format!("{} ", h)));
        
        match level {
            Some(h) => {
                // 输出之前的 chunk
                if !current_content.is_empty() {
                    chunks.push((current_headers.clone(), current_content.clone()));
                    current_content.clear();
                }
                // 更新 header
                let header_text = trimmed.trim_start_matches(&format!("{} ", h)).trim().to_string();
                current_headers.insert(h.clone(), header_text);
            }
            None => {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }
    }
    
    if !current_content.is_empty() {
        chunks.push((current_headers, current_content));
    }
    
    chunks
}
```

### 与现有代码的整合

| 现有 | 关系 | 说明 |
|------|------|------|
| `retriever::Document` | **复用** | transformer 输入输出 |
| `Indexer::index(documents)` | **复用** | chunks 直接喂入 |
| `Embedding` | 不变 | 与 transformer 独立 |
| `DocumentParser` | 共存 | parse 后 wrap 为 `retriever::Document` |
| `document::Document` | 共存 | 保留，两个 Document 类型的合并是后续 refactor |

### 风险与权衡

| 风险 | 影响 | 缓解 |
|------|------|------|
| 两个 Document 类型并存造成困惑 | 用户不知道用哪个 | doc comment 明确说明 + 用例展示 |
| RecursiveCharacterTextSplitter 复杂算法 bug | 测试覆盖核心路径 | 6 个 test cases + edge cases |
| Unicode 字符数计算 | chunk_size 按 char 不按 byte | 显式使用 `.chars().collect()` |
| Markdown header 解析不健壮 | 不支持 `##` 后面有空格/无空格 | 标准化 header 前缀格式 |

### 实施分解

| Step | 内容 | 估时 |
|------|------|------|
| 1 | 创建 `src/components/transformer/mod.rs` + trait | 10min |
| 2 | CharacterTextSplitter + 4 unit tests | 20min |
| 3 | RecursiveCharacterTextSplitter + 3 unit tests | 30min |
| 4 | MarkdownHeaderSplitter + 2 unit tests | 20min |
| 5 | 集成到 `src/components/mod.rs` | 5min |
| 6 | 1 个 transformer_name integration test | 5min |
| 7 | 跑全套测试 + 修 edge cases | 20min |
| 8 | commit + merge | 10min |
| **总计** | | **~2h** |

## 后续工作（不在本 spec 范围）

- **统一 Document 类型**（合并 `document::Document` 和 `retriever::Document`）
- **TokenTextSplitter**（需要 tokenizer 依赖）
- **CodeSplitter**（按 function/class 切分代码）
- **HTMLSplitter**（按 HTML 标签切分）
- **Embedding-aware splitter**（基于 embedding 相似度的语义切分）
