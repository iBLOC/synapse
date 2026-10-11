import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Synapse',
  description: 'Rust-based intelligent agent development framework',
  // Project Pages serves the site at /<repo>/. `build:pages` rewrites this
  // line via scripts/write-base.mjs; `npm run dev` keeps '/' so the
  // local server works without any extra setup.
  base: "/synapse/",
  cleanUrls: true,
  lastUpdated: true,
  appearance: 'dark',
  ignoreDeadLinks: true,

  // Two locales: English at the root (existing URLs preserved), Simplified
  // Chinese under /zh/. VitePress renders the navbar language switcher
  // automatically once `locales` is present. Pages exist as separate trees:
  // docs/guide/intro.md (EN) ↔ docs/zh/guide/intro.md (ZH).
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      themeConfig: {
        siteTitle: 'Synapse',

        nav: [
          { text: 'Guide', link: '/guide/intro' },
          { text: 'Concepts', link: '/concepts/architecture' },
          { text: 'How-to', link: '/howto/simple-agent' },
          { text: 'Examples', link: '/guide/examples' },
          {
            text: 'API',
            items: [
              { text: 'API Reference', link: '/reference/agent-trait' },
              { text: 'Rustdoc', link: '/api/synapse/' },
            ],
          },
          {
            text: 'v0.1.0',
            items: [
              { text: 'Changelog', link: '/CHANGELOG' },
              { text: 'Roadmap', link: '/roadmap' },
              { text: 'Comparison', link: '/comparison' },
            ],
          },
        ],

        sidebar: {
          '/guide/': [
            {
              text: 'Introduction',
              items: [
                { text: 'What is Synapse?', link: '/guide/intro' },
                { text: 'Quick Start', link: '/guide/quick-start' },
                { text: 'Examples', link: '/guide/examples' },
              ],
            },
            {
              text: 'Concepts',
              items: [
                { text: 'Architecture', link: '/concepts/architecture' },
                { text: 'Agent', link: '/concepts/agent' },
                { text: 'Tool', link: '/concepts/tool' },
                { text: 'Model', link: '/concepts/model' },
                { text: 'Document & RAG', link: '/concepts/document-rag' },
                { text: 'MultiAgent', link: '/concepts/multiagent' },
                { text: 'Lobe', link: '/concepts/lobe' },
                { text: 'MCP', link: '/concepts/mcp' },
                { text: 'A2A Protocol', link: '/concepts/a2a' },
                { text: 'ACP Client', link: '/concepts/acp' },
                { text: 'WASM Sandbox', link: '/concepts/wasm' },
                { text: 'Ziti Transport', link: '/concepts/ziti' },
              ],
            },
          ],
          '/howto/': [
            {
              text: 'How-to Guides',
              items: [
                { text: 'Build a Simple Agent', link: '/howto/simple-agent' },
                { text: 'Add Tools to an Agent', link: '/howto/agent-with-tools' },
                { text: 'Multi-turn ReAct Agent', link: '/howto/react-agent' },
                { text: 'Build a RAG Pipeline', link: '/howto/rag-pipeline' },
                { text: 'MultiAgent: Router', link: '/howto/multiagent-router' },
                { text: 'MultiAgent: Orchestrator', link: '/howto/multiagent-orchestrator' },
                { text: 'MultiAgent: Handoff', link: '/howto/multiagent-handoff' },
                { text: 'MultiAgent: Hybrid', link: '/howto/multiagent-hybrid' },
                { text: 'Connect to MCP Server', link: '/howto/mcp-server' },
                { text: 'Cross-Process A2A', link: '/howto/a2a-cross-process' },
                { text: 'Deploy as WASM Module', link: '/howto/wasm-deploy' },
              ],
            },
          ],
          '/reference/': [
            {
              text: 'API Reference',
              items: [
                { text: 'Agent Trait', link: '/reference/agent-trait' },
                { text: 'BaseChatModel', link: '/reference/base-chat-model' },
                { text: 'DocumentTransformer', link: '/reference/document-transformer' },
                { text: 'MultiAgent Patterns', link: '/reference/multiagent-patterns' },
                { text: 'Transport Layer', link: '/reference/transport' },
              ],
            },
          ],
        },

        socialLinks: [
          { icon: 'github', link: 'https://github.com/iBLOC/synapse' },
        ],

        footer: {
          message: 'Released under the Apache License 2.0',
          copyright: `Copyright © 2024-2026 Synapse Contributors`,
        },

        editLink: {
          pattern: 'https://github.com/iBLOC/synapse/edit/master/docs/:path',
          text: 'Edit this page on GitHub',
        },

        search: {
          provider: 'local',
        },

        outline: {
          level: [2, 3],
          label: 'On this page',
        },

        docFooter: {
          prev: 'Previous',
          next: 'Next',
        },
      },
    },

    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: {
        siteTitle: 'Synapse',

        nav: [
          { text: '指南', link: '/zh/guide/intro' },
          { text: '概念', link: '/zh/concepts/architecture' },
          { text: '实操', link: '/zh/howto/simple-agent' },
          { text: '示例', link: '/zh/guide/examples' },
          {
            text: 'API',
            items: [
              { text: 'API 参考', link: '/zh/reference/agent-trait' },
              { text: 'Rustdoc', link: '/api/synapse/' },
            ],
          },
          {
            text: 'v0.1.0',
            items: [
              { text: '更新日志', link: '/CHANGELOG' },
              { text: '路线图', link: '/zh/roadmap' },
              { text: '对比', link: '/zh/comparison' },
            ],
          },
        ],

        sidebar: {
          '/zh/guide/': [
            {
              text: '入门',
              items: [
                { text: 'Synapse 是什么？', link: '/zh/guide/intro' },
                { text: '快速开始', link: '/zh/guide/quick-start' },
                { text: '示例', link: '/zh/guide/examples' },
              ],
            },
            {
              text: '概念',
              items: [
                { text: '架构', link: '/zh/concepts/architecture' },
                { text: 'Agent', link: '/zh/concepts/agent' },
                { text: 'Tool', link: '/zh/concepts/tool' },
                { text: 'Model', link: '/zh/concepts/model' },
                { text: 'Document 与 RAG', link: '/zh/concepts/document-rag' },
                { text: 'MultiAgent', link: '/zh/concepts/multiagent' },
                { text: 'Lobe', link: '/zh/concepts/lobe' },
                { text: 'MCP', link: '/zh/concepts/mcp' },
                { text: 'A2A 协议', link: '/zh/concepts/a2a' },
                { text: 'ACP 客户端', link: '/zh/concepts/acp' },
                { text: 'WASM 沙箱', link: '/zh/concepts/wasm' },
                { text: 'Ziti 传输层', link: '/zh/concepts/ziti' },
              ],
            },
          ],
          '/zh/howto/': [
            {
              text: '实操指南',
              items: [
                { text: '构建一个简单 Agent', link: '/zh/howto/simple-agent' },
                { text: '给 Agent 添加 Tool', link: '/zh/howto/agent-with-tools' },
                { text: '多轮 ReAct Agent', link: '/zh/howto/react-agent' },
                { text: '构建 RAG Pipeline', link: '/zh/howto/rag-pipeline' },
                { text: 'MultiAgent：路由', link: '/zh/howto/multiagent-router' },
                { text: 'MultiAgent：编排', link: '/zh/howto/multiagent-orchestrator' },
                { text: 'MultiAgent：Handoff', link: '/zh/howto/multiagent-handoff' },
                { text: 'MultiAgent：混合', link: '/zh/howto/multiagent-hybrid' },
                { text: '连接 MCP Server', link: '/zh/howto/mcp-server' },
                { text: '跨进程 A2A', link: '/zh/howto/a2a-cross-process' },
                { text: '部署为 WASM 模块', link: '/zh/howto/wasm-deploy' },
              ],
            },
          ],
          '/zh/reference/': [
            {
              text: 'API 参考',
              items: [
                { text: 'Agent Trait', link: '/zh/reference/agent-trait' },
                { text: 'BaseChatModel', link: '/zh/reference/base-chat-model' },
                { text: 'DocumentTransformer', link: '/zh/reference/document-transformer' },
                { text: 'MultiAgent 模式', link: '/zh/reference/multiagent-patterns' },
                { text: 'Transport 层', link: '/zh/reference/transport' },
              ],
            },
          ],
        },

        socialLinks: [
          { icon: 'github', link: 'https://github.com/iBLOC/synapse' },
        ],

        footer: {
          message: '基于 Apache License 2.0 发布',
          copyright: `版权所有 © 2024-2026 Synapse Contributors`,
        },

        editLink: {
          pattern: 'https://github.com/iBLOC/synapse/edit/master/docs/:path',
          text: '在 GitHub 上编辑此页',
        },

        search: {
          provider: 'local',
          options: {
            translations: {
              container: {
                placeholder: '搜索文档',
                resetButton: { titleText: '清除查询', buttonText: '清除' },
                noResultsText: '没有找到结果',
              },
              button: { buttonText: '搜索文档', buttonAriaLabel: '搜索文档' },
            },
          },
        },

        outline: {
          level: [2, 3],
          label: '本页内容',
        },

        docFooter: {
          prev: '上一篇',
          next: '下一篇',
        },
      },
    },
  },

  head: [
    ['meta', { name: 'theme-color', content: '#ff6b35' }],
    ['meta', { property: 'og:title', content: 'Synapse — Agent Development Framework' }],
    ['meta', { property: 'og:description', content: 'Rust agent framework: ReAct, MultiAgent, RAG, MCP, A2A, WASM' }],
    ['meta', { property: 'og:type', content: 'website' }],
  ],

  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark',
    },
    lineNumbers: false,
    container: {
      tipLabel: 'Tip',
      warningLabel: 'Warning',
      dangerLabel: 'Danger',
      infoLabel: 'Info',
      detailsLabel: 'Details',
    },
  },
})
