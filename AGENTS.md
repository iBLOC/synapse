**Updated**: 2026-11-18 (**「叶区入口升级成「双形」** —— 端脑的 JSON-RPC 信封与旧的扁平报文**在同一个入口**都被认；顺带把「注册成功、请求 404」那笔债还掉」** —— 承 父仓 U+37B2 条（线上接线面那一刀）＋ 用户「继续」，本条记**这一侧落了什么**、**判据**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-18 | authority=measured | scope=project | source=承 父仓 U+37B8 条（线上接线面那一刀）＋ 用户「继续」 ｜ 改动：synapse（**改** `src/lobe/a2a_server.rs`（**双形入口**：两套报文解成一份 ＋ 身份注入 ＋ 按来者的形状回话 ＋ send 挂三处路径）＋ **改** `src/lobe/mod.rs`（导出）＋ **新** `tests/lobe_a2a_wire.rs`（判据放在**集成测试**里 ⇒ ⇒ 从 crate 外面打线））｜ 读数：synapse 线形状测试 **8/8**（含**真回环**那一条）＋ 本体闸 **12/12**（自检 **18/18**）＋ 端脑 lib 编译**零 error** ＋ 顶叶单测 **100/100** | status=active | supersedes=none

① **✅ 入口从「一种形状」变成「两种形状」** —— 端脑发的是 **v0.2 JSON-RPC 信封**（`jsonrpc` ／ `method` 为 `tasks/send` ／ `params` 里才是 `message`），而叶区的 skill 是照 **v0.1 扁平面**（`skill` 与 `message` 平级）写的 ⇒ ⇒ 只认一种 ⇒ ⇒ 另一种到了入口就是「skill 读不到（4001）＋ ctx 是空的 ⇒ ⇒ 报 missing field `user_id`」⇒ ⇒ 看着像**对面叶区坏了**；⇒ ⇒ 现在 `parse_send_request` 把两套报文解成**同一份** `SendRequest` ⇒ ⇒ **谁都不用改自己的线**。

② **✅ 身份不许编** —— `JsonCtx` 的 `user_id` 是必填，而端脑把它打包进 `metadata.name` ⇒ ⇒ 入口**解包**那份并把 `user_id` **注入**上下文 ⇒ ⇒ 一份请求**真的没有**身份 ⇒ ⇒ **报错 4002**，不编一个 ⇒ ⇒ 编一个的后果是「所有人的记忆混进一个桶」。

③ **✅ 回话按来者的形状** —— JSON-RPC 来 ⇒ ⇒ 回 `{jsonrpc,id,result:{task:{status:{message}}}}`（`status.message` 必须是**字符串** ⇒ ⇒ 对面是按字符串读的 ⇒ ⇒ 给对象就是「调用成功、回话却是空的」）；旧的扁平来 ⇒ ⇒ **仍是裸 payload**（老调用方是这么读的）⇒ ⇒ 不破坏。

④ **✅ send 挂在三处路径** —— `/a2a/tasks/send`（规范面）＋ **`/api/a2a`**（叶区注册地址用的就是它）＋ **`/`**（卡片公布的基址）⇒ ⇒ 端脑 POST 到**注册地址**（它不追加路径）⇒ ⇒ 注册地址必须**真是**一个端点 ⇒ ⇒ 否则「注册成功、请求 404」。

⑤ **⚠️ 顺手订正一句假话** —— 文件头原声称「Wraps `synapse::a2a::server::A2AServerBuilder`」⇒ ⇒ 实现里**根本没有**它 ⇒ ⇒ 改掉了（**注释在骗人**）。

⑥ **⚠️ 读数** —— synapse 线形状测试 **8/8**（含**真回环**那一条：JSON-RPC 信封 ⇒ ⇒ skill 真跑 ⇒ ⇒ 附件真到 ⇒ ⇒ 身份真注入）＋ 本体闸 **12/12**（自检 **18/18**）＋ 端脑 lib 编译**零 error** ＋ 顶叶单测 **100/100**（+4）＋ 两道真文件上的真变异各恰好 O12 一条红、还原后 sha256 逐字一致。

⑦ **⚠️ 仍然欠着的** —— ❶ **真机那一格仍空**（真手机 ⇒ ⇒ 真表格 ⇒ ⇒ 真海马体三元组）⇒ ⇒ 本刀证据是**回环级 ＋ 单测级** ⇒ ⇒ **不许说「真机已验证」**；❷ **未提交**（用户本轮未要求；且本仓工作树有兄弟会话的大量在飞面 ⇒ ⇒ 落链时必须按路径逐个 add，不许扫全量）。

⑧ **⚠ 圈码**：本仓的台本**不占码位**（它是「引用别本」的那种账）⇒ ⇒ 本条**不占**；父仓那条占 U+37B8、端脑那条占 U+37BA。

**Updated**: 2026-11-17 (**「分发链最后两格：**升级**不必再改代码（摘要从代码变成可更新的数据）＋ **崩溃自愈**」** —— 承影仓 U+3565 条（配置面与启动命令）那族，本条记**取证**、**三处落点**、**判据与变异**、**真机演练**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-17 | authority=measured | scope=project | source=影仓 U+3565 条（配置面与启动命令）与影仓 U+354A 条（参数对齐）｜ 改动：`src/tools/web/engine_binary.rs`（加 `DigestManifest` ＋ `resolve_expected_digest` ＋ `verify_artifact_with`）＋ `src/tools/web/tests.rs`（判据 13 条）＋ `examples/verify_web_engine_artifact.rs`（增 `--manifest`／`--asset`／`--emit-manifest`）⇒ ⇒ **3 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib tools::web` **49 过 / 0 红**（上一刀 36 ＋ 本刀 13）＋ 全量 ****490 过 / 0 红**** ＋ 变异三处**当场红**、复原 sha256 **逐字一致** ｜ 闸：本仓无脚本闸（闸在影仓） | status=active | supersedes=none

① **⚠️ 取证：摘要的官方出处是**机器可读**的，而且它**不是我们抄的** —— ❶ GitHub 的 **release API 逐资产带 `digest` 字段**（`sha256:…`）⇒ ⇒ 实测 v0.2.7 的 20 个资产 digest **与本仓内置钉表逐字一致** ⇒ ⇒ 钉表是对的，且不必再手抄。❷ 「换版本要整表换摘要」的根因：摘要表是**代码** ⇒ ⇒ 运维升级引擎**得等开发者**；官方已有 **10 个 release** ⇒ ⇒ 这个洞会反复被踩。❸ 本机 **release 下载直连不通**（curl：Connection was reset）⇒ ⇒ **API 通**（走系统代理）⇒ ⇒ 取摘要与下载是两条路。

② **✅ 落点 A：把摘要从「代码」变成「可更新的数据」（`engine_binary.rs`）** —— ❶ `DigestManifest{version, source_url, retrieved_at, assets[]}` ＋ `AssetDigest`（serde）＋ `from_builtin()`／`load()`／`to_json()`／`digest_of()`。❷ `resolve_expected_digest(asset, manifest)` —— **纯函数**，口径三条：锚有清单没有 ⇒ ⇒ **锚**；锚没有清单有 ⇒ ⇒ **清单**（升级那格）；两边都有 ⇒ ⇒ **看版本**（同版必须一致，不一致 ⇒ ⇒ **响亮报错**；不同版 ⇒ ⇒ **清单** —— 清单说的就是它自己那一版，锚说的是另一版 ⇒ ⇒ 混用会拿错摘要）。❸ `verify_artifact_with` 返回 `Verified{digest, source}` ⇒ ⇒ **出处是返回值**。❹ 清单里的 sha256 必须**合法 64 位十六进制** ⇒ ⇒ 手滑写错在**读的时候**就响亮地失败。

③ **✅ 落点 B：CLI（`examples/verify_web_engine_artifact.rs`）** —— `--manifest`／`--asset`／`--emit-manifest` ⇒ ⇒ 输出**说清用的是锚还是清单**；清单是别版时**提醒**版本差 ⇒ ⇒ 不让人以为「锚失效了」。

④ **✅ 判据与变异（新增 13 条）** —— ❶ 锚可导出／JSON 往返／同版一致⇒锚说了算。❷ 同版不一致⇒冲突报错／别版⇒清单接管／锚没听过的资产⇒清单接管／清单没这个资产⇒锚接管／两边都没有⇒拒。❸ 坏 JSON⇒响亮失败／没版本⇒拒／坏 hex⇒拒。❹ **承载级 1**：文件⇒摘要⇒比对⇒**并把出处说出来**；清单对不上⇒拒。❺ **变异三处，当场红**：①「别版 ⇒ 清单」改成「别版 ⇒ 锚」⇒ ⇒ **恰好那一个机制**在**两层**都红（纯函数层 ＋ 承载级校验层）；②「同版冲突报错」改成「静默取清单」⇒ ⇒ 恰好 1 条红；③ 去掉 hex 校验 ⇒ ⇒ 恰好 1 条红。❻ 复原后 **sha256 逐字一致** ＋ **推 mtime**。

⑤ **✅ 真机演练（L3）** —— ❶ 官方 API 取 v0.2.6 清单（20 个资产）⇒ windows 资产 digest＝`0a110d06…a6242`。❷ 真下 v0.2.6 的 zip（69,076,668 字节）⇒ sha256 **与官方 API 逐字一致**。❸ 用清单核它 ⇒ **OK**（source＝清单）；不用清单 ⇒ **FAIL**（锚拿 0.2.7 的摘要对不上）⇒ ⇒ **锚不误放行，清单才放行**。❹ 清单改成同版但摘要留别版 ⇒ ⇒ **digest conflict** ＋ 退出码 1。❺ 脚本拿对不上的包 ⇒ ⇒ **拒绝启动**（端口无监听 ⇒ ⇒ 引擎没起）。

⑥ **⚠️ 仍然欠着的** —— ❶ 清单**没有签名** ⇒ ⇒ 谁拿到写权限谁就能换清单 ⇒ ⇒ 「同版冲突报错」只挡住与锚矛盾的那一类 ⇒ ⇒ 要更强得给清单签名（没做）。❷ **自动下载**那一格仍没有（本机直连 github.com 不通 ⇒ ⇒ 得走代理 ⇒ ⇒ 脚本没做代理配置）。❸ 真 CI 待取（本仓未推远端）。
**Updated**: 2026-11-16 (**「厂商认领按**主机名**优先 ＋ 「上游没列的条目」按 id 算 —— 出厂清单不再被一次成功的刷新整份冲掉」** —— 承影仓 U+355C 条（MiniMax 出图模型消失那族），本条记**取证**、**落点**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=影仓 U+355C 条（MiniMax 出图模型消失那族）｜ 改动：`src/providers/catalog.rs`（`PresetDocList` ＋ `OFFICIAL_HOSTS` ＋ `base_host()` ＋ `preset_doc_list()` ＋ `preset_models_missing()` ＋ `ModelSummary.model_source`）⇒ ⇒ **1 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib providers::catalog` **30 过 / 0 红**（上一刀 26 ＋ 本刀 4）＋ 全量 **475 过 / 0 红** ｜ 闸：本仓无脚本闸（闸在影仓：**14 绿 / 8 红**，八道红**零命中本刀文件**；另有一道红命中**上一刀本仓自己写的**那个判据文件〔R5 替身空转，在影仓那一侧〕⇒ ⇒ **已认领并修掉**）| status=active | supersedes=none

① **⚠️ 取证：上游那份清单**天生**只列对话模型** —— ❶ 官方文档 `https://platform.minimaxi.com/docs/api-reference/models/openai/list-models.md` 的 `GET /v1/models` 响应示例**逐字只有两条**（`MiniMax-M3.1-Flash`／`MiniMax-M3`）⇒ ⇒ 出图／视频／语音模型**不在那个列表里**。❷ 官方「模型列表」页 `https://platform.minimaxi.com/docs/faq/models.md` 分四类：语言 2／视频 4／图片 3（**MiniMax-H3-Image**／image-01／image-01-live）／语音 6 ⇒ ⇒ 与本仓出厂清单那份**逐字一致**。❸ 本机存量 providers.json 里那条 MiniMax 的**上一次真实拉取**：**9 条全是语言模型** ⇒ ⇒ 与官方文档吻合。

② **✅ 落点（口径只留一份）** —— ❶ `PresetModelDoc` → `PresetDocList { preset_id, doc, models }`，`PRESET_MODELS` → `PRESET_DOC_LISTS`（文档来源跟清单**绑在一处** ⇒ ⇒ 换清单不必改别处）。❷ `OFFICIAL_HOSTS` ＋ `base_host()`（去路径／去端口／小写）＋ `preset_doc_list(preset_id, base_url)@：**主机名优先 ＋ 预设 id 兜底** ⇒ ⇒ 主机名是语义上的归属（这份清单说的是**这家上游**）⇒ ⇒ 自定义条目指向官方域也认得；认不出（自建代理／镜像站／老域）⇒ ⇒ 退回预设 id（老域 `api.minimax.chat`／`api.minimaxi.com` 也登记 ⇒ ⇒ 存量条目还指着它 ⇒ ⇒ 仍是同一家厂商）。❸ `preset_models_missing(list, upstream_ids)@：**按 id 算**（上游列过的不重复；上游将来把出图模型列进去 ⇒ ⇒ 自动就不补了）。❹ `ModelSummary` 加 `model_source`（只读面填、**盘上不存**）。

③ **✅ 判据与读数** —— ❶ 新判据 4 条：厂商**先按主机名**认出来／**主机名压过预设 id**／「上游没列的」**按 id 算**／主机名**去路径去端口**后比较。❷ `providers::catalog` **30 过 / 0 红**（＋4）；全量 **475 过 / 0 红**。❸ 端脑那一侧把这份清单用起来（读面合并 ＋ 逐条标来源）⇒ ⇒ 读数见端脑 U+355E 条。

④ **⚠️ 仍然欠着的** —— ❶ **「这份清单该由谁维护」的真相还没到**（上游那份清单只列对话模型 ⇒ ⇒ 出厂清单是**唯一**能说清「这家上游有什么」的地方 ⇒ ⇒ 它得跟上游文档同步 ⇒ ⇒ 现在是**手工**的 ⇒ ⇒ 升级路径尚未钉）。❷ 本仓无脚本闸 ⇒ ⇒ 这一刀的判据只在本仓跑（影仓那三道圈码闸管不到本仓）。❸ 真 CI 待取（未推远端）。

**Updated**: 2026-11-16 (**「真上游那一格：上游的 `architecture.input_modalities` 真会被读到 ＋ 来源标签真是 `upstream`」** —— 承影仓 U+3543 条（输入格式收口那族），本条记**取证（真上游）**、**⚠️ 真上游照出的那个缺口**、**落点**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=影仓 U+3543 条（输入格式收口那族）的「真上游读数」那一格 ＋ OpenRouter 真响应（2026-11-16 取）｜ 改动：`src/providers/input_modalities.rs`（新增 `pick_upstream_modalities` ＋ `upstream_input_modalities` ＋ 1 条口径判据）＋ `src/providers/catalog.rs`（`RawModel` 收拢成模块级一个 ＋ 认上游三种形状 ＋ **上游真值优先** ＋ 真上游夹具判据 2 条）⇒ ⇒ **2 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib providers::` **88 过 / 0 红**（上一轮 85 ＋ 本刀 3）＋ 全量 **475 过 / 0 红** ｜ 真上游读数在**端脑**那一侧（见端脑 U+355D 条）| status=active | supersedes=none

① **⚠️ 取证（真上游）** —— ❶ `GET https://openrouter.ai/api/v1/models` **不用 key 也 200** ⇒ ⇒ 真上游那一格**不需要凭据** ⇒ ⇒ 可复现（⇒ ⇒ 也不会把任何密钥写进仓）。❷ 真响应 **467 条**模型，**每条**都带 `architecture.input_modalities` ⇒ ⇒ 上游把 PDF 写成 **`file`**（⇒ ⇒ 端脑这边要归一成 `pdf`）：`google/gemini-4-hydrogen` ⇒ `audio,file,image,text,video`；`anthropic/claude-sonnet-4.5`／`openai/gpt-4o-mini` ⇒ `text,image,file`。❸ 顶层还有 `id`／`context_length`／`canonical_slug` 等一堆字段 ⇒ ⇒ **上游的形状比我们的解析器认的多**。

② **⚠️ 取证：真上游跑出来 ⇒ ⇒ **照出一个真缺口**（不是设计问题）** —— ❶ 端脑**刷新面**（`/models/providers/{id}/fetch-models`）**自己打上游**、读**原始 JSON** ⇒ ⇒ `architecture.input_modalities` 能活。❷ 端脑**预览面**（「测试并获取模型」）走 **synapse 的 `parse_openai_models`** ⇒ ⇒ 而那个解析器的 `RawModel` **只声明了** id／display_name／created／context_* ⇒ ⇒ **`architecture` 整块没读** ⇒ ⇒ 上游真值在**解析层**就没了 ⇒ ⇒ 预览面只能说「按官方文档预设的」（来源 `catalog`）⇒ ⇒ **来源标签说谎**（本仓纪律：失败不许长得像成功）。❸ ⇒ ⇒ **真上游那一格的价值就在这儿**：假上游的夹具照过 ⇒ ⇒ 只有**真上游**才照出这个洞（假上游没有 `architecture` 这个字段 ⇒ ⇒ 丢不丢都看不出来）。

③ **✅ 落点：目录层认上游三种形状 ＋ **上游真值优先**（不再是「只有出厂清单」）** —— ❶ `RawModel` **收拢成模块级一个**（两个 API 标准共用）＋ 加三份候选：`architecture.input_modalities`／顶层 `input_modalities`／`modalities.input`。❷ **共享口径**：`input_modalities::pick_upstream_modalities`（三份候选里取**第一个非空的**）＋ `upstream_input_modalities(&Value)`（端脑写面用它）⇒ ⇒ **目录层与端脑写面不再各写一套**（以前两处各写 ⇒ ⇒ 上游改形状就只对一边生效 ⇒ ⇒ 两边说法会打架）。❸ 判据改成**三档**（目录层给前两档，**第三档留给端脑读面**）：**上游真值优先** ⇒ ⇒ 上游说了 ⇒ ⇒ **归一后**的词（`file` ⇒ `pdf`）＋ 来源 `upstream`；上游没说 ⇒ ⇒ 出厂清单 ＋ `catalog`；都没有 ⇒ ⇒ **留空 ＝ 不知道**（**不许**填兜底 `text` —— 那是端脑读面第三档的活 ⇒ ⇒ 目录层替它做 ⇒ ⇒ 来源标签会说谎）。

④ **✅ 判据与读数** —— ❶ `providers::` **88 过 / 0 红**（＋3）；全量 **475 过 / 0 红**。❷ **真上游夹具**（用**真数据**，不自造形状）：`google/gemini-4-hydrogen` 的**顺序**按上游那份（`audio/pdf/image/text/video`）⇒ ⇒ 与出厂清单那份（`text,image,audio,video,pdf`）**连顺序都不同** ⇒ ⇒ 钉住「读的是上游、不是清单」；`claude-sonnet-4.5`／`gpt-4o-mini` ⇒ ⇒ 上游**多一档 `pdf`**（出厂清单只说 text,image）⇒ ⇒ 两边**不一样** ⇒ ⇒ 判据**有牙**。❸ 上游说了**不算数的词**（`tools`／`function_calling`）⇒ ⇒ **不算模态** ⇒ ⇒ 落回出厂清单（不许当「上游真给了」）。❹ 谁都不认得 ⇒ ⇒ **留空**（不填兜底 `text`）。

⑤ **⚠️ 仍然欠着的** —— ❶ 归一（`file` ⇒ `pdf`）在**目录层**做（**盘上那份仍存上游原值**、由端脑读面归一 —— 两个形状各司其职）⇒ ⇒ **别把这两处混成一处**。❷ 另两种上游形状（顶层 `input_modalities`／`modalities.input`）**只有夹具**、没有真上游读数。❸ 真上游只跑了 **OpenRouter** 一家。

**Updated**: 2026-11-16 (**「回环对端绕过代理」这条修法**抽成共用模块**并铺到全仓回环路径 —— 顺带把根因查实：系统那份绕过名单**对回环不生效**，且一个 `NO_PROXY` 环境变量就把它**整份顶掉**」** —— 承上一条末尾「仍然欠着」①（别的打回环路径仍会被系统代理截）；本条记**取证（根因逐字查实）**、**两处落点**、**判据与变异**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条末尾「仍然欠着」① ｜ 改动：`src/http_client.rs`（**新**：共用模块）＋ `src/lib.rs`（一行 `pub mod http_client;`）＋ `src/a2a/client.rs` ＋ `src/a2a/v1_compat.rs` ＋ `src/lobe/encephalon_registration.rs` ＋ `src/lobe/service_registration.rs` ＋ `src/lobe/provider_set.rs` ＋ `src/decision/store.rs` ＋ `src/transport/sse.rs`（**7 处统一采纳**）＋ `tests/loopback_proxy_bypass.rs`（**新**，5 条）＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ⇒ ⇒ **12 文件** ＋ **零新增依赖** ＋ **零新增端点** ｜ 读数：全量 `cargo test --lib --tests`（带 `NO_PROXY`）lib **475 过 / 0 红** ＋ 新判据 **5 过 / 0 红** ＋ 变异**当场红**（lib 9 红 ／ 新判据**恰好 3 红**）＋ clippy **本刀文件 0 归责发现** ｜ 闸：本仓无脚本闸（闸在影仓） | status=active | supersedes=none

① **⚠️ 取证一：上一条登记的欠账**——「别的打回环的产品路径仍会被系统代理截」——本刀先把它量成数：今天**不带** `NO_PROXY` 跑全量 ⇒ ⇒ **8 条红**（`providers::catalog::list_models_*` 4 条 ＋ `lobe::encephalon_registration::*` 4 条）⇒ ⇒ 那 4 条 lobe 的红**就是这条欠账**（不是别的病）。

② **⚠️ 取证二：根因逐字查实（读上游源码 ＋ 测试进程内自测）** —— ❶ 代理层是 hyper-util 0.1.21（reqwest 0.13.1 的 `client-proxy-system`）⇒ ⇒ `Matcher::from_system()` = **环境变量** ＋ **Windows 注册表**两份来源。❷ 注册表的 `ProxyOverride` **确实被读**，且 `127.*` 会被归一化成 CIDR `127.0.0.0/8`（`ipv4_wildcard_to_cidr`）⇒ ⇒ **但绕过没生效**：实测（**一个环境变量都不设**，在测试进程内自量）——裸 `reqwest::Client::new()` 打回环上游 ⇒ ⇒ **`502` 空 body**，且**上游一次没被碰到**（命中数 0）⇒ ⇒ 系统自己那份名单**对回环不生效**。❸ 另：**环境变量 `NO_PROXY` 一旦非空，注册表那份就整份作废**（上游源码 `win::with_system` 里 `if builder.no.is_empty()`）⇒ ⇒ 「系统知道要绕回环」不是产品能指望的东西。❹ ⇒ ⇒ 于是把代码注释与文档里「reqwest **不吃**系统的绕过名单」改成实测口径（**读、但没生效；且环境变量一设就顶掉注册表**）——**写错根因的注释比没有注释更坏**。

③ **✅ 落点 A：共用模块 `src/http_client.rs`（新）** —— ❶ `is_loopback(url)`（`localhost`／`127.0.0.0/8`／`::1`；**私网段故意不算** —— 有些部署真要把私网走代理）＋ `builder_for(url)`（回环 ⇒ `.no_proxy()`）＋ `client_for(url)`。❷ 规则**只有一份** ⇒ ⇒ 任何一处漏改就是漏改，不会出现「两份规则打架」。❸ 单测 1 条（回环／非回环各一串）。

④ **✅ 落点 B：7 处回环路径统一采纳** —— `a2a::client`（A2A 客户端）＋ `a2a::v1_compat` ＋ `lobe::encephalon_registration`（端脑注册）＋ `lobe::service_registration`（服务注册）＋ `lobe::provider_set`（OpenAI 兼容 LLM 客户端）＋ `decision::store`（决策存储）＋ `transport::sse`（SSE 传输）⇒ ⇒ **零新增依赖、零新增端点**。

⑤ **✅ 判据：`tests/loopback_proxy_bypass.rs`（5 条，**机器无关**）** —— ❶ **自造一个假代理**：回环上起一个小服务，**永远回 `502` 空 body**（这正是真代理够不到本机回环时的签名）⇒ ⇒ 再把 `HTTP_PROXY` 指向它。❷ **自造一个假上游**：回环上回 `200`。❸ 于是结论**与本机注册表里的系统代理无关** ⇒ ⇒ 任何机器、任何 CI 同一个结论 ⇒ ⇒ 不会像「靠本机恰好开着代理才红」那样变成假绿。❹ 5 条：回环对端**不许**被交给代理 ／ **负控**（没有这条修法的客户端 ⇒ ⇒ 确实被假代理吞掉 ⇒ ⇒ 证明判据有牙）／ **远端对端仍走代理**（不许无脑全绕）／ lobe 的 LLM 客户端打到回环 ／ 服务注册打到回环。❺ ⚠ `NO_PROXY` 必须写成**不含回环**的值（`blocked.invalid`）—— 环境变量一旦非空，注册表那份就作废 ⇒ ⇒ 写成 `127.0.0.1` 的话假上游自己就被绕过了 ⇒ ⇒ 负控会**假绿**。

⑥ **✅ 变异（当场红）** —— 把 `is_loopback` 哑成恒 `false` ⇒ ⇒ lib **9 红**（模块单测 1 ＋ lobe 4 ＋ 兄弟在飞的 `providers::catalog` 4）＋ 新判据**恰好 3 红**（症状逐字 `left: 502 right: 200`）⇒ ⇒ 而**负控那两条照绿**（它们本就不该依赖这条修法）⇒ ⇒ 判据**有牙且不误伤**。复原后新判据 **5 过 / 0 红**。

⑦ **✅ 读数** —— ❶ 全量（带 `NO_PROXY`）：lib **475 过 / 0 红** ＋ **28 个测试二进制全绿**（lib ＋ 27 个集成测试，其中新判据 **5 过 / 0 红**）＋ exit 0。❷ 全量（**不带** `NO_PROXY`）：**8 红 → 4 红** ⇒ ⇒ 只剩兄弟在飞的 `src/providers/catalog.rs` 那 4 条（症状逐字 `Status(502, "")`）⇒ ⇒ **不代修**。❸ clippy：本刀文件 **0 归责发现**（`src/a2a/v1_compat.rs` 4 处 ＋ `tests/tools_node_test.rs` 1 处是**存量**，不在本刀改的行上）。

⑧ **⚠️ 途中一格：一个变异窗口里冒出来的 2 条 `tools::web` 红** —— 那两条是**纯参数映射断言、零 HTTP** ⇒ ⇒ 与代理无关 ⇒ ⇒ 查 `src/tools/web/engine.rs` 的 mtime：兄弟在那一窗口里**正在改**它 ⇒ ⇒ 那是**兄弟在飞面的瞬态**，不是本刀造成 ⇒ ⇒ **如实记，不归责**。

⑨ **⚠️ 仍然欠着的** —— ❶ 兄弟的 `src/providers/catalog.rs` 那 4 条红**不代修**（它此刻仍在飞面）。❷ 真 CI 待取（未推远端）。❸ 本刀**没有真机**。❹ 修法只覆盖 **HTTP 客户端**这一层：端脑/neuron 仓**零改动** ⇒ ⇒ 省的是「**那一发合成**」，不是「模型常驻」。❺ 私网段**故意**不走绕过 ⇒ ⇒ 走代理的部署里那类对端仍会被代理截（**漏检只白烧，不吞声音**）。

⑩ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位。
**Updated**: 2026-11-16 (**「web 能力面的交互动作**逐字**对过真引擎 —— 上一刀的参数映射有**三处硬错**＋**一处静默做错事**；顺带把**引擎发行包的摘要**钉进仓里」** —— 承本仓上一条（web 能力面落到 synapse）与影仓 U+3544／U+3546 条那族，本条记**取证**、**三处落点**、**途中三格**、**判据与变异**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=承本仓上一条（web 能力面）与影仓 U+3544 条（obscura 内嵌那族）｜ 改动：`src/tools/web/engine.rs`（**重写**）＋ `src/tools/web/engine_binary.rs`（**新**）＋ `src/tools/web/mod.rs` ＋ `src/tools/web/tests.rs` ＋ `examples/verify_web_engine_artifact.rs`（**新**）＋ `src/tools/mod.rs`（一行 `pub mod web;` 不变）⇒ ⇒ **5 文件** ＋ **零新增依赖**（`sha2`／`hex` 早已是 direct 依赖）＋ 零新增端点 ｜ 读数：`cargo test --lib tools::web` **36 过 / 0 红**（**连跑三轮全绿**；含真 obscura 真链路 **2 条**：抓取 ＋ **交互点击落点**）＋ synapse 全量 `cargo test --lib` **471 过 / 0 红** ＋ 变异三处**当场红**、复原后 sha256 **逐字一致**（FINAL ＝ 基线）＋ clippy 本刀文件 **0 归责发现**（2 处 `double_must_use` 是 `#[async_trait]` **宏生成**的 ⇒ ⇒ 仓内 74 行同款）｜ 闸：本仓无脚本闸（闸在影仓） | status=active | supersedes=none

① **⚠️ 取证一：真引擎的**错误**逐字回话（不是推理）** —— ❶ 上一条把动作模型写成「一个 target 字符串 ＋ 一个 value 字符串」⇒ ⇒ **参数名对不上**，真引擎（obscura 0.2.7，MCP over HTTP）当场回：
　　・`browser_type` 的参数叫 **`text`**（**不是** `value`）⇒ ⇒ 「**Error: Missing text parameter**」；
　　・`browser_select_option` **只收 `selector`**（**不收 `ref`**）⇒ ⇒ 「**Error: Missing selector parameter**」；
　　・`browser_fill_form` 的 `fields` 是**数组**⇒ ⇒ 传对象 ⇒ 「**Error: Missing fields array**」；
　　・`browser_scroll` 不给 `direction` ⇒ ⇒ **不报错**，「**Scrolled element into view**」⇒ ⇒ ⚠️ **静默做错事**（把「翻页」做成了「把元素滚进视野」）⇒ ⇒ 比报错更坏。
❷ `tools/list` ＝ **38 个工具**、schema 全文 **13,260 字符**；交互工具的准确形状逐字为：`browser_click {ref|selector}`／`browser_fill {ref|selector, value*}`／`browser_type {ref|selector, text*}`／`browser_press_key {key*, selector}`／`browser_select_option {selector*, value*}`／`browser_scroll {direction, amount, ref, selector}`／`browser_fill_form {fields*=[{ref|selector, value, type}], submit_ref, submit_selector}`。
❸ **ref 按导航纪元编号**（`e1-1` → `e2-1` → `e3-1`…）⇒ ⇒ 换页后**旧 ref 立刻失效**（逐字回「unknown ref 'e1-1'; call browser_snapshot first to refresh the ref table」）⇒ ⇒ ref 必须来自**当前这一次页面加载**的 snapshot。

② **✅ 落点 A：动作模型拆成「目标两种互斥形状」＋ 纯函数映射（`engine.rs` 重写）** —— ❶ 新增 `Target{Ref, Selector}`（**不再拿一个字符串去猜**）＋ `ScrollDirection`＋`FieldKind`＋`FieldSpec`；`WebAction::Act` 带上 `fields`（数组）/`direction`/`amount`/`submit`。❷ 映射抽成**纯函数** `obscura_tool()` ＋ `obscura_args()` ⇒ ⇒ 判据能**逐字**钉住参数名（不靠网络、不靠真引擎）⇒ ⇒ 换引擎只改这一处。❸ 引擎**只收 selector** 的动作（select／press）拿到 ref ⇒ ⇒ **说实话**（新增 `WebError::BadArgument`）⇒ ⇒ 不把 ref 硬塞进 selector（那会让引擎默默做别的事）。❹ **scroll 没有方向 ⇒ 拒**（引擎层兜底）⇒ ⇒ 不给引擎「默默把翻页做成滚元素」的机会。

③ **✅ 落点 B：门面这一层说人话（`mod.rs`）** —— ❶ `web_act` 入参拆开成 `ref`／`selector`／`value`／`key`／`direction`／`amount`／`fields`（**数组**）／`submit_ref`／`submit_selector`；`target` 保留作「猜」的别名（带 CSS 记号＝选择器，否则＝引用）。❷ `fields` 给成对象 ⇒ ⇒ 门面**先**说「要数组」⇒ ⇒ 别让模型去撞引擎的错误。❸ **scroll 缺方向在门面就拒**（这是**我们自己的工具契约**，不是引擎特性）⇒ ⇒ 引擎层那道拒是第二道防线。❹ `WebError::BadArgument` ⇒ ⇒ `ToolError::InvalidParameters`（**失败要长得像失败**）。

④ **✅ 落点 C：分发链的摘要钉死（`engine_binary.rs` **新** ＋ CLI 例）** —— ❶ 官方 release 资产页**逐资产带 sha256 摘要**（v0.2.7 共 **20 个资产**）⇒ ⇒ 把摘要**钉进仓里**（逐字抄）＋ `sha256_file()`（**流式** ⇒ 72MB 发行包不整个进内存）＋ `verify_artifact()`（**对不上就报错**）。❷ `examples/verify_web_engine_artifact.rs`：文件名就是资产名 ⇒ ⇒ 一行命令核完。❸ ⚠️ **边界要说清**：官方只给**发行包**的摘要 ⇒ ⇒ 解压出来的 `obscura.exe` **没有可比对的值**。❹ 实测：本机 `obscura-x86_64-windows.zip` 的 sha256 ＝ 官方 `be983b66…7fb3` **逐字相同**⇒ ⇒ **跑的就是官方那一份**。

⑤ **⚠️ 途中三格** —— ❶ **判据写在错误的层 ⇒ ⇒ 当场红**：先写了两条「门面级」判据（用假引擎）⇒ ⇒ **红** ⇒ ⇒ 根因是「参数合法性住在**引擎层**，假引擎根本走不到那儿」⇒ ⇒ 分层定死：**引擎特性**（select 只收 selector）留引擎层、**我们自己的契约**（scroll 必须有方向）提门面 ⇒ ⇒ 各归其位。❷ **`Set-Location` 不改 .NET 的进程 CWD** ⇒ ⇒ 变异脚本里 `[System.IO.File]::ReadAllText('相对路径')` 打到了**另一个仓**⇒ ⇒ 变异脚本一律**绝对路径**。❸ **JS 模板串会被 Rust 源码里的反引号截断** ⇒ ⇒ 写大段 Rust 源码用**占位符**再替换。

⑥ **✅ 判据与变异** —— ❶ 本条新增 **15 条**（纯映射 6／门面 3／**线级 1**／摘要 4／真链路 1）⇒ ⇒ `cargo test --lib tools::web` **35 过 / 0 红**。❷ **线级那条比纯函数那条更硬**：假引擎在**回环**上（正是系统代理会插手的那种端点）⇒ ⇒ 一次过「门面 → 引擎 → 线上 JSON」整条链，断言引擎**真的收到**的参数名（`text` 不是 `value`／`direction` 在／`fields` 是数组）。❸ **真链路两条都真跑**（不是 skip）：`https://example.com/` 抓取 ＋ **按 snapshot 里的 ref 点链接 → 落到 iana.org**。❹ **变异三处，当场红**：① `browser_type` 的 `text` 改回 `value` ⇒ ⇒ 恰好 type 那条 ＋ 线级那条红；② `select` 改成「把 ref 硬塞进 selector」⇒ ⇒ 恰好 select 那条红；③ 去掉门面的 scroll 方向检查 ⇒ ⇒ 恰好「scroll 无方向被拒」那条红。❺ 每处复原后 **sha256 逐字一致** ＋ **推 mtime**（cargo 指纹会复用变异期的旧二进制）⇒ ⇒ 复原后复跑绿。
❻ ⚠️ **途中又踩一格 flaky**：两条真链路判据**共用一个引擎＝一个浏览器一个页面** ⇒ ⇒ Rust 的测试默认**并发**跑 ⇒ ⇒ 一条在 navigate、另一条在点链接 ⇒ ⇒ 读到对方的页面 ⇒ ⇒ 过滤跑时红过一次 ⇒ ⇒ 加一把 `REAL_ENGINE_LOCK`（`tokio::sync::Mutex`）把两条真链路**串行化** ⇒ ⇒ 连跑**三轮全绿**。

⑦ **⚠️ 仍然欠着的** —— ❶ **端脑那侧仍然没有交互确认流** ⇒ ⇒ `web_act` 仍不进白名单、provider 仍不列它（两道锁，本仓改不动）。❷ **真模型那一格（L4）仍打不动**（本机 provider 直连要桌面运行时注入的会话 token ⇒ ⇒ 脚本直连一律 401）。❸ **分发链只做了「校验」那一格**：**升级**（换版本要整表换摘要）＋ **崩溃自愈** ＋ **移动端位置**（obscura 无 Android/iOS build）都没做。❹ **`browser_wait_for`／`browser_wait_for_text`／`browser_get_attribute`／`browser_extract` 的细形状**只登记未接（门面没暴露这些动作）。❺ **真 CI 待取**（本刀未推远端）。❻ **端脑／移动端／桌面端本轮零改动** ⇒ ⇒ 那三本账本不占号。
❼ ⚠️ **端脑那一格（跨仓编译完整性）未验**：端脑自己的 `src/api/providers.rs` 加了 `model_source` 字段（**未提交**，兄弟在飞）⇒ ⇒ 端脑仓编译在 `E0063: missing field 'model_source'` 处红 ⇒ ⇒ **与本刀无关**（本仓零碰 `providers` 面）⇒ ⇒ 按纪律**不代修** ⇒ ⇒ 但「**端脑对着新 synapse 仍能编**」这一格**没验成**（端脑用到的 `ObscuraEngine::new`／`WebPolicy::detect`／`WebSession::new`／`WebFetchTool::new`／`WebReadTool::new` **签名一个都没改** ⇒ ⇒ 面上应当无碍，但**没跑过就是没跑过**）。
# Synapse — 神经突触层 (Shared Foundation Library)

**Updated**: 2026-11-16 (**「真模型那一格闭合 —— ADK 作服务端、用真 Gemini 回答 synapse 的消息，两条路都对；顺手补掉模板里『两个分支答复前缀相同』的证据漏洞」** —— 承上一条末尾「仍然欠着」②（真模型未复验）；本条记**取证（代理复活 ＋ 三档对照）**、**两处落点（模板前缀分开 ＋ 文档同步）**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条末尾「仍然欠着」② ｜ 改动：`examples/adk_interop_agent/interop_agent/agent.py`（两个分支**前缀分开**：`ADK_OFFLINE_OK` ／ `ADK_MODEL_OK`）＋ `examples/adk_interop_agent/README.md` ＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ＋ `docs/guide/examples.md` ＋ `docs/zh/guide/examples.md` ⇒ ⇒ **6 文件** ｜ 读数：真模型 **`PEER_REPLY ADK_MODEL_OK INTEROP PROBE`** ＋ **`STREAM_REPLY ADK_MODEL_OK INTEROP PROBE`**；坏 key 因果负控 **`PEER_REPLY_ERROR 401 UNAUTHENTICATED…`** ＋ **`STREAM_ERROR agent error: 401 UNAUTHENTICATED…`**；离线档 **`PEER_REPLY ADK_OFFLINE_OK INTEROP PROBE`** ｜ 闸：本仓无脚本闸 | status=active | supersedes=none

① **⚠️ 取证一：代理复活了 ⇒ ⇒ 那一格不用再等** —— ❶ 逐字（本机）：代理口 `127.0.0.1:7897` **在听**（`Test-NetConnection` ⇒ True），注册表 `ProxyEnable=1` ／ `ProxyServer=127.0.0.1:7897`。❷ `https://generativelanguage.googleapis.com/v1beta/models` **直连可达**（回 **403** ＝ 只是没带 key，不是超时）⇒ ⇒ 上一条「Google 直连不通」的判断**已过期**。❸ ⇒ ⇒ **真模型那一格本轮实测**（不是引用早前的读数）。

② **⚠️ 取证二：仓里那份 agent 模板有个**证据漏洞** —— 两个分支答复前缀一样** —— ❶ 原先两个分支都用 `ADK_OK` ⇒ ⇒ **离线档的答复与模型的答复逐字相同** ⇒ ⇒ 「探针回了 ADK_OK」**不能**证明是模型答的（当时是靠坏 key 负控才敢下结论）。❷ ⇒ ⇒ 改成两个前缀：离线档 `ADK_OFFLINE_OK`、模型档 `ADK_MODEL_OK` ⇒ ⇒ **答复自己就说明是哪一档答的** ⇒ ⇒ 探针的输出**自证**。❸ 顺带发现一个读法上的好处：三台服务端**各答各的**，一眼能分。

③ **✅ 落点 A：模板两个分支前缀分开（仓内那份 agent）** —— ❶ `examples/adk_interop_agent/interop_agent/agent.py`：`OFFLINE_PREFIX = "ADK_OFFLINE_OK"` ／ `MODEL_PREFIX = "ADK_MODEL_OK"`。❷ **key 仍然只在环境变量里**（ADK 的 `Gemini` 类自己按 `GOOGLE_API_KEY` → `GEMINI_API_KEY` 取）⇒ ⇒ 模板文件里一个 key 都不写。❸ 本轮验的**就是仓里这份模板**（不是 TEMP 里的残留）⇒ ⇒ 仓库产物自己被验过了。

④ **✅ 落点 B：文档与 README 同步（两处前缀）** —— `examples/adk_interop_agent/README.md` ＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ＋ `docs/guide/examples.md` ＋ `docs/zh/guide/examples.md` ⇒ ⇒ 五处都写清「离线档回 `ADK_OFFLINE_OK`、模型档回 `ADK_MODEL_OK`，**两个前缀故意不同**」。

⑤ **✅ 读数（三档对照，同一支探针、同一台客户端）** —— ❶ **真模型档**（8001，好 key，`INTEROP_MODEL=gemini-3.5-flash`）：`PEER_REPLY ADK_MODEL_OK INTEROP PROBE` ＋ **`STREAM_REPLY ADK_MODEL_OK INTEROP PROBE`** ⇒ ⇒ 答复逐字带 `ADK_MODEL_OK` ⇒ ⇒ **只能是模型那一档**。❷ **因果负控**（8002，**只换 key**，其余全同）：`PEER_REPLY_ERROR 401 UNAUTHENTICATED. {'error': {'code': 401, 'message': 'API key not valid…` ＋ `STREAM_ERROR agent error: 401 UNAUTHENTICATED…` ⇒ ⇒ **失败被当成失败**（刀 4 修的那条缺陷，被真上游的 401 逐字验证）⇒ ⇒ 模型调不通就是这条 ⇒ ⇒ 8001 那条**只能**是模型答的。❸ **离线档**（8003，**无 key、无网络**）：`PEER_REPLY ADK_OFFLINE_OK INTEROP PROBE` ＋ `STREAM_REPLY ADK_OFFLINE_OK INTEROP PROBE` ⇒ ⇒ 这一档**能进 CI**。❹ **反方向仍绿**（ADK 的 `RemoteA2aAgent` → synapse 服务端）：`LIVE_TEXT SYNAPSE_REPLIED_OK` ／ `LIVE_HAS_ERROR False` ／ `LIVE_ROUND_OK True`；负控逐字复现「A2A response stream ended before task t-pre-fix reached a terminal state.」。❺ **ADK 真模型的流式形状**（逐字事件序列）：`task(SUBMITTED)` → `statusUpdate(WORKING)` → `statusUpdate(WORKING，**message 里带答复 ADK_MODEL_OK …**)` → `artifactUpdate(lastChunk=true，**artifact 里带同一份答复**)` → `statusUpdate(COMPLETED)` ⇒ ⇒ **模型的答复确实落在 artifact 里** ⇒ ⇒ 客户端「artifact 优先、status 只兜底」的口径是对的。

⑥ **⚠️ 仍然欠着的** —— ❶ **回环代理那条修法目前只落在 A2A 客户端**：别的打回环的产品路径（`lobe::encephalon_registration` 的注册、`lobe::service_registration`、`lobe::provider_set` 打本地 OpenAI 兼容端、`decision::store`、`transport::sse`、以及**兄弟在飞**的 `providers::catalog::list_models`）**仍会被系统代理截** ⇒ ⇒ 本机不带 `NO_PROXY` 跑全量时 **8 条红**（`providers::catalog::list_models_*` 4 条 ＋ `lobe::encephalon_registration::*` 4 条）⇒ ⇒ **环境不是代码**，但**修法没铺开**。❷ 真 CI 待取（未推远端）。❸ 本刀**没有真机**。❹ 兄弟的 `src/providers/catalog.rs` 仍在飞 ⇒ ⇒ 那 4 条**不代修**。

⑨ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位。
**Updated**: 2026-11-16 (**「流式路径那三条规矩 ＋ 回环对端不许走代理 —— 客户端侧的四个缺陷，一个比一个隐蔽」** —— 承上一条末尾「仍然欠着」④（流式路径只读 artifact）；本条记**取证（三个缺陷各自是怎么暴露的）**、**四处落点**、**判据与变异**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条末尾「仍然欠着」④ ｜ 改动：`src/a2a/client.rs`（流式三条规矩 ＋ 回环代理绕过）＋ `tests/a2a_client_stream.rs`（**新**，6 条）＋ `examples/a2a_interop_client.rs`（补流式那条腿）＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ⇒ ⇒ **6 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib` **455 过 / 0 红** ＋ 新判据 **6 过 / 0 红** ＋ 变异三处当场红（**4 红**／**1 红**／**2 红**）＋ clippy **0 error**、本刀文件 **0 warning** ｜ 闸：本仓无脚本闸 | status=active | supersedes=none

① **⚠️ 取证一：三个缺陷是在**写判据的过程中**逮到的，不是读代码想出来的** —— ❶ 「答复只在 `status.message` 里 ⇒ 流式客户端看不到」：**上一条台账登记的欠账**（我自己写的）。❷ 「流式 Final 只带最后一块 ⇒ 答复被截断」：**我自己的判据暴露的** —— 判据期望 ``"hello world"``，实测 Final 只有 ``"world"`` ⇒ ⇒ 对端**分块**流式送答复时**答复被截断**（A2A 的 `append=true` 语义就是「续」，客户端必须**累加**）。❸ 「回环对端被系统代理吞掉 ⇒ 客户端连不上本机对端」：**判据全红时暴露的** —— 症状逐字 ``failed to parse JSON-RPC response: EOF while parsing a value at line 1 column 0``（代理那个**空 body 的 502**）。

② **⚠️ 取证二：第三个缺陷的根因是「reqwest 不吃系统的绕过名单」** —— ❶ 逐字（本机注册表）：`ProxyEnable=1` ／ `ProxyServer=127.0.0.1:7897` ／ `ProxyOverride=localhost;127.*;192.168.*;10.*;172.16.*-172.31.*;*.local;<local>` ⇒ ⇒ **系统自己知道要绕过回环**。❷ 但 reqwest **只取 `ProxyServer`，不取 `ProxyOverride`** ⇒ ⇒ 回环请求被丢给代理 ⇒ ⇒ 代理够不到本机自己的回环口 ⇒ ⇒ **`502 Bad Gateway`（空 body）**。❸ 量到的对照（同一台机器、同一个回环对端）：`reqwest::Client::new()` ⇒ **502**（空 body）｜`reqwest::Client::builder().no_proxy()` ⇒ **200**｜**环境里一个代理变量都没有**（`HTTP_PROXY`／`HTTPS_PROXY`／`ALL_PROXY` 逐个 `Err(NotPresent)`）⇒ ⇒ 是 **reqwest 从操作系统那里抓的**，不是环境变量。❹ ⇒ ⇒ 这**不是测试的问题，是产品缺陷**：synapse 自己的 A2A 客户端在有系统代理的机器上**连不上本机的对端**（ADK 或 synapse 自己）。

③ **✅ 落点 A：客户端的 HTTP 通道对回环对端绕过代理** —— ❶ 新增 `peer_http_client(peer_url)` ＋ `peer_is_loopback(peer_url)`（`localhost`／`127.0.0.0/8`／`::1`；**私网段故意不算** —— 有些部署真要把它们走代理）⇒ ⇒ 回环对端 `.no_proxy()`，远端对端**保留**系统代理。❷ 两个建客户端的地方都改用它：`with_extensions`（单发与流式共用）与 `get_agent_card`（它自己另建了一个 client）。

④ **✅ 落点 B：流式路径的三条规矩** —— ❶ **答复是整个 artifact**：按 `artifact_id` 累加（`append=false` ⇒ **替换**，`append=true` ⇒ **续**），`lastChunk=true` 时 Final 带**累加出来的全文** ⇒ ⇒ 分块流式送答复不再被截断。❷ **流结束了 ≠ 交互结束了**：流收尾时若**没见到终态**、又没有答复也没有错误 ⇒ ⇒ 发 `Error`（逐字「the peer's stream ended before the task reached a terminal state」）⇒ ⇒ 与 ADK 客户端同口径。❸ **答复可能在 status message 里**：全程没有 artifact 带过文本、而终态到了 ⇒ ⇒ 用 status message 的文本补一个 Final；**只有**在 artifact 一个字都没带过时才用它 ⇒ ⇒ ADK 那种「两处都有」**不会**显示两遍。

⑤ **✅ 判据与变异** —— ❶ `tests/a2a_client_stream.rs` **6 条**（**真起 socket**，假对端按 A2A v1.0 吐 SSE）：答复只在 status message 里也要到调用方｜流被截断必须报失败｜ADK 那种「两处都有」只报一次｜健康流：答复一次、无失败｜`append=false` 是**替换**｜失败任务只报一次失败。❷ **变异三处，当场红**：关掉代理绕过 ⇒ ⇒ **4 红**（症状逐字 `EOF while parsing a value at line 1 column 0`）｜Final 只带最后一块 ⇒ ⇒ **恰好 1 红**（逐字 `left: ["world"] right: ["hello world"]`）｜去掉流尾的终态校验与 status 回退 ⇒ ⇒ **恰好 2 红**。❸ 全量：带 `NO_PROXY` 时 `cargo test --lib` **455 过 / 0 红**、`--lib --tests` 全绿；⚠️ **不**带 `NO_PROXY` 时有 **8 条红**（`providers::catalog::list_models_*` 4 条 ＋ `lobe::encephalon_registration::*` 4 条）—— 那是**别的产品路径**（不是本刀的）打回环假上游被代理截了 ⇒ ⇒ **环境不是代码**，且**不代修**（`src/providers/catalog.rs` 此刻还是兄弟在飞面）。❹ clippy：**0 error**；本刀文件 **0 warning**。

⑥ **✅ 活服务端读数（真 ADK 2.14.1 服务端，端口 8001）** —— ❶ **两条路都对**：`PEER_REPLY ADK_OK INTEROP PROBE`（单发，从 artifacts）＋ **`STREAM_REPLY ADK_OK INTEROP PROBE`**（流式，从累加出来的 artifact）⇒ ⇒ 答复没被截、也没报两遍。❷ 反方向仍绿（上一刀读数，本刀未改服务端）：`LIVE_TEXT SYNAPSE_REPLIED_OK` ／ `LIVE_HAS_ERROR False` ／ `LIVE_ROUND_OK True`。❸ 仓内那支探针（`examples/a2a_interop_client.rs`）现在**两条路都打** ⇒ ⇒ 一支探针看两格。

⑦ **⚠️ 途中那几格** —— ❶ **回环代理那格不是读文档读出来的，是判据全红时追出来的** —— 症状（空 body 的 502）先指向「形状不对」，顺着量下去才发现是**代理**（与 `web` 那一族登记过的「引擎 client 必须 `no_proxy()`」同一个病，只是这次长在 A2A 客户端上）。❷ **`Copy-Item` 复原会保留 mtime** ⇒ ⇒ cargo 复用旧二进制 ⇒ ⇒ 每次复原后**推 `LastWriteTime`**（上一条台账已记，本刀复用同一手法）。❸ 兄弟的 `src/providers/catalog.rs` 中途又进编译错态（`modalities` 未定义）⇒ ⇒ **不代修**，等它自愈（几分钟后自愈）。

⑧ **⚠️ 仍然欠着的** —— ❶ **本仓只有这一处 HTTP 通道修了**：别的打回环的产品路径（`providers::catalog` 的 `list_models`、`lobe::encephalon_registration` 的注册）**仍然会被系统代理截** ⇒ ⇒ 那是**存量债**，登记在此，**不代修**（`src/providers/catalog.rs` 是兄弟在飞面）。❷ **真模型那一格仍未复验**（上一条的欠账①：Google 直连不通）⇒ ⇒ 代理恢复后重跑。❸ 真 CI 待取（本刀未推远端）。❹ **流式路径只把 artifact 的文本当答复**：若对端把答复放在**非文本** part（图片／数据）里，流式路径不会给 Final ⇒ ⇒ 未修，登记在此。❺ 本刀**没有真机**。

⑨ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位。
**Updated**: 2026-11-16 (**「客户端把答复取错了地方 —— ADK 官方口径：答复在 `artifacts`，失败态是**错误**不是答复」** —— 承上一条（ADK 用户视角闭环）末尾那一格；本条记**取证（ADK 官方转换器逐字 ＋ 活 ADK 服务端上的缺陷取证）**、**两处落点（客户端取答复改口径 ＋ 失败态不冒充成功）**、**判据与变异**、**活服务端读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条末尾「仍然欠着」①（ADK 作服务端用真模型回答）＋ Google ADK 2.14.1 官方转换器逐字（`google/adk/a2a/converters/to_adk_event.py`）｜ 改动：`src/a2a/client.rs`（`send_message` 改读 artifacts ＋ 失败/取消/拒绝态报错）＋ `tests/a2a_client_reply.rs`（**新**，5 条）＋ `examples/adk_interop_agent/`（**新**：agent.py ＋ agent.json ＋ README）＋ `examples/a2a_interop_client.rs`（回声探测器）＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ＋ `docs/guide/examples.md` ＋ `docs/zh/guide/examples.md` ⇒ ⇒ **9 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib` **451 过 / 0 红** ＋ 新判据 **5 过 / 0 红** ＋ 真变异「退回旧写法 ⇒ 恰好 **4 红 / 1 绿**」＋ clippy **0 error**、本刀文件 **0 warning** ｜ 闸：本仓无脚本闸；影仓状态面闸**本刀未落影仓文件** | status=active | supersedes=none

① **⚠️ 取证一：ADK 官方口径 —— 答复在 `artifacts`，`status.message` 只在边界态当答复，失败态是错误** —— ❶ 逐字 `google/adk/a2a/converters/to_adk_event.py` 的 `convert_a2a_task_to_event`：答复取自 `a2a_task.artifacts`（其逐字注释是「**A task carries one artifact per non-partial event the remote agent produced, so reading only the last one drops every earlier output**」⇒ ⇒ **全部 artifact 的 parts 摊平**，不是只读最后一个）。❷ `status.message` 只在三种情形当答复：`input_required` ／ `auth_required` ／（**没有 artifact** 且状态落在边界集 `_TASK_RESPONSE_BOUNDARY_STATES`＝`{input_required, auth_required, completed, failed, canceled, rejected}`）。❸ **失败态**（`_is_failed_status` **只认** `TASK_STATE_FAILED`）⇒ ⇒ `status.message` 是**错误** ⇒ ⇒ `_mark_a2a_task_failed` 给事件盖上 `error_code` ＋ `error_message`（默认逐字 `Remote agent task failed`）⇒ ⇒ **不当内容交回去**。❹ ⇒ ⇒ **Synapse 客户端原来的写法（`task.history.last()`）三条全违反**。

② **⚠️ 取证二：两个真缺陷是在**活的 ADK 服务端**上逮到的（不是推演）** —— ❶ **缺陷一（答复取错地方）**：synapse 服务端单发路径的 `history` 里**只有调用方那条**（逐字实测：`history` ＝ `[{"role":"ROLE_USER","parts":[{"text":"synapse raw probe"}]}]`，`artifacts` ＝ `[{"parts":[{"text":"SYNAPSE_REPLIED_OK"}]}]`）⇒ ⇒ 旧写法取到的是**调用方自己的原话**（实测 `PEER_REPLY interop probe`，正确答复应是 `SYNAPSE_REPLIED_OK`）。❷ **缺陷二（失败长得像成功）**：打坏 key 的 ADK（8002）⇒ ⇒ 旧写法报出 **`PEER_REPLY 401 UNAUTHENTICATED. {'error': ...}`** ⇒ ⇒ 一条**格式完好的答复**说的却是失败。❸ ⚠️ **一个会骗人的巧合**：**ADK 服务端**是 artifacts **和** history **两处都有**答复 ⇒ ⇒ 旧写法打 ADK 时**碰巧是对的**（history 最后一条正好是答复）⇒ ⇒ 这个缺陷**只在打 synapse 自己时暴露** ⇒ ⇒ **别被「打 ADK 好好的」骗了**。

③ **✅ 落点 A：客户端取答复改成官方口径** —— ❶ 新增 `task_reply_from_task(&a2a::Task)`：**失败／取消／拒绝** ⇒ ⇒ 直接 `Err`（**带对端原文**，不吞）⇒ ⇒ 其余：**artifacts 优先**（全部 artifact 的 parts 摊平）⇒ ⇒ 其次**边界态**的 `status.message` ⇒ ⇒ 最后才退到 `history` 里 **`role = agent` 的那一轮**（**不是最后一条**）。❷ `history` 只在最后兜底且**只认 agent 自己那一轮** ⇒ ⇒ 调用方的原话**永远**不会被当成答复。❸ 取答复的文本拼接用 **`join("")`**（与本文件里流式 `Final` 的口径一致）⇒ ⇒ 非文本 part（图片/数据）交给 SDK 自己的渲染，不被静默丢掉。

④ **✅ 落点 B：ADK 作服务端的 agent 模板进仓（含**不需要 key、不需要网络**那一档）** —— ❶ `examples/adk_interop_agent/interop_agent/agent.py`：`INTEROP_MODEL=offline` ⇒ ⇒ 走**自定义 `BaseAgent`**（`_run_async_impl` 直接产出 Event）⇒ ⇒ **不调模型** ⇒ ⇒ **不需要 key、不需要网络** ⇒ ⇒ 这条腿**能进 CI**。❷ 设 `INTEROP_MODEL=gemini-3.5-flash` ⇒ ⇒ 走 ADK 的 `Gemini`（**原生 `google-genai` 路径**，**不是** `LiteLlm`）⇒ ⇒ **key 只在环境变量里**（ADK 自己按 `GOOGLE_API_KEY` → `GEMINI_API_KEY` 取）⇒ ⇒ **agent 文件里一个 key 都不写**。❸ `agent.json` 的 `url` 必须是**真实挂载点** `/a2a/interop_agent` ⇒ ⇒ ADK **原样**抄进卡片（`supportedInterfaces[0].url`，**不改写**）⇒ ⇒ 卡片若写根路径 ⇒ ⇒ 照卡片办事的客户端 `POST /` 回 **404**、`GET /` 回 **405**。

⑤ **✅ 判据与变异** —— ❶ `tests/a2a_client_reply.rs` **5 条**（**真起 socket**）：❽ 答复是 **artifact** 而不是调用方原话（打**真 synapse 服务端**）｜❾ 失败任务必须报成**失败**（假对端回 `TASK_STATE_FAILED` ＋ `401 UNAUTHENTICATED`）｜❿ 只有 history 时取 **agent 那一轮**｜⓫ artifact 与 history 都有时 **artifact 赢**（ADK 的双份形状）｜⓬ 取消的任务不当答复。❷ **真变异**：把 `send_message` 退回旧写法（`history.last()`）⇒ ⇒ **恰好 4 红 / 1 绿**（逐字：`a_failed_task...` 红且报「the client answered: CALLER_QUESTION」、`the_answer_is_the_artifact...` 红且报 `left: "CALLER_QUESTION" right: "AGENT_ANSWER"`、`the_artifact_wins...` 红、`a_canceled_task...` 红；**唯一绿**的正是旧写法也对的「只有 history」那条）⇒ ⇒ 判据**有牙**。❸ 复原后 `cargo test --lib` **451 过 / 0 红**；`--lib --tests` 只有 **`mcp_test` 3 条红**，根因是本机 `python3` 是 **Microsoft Store 占位符** ⇒ ⇒ 设 `$env:PYTHON` 后 **3 过 / 0 红** ⇒ ⇒ **环境不是代码**。❹ clippy：**0 error**；**本刀文件 0 warning**（顺手收掉自己那两条 `collapsible_if`）。

⑥ **✅ 活服务端读数（真 ADK 2.14.1 服务端，端口 8001）** —— ❶ **成功腿（离线档，不需 key、不需网络）**：卡片逐字 `supportedInterfaces[0].url = http://127.0.0.1:8001/a2a/interop_agent`（**与真实挂载一致**）＋ 客户端 **`PEER_REPLY ADK_OK INTEROP PROBE`** ⇒ ⇒ 答复确实从 artifact 取到了。❷ **修前/修后对照（同一台活服务端，模型调用失败那一格）**：**修前 `PEER_REPLY All connection attempts failed`**（失败被当成答复）⇒ ⇒ **修后 `PEER_REPLY_ERROR All connection attempts failed`**（失败被当成失败）⇒ ⇒ 同一台服务端、同一支探针，**唯一变量是客户端那一行**。❸ **反方向仍绿**（ADK 客户端 → synapse 服务端）：`LIVE_TEXT SYNAPSE_REPLIED_OK` ／ `LIVE_HAS_ERROR False` ／ `LIVE_ROUND_OK True`；负控 `PREFIX_ERROR A2A response stream ended before task t-pre-fix reached a terminal state.` 逐字复现。❹ **流式形状（逐字事件序列）**：`task(SUBMITTED)` → `statusUpdate(WORKING)` → `statusUpdate(WORKING，**message 里带答复**)` → `artifactUpdate(lastChunk=true，**artifact 里带同一份答复**)` → `statusUpdate(COMPLETED)` ⇒ ⇒ **答复在 artifact 里** ⇒ ⇒ synapse 的流式路径读 artifact 就够 ⇒ ⇒ **两处都读会把答复显示两遍** ⇒ ⇒ 流式路径**不改**（这条是量出来的，不是猜的）。

⑦ **⚠️ 途中那几格** —— ❶ **ADK 没有 "offline" 模型**（逐字搜遍 `google/adk` 只找到 OAuth 的 `access_type: offline`）⇒ ⇒ 离线档只能自己写一个 `BaseAgent`；好在 ADK 的 agent loader 接受 `root_agent` 是 `BaseAgent` 实例 ⇒ ⇒ 模板才立得住。❷ ⚠️ **本机 Google 直连不通**（`generativelanguage.googleapis.com` 直连超时；系统代理 `127.0.0.1:7897`／`7890`／`1080` 全灭；`ProxyEnable=0`）⇒ ⇒ **真模型那一格本轮无法复验** ⇒ ⇒ 「真模型」的读数取自**本刀早前（代理在线时）**那次实测（`PEER_REPLY ADK_MODEL_OK INTEROP PROBE`；坏 key 因果负控 `401 UNAUTHENTICATED`）⇒ ⇒ **别把早前的读数说成这一轮的**。❸ **ADK 服务端只认 `A2A-Version: 1.0`**（`0.3` 或缺头 ⇒ `-32009 VERSION_NOT_SUPPORTED`，逐字「A2A version '0.3' is not supported by this handler. Expected version '1.0'.」）⇒ ⇒ 与 synapse 服务端同一口径。❹ ⚠️ **复原文件后必须 `touch`**：本刀用 `Copy-Item` 复原 `client.rs` ⇒ ⇒ **保留了旧 mtime** ⇒ ⇒ cargo 认为源码没变 ⇒ ⇒ **复用了变异期的二进制** ⇒ ⇒ 第一次跑出来的「修后」读数其实**是修前** ⇒ ⇒ 差点把结论写反 ⇒ ⇒ 复原后把 `LastWriteTime` 置成现在（或 `touch`）再跑。

⑧ **⚠️ 仍然欠着的** —— ❶ **真模型那一格本轮未复验**（网络不通）⇒ ⇒ 代理恢复后重跑：设 `INTEROP_MODEL=gemini-3.5-flash` ＋ `GOOGLE_API_KEY` ⇒ ⇒ 期望 `PEER_REPLY ADK_MODEL_OK INTEROP PROBE`。❷ **真 CI 待取**（本刀未推远端）。❸ 本刀**没有真机**（全在仓内与活服务端）⇒ ⇒ 「装上去的 ＝ 提交编出来的」那一格**本刀未触及**。❹ **流式路径仍只读 artifact** ⇒ ⇒ 若某个对端**只**把答复放在 `status.message`（不带 artifact），synapse 的流式客户端**看不到答复** ⇒ ⇒ ADK 不在这一类（它两处都有）⇒ ⇒ 这一格**没修**，登记在此。❺ 存量 clippy 告警（`card.rs` 的 `derivable_impls`、`store.rs`／`v1_compat.rs` 一批、`events/mod.rs` 的 `unused_variables`、`providers/model_capabilities.rs` 的空行等）**未代修**（本仓无 `-D warnings` 闸，且与本刀无关）。❻ `mcp_test` 需要 `$env:PYTHON`（本机 `python3` 是 Store 占位符）⇒ ⇒ CI 上要么装好 `python3`，要么显式设 `PYTHON`。❼ 影仓侧**本刀未落任何文件** ⇒ ⇒ 影仓那三道跨端闸不受影响；⚠️ 影仓状态面闸当前 4 处红**全是兄弟在飞面**（桌面／移动台账条目缺信封字段 `status`）⇒ ⇒ **不是本刀引入，不代修**。

⑨ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位。
**Updated**: 2026-11-16 (**「Gemini 2.5/3.x 那一族按官方逐字**逐模型**登记 —— 2.5-flash **没有 PDF**，2.5-flash-lite／2.5-pro／3.x 有；omni 只有三档」** —— 承影仓 U+3543 条（输入格式收口那族），本条记**取证（官方逐字）**、**两处落点**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=影仓 U+3543 条（输入格式收口那族）＋ Google 官方每模型页逐字（2026-11-16 取）｜ 改动：`src/providers/input_modalities.rs`（补 7 条 Gemini 规则）＋ `src/providers/catalog.rs`（`ModelSummary` 加模态两字段 ＋ 两处构造点按出厂清单填充）⇒ ⇒ **2 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib providers::input_modalities` **8 过 / 0 红** ＋ `cargo test --lib providers::` **85 过 / 0 红** ＋ 全量 **451 过 / 0 红**（⚠️ 首跑 8 条红是**本机系统代理**把回环假上游截成 502 ⇒ ⇒ 负控**摘掉本刀整个模块**仍红 ⇒ ⇒ 带 `NO_PROXY=127.0.0.1` 重跑全绿 ⇒ ⇒ **环境不是代码**）| status=active | supersedes=none

① **⚠️ 取证：官方**每模型页**逐字（不是一个族一句）** —— ❶ `gemini-2.5-pro` 页逐字「**Inputs** Audio, images, video, text, and PDF」⇒ ⇒ 五档全中。❷ `gemini-2.5-flash` 页逐字「Text, images, video, audio」⇒ ⇒ **没有 PDF**（与 2.5-pro 只差一个后缀 ⇒ ⇒ **一条通配就会错**）。❸ `gemini-2.5-flash-lite` 页逐字「Text, image, video, audio, PDF」⇒ ⇒ 五档。❹ `gemini-2.0-flash` 页逐字「Text, images, video, audio」⇒ ⇒ **没有 PDF**。❺ `gemini-3-pro-preview`／`gemini-3-flash-preview` 页逐字「Text, Image, Video, Audio, and PDF」⇒ ⇒ 五档。❻ `gemini-omni`（model code `gemini-omni-1.2-flash`）页逐字「**Input** Text, Image, Video」⇒ ⇒ **只有三档**（不吃音频／PDF）。❼ 每条规则各带**那个模型页**的地址（不是族首页）⇒ ⇒ 界面能说「这是按哪一页说的」。

② **✅ 落点 A：真源表补 7 条 Gemini 规则（逐模型，不许通配）** —— ❶ 新增 `gemini-2.5-pro`／`gemini-2.5-flash`／`gemini-2.5-flash-lite`／`gemini-2.0-flash`／`gemini-3-pro-preview`／`gemini-3-flash-preview`／`gemini-omni-flash` 七条 ⇒ ⇒ 规则**按 pattern 长度降序**试 ⇒ ⇒ `gemini-2.5-flash-lite`（更长）**先于** `gemini-2.5-flash` 命中 ⇒ ⇒ 不许被短的那条吃掉。❷ 判据把这一点钉死（`flash-lite` 与 `flash` **只差一个 `-lite`** 却**差一个 PDF** ⇒ ⇒ 两条必须**各自**落到官方逐字）。

③ **✅ 落点 B：`catalog::ModelSummary` 带模态两字段（一处形状 ⇒ ⇒ 三处共用）** —— ❶ 加 `input_modalities`（`Vec<String>`，`serde(default)` ⇒ ⇒ 旧数据不受影响 ＋ 空 ＝ **不知道**，**不是**「只能吃文本」）＋ `input_modalities_source`（`Option<String>`）⇒ ⇒ 这一份形状被**目录解析路径**、**端脑预设清单回退面**、**桌面 Tauri `provider_list`** 三处共用 ⇒ ⇒ 一份真源。❷ 两处构造点按**出厂清单**填（认不出 ⇒ ⇒ **留空** ⇒ ⇒ 界面不乱画）。❸ ⚠️ **这里不许填兜底 `text`** —— 兜底是**端脑读面**的活（三档判据：上游 → 出厂清单 → 兜底）⇒ ⇒ 目录层就填兜底 ⇒ ⇒ 「读面算出来的来源」会被提前烧成「出厂清单」⇒ ⇒ **来源标签会说谎**。

④ **✅ 判据与读数** —— ❶ `providers::input_modalities` **8 过 / 0 红**（八条：文档族逐条落到官方逐字说的模态／更具体的 pattern 先命中／**未登记的不许编**（DeepSeek ⇒ `None`）／每条规则都带 https 文档地址／模态词都在词表里／上游词归一／大小写不敏感／文档地址与模态查找一致）。❷ `providers::` **85 过 / 0 红**。❸ 全量 **451 过 / 0 红**（含兄弟在飞的 a2a 一族）。

⑤ **⚠️ 仍然欠着的** —— ❶ **Claude 那族**文档在本机**区域被屏蔽**（页面逐字是「App unavailable in region」）⇒ ⇒ 只按 Anthropic 官方 `llms.txt` 逐字列出的**地址**登记 ⇒ ⇒ 待能在访问该页的机器上**再核一次逐字**才闭合。❷ **DeepSeek 整族不登记**（官方站是 JS 壳页 ⇒ ⇒ 取不到逐字）⇒ ⇒ 落兜底 `text`（对 `deepseek-chat`／`deepseek-reasoner` 恰好对）⇒ ⇒ 按纪律**不填没核过的**。❸ **真上游读数缺**（本刀判据全用**假上游**）⇒ ⇒ 「真供应商给的 `architecture.input_modalities` 真会被读」**未在真上游跑过**（OpenRouter 真上游有那个字段）⇒ ⇒ 真机那一格也没跑。❹ Gemini **4 族**那条（`gemini-4-carbon`）沿用上一刀的官方逐字（`Modalities: Text, Image, Audio, Video, PDF`）⇒ ⇒ 本刀**没重核**（不在本刀改动面）。

**Updated**: 2026-11-16 (**「web 能力面落到 synapse —— obscura 那个浏览器引擎收口成**三个自有名**的工具（`web_fetch`／`web_read`／`web_act`）：引擎在**外进程**，门在我们这一层，名字也是我们自己的」** —— 承影仓 U+3544 条（obscura 能不能内嵌那族），本条记**取证**、**三处落点**、**途中四格**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=影仓 U+3544 条（用户诉求逐字：「https://github.com/h4ckf0r0day/obscura  评估这个项目能否内嵌作为默认工具 能力沉淀到synapse中」）｜ 改动：`src/tools/web/`（policy.rs／engine.rs／mod.rs／server.rs／tests.rs **全新**）＋ `src/tools/mod.rs`（一行 `pub mod web;`）⇒ ⇒ **6 文件** ＋ 零新增依赖 ＋ 零新增端点 ｜ 读数：`cargo test --lib tools::web` **20 过 / 0 红** ＋ 真 obscura 真链路 **1 过 / 0 红**（0.75s）＋ 全量 **451 过 / 0 红**（⚠️ 首跑 8 条红是**本机系统代理**把回环假上游截成 502 ⇒ ⇒ 负控**摘掉本刀整个模块**仍红 ⇒ ⇒ 带 `NO_PROXY=127.0.0.1` 重跑 ⇒ ⇒ 全绿 ⇒ ⇒ **环境不是代码**）| status=active | supersedes=none

① **⚠️ 取证（本仓这一侧）** —— ❶ **synapse 的 `Tool` trait 与端脑的 agent loop 是**两套** registry**（端脑那套只有 MCP＋WASM 两个 provider）⇒ ⇒ 在 synapse 里写 `Tool` **端脑看不见** ⇒ ⇒ 所以这一层要**出成 MCP server**（`SimpleMcpServer`）⇒ ⇒ 端脑把它当普通 MCP 服务器注册（数据面，`McpServerRecord.transport`）⇒ ⇒ 既有先例：桌面端 `feishu_mcp` 就是这么出的。❷ **端脑的模型看到的是裸名**（`mcp:<server>:<tool>` 取最后一段）⇒ ⇒ 引擎自带的 `browser_*` 会**撞** Playwright MCP ⇒ ⇒ 这一层必须**自有名**。❸ 引擎是 0.x ⇒ ⇒ 工具名随时会变 ⇒ ⇒ 门面**运行时自省**（`tools/list` 拿清单，缺就报缺，**不静默**）。

② **✅ 落点 A：`src/tools/web/` 四个模块** —— **policy.rs**（门：scheme／字面 IP／特殊名字／名字解析后逐个答案过门／fake-IP 探测；IP 判定复用 `wasm::host_http::ip_is_disallowed_with`）｜**engine.rs**（`WebEngine` trait：`capabilities()`＋`tool_for()`＋`invoke()`；`ObscuraEngine` 走 MCP over HTTP，**运行时自省**引擎工具名）｜**mod.rs**（`WebSession`＋`WebPolicy`＋三个工具）｜**server.rs**（出 MCP server；工具失败按 MCP 约定回 `isError: true` 而不是 JSON-RPC 错误）。

③ **✅ 落点 B：门 + 只读优先** —— `WebPolicy::detect()` **探测**这台机器的解析器是不是 fake-IP（不是假设）⇒ ⇒ 是就只放开 benchmark 段；**字面**私网／环回／link-local／云元数据**一律拒**（引擎自己的逃生口会把云元数据一起放开 ⇒ ⇒ 这一层必须比它严）⇒ ⇒ `web_act` 默认关（端脑拦截点 P0 没有客户端确认流 ⇒ ⇒ `NeedsConfirmation` 等于被拒）。

④ **✅ 落点 C：引擎 client 必须 `no_proxy()`** —— ⚠️ 实测：Windows 系统代理开着（Clash `127.0.0.1:7897`）时，reqwest 把**回环**上的 MCP 端点也发给代理 ⇒ ⇒ 代理够不到那个端口 ⇒ ⇒ **502 Bad Gateway（空 body）**（.NET 默认绕过本地，reqwest 不绕）⇒ ⇒ 同一台机器上 PowerShell 探针能通、Rust 引擎却 502 ⇒ ⇒ 这条判据已钉进 `the_engine_reaches_a_local_mcp_endpoint_even_under_a_system_proxy`。

⑤ **⚠️ 途中四格** —— ❶ **判据不能机器依赖**（fake-IP 机器上域名会被门拒 ⇒ ⇒ 假引擎那几条改用**字面公网 IP**）｜❷ **负控要对着症状查**（先怪 MCP 握手 ⇒ ⇒ 对照实验证明真因是代理）｜❸ **「没握手就 502」是误判**（不握手也 200）｜❹ **全量 8 条红是环境不是代码**（负控：摘掉本刀整个模块仍红 ⇒ ⇒ 与影仓 U+3543 条同一个病）。

⑥ **✅ 判据与变异** —— `tools::web` **20 条**（门／截断／不切坏多字节／缺能力／失败不静默／交互默认拒／只露 `web_*`／MCP 失败回 `isError`／回环端点在代理下可达／真链路环境变量门控）；**变异三处当场红**（摘 `.no_proxy()` ⇒ 1 红；门放行字面私网 ⇒ 4 红；去掉能力自省 ⇒ 1 红），复原后 20/0 绿。

⑦ **⚠️ 闸读数** —— 本刀**只在 synapse 一个仓**落文件 ⇒ ⇒ 影仓那三道跨端闸（客户端工具／工具拦截／i18n）**不受影响**；⚠️ 存量缺陷登记：`synapse::mcp::StdioMcpClient` 用 LSP 式 `Content-Length` 分帧，**与 MCP 规范（按行 JSON）不符** ⇒ ⇒ 规范 MCP server 的 stdio 对不上 ⇒ ⇒ 本刀不代修。

⑧ **⚠️ 仍然欠着的** —— ❶ **端脑那侧没接线**（`ToolPolicy.allowed` 未加 `web_*`、`default_registry` 未挂这个 server ⇒ ⇒ **模型今天还看不到**）｜❷ 交互动作的确认流没有 ｜❸ 引擎进程的分发链（75MB 二进制＋worker＋V8 运行库）没设计 ｜❹ obscura **没有 Android/iOS build** ⇒ ⇒ 移动端没有位置 ｜❺ 只逐个对齐了 9 个引擎工具的参数名，其余几个（`browser_type`／`browser_scroll`／`browser_fill_form`）还没验 ｜❻ 引擎装的 sha256 校验／升级／崩溃自愈没做 ⇒ ⇒ **默认只能是「能力探测后按需开启」** ｜❼ 真 CI 待取。

⑨ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位（影仓那条占 U+3544）。

**Updated**: 2026-11-16 (**「出厂『模型 → 输入模态』清单 —— 一份真源，每条规则都带官方文档出处」** —— 承影仓 U+3543 条（输入格式收口成一份真源），本条记**落点**、**取证纪律**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=影仓 U+3543 条（用户追问逐字：「每个模型支持什么格式的输入现在是靠实时获取还是在 synapse 中有枚举类」）｜ 改动：`src/providers/input_modalities.rs`（**新**）＋ `src/providers/glob.rs`（**新**）＋ `src/providers/model_capabilities.rs`（匹配器搬去共用模块）＋ `src/providers/mod.rs` ⇒ ⇒ **4 个文件** ＋ 零新增依赖 ｜ 读数：`cargo test --lib providers::` **85 过 / 0 红**（新 12 条）＋ 全量 **431 过 / 0 红**（⚠️ 首跑 4 条红是本机代理把假上游截成 502 ⇒ ⇒ 带 `NO_PROXY=127.0.0.1` 重跑 ⇒ ⇒ 24/0 绿 ⇒ ⇒ **环境不是代码**）｜ status=active | supersedes=none

① **⚠️ 取证纪律：只写文档里真有的** —— 每条规则都带**官方文档地址** ⇒ ⇒ 界面能说「这是按哪份文档预设的」⇒ ⇒ 也能被人**逐条复核**。⇒ ⇒ **取不到官方文档逐字的 ⇒ 不登记**（宁可没有，不编）：**DeepSeek 整族不登记**（官方文档站在本机取到的页面里逐字没有模型名）⇒ ⇒ 落兜底 `text`（而这恰好是对的）；**Claude 那族**登记但注明可核性（官方页在本机**区域被屏蔽** ⇒ ⇒ 地址由 Anthropic 官方 `llms.txt` 索引逐字列出）。

② **✅ 落点** —— ❶ 新文件 `input_modalities.rs`：**词表**（`text`/`image`/`audio`/`video`/`pdf`）＋ **家族规则表**（每条带官方文档地址）＋ `catalog_input_modalities()` / `catalog_input_modalities_doc()` / `normalize_modality()`（上游词归一：`file` ⇒ `pdf`；`tools`/`function_calling` **不是模态** ⇒ ⇒ 丢掉）。❷ **与能力档是两张表**（那张判「是哪一类」，这张判「吃什么」）⇒ ⇒ 两个轴不许混进一张表。❸ 匹配语义与能力档**同一种**（`glob.rs` 抽成共用模块：`*` 包裹 ⇒ 子串命中；无前导 `*` ⇒ 严格前缀；规则按 pattern 长度降序 ⇒ 更具体的先试）。❹ **非 ASCII 的 id 不 panic**（字节下标落在字符中间 ⇒ ⇒ 改用 `get` 判 None 当不匹配）。

③ **✅ 判据与读数** —— ❶ **＋12 条**：八家家族逐条落到**文档说的**模态（夹具全是官方真 id）＋ 更具体的 pattern 先命中（`gpt-4o-transcribe`／`grok-voice-transcribe-2.0`／`grok-imagine-video-1.5` 三条同族陷阱）＋ **未登记的不许编**（DeepSeek ⇒ None）＋ 每条规则都带 https 文档地址 ＋ 模态词都在词表里 ＋ 文档查询与模态查询一致 ＋ 上游词归一 ＋ glob 四条。❷ `cargo test --lib providers::` **85 过 / 0 红**；全量 **431 过 / 0 红**。

④ **⚠️ 仍然欠着的** —— ❶ **Gemini 2.5/3.x 那一族没登记**（官方模型页逐字只覆盖到 Gemini 4 族）⇒ ⇒ 它们落兜底 `text` ⇒ ⇒ **偏保守** ⇒ ⇒ 要扩得去取那几页逐字。❷ **DeepSeek 整族不登记** ⇒ ⇒ 哪天取到官方逐字再登记。❸ **Claude 那族**要在能访问该页的机器上**再核一次逐字**。❹ 清单**只管云供应商模型**；本地/自持模型那份表在移动端（`known_models.dart`）⇒ ⇒ 两处**不许各抄一份**。

⑤ **⚠ 圈码：本仓账本体例无码位**（与本仓其余条目一致）⇒ ⇒ 本条不占码位。

**Updated**: 2026-11-16 (**「ADK 用户视角的闭环 —— 用 ADK 自己的 `RemoteA2aAgent` 跑通一轮，并做出因果负控：修之前那三个缺陷对 ADK 用户就是『答复到了却整轮被判失败』」** —— 承上一条（跨实现互证）末尾「仍然欠着」的第一格；本条记**取证（ADK 那个专属扩展的规范逐字 ＋ ADK 客户端判定逻辑逐字 ＋ 真跑一轮 ＋ 因果负控）**、**落点（跨实现探针进仓）**、**读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条的「仍然欠着」① ｜ 改动：`examples/adk_remote_probe.py`（**新**，ADK 自己的 `RemoteA2aAgent` ＋ 因果负控）＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ⇒ ⇒ **3 个文件**（**零 Rust 改动** —— 这一格是「测」，不是「改」）｜ 读数：LIVE **`LIVE_TEXT SYNAPSE_REPLIED_OK` ／ `LIVE_HAS_ERROR False`**；负控 PREFIX **`PREFIX_ERROR A2A response stream ended before task t-pre-fix reached a terminal state.`** ｜ 闸：Rust 侧未动 ⇒ ⇒ `cargo test --lib --tests` 仍全绿 ｜ status=active | supersedes=none

① **⚠️ 取证：ADK 那个专属扩展到底是什么（逐字）** —— ❶ 官方文档逐字（`adk.dev/a2a/a2a-extension/`，标题是「**A2A extension for improved reliability**」）：它「instructs the server to use the **updated agent executor** implementation」，修的是「Message duplication／Output misclassification／Sub-agent data loss」三件事，**全部发生在 ADK 自己的 executor 里**。❷ ⇒ ⇒ **它的语义是「用 ADK 的新 executor」** ⇒ ⇒ **synapse 不该宣称支持它**（synapse 没有 ADK executor）⇒ ⇒ 上一条判「ADK 专属、对 synapse 不适用」是对的，现在有逐字依据而不是推测。❸ **但客户端的判定是 wire 级的**（逐字）：`_remote_a2a_agent.py` 里 `if metadata and _compat.metadata_get(metadata, _NEW_A2A_ADK_INTEGRATION_EXTENSION):` ⇒ ⇒ 客户端看的是**响应 metadata 里有没有以那个 URI 为键** ⇒ ⇒ ADK 服务端自己写的是 `{ "<URI>": {"adk_agent_executor_v2": true} }` ⇒ ⇒ synapse 若要冒充就得写同样的键 ⇒ ⇒ **本刀不写**（不冒充一件自己做不到的事）。❹ ⇒ ⇒ **synapse 的合法回答已经实现了**：ADK 客户端总会请求这个 URI（`use_legacy=False` 默认加）⇒ ⇒ synapse 回 `unsupportedExtensions: ["https://google.github.io/adk-docs/a2a/a2a-extension/"]` ⇒ ⇒ **明确报出「不认识」而不是静默忽略** ⇒ ⇒ ADK 据此走 legacy 路径 ⇒ ⇒ 而那**本来就是**普通 A2A 对端该走的路（文档逐字「providing backward compatibility with existing A2A agents」）。❺ 顺带一条**头名事实**：ADK 的 `HTTP_EXTENSION_HEADER` 逐字是 **`A2A-Extensions`**（**规范当前名**，**不是** `X-A2A-Extensions`）⇒ ⇒ 上一条「两个头名都接」在 ADK 这条实际路径上接的是**规范那个**；`X-A2A-Extensions` 是更早草案的名字。

② **✅ 落点：跨实现探针进仓（零 Rust 改动）** —— ❶ 新增 `examples/adk_remote_probe.py`：**ADK 自己的 `RemoteA2aAgent`** ＋ `InMemoryRunner` ＋ `InMemorySessionService` ⇒ ⇒ **不需要模型 key**（`RemoteA2aAgent` 是纯转发，**它自己不调模型**）⇒ ⇒ 这一格**不必等 key**。❷ 探针里带**因果负控**：进程内起一个假 A2A 对端，**复刻修前的 synapse 行为**（答复以 artifact 到达、但任务不到终态）⇒ ⇒ 同一支探针、同一个 ADK 客户端，**唯一变量是「有没有终态事件」**。❸ 顺带堵掉一处探针自身的错：ADK 的 `Event.is_final_response` 是**方法**不是属性（第一版打印出来是个 bound method）。

③ **✅ 读数（真跑，逐字）** —— ❶ **LIVE（修好的 synapse）**：`LIVE_TEXT SYNAPSE_REPLIED_OK` ／ `LIVE_HAS_ERROR False` ／ `LIVE_ROUND_OK True` ⇒ ⇒ ADK 用户视角拿到的是**答复**、不是错误。ADK 的 `custom_metadata` 里逐字可见 `a2a:response` = `{'status': {'state': 'TASK_STATE_COMPLETED'}, 'artifacts': [{'name': 'response', 'parts': [{'text': 'SYNAPSE_REPLIED_OK'}]}]}` ＋ `a2a:request` 里用户消息**原样到达**（`'text': 'interop probe from ADK RemoteA2aAgent'`）。❷ **PREFIX（负控，假对端复刻修前行为）**：`PREFIX_ERROR A2A response stream ended before task t-pre-fix reached a terminal state.` ／ `PREFIX_HAS_ERROR True` ⇒ ⇒ **同一个 ADK 客户端、唯一变量是终态事件** ⇒ ⇒ 因果闭合。❸ ⚠️ **上一条那句「失败长得像成功」要改得更准**：负控那一轮**答复也到了**（`PREFIX_ANSWER PRE_FIX_ANSWER`）⇒ ⇒ 准确的说法是 **「答复到了，但 ADK 判它整轮失败」** ⇒ ⇒ 父 agent 拿到的是失败 ⇒ ⇒ 委派失败 ⇒ ⇒ **答复在手却把整轮丢了**。❹ **这一格不需要 Gemini key** ⇒ ⇒ 「仍然欠着」第一格的**协议侧已经闭合**。

④ **⚠️ 仍然欠着的** —— ❶ **ADK 作为服务端、用真模型回答的那一半，仍欠一个 Gemini key** ⇒ ⇒ 协议面两边都通（synapse→ADK 的消息到达执行器并跑到模型调用层，逐字只差 key）⇒ ⇒ **模型面**未证 ⇒ ⇒ 等 key 到位后，同一支 `adk_remote_probe.py` 换成指向 ADK 服务端即可补上。❷ 两支探针都依赖**临时目录的隔离 venv**（TEMP 下）⇒ ⇒ 复现要自己装 `google-adk[a2a]`（带入 `a2a-sdk 1.x`）。❸ 上一条的其余各格不变（单发路径看不到事件 metadata＝A2A v1.0 规范固有；`src/lobe/a2a_server.rs` 旧路由面未动；未提交；兄弟在飞面零碰）。❹ **ADK 专属扩展**：本刀**明确不宣称支持**（理由见①❷❸）⇒ ⇒ 记在这里，**免得被当成漏做** —— 若将来 synapse 真要跑 ADK 的 executor 语义，才轮到实现它。

**Updated**: 2026-11-16 (**「跨实现互证：拿 ADK 自己的 `a2a-sdk` 客户端真连 synapse —— 逮到三个把整轮变成失败的缺陷，并当场修掉」** —— 承上一条（A2A 升 v1.0 线）末尾明写的「仍然欠着」第一格：**仍未对着真实 ADK 跑互操测试**；本条记**取证（真装 `google-adk` 2.14.1 ＋ `a2a-sdk` 1.3.0，真起 ADK `api_server --a2a` 与真 synapse server，双向互证）**、**三个缺陷的根因与修法**、**判据与真变异**、**跨实现读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=上一条的「仍然欠着」①（跨实现互证）＋ 真实 ADK 实例 ｜ 改动：`src/a2a/server.rs`（执行器发终态事件 ＋ 卡片 `streaming` 说实话 ＋ 能力闸与卡片同源 ＋ token delta 的 artifact 建/续）＋ `tests/a2a_task_lifecycle.rs`（新，6 条）＋ `tests/a2a_extension_negotiation.rs`（线格式修回 protobuf-JSON ＋ EchoAgent 改回显 ＋ 断言「字真的到了」）＋ `examples/a2a_sdk_probe.py`（新，跨实现探针）＋ `examples/a2a_interop_server.rs`（lint）＋ `docs/concepts/a2a.md` ＋ `docs/zh/concepts/a2a.md` ⇒ ⇒ **7 个文件**（**不碰** `src/providers/catalog.rs` ／ `src/providers/openai.rs` —— 那是兄弟在飞面）｜ 读数：`cargo test --lib` **419 passed / 0 failed**（本刀新增 0、改了 1 条旧的）＋ `tests/a2a_task_lifecycle.rs` **6 passed / 0 failed**（新）＋ `tests/a2a_extension_negotiation.rs` **4 passed / 0 failed** ｜ 闸：clippy **0 error**，本刀文件 **0 warning** ｜ **⚠⚠ 真变异当场红**：掐掉终态事件 ⇒ ⇒ **恰好 5 条生命周期判据红**，读数逐字 `left: String("TASK_STATE_SUBMITTED")` ／ `right: "TASK_STATE_COMPLETED"`（＝升级前的行为）⇒ ⇒ 扩展那 4 条**仍绿** ⇒ ⇒ 变异精准命中、没误伤 ｜ status=active | supersedes=none

① **⚠️ 取证：两套实现都真跑起来了** —— ❶ **对端**：隔离 venv（TEMP 下）装 `google-adk[a2a]==2.14.1` ⇒ ⇒ 带入 **`a2a-sdk 1.3.0`**（＝A2A v1.0 线）⇒ ⇒ 拿它当**客户端**（ADK 的真实依赖，比 `RemoteA2aAgent` 更精准也更轻）—— **ADK 专属那一套没跑通不是死路**：协议层不依赖模型。❷ **ADK 服务端**：真起 `adk api_server --a2a --port 8001` ⇒ ⇒ 卡片在 **`/a2a/interop_agent/.well-known/agent-card.json`**（**不是**根路径）⇒ 卡片逐字 `protocolVersion: "0.3.0"`（ADK 2.14.1 仍走 0.3 兼容档）＋ `capabilities: {streaming: true}`。❸ **synapse 服务端**：`cargo run --example a2a_interop_server -- 9099` ⇒ 卡片逐字 `supportedInterfaces[0].protocolVersion = "1.0"`。❹ **版本闸在服务端是逐字实现的**（`a2a-server-lf-0.5.3/src/jsonrpc.rs` 的 `check_a2a_version`）：版本从 **`A2A-Version` 头**取（不是 params），**缺头按 `"0.3"` 解释**，而本构建只支持 major 1 ⇒ ⇒ 裸请求逐字被拒：`VERSION_NOT_SUPPORTED: version not supported: 0.3`（实测）。

② **⚠️ 跨实现才暴露的三个缺陷（升级后仍然全在）** —— ❶ **任务永远不到终态**：实测 a2a-sdk 客户端拿到的 Task 逐字 `TASK_STATE_SUBMITTED`，`GetTask` 也逐字 `TASK_STATE_SUBMITTED**，而 artifact 里**答复已经在**（`SYNAPSE_REPLIED_OK`）。根因：**`ArtifactUpdate` 不是终态事件**（A2A v1.0 只有带终态的 `Task` ／ `StatusUpdate` 才是），而 synapse 把代理的 `Final` 映射成 artifact ⇒ ⇒ 执行器从不发终态事件 ⇒ ⇒ 服务端 SDK 的 `is_terminal_event` 永远不成立。
　　⇒ ⇒ **ADK 自己的源码就是判决书**：`google/adk/a2a/agent/_remote_a2a_agent.py` 的 `_UNFINISHED_TASK_STATES = {TS_UNKNOWN, TS_SUBMITTED, TS_WORKING}` ＋ 逐字 `"A2A response stream ended before task {id} reached a terminal state."` ⇒ ⇒ ADK 把这种任务读成**流被截断**并抛错，**不是**把答复交回去 ⇒ ⇒ 「答复到了但任务没结束」在 ADK 眼里等于**失败**。
❷ **卡片谎报 `streaming: false`**：实测卡片逐字 `capabilities: {streaming: false, ...}`，而这个 server **确实**提供 `SendStreamingMessage`（JSON-RPC 路由里有 `handle_streaming_request`）⇒ ⇒ 代价是实打实的：a2a-sdk 客户端读卡片后**退回单发路径** ⇒ ⇒ 而 A2A v1.0 的单发路径返回的是**从 task_store 重新加载**的 Task ⇒ ⇒ **事件上的 metadata 整个丢掉** —— 实测单发路径上 `activatedExtensions` 读出来是 `{}`，流式路径上才看得到。
❸ **token delta 让整轮直接失败**：实测一个「先吐 token、再收尾」的代理 ⇒ ⇒ 服务端逐字回 `INVALID_AGENT_RESPONSE`（`invalid agent response`）。根因在服务端 SDK 的 `apply_event_to_task`：**`append=true` 要求 artifact 已经存在**（不存在就报错），而 synapse 把每个 token delta 都映射成 `append=true` ⇒ ⇒ 第一个 delta 就撞上 ⇒ ⇒ **真实聊天代理（会吐 token）整轮失败**。

③ **✅ 修法（三处，都在执行器周边）** —— ❶ **执行器发终态事件**（`src/a2a/server.rs`）：代理的流结束后，若还没有终态事件 ⇒ ⇒ 补一个 `StatusUpdate`：见过 `Final` ⇒ ⇒ `completed`；**没见过** ⇒ ⇒ `failed` ＋ 真因消息（**不许报完成** —— 报完成就是「失败长得像成功」）。❷ **卡片说实话 ＋ 能力闸与卡片同源**：`build_app_with_store` 里把卡片的 `capabilities.streaming` 置真（这个 server 只有一条执行路径：`stream_run`），并把**同一份**能力喂给 `DefaultRequestHandler::with_capabilities` ⇒ ⇒ 卡片与能力闸不再各说各话。❸ **token delta：第一个建 artifact，之后的才续**：服务端 SDK 的规则是 `append=false` 建/替换、`append=true` 续 ⇒ ⇒ 第一个 delta 走 `append=false`、后面的走 `append=true` ⇒ ⇒ 顺序反了就是「明明有输出，却整轮失败」。

④ **✅ 判据与真变异** —— ❶ 新增 `tests/a2a_task_lifecycle.rs`（**6 条**，全部真起 socket）：任务过网到终态／存下来的任务是终态／流式路径以终态收尾／卡片宣称 streaming／没给最终答复 ⇒ ⇒ 任务**失败**／真·流式代理也到终态。❷ 改 `tests/a2a_extension_negotiation.rs`：**线格式修回 protobuf-JSON**（`parts` 里的 oneof 成员是**摊平**的：`{"text": "..."}`；原来的 `{"content": {"text": "..."}}` 会被当成未知字段**丢掉** ⇒ ⇒ 请求照样成功、只是字一个都没到）＋ **EchoAgent 改成回显入参** ＋ 断言「字真的到了」⇒ ⇒ 那条判据从此不再只验头。❸ **⚠⚠ 真变异当场红**：把终态事件掐掉（`if false && !terminal_sent`）⇒ ⇒ **恰好 5 条生命周期判据红** ⇒ ⇒ 而扩展那 4 条**仍绿** ⇒ ⇒ 变异精准命中、没误伤。❹ 全量 `cargo test --lib --tests` 全绿；clippy **0 error** 且本刀文件 **0 warning**（顺手改掉自己那两条：example 里多余的 `&`、测试里多余的 `let` 绑定）。

⑤ **✅ 跨实现读数（修完之后）** —— ❶ **ADK 的 a2a-sdk 1.3.0 客户端 → synapse**：卡片逐字 `streaming: true` ／ `protocolVersion: 1.0` ／ 扩展清单在；**客户端默认路径**（`create_client` ＋ `send_message`）逐字：`artifact_update`（答复 `SYNAPSE_REPLIED_OK`）→ `status_update`（`TASK_STATE_COMPLETED`），且**扩展协商结果可见**（`activatedExtensions: ["https://example.com/ext/v1"]` ＋ `unsupportedExtensions: ["https://example.com/not-implemented/v1"]`）⇒ ⇒ 修之前同一路径上读出来是 `{}`。❷ `GetTask` 逐字 `TASK_STATE_COMPLETED`、artifact 逐字 `["SYNAPSE_REPLIED_OK"]` ⇒ ⇒ **存下来的任务是终态**（跨实现读数）。❸ **负控**：去掉 `A2A-Version` 头 ⇒ ⇒ 逐字 `VersionNotSupportedError version not supported: 0.3` ⇒ ⇒ 上面那次成功是**挣来的**，不是白捡的。❹ **反向（synapse client → ADK）**：卡片读到（`interop_agent` ／ `1.0.0`），消息**送达 ADK 的执行器**并跑到模型调用层，逐字只差 Gemini key（`No API key was provided...`）⇒ ⇒ **协议面通**；**模型面**欠一个 key（与协议无关）。❺ **跨档位也通**：ADK 侧卡片逐字仍是 `"0.3.0"`（那是 ADK 2.14.1 自己的档位）⇒ ⇒ a2a-sdk 1.x 会为 0.3 对端选兼容 transport ⇒ ⇒ synapse（1.0 线）↔ ADK（0.3 档）两边都通。

⑥ **⚠️ 仍然欠着的** —— ❶ **ADK 侧的完整闭环仍欠一个 key**：ADK 端用 Gemini 模型 ⇒ ⇒ 没 key 就只能证到「消息到达执行器并尝试调模型」⇒ ⇒ **协议通**已证，**模型通**未证 ⇒ ⇒ 要证到底得给 ADK 一个可用的 Gemini key（或换成不依赖模型的 ADK agent 形态）。❷ **ADK 专属扩展**（`https://google.github.io/adk-docs/a2a/a2a-extension/`）仍未实现 —— 理由同上一条：那是 ADK executor 语义（消息重复／输出误判／子代理丢数），**对 synapse 这套通用服务端不适用** ⇒ ⇒ 本仓实现的是规范**通用**扩展机制（两种头名都接 ＋ 不支持的报出来而不是消失）。❸ **故意走单发的客户端仍看不到扩展协商结果** —— 那是 A2A v1.0 **规范本身**的形状（单发返回的是从 store 重载的 Task，事件 metadata 不在其中）⇒ ⇒ 卡片已宣称 streaming ⇒ ⇒ 默认路径就走流式 ⇒ ⇒ 但这一格记在这里，不藏。❹ **`src/lobe/a2a_server.rs` 那一套旧路由面（`/.well-known/agent.json` ＋ `/a2a/tasks/send`）没动** —— 它是 lobe 的老面（且文件头声称「Wraps `A2AServerBuilder`」与实现不符）⇒ ⇒ 与本刀的 v1.0 JSON-RPC 面是两套 ⇒ ⇒ 本刀**不代修**，登记在此。❺ **未提交**：本刀全部改动留在工作树面（`git status` 可见）⇒ ⇒ 且 **`src/providers/catalog.rs` ／ `src/providers/openai.rs` 是兄弟会话的在飞面，一行未碰** ⇒ ⇒ 落链时必须按路径逐个 add，不许扫全量。

**Updated**: 2026-11-16 (**「供应商目录要能回答『这个上游压根没有模型列表接口』—— 按官方文档预设一份清单，并且不许把 MiniMax 拆成三条」** —— 承影仓 U+3541 条（用户诉求逐字：「为什么云供应商中minmax有三四个，而且有的无法获取可用模型。如果供应商不支持接口获取可用模型需要根据供应商接口文档预设」），本条记**取证（官方文档逐字 ＋ 逐条探测各家 /models 端点）**、**落点（目录加什么、收掉什么）**、**判据与读数**，以及**仍然欠着的**.)

> 信封：epoch=2026-11-16 | authority=measured | scope=project | source=用户诉求逐字（影仓 U+3541 条）＋ 官方文档逐字（MiniMax FAQ「模型列表」页 `platform.minimaxi.com/docs/faq/models.md`；Google `ai.google.dev/gemini-api/docs/models/gemini`；百度千帆 ERNIE 公告 `ai.baidu.com/ai-doc/WENXINWORKSHOP/4mchtzl8s`；MiniMax 模型列表接口 `platform.minimaxi.com/docs/api-reference/models/openai/list-models.md`）｜ 改动：`src/providers/catalog.rs`（新增 `PresetModel` ＋ `preset_models()` ＋ `preset_models_doc()` ＋ `PRESET_MODELS` 表 ＋ 收掉 `minimax-h3` 那条预设 ＋ 4 条新判据 ＋ 改 1 条旧判据）⇒ ⇒ **1 个文件** ＋ 零新增依赖 ｜ 读数：`cargo test --lib providers::catalog` **24 过 / 0 红**（上轮 20 ⇒ 新增 4）｜ status=active | supersedes=none

① **⚠️ 取证：官方文档逐字** —— ❶ **MiniMax** 的「模型列表」页逐字列出**可用**模型：语言 `MiniMax-M3.1-Flash`／`MiniMax-M3`；视频 `MiniMax-H3`／`MiniMax-Hailuo-2.3`／`MiniMax-Hailuo-2.3-Fast`／`MiniMax-Hailuo-02`；图片 `MiniMax-H3-Image`／`image-01`／`image-01-live`；语音 `speech-2.8-hd`／`speech-2.8-turbo`／`speech-2.6-hd`／`speech-2.6-turbo`／`speech-02-hd`／`speech-02-turbo`；而 M2.x／music-* 列在「**历史模型**」⇒ ⇒ **已下线的不许进清单**（抄进去就是「拿死模型当默认」的老毛病）。❷ **MiniMax 的模型列表接口是有的**（官方 `GET /v1/models`，回 `{"object":"list","data":[…]}`）⇒ ⇒ 「拉不到可用模型」**不是**因为 MiniMax 没这个接口。❸ **Google Gemini**：官方模型页逐字有 `gemini-2.5-pro`／`gemini-2.5-flash`／`gemini-2.5-flash-lite`／`gemini-3-flash-preview`／`gemini-3.8-flash`／`gemini-3.6-flash`／`gemini-3.5-flash-lite`／`gemini-3.1-flash-lite`／`gemini-4-argon`／`gemini-4-carbon`／`gemini-4-hydrogen` ⇒ ⇒ 而它的 **OpenAI 兼容面没有列表接口**（实测 `GET /v1beta/openai/models` ⇒ 404）。❹ **百度千帆**：官方 ERNIE 公告逐字有 `ernie-4.5-turbo-128k-preview`／`ernie-4.5-turbo-vl-preview`／`ernie-4.5-vl-28b-a3b`／`ernie-4.5-21b-a3b`／`ernie-4.5-0.3b` ⇒ ⇒ 而 `GET /v2/models` ⇒ **302**（不是列表）。

② **✅ 落点 A：目录里加一张「按官方文档预设的模型清单」** —— ❶ 新增 `PresetModel { id, capability }` ＋ `preset_models(preset_id)` ＋ `preset_models_doc(preset_id)`（**出处**＝官方文档地址）⇒ ⇒ 键是**预设 id**（不是供应商名）⇒ ⇒ 用户把 base URL 改成别家时这张表**不会被误用**。❷ 表里只登记三条预设：`minimax`／`minimax-cn`（同一份清单）＋ `gemini` ＋ `qianfan`。❸ **只写文档里真有的 id** —— 取不到官方文档就**不填**（宁可没有，不编）：编出来的 id 会让用户撞「模型不存在」，那比「没有清单」更坏。

③ **✅ 落点 B：`minimax-h3` 那条收掉（MiniMax 三条 ⇒ 两条）** —— ❶ 那条另立的理由先「**域不同**」（2026-11-12）⇒ ⇒ 域对齐后（2026-11-15）换成「**默认模型不同**」（它是出图模型）⇒ ⇒ **今天这条理由也没了**：一条供应商本来就能有**多个模型**（默认模型只是其中一条）⇒ ⇒ 有了预设清单 ⇒ ⇒ `MiniMax-H3-Image`（出图）／`MiniMax-H3`（视频）／`speech-*`（语音）本来就列在 `minimax` 那条里 ⇒ ⇒ 「为了出图再单开一条供应商」**不再需要**。❷ 保留的**两条**是**按区域**分的（国际 `api.minimax.io/v1`／国内 `api.minimax.cn/v1`）—— 那是真的两条线（两套域、两套计费、两种密钥）⇒ ⇒ 收掉的是**第三条**，不是按区域那两条。❸ 存量条目**不受影响**（预设只管「新建时的默认」⇒ ⇒ 已经在用 `minimax-h3` 的那台大脑，老条目自带 api_base ⇒ 照旧）。

④ **✅ 判据（新增 4 条 ＋ 改 1 条）** —— ❶ 目录里的 MiniMax **恰好两条**（`minimax`／`minimax-cn`）❷ MiniMax 那条清单里**必须有**出图／视频／语音模型（否则「收掉 H3 那条」＝「删功能」，那不叫去重）＋ **默认模型必须在清单里**（不在 ⇒ 用户建完第一次调用就撞「模型不存在」）❸ **已下线的模型不许进清单**（M2／music／abab 三族负控）❹ 没登记的预设就说「没有」（`None` 是合法答案，不许瞎填）❺ 改：旧那条断言「`minimax-h3` 存在」⇒ 改成「它**不在了**」。

⑤ **⚠️ 仍然欠着的** —— ❶ **讯飞星火（`spark`）没登记**：它的 `GET /v1/models` 回的是 HMAC 错（不是列表）⇒ ⇒ 确实该预设，但官方文档页取不到逐字 ⇒ ⇒ **不填**（宁缺勿错）。❷ **豆包（`doubao`）没登记**：`GET /api/v3/models` ⇒ 401 ⇒ ⇒ 接口在 ⇒ ⇒ 不该用预设盖住真实接口。❸ **Gemini 的出图／TTS／ASR 模型没进清单**（只收了对话那一族）⇒ ⇒ 出图模型在 OpenAI 兼容面上是否可用没逐条核 ⇒ ⇒ 保守只收对话族。❹ 预设清单的**模态词**在端脑那一侧是**读面**算的（出厂清单／族缀推断）⇒ ⇒ 清单里的 `capability` 只是文档原文的备份。

**Updated**: 2026-11-16 (A2A 升到 **v1.0 线** + **扩展协商真走通** —— 承影仓报「是否已完整兼容 adk.dev」) —— 实测发现两件事：① synapse 自己的 `synapse::adk` 与 Google ADK **同名不同物**（它是自研的 Rust 模块，与 adk.dev 无 API 同构关系）⇒ ⇒ 「兼容」只能是**协议层互操作**；② 当时真正通的只有 A2A 那一跳 ⇒ ⇒ 且停在**旧档位**（a2a-lf 0.3.0 → 协议版本写死 "0.3"）。**本刀：升至 A2A v1.0 线（`a2a-lf` 0.3.0→0.4.2，`a2a-server-lf` 0.4.0→0.5.3，`a2a-client-lf` 0.2.0→0.2.9，`a2a-pb` 0.1.8→0.3.2）且三方对齐（0.5.3 恰好依赖 a2a-lf 0.4.2 + a2a-pb 0.3.2）**；实现扩展激活（卡片声明 → 请求头读取 → 协商 → 结果回显）**；**读数：`cargo test --lib` **401→415 passed / 0 failed**（本刀新增 14）＋新集成判据 `tests/a2a_extension_negotiation.rs` **4 passed / 0 failed**（真起 socket）；**⚠⚠ 真变异 4 条全部当场红**（否控有牙）；clippy **0 error**（本刀不新增 warning）；⚠⚠ **两条红非本刀**：`mcp_test` 3 条（本机 `python3` 是 Microsoft Store 占位符 ⇒ 子进程秒退 ⇒ 设 `$env:PYTHON` 指真 Python 3.13 即全绿）与 `onepanel` 那条 doctest（`git diff` 对该文件为空 ⇒ 存量）。**仍然欠着的**：① **仍未对着真实 ADK 跑互操测试**（上面的 4 条只证「协议机制在 synapse 内部自洽」，跨实现互证**仍是空**）；② **ADK 专属扩展**（`https://google.github.io/adk-docs/a2a/a2a-extension/`）本刀**没有实现**—— 本刀做的是 A2A **规范通用扩展机制**（头名两种写法都接：规范当前 `A2A-Extensions` ＋ ADK 仍在用的 `X-A2A-Extensions`）⇒ ⇒ ADK 客户端能看懂 synapse 回了哪些扩展、哪些没被采纳（不支持的也报出来，不消失），但**ADK 那套 executor 语义（消息重复/输出误判/子代理丢数）synapse 不适用**；③ `A2A-Version` 头的位置：规范§3.6.1/3.6.2 规定**缺头时按 "0.3" 解释**⇒ ⇒ 新版 SDK 对 0.3 一律拒绝（实测：`VERSION_NOT_SUPPORTED: version not supported: 0.3`）⇒ ⇒ **所有对端必须显式发 `A2A-Version: 1.0`** ⇒ ⇒ synapse 自己的 client 由 SDK 自动发（`VERSION` 常量已是 "1.0"）不用改，但**手写对端必须自己发**；④ 本刀实现的是「头到了且回显了」，**不是「ADK 能调 synapse」**—— 真正的互证需要一个真 ADK `api_server --a2a` 实例。
**Updated**: 2026-11-15 (门的策略分层 —— **fake-IP 环境**下「解析后逐个查」会把**所有**公网名字都拒掉) —— 承本仓前一条（wasm 出网带门）与影仓㔫条⑬⑮；**实测到的环境事实**：本机 DNS 把**任何**域名解析成 `198.18.0.70`/`198.18.0.120`（Clash 类 fake-IP 栈；`api.open-meteo.com`→198.18.0.70、`github.com`→198.18.0.120）⇒ 而 198.18.0.0/15 是 IANA 的 benchmark 段（本地保留）⇒ ⇒ 原判据（**hostnames 一律解析后逐个查，任一答案命中即拒**）在这类机器上**把每个公网名字都拒掉** ⇒ ⇒ **整个出网能力在那类机器上等于不可用**（这不是判据的 bug，是「解析」这一步的前提在那台机器上不成立）；**策略分层怎么解**：`HttpPolicy::allow_fakeip_range`（默认**关**＝严）＋ `guard_url_with(url, policy)`（`guard_url(url)` 保持严格默认）＋ `HttpTransport::policy()`（默认实现＝严格；**宿主可自报**它要的策略，运行时把它同时用于 **ABI 侧的门**与 socket 侧的门 ⇒ 两道门同源，不漂移）；**该模式下门不再解析名字**（理由：fake-IP 下名字由**代理解析**，我们的解析结果无意义；且不解析 ⇒ 不依赖 DNS、也不误判）⇒ 只保留：**字面量**私网/环回/link-local 等仍拒、`localhost`/`.localhost`/`.local`/`.internal` 名字仍拒；**代价（写进文档，不藏）**：fake-IP 模式下**名字型 SSRF 不再由我们挡**（指向内网的名字与公网名字在解析上不可分）⇒ 那一段由**用户自己的代理策略**兜；读数：`cargo test --lib wasm::host_http` **19 passed / 0 failed**（新增三格：benchmark 段**默认仍拒**／放松档**只放那一个段**（私网/环回/特殊名字照拒）／**放松档不解析名字**（`.invalid` 也放行 ⇒ 证明不依赖 DNS；严格档同一个名字必须被拒）——⚠️ 严格档那格**只断言「被拒」不断言错因**，因为错因随解析器而变（fake-IP ⇒ LocalOrPrivate；解析器死 ⇒ Resolve））＋ 全库 `cargo test --lib` **391/0**（⚠️ 其间出现过**一次**红且**三次连跑未复现**，名字未捕获 ⇒ 记「一次性波动，未复现」不冒充已知）；⚠️ **仍然欠着的**：①「门不解析 ⇒ 无法自己判名字指向内网」这条代价没有第二道防线（要第二道得靠 transport 侧再解析/按 IP 直连）；② fake-IP 检测放在**宿主**（端脑）而不是 synapse（synapse 不该猜用户的网络环境）；③ 端脑侧「每台机器检测一次」的检测点/时点（启动 vs 首次请求）仍在端脑仓那一侧落地；④ 真公网端到端仍待真实例（假服务器只证形状);
**Updated**: 2026-11-15 (wasm 沙箱开出网能力 —— 但**带门**：`env.http_request` 宿主导入 ＋ SSRF 门 ＋ 限额 ＋ 传输由调用方注入) —— 承影仓台账㔫条⑫（用户裁定「开源 http 封装型 MCP ＝ 单开仓库打包成 wasm 插大脑」；本刀是该路线的**第一刀**：wasm 此前**完全出不了网**——`WasmRuntime::execute` 是纯字符串进出、grep `wasi|http|linker|import|Func::new` **零命中**、实例化藏在 execute 内 ⇒ 「打包成 wasm」若不能出网就等于废掉)；**归属切分**：**synapse 拥有门＋ABI＋限额＋缓冲纪律**（`src/wasm/host_http.rs`：**仅 http/https**、**拒绝 localhost/环回/私有/保留地址**（v4 RFC1918＋link-local＋multicast＋unspecified＋CGNAT＋doc＋benchmark；v6 loopback/ULA/link-local/multicast/unspecified；**v4-mapped v6 回查内嵌 v4**）、`localhost`/`.localhost`/`.local`/`.internal` 名字拒绝、**hostnames 一律解析后逐个查**（任一答案命中即拒 ⇒ DNS rebinding 到内网也拒）、userinfo/端口先剥再查）；**调用方注入 transport**（`HttpTransport` trait）⇒ synapse **不加 reqwest::blocking**、不引入嵌套 runtime（宿主 `call_wasm` 本就在 runtime 外跑）；**ABI**：`env.http_request(req_ptr,req_len,resp_ptr,resp_cap) -> i32`（模块自己管两块缓冲、宿主只往模块给的地址写；负数＝错误码，永不半截写入）；**限额**：每轮执行 `max_requests`（默认 8）／请求体 32 KiB／响应体 256 KiB；读数：`cargo test --lib wasm::host_http` **16 passed / 0 failed**（含**端到端 ABI 两条**：真模块调 import 打到私网 ⇒ 拿到 `E6`（门码）且**传输器一次没被叫**；无传输器时旧 `execute` 路径仍能实例化带该 import 的模块 ⇒ `E1`，旧路径不破）＋ 反向验证（**把门摘成放行 ⇒ 恰好 6 红**、含端到端那条 ⇒ 门有牙）＋ 全库 `cargo test --lib` **388 passed / 0 failed**；⚠️ **残留（已写进模块文档，不藏）**：门先解析再让传输器二次解析 ⇒ 有一段 TOCTOU，恶意 DNS 可竞之 ⇒ 收紧要靠「解析后按 IP 直连」（P4b 落传输器时定夺）；⚠️ 本刀**只给 wasm MCP 家族**用，不给通用 agent 沙箱开网);
**Updated**: 2026-11-12 (能力标注补 **MiniMax H3** 族 ＋ 新预设 `minimax-h3` —— 被问「支持 MiniMax 最新的 H3 image 吗」时实测：image_gen 词表只有 dall-e／flux／stable-diffusion／sd3／seedream／cogview／kolors／image-01 ⇒ ⇒ **`MiniMax-H3-Image` 一个都不命中 ⇒ 落 `None`**（能添加、但不被认成图片模型 ⇒ 不进 `image_gen` 分档候选）；⇒ ⇒ 补 `h3-image`／`minimax-h3` 两段，**不认宽泛的 `image`**（那也会命中视觉理解类 id，把理解模型标成出图模型会让下游真去调它 ⇒ 负控 `some-vl-image-understanding → None` 钉住）；⇒ ⇒ 预设**另立一条** `minimax-h3`（`https://api.minimax.io/v1`，default_model = `MiniMax-H3-Image`），**不动**老的 `minimax`／`minimax-abab`：H3 的域是 `api.minimax.io`（海外）／`api.minimax.cn`（国内）、出图走**复数** `POST /v1/images/generations`（OpenAI Images 兼容），老 `image-01` 走**单数** `/v1/image_generation` ⇒ 改老预设的 base_url 会把正在用它跑对话的人一起打断；⇒ 读数：`cargo test --lib providers::catalog` ⇒ **15 passed / 0 failed**（H3 三兄弟 `MiniMax-H3-Image`／`-Highspeed`／`-Context-IR` ＋ `minimax/h3-image` ＋ 视觉理解负控）；⚠️ 「识别成图片模型」≠「调得通」—— 出图由端脑 images 门面代理，而端脑那边原先的**本地约束（≤2048）会误杀云上游的大尺寸**（同刀修好，见 telencephalon ㌌ 条）);
2026-11-11 (云端模型目录**能力标注**补上生成族 —— 云端 `/models` 列表此前一律 `capability: None`（连 whisper-1／tts-1 都不标 ⇒ 界面分不出聊天／语音／生图）；新增 `catalog::infer_capability` —— **保守**推断，只认「名字自己说清模态」的族缀：rerank（**先于** embedding 判 —— `BAAI/bge-reranker-v2-m3` 两个族缀都有）／ embedding（embedding·-embed·bge-）／ asr（whisper）／ tts／ image_gen（dall-e·flux·stable-diffusion·sd3·seedream·cogview·kolors·image-01）／ video_gen（video-01·seedance·kling）；认不出 ⇒ `None` —— 宁缺勿错，聊天模型一个都不许误标；vision／ocr／world_gen 无公认名字形状，**不推断**（world_gen 预留）；`parse_openai_models`／`parse_claude_models` 两条云端解析腿都接上；读数：catalog 模块 **12 ⇒ 15 passed / 0 failed**（新增 3 组：族缀标注／openai 列表携带能力／claude 目录保守回 None）＋ providers 模块 **48 ⇒ 51** ＋ 全库默认 feature **364 passed / 0 failed**；⚠️ **落盘腿未动** —— 端脑 `fetch_models` 用自己的 `parse_models_from_response`（providers.rs 当时有在飞 M，按在飞纪律不碰）⇒ 预览腿已标注、刷新落盘腿暂不标注，这个不一致留给端脑那一刀);
2026-11-06 (OpenZiti v2 线收口 —— **真网 5 条 #[ignore] 全绿**：`cargo test --offline --features ziti --lib -- --ignored --nocapture` ⇒ `5 passed; 0 failed; finished in 6.48s`，EXIT=0（quickstart router @ 192.168.31.170 在线；client/host 身份在 shadow/.ziti/；宿主走生产 serve 入口 example `ziti_v2_brain`；逐条读数：HTTP 200＋体对 / WS 101＋回帧 / SSE 200＋text/event-stream＋分帧 / 本地代理 200＋体对 / 整链 overlay 自宿主自客户端一条命令全绿）；lib 测试 **371 passed / 0 failed / EXIT=0**，本线 3 处 warning 清零（live_tests 两处多余 mut ＋一处未用 import）；手册 `docs/concepts/ziti.md` 与本文件 ziti 段改写为 v2 自建叙事（第三方 ziti-sdk v1 已删出依赖图；`../ziti-rust` crates/ziti-v2 path 依赖；守门员闸 `check-ziti-v2-wire.mjs` ＋ `check-ziti-face.mjs` 全绿、阻断 0 项）);
2026-11-01 (synapse 库级默认监听改回环 —— 宽绑定必须显式；两条理由，缺一条都不够： ① **暴露面**（默认安装不该在局域网可读）； ② **形状**（`build()` 把 card 的 URL 直接从**监听地址**拼出来 ⇒ ⇒ 宽绑定 ⇒ ⇒ card 报 `http://0.0.0.0:8090` ⇒ ⇒ `0.0.0.0` **不是一个可连接的地址** ⇒ ⇒ 「注册成功、永远连不上」 —— 又一个「失败长得像成功」）； 新增 `lobe_app::DEFAULT_LOBE_BIND`（`127.0.0.1:8090`）与 `a2a::server::DEFAULT_A2A_BIND` （`127.0.0.1:8080`）两个常量 ⇒ ⇒ 宽绑定只能显式（`LOBE_BIND` / `.bind(…)`）； 证据：`cargo test --lib` **336 ⇒ 338 passed / 0 failed**（两条新守卫）⇒ ⇒ 反向验证 （两个常量改回宽绑定 ⇒ ⇒ **恰好 2 红**、其余 336 仍绿）⇒ ⇒ 父仓闸 `check-bind-exposure.mjs --lobes` **0 条 / EXIT=0**（默认档也 0 条）；提交 `1046ff7`);
2026-11-01 (「监听地址」与「公布地址」分成两个槽 —— 病根是**一个字段说两件事**：宽绑定时监听地址是 `0.0.0.0:19500`，而 `0.0.0.0` **不是一个可连接的地址**（它是「监听所有网卡」）⇒ ⇒ 单机上碰巧能通、换台机器永远连不上 ⇒ ⇒ 「注册成功、永远连不上」（又一个「失败长得像成功」）⇒ ⇒ 一份解析器落在 `synapse::net::bind`（`parse_bind` / `resolve_listen_and_public` / `sanitize_public_url` / `public_url_for_base`；端脑仓那份改成**委托**，路径不变 ⇒ ⇒ **别各写一套** —— 每个叶区仓都依赖 synapse、却都没有端脑依赖 ⇒ ⇒ 放端脑仓叶区就**用不上**）；⇒ ⇒ 叶区注册通道（`lobe::register_with_encephalon`）请求体多一个 `public_url` 槽（显式 `LOBE_PUBLIC_URL` 优先，没给就把监听地址**洗成可连接的**）；⇒ ⇒ ⚠️⚠️ 顺手逮到一条**更老的病**：**改名收口漏了共享库** —— 这个函数一直打在**老路径** `/encephalon/lobes/{name}`，而端脑 v0.3 只挂 `/telencephalon/lobes/{name}` ⇒ ⇒ **404** ⇒ ⇒ 注册**静默失败**（叶区只 `warn!` 一行）⇒ ⇒ 颞叶/顶叶/枕叶/前额叶**从来没注册成功过** ⇒ ⇒ 「端脑知道自己有哪些叶区」有一半是自己种的占位 ⇒ ⇒ 现在**先打新路径**、**只有 404** 才回落到老路径一次（401/403 **不**回落 ⇒ ⇒ 不掩盖鉴权问题）; 2026-10-30 (A2A 工具事件通道 `a2a::tool_event` —— 工具调用以前过不了 A2A 这一跳：官方 `StreamResponse` 只有 `Task`/`Message`/`StatusUpdate`/`ArtifactUpdate` 四个变体、**没有任何一类能承载工具调用**，服务端把 `ToolStart` 拍平成纯文本 `Calling tool: name(args)`、客户端又把不是失败的 `StatusUpdate` **全部丢弃** ⇒ 工具事件在线上被消灭，下游（如 telencephalon 端脑的 `tool_call` 分支）永远不可达。修法：用 `StatusUpdate.metadata` 开一条**结构化旁路**（键 `synapse_tool_event`，值 `{kind: start|call|end, ...}`），服务端写、客户端还原回 `AgentStreamEvent`；**纯文本照样保留** ⇒ 老对端行为不变、线上没删任何东西。反向验证：短路客户端还原 ⇒ 圆跳测试 `a2a::server::tests::tool_call_survives_the_full_a2a_hop` FAILED（`left: 0, right: 1`）); 2026-10-11 (two-locale docs site EN root + `docs/zh/`, copy-as-Markdown + `llms.txt` via postbuild, `examples/` runnable samples; `synapse::acp` client module — drives external ACP agents via the official `agent-client-protocol` 2.2 SDK per `1429a95`); 2026-09-25 (`transport::ziti` per `8ff2294` (2026-09-09), audio providers per `cdb6249` (2026-08-22); A2A metadata wire-forwarding via `__encephalon_meta_v1__` per `135f1fd`; WASM-A2A bridge sync path; zero-warnings pass prior baseline 2026-07-19)
**Status**: ✅ v0.4 lobe module complete + WASM-A2A sync bridge + A2A metadata round-trip via `__encephalon_meta_v1__` + `transport::ziti` zero-trust A2A over OpenZiti + `acp` client driving external ACP agents (opencode / claude-code-acp / codex-acp)

## Role

Shared foundation library for the Shadow platform. Provides A2A protocol, WASM runtime, LLM providers, and the new `synapse::lobe` module for stateless skill lifecycle.

## Architecture

```
synapse/src/
├── a2a/                    # Agent-to-Agent protocol (card, client, server)
├── acp/                    # ACP client (opencode / claude-code-acp / codex-acp via official agent-client-protocol 2.2 SDK; added 2026-09-26)
├── adk/                    # Agent Development Kit (OrchestratorAgent)
├── agent/                  # Agent trait + orchestrator
├── callbacks/              # Event callbacks
├── channels/               # Channel plugins (dock, router)
├── coding_tools/           # Code-shaped tool helpers (added 2026-07+)
├── components/             # Model, RAG, embedding, indexer
├── compose/                # Graph composition (chain, workflow)
├── llm/                    # LLM implementations (OpenAI, Anthropic)
├── lobe/                   # Stateless skill lifecycle glue (v0.4)
├── mcp/                    # Model Context Protocol
├── memory/                 # Memory helpers (added 2026-07+)
├── protocol/               # HybridRAG, Rerank, Retriever
├── providers/              # LLMProvider trait + implementations + audio::{Asr,Tts}Provider (added 2026-08-22)
├── scheduler/              # Task scheduling
├── schema/                 # Message, Content, RoleType
├── session/                # Session management
├── skills/                 # Skill definitions
├── stream/                 # Stream reader/writer
├── structured/             # Structured output helpers (added 2026-07+)
├── task/                   # Task management
├── thought/                # Thought chains
├── trace_context.rs        # Distributed tracing context (added 2026-07+)
├── transport/              # HTTP client
├── utils/                  # Utilities
└── wasm/                   # WASM runtime (package, manager, registry) — call_entry reads N bytes from linear memory at offset `ptr` (closes WASM-A2A sync bridge)
```

## Build

```bash
cd synapse
cargo build                    # default — no acp, no ziti, no audio
cargo build --features acp     # opt in to ACP client integration
```

## Test

```bash
cd synapse
cargo test --lib lobe          # lobe module tests
cargo test --lib acp           # ACP client (requires --features acp)
```

## lobe Module (v0.4)

The `synapse::lobe` module provides lifecycle glue for stateless skill executors:

### Traits
- `StatelessSkill` — Every lobe skill implements this
- `SkillContext` — Context passed to skill (user_id, history_summary, extra)
- `LobeEnv` — Env-driven config trait
- `EmotionProvider` — Emotion detection trait

### Helpers
- `HistorySummarizer` — LLM summarization on overflow
- `ProviderSet` — Bundles TTS/ASR/LLM/Emotion providers
- `register_with_encephalon()` — POST AgentCard to encephalon
- `build_a2a_router()` — axum router for A2A endpoints
- `LobeApp` — Convenience builder (register + serve in 10 lines)

### Defaults（默认监听 ⇒ 回环）
- `lobe_app::DEFAULT_LOBE_BIND` = `127.0.0.1:8090` —— `LobeApp::build()` 的默认监听
- `a2a::server::DEFAULT_A2A_BIND` = `127.0.0.1:8080` —— `serve_with_store()` 的默认监听
- ⚠️ **宽绑定必须显式**（`LOBE_BIND=0.0.0.0:PORT` 或 `.bind(…)`）⇒ ⇒ 两条理由，缺一条都不够：
  ① **暴露面** —— 默认安装不该在局域网可读；
  ② **形状** —— `build()` 把 card 的 URL 直接从**监听地址**拼出来 ⇒ ⇒ 宽绑定 ⇒ ⇒ card 报
     `http://0.0.0.0:8090` ⇒ ⇒ `0.0.0.0` **不是一个可连接的地址** ⇒ ⇒ 「注册成功、永远连不上」。
- ⚠️ 想跨机 ⇒ **两个都要显式**：监听地址 + 公布地址（`LOBE_PUBLIC_URL`；宽绑定时自动回落回环）。

### Re-exports
- `a2a::card::*` — AgentCard, Authentication, Capabilities, Provider, SkillReference
- `a2a::protocol::*` — Message, MessagePart, MessageRole, Task, TaskState
- `providers::LLMProvider` — LLM provider trait

## Dependencies

- tokio 1 — Async runtime
- serde 1 — Serialization
- async-trait 0.1 — Async traits
- axum 0.8 — HTTP server (for a2a_server)
- reqwest 0.13 — HTTP client (with multipart feature for audio providers)
- ziti-v2 (path dep `../ziti-rust/crates/ziti-v2`) — **optional**, behind `feature = "ziti"` (zero-trust A2A over OpenZiti; self-built **v2** wire layer — third-party ziti-sdk v1 removed from the dep graph 2026-11-04)
- agent-client-protocol 2.2 — **optional**, behind `feature = "acp"` (drives external ACP agents over stdio)

## `transport::ziti` module (added 2026-09-09)

Zero-trust A2A over OpenZiti. Gated by `feature = "ziti"`
because there is **no official OpenZiti Rust SDK** — the only
binding on crates.io is the third-party `jaschadub/ziti-rust`
v0.3.x (MIT, community-maintained). Default callers don't pull
`ziti-sdk`; opt in when you need dark-service mode.

Public surface:
- `ZitiGateway::new(identity_path)` — load an OpenZiti identity
  JWT, get a context.
- `gateway.dial(service_name)` / `gateway.listen(service_name)`
  — peer connection + dark-service listener.
- `ZitiListener::accept()` — pull one incoming `ZitiStream`.
- `serve(router, identity_path, service_name)` — one-call convenience
  that does identity load + dark-service register + accept loop +
  `serve_ziti_stream` per connection. Replaces the 4-line dance
  callers used to write.
- `serve_ziti_stream(stream, router)` — serve one axum `Router`
  on one accepted `ZitiStream` (in-process glue: hyper over Ziti).
- `ZitiHttpClient` / `ConnectionPool` / `ZitiConfig` /
  `ConfigWatcher` — outbound HTTP + reusable streams + JSON config.

### Wire layer (v2, self-built since 2026-11-04)

OpenZiti ships official SDKs for Go / C / Swift / Java / Python /
C# but **not Rust**. The old third-party `ziti-sdk` crate (v1
protocol) has been **removed from the dependency graph** — it
cannot connect to a v2 controller. The wire layer is now **ours**:
`ziti-v2` in the sibling repo `../ziti-rust` (`crates/ziti-v2`),
wired in as a path dependency. synapse keeps only a re-export
facade (`src/transport/ziti/v2/mod.rs`).

Guard rails: parent-repo gate `check-ziti-v2-wire.mjs` pins the
protocol constants / wire invariants / live-test entry points;
live proof in `src/transport/ziti/live_tests.rs` (5 `#[ignore]`d
tests over a real network) read **5 passed / 0 failed in 6.48s**
on 2026-11-06 over the quickstart router.

### Migration posture

The module is insulated from callers by the `ZitiGateway` /
`ZitiListener` / `ZitiStream` API. That insulation survived the
v1 → v2 swap (and the move to the `ziti-rust` repo) with zero
downstream changes; future wire swaps stay local to `ziti-v2` +
`gateway.rs` / `listener.rs`.

## `acp` Client module (added 2026-09-26)

Client-only integration that drives external coding agents speaking the
[Agent Client Protocol][acp] (opencode, claude-code-acp, codex-acp,
…) over stdio and surfaces their output as a normal Synapse
[`AgentStreamEvent`] stream. Implements the ACP *Client* role inside
Synapse; speaking the ACP *Agent* role is out of scope for this module.

[acp]: https://agentclientprotocol.com/

Public surface:
- `AcpAgentSpec::opencode()` / `claude_code()` / `codex()` — built-in
  presets for the three reference ACP shims; or `AcpAgentSpec::new(name, cmd)`
  + `.arg(...)` / `.env(k, v)` for fully custom commands.
- `AcpClient::connect(spec)` — spawn the agent subprocess, run the
  `initialize` handshake, start a long-lived dispatch task. Returns
  once the handshake completes; the task keeps running for the
  lifetime of the client.
- `client.session().prompt(text)` — send one prompt; returns a
  `StreamReader<AgentStreamEvent>`. Each call opens a fresh ACP
  session (`session/new`) on the shared connection — connection and
  initialize handshake are reused across prompts.
- `AcpError::Sdk` / `AcpError::Cancelled` — the error enum.

Wire model:

```text
 ┌────────────────┐    stdio (JSON-RPC)   ┌────────────────────────┐
 │ synapse::acp   │ ◀────────────────────▶│ opencode / claude-code  │
 │ AcpClient      │                        │ (external subprocess)   │
 └────────────────┘                        └────────────────────────┘
```

The SDK spawns the subprocess for us (via `AcpAgent::new(AcpAgentConfig)`);
we never touch `tokio::process::Command` directly. Each `AcpSessionHandle`
owns one ACP session over one stdio transport.

### Why the official SDK

Four correctness traps we explicitly avoided by depending on
`agent-client-protocol` instead of hand-rolling a JSON-RPC client:

1. **Protocol version negotiation** — the SDK downgrades/refreshes based
   on `InitializeResponse.protocol_version`. A hand-rolled client would
   freeze on `V1` and silently break against v2 agents.
2. **Request/response correlation** — `SentRequest::block_task().await`
   plus internal request-id bookkeeping eliminates the "which response
   belongs to which request" race. DIY clients almost always get this
   wrong eventually.
3. **`$/cancelRequest` semantics** — UI cancel round-trips correctly
   even when a proxy sits in front. DIY clients typically forget the
   JSON-RPC cancel notification and leak subprocess work after stop.
4. **stdio framing** — `Stdio` handles `Content-Length` headers vs
   line-delimited frames across agents. `AsyncBufReadExt::lines()` does
   not.

Bonus forward-compatibility the SDK buys us for free:
- `Conductor` / `Proxy` roles in `role::acp` are ready for multi-agent
  fan-out; we don't need to redesign the connection layer when
  shadow-desktop starts driving opencode + claude-code-acp side-by-side.
- `with_mcp_server` lets Synapse inject its own MCP servers into a
  session — agents call our tools without us opening a second port.
- `SessionUpdate` is `#[non_exhaustive]`, which forces
  `translate_update` to go through `serde_json` round-trip. New schema
  variants fall through to the `_ => Vec::new()` arm instead of breaking
  compilation.

### SDK caveat

ACP ships an official SDK for Rust — unlike OpenZiti — so there is no
vendor-strategy caveat here. The crate (`agent-client-protocol`, Apache-2.0)
is the official binding maintained at [agentclientprotocol/rust-sdk][sdk].
Pin to `2.2` for now; bump on minor releases only after reading the
changelog, since the public API is still pre-1.0.

[sdk]: https://github.com/agentclientprotocol/rust-sdk

### Build & test

Gated by `feature = "acp"` (default off) and **non-wasm only** — the
upstream SDK does not expose `AcpAgent` / `Stdio` on `target_arch =
"wasm32"`.

```bash
cargo build  -p synapse --features acp
cargo test   -p synapse --features acp --lib acp
```

Nine unit tests cover `AcpAgentSpec` builder behavior, the
`SessionUpdate → AgentStreamEvent` translator (Pending/InProgress →
`ToolStart`, Completed/Failed → `ToolEnd`, `AgentMessageChunk` →
`TokenDelta`), and the `SessionNotification` envelope parser
(envelope / bare-update / unparsable / round-trip).

### Migration posture

Callers depend only on `AcpClient` / `AcpSessionHandle` / `AcpAgentSpec`
/ `AcpError`. Wire-level types (`SessionUpdate`, `ToolCallStatus`, …)
are hidden behind `translate.rs`, so a future SDK major bump (v3) that
renames or restructures those types is a localized change to the
translator — public API stays put.

### Known gaps (future work)

- **Outbound `Dispatch::Request`** from the agent (e.g. fs/read) is
  answered with a generic `InternalError`. When the time comes to let
  agents call back into Synapse, plug `Builder::on_receive_request`
  into the `connect_with` chain.
- **Capability advertisement** — we always send `ProtocolVersion::V1`
  regardless of what the agent advertised. If we start depending on v2
  features (`V2SessionBuilder`, MCP-over-ACP), switch the
  `Client::default()` to `Client::default().v2()` so negotiation
  fails loud when the agent is too old.

## Audio Providers (added 2026-08-22)

```rust
use synapse::providers::audio::{AsrProvider, TtsProvider};

let asr = AsrProvider::for_endpoint("http://localhost:60080".into(), "whisper-1".into());
let text = asr.transcribe(audio_bytes, "wav", Some("zh")).await?;

let tts = TtsProvider::for_endpoint("http://localhost:60080".into(), "tts-1".into(), "alloy".into());
let mp3 = tts.synthesize("你好", "mp3").await?;
```

替代各 Agent 中直接 POST `/v1/audio/{transcriptions,speech}` 的样板代码。

## LLM Provider convenience (added 2026-08-22)

- `OpenAIProvider::for_endpoint(base_url, model)` — 内部 OpenAI 兼容 endpoint 构造（无 API key）
- `Message::user(text)` / `Message::assistant(text)` / `Message::system(text)` — 工厂方法
- `Message::text()` — 提取纯文本响应
- `AgentCard::placeholder(name)` — 占位 AgentCard（替代 `new(name, "", "", "0.0.0")`）
- `ReqwestHttpClient::with_timeout(Duration)` — 带超时的 reqwest HTTP 客户端

## Documentation / 文档索引

The docs site is a **two-locale VitePress tree**: English pages at the site root, 简体中文 pages under `docs/zh/` mirroring the same paths. VitePress `locales` in `docs/.vitepress/config.ts` renders the navbar language switcher; nav/sidebar labels are translated per locale. Content pages exist in BOTH trees — when you add or change a page under `docs/guide|concepts|howto|reference/` or `docs/index.md`, write the Chinese counterpart under `docs/zh/<same-path>` in the same change. Code blocks, identifiers and `/api/*` (rustdoc) links are shared and stay untranslated; internal site links in `zh/` pages are prefixed with `/zh`. Excluded from translation (stay English-only / root-only): `docs/CHANGELOG.md` (release log, high churn), `docs/SECURITY.md` / `docs/CONTRIBUTING.md` (1-line stubs), the legacy Chinese-primary guide pages (`framework-alignment`, `lcel-pipe`, `memory`, `observability`, `opentelemetry-*`, `structured-output`, `typed-lambda`) and `docs/superpowers/` (internal specs, not in nav).

**AI-friendliness is a build feature**: `docs/scripts/postbuild.mjs` (chained into `npm run build` / `build:pages`) copies every source `.md` into `dist/` at its original path (stable raw-markdown URLs — `https://ibloc.github.io/synapse/<path>.md`), and generates `llms.txt` (curated index) + `llms-full.txt` (full corpus) at the site root. The theme adds a "Copy page as Markdown" button under every page (`docs/.vitepress/theme/components/DocActions.vue`). Keep that script in the build chain when touching `docs/package.json`.

**Runnable examples** live in `examples/*.rs` at the repo root and are compiled by `cargo build --examples` — they are the executable source of truth for how to call the public API. When a how-to page's code block and an example disagree, the example (compiler-checked) wins; fix the docs. The Examples nav page (`docs/guide/examples.md` + `docs/zh/guide/examples.md`) indexes them.

Concept docs under `docs/concepts/` are the deep-dive for each major module. Concept 文档在 `docs/concepts/` 下，每个核心模块一份：

- [`docs/concepts/a2a.md`](docs/concepts/a2a.md) — A2A protocol (client + server + TaskStore trait) / A2A 协议（client + server + TaskStore trait）
- [`docs/concepts/acp.md`](docs/concepts/acp.md) — `synapse::acp` client driving opencode / claude-code-acp / codex-acp / 驱动 opencode / claude-code-acp / codex-acp
- [`docs/concepts/lobe.md`](docs/concepts/lobe.md) — `synapse::lobe` v0.4 stateless skill lifecycle glue / 无状态 skill 生命周期 glue
- [`docs/concepts/ziti.md`](docs/concepts/ziti.md) — `transport::ziti` zero-trust A2A over OpenZiti / 零信任 A2A over OpenZiti
- [`docs/concepts/architecture.md`](docs/concepts/architecture.md) — full module tree / 全模块树
- [`docs/concepts/mcp.md`](docs/concepts/mcp.md) — Model Context Protocol (client + server) / Model Context Protocol（client + server）
- [`docs/concepts/wasm.md`](docs/concepts/wasm.md) — WASM agent ABI v1 + WasiHttpClient / WASM agent ABI v1 + WasiHttpClient
- [`docs/concepts/multiagent.md`](docs/concepts/multiagent.md) — Router / Orchestrator / Handoff / Hybrid / 四种模式
- [`docs/concepts/agent.md`](docs/concepts/agent.md) · [`model.md`](docs/concepts/model.md) · [`tool.md`](docs/concepts/tool.md) · [`document-rag.md`](docs/concepts/document-rag.md) · [`llm-retry-policy.md`](docs/concepts/llm-retry-policy.md) · [`session-event-log.md`](docs/concepts/session-event-log.md) · [`tool-namespace.md`](docs/concepts/tool-namespace.md) · [`waterfall-events.md`](docs/concepts/waterfall-events.md) — remaining concept docs / 其余概念文档

Bilingual README at root: [`README.md`](README.md) (zh primary + en mirror / 中文主 + 英文镜像).

## Commits

| Hash | Description |
|------|-------------|
| 1728752 | chore: eliminate all remaining warnings (89 → 0) across the workspace |
| 135f1fd | feat: forward encephalon's packed `metadata['name']` through `into_a2a_message` via `__encephalon_meta_v1__` prefix (decode via `decode_metadata_from_name`) — closes the v0.1 metadata propagation gap |
| 9b19949 | docs: allow `ambiguous_glob_reexports` in lib.rs (cosmetic lint exemption, documented) |
| WASM-A2A bridge | `call_entry` reads N bytes from linear memory at offset `ptr` (was returning placeholder); combined with encephalon's `invoke` no longer discarding `_result`, real WASM agents with `handle(json) -> json` reach encephalon end-to-end |
| 754c5a8 | feat: add synapse::lobe module — v0.4 stateless skill lifecycle glue |
| 207bef6 | docs: migrate project-level + family docs to /home/ibloc/Projects/docs/ |
| c96f4f6 | docs: phase 4.5 status + prefrontal URL convention in shadow contract |
| 1429a95 | feat(acp): add `synapse::acp` client module — drive external ACP agents (opencode / claude-code-acp / codex-acp) over stdio via the official `agent-client-protocol` 2.2 SDK; gated by `feature = "acp"` (off by default, non-wasm only) |
