import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Synapse',
  description: 'Rust-based intelligent agent development framework',
  lang: 'en-US',
  cleanUrls: true,
  lastUpdated: true,
  appearance: 'dark',
  ignoreDeadLinks: true,

  head: [
    ['meta', { name: 'theme-color', content: '#ff6b35' }],
    ['meta', { property: 'og:title', content: 'Synapse — Agent Development Framework' }],
    ['meta', { property: 'og:description', content: 'Rust agent framework: ReAct, MultiAgent, RAG, MCP, A2A, WASM' }],
    ['meta', { property: 'og:type', content: 'website' }],
  ],

  themeConfig: {
    siteTitle: 'Synapse',

    nav: [
      { text: 'Guide', link: '/guide/intro' },
      { text: 'Concepts', link: '/concepts/architecture' },
      { text: 'How-to', link: '/howto/simple-agent' },
      { text: 'API', link: '/reference/agent-trait' },
      {
        text: 'v0.1.0',
        items: [
          { text: 'Changelog', link: '/changelog' },
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
            { text: 'MCP', link: '/concepts/mcp' },
            { text: 'A2A Protocol', link: '/concepts/a2a' },
            { text: 'WASM Sandbox', link: '/concepts/wasm' },
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
      { icon: 'github', link: 'https://github.com/synapse-rs/synapse' },
    ],

    footer: {
      message: 'Released under the Apache License 2.0',
      copyright: `Copyright © 2024-2026 Synapse Contributors`,
    },

    editLink: {
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

  editLink: {
    pattern: 'https://github.com/synapse-rs/synapse/edit/master/docs/:path',
    text: 'Edit this page on GitHub',
  },
})
