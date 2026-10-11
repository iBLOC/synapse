<script setup>
// "Copy page as Markdown" + raw .md link, shown under every doc page.
//
// The postbuild script (docs/scripts/postbuild.mjs) copies every source
// .md file into dist at its original relative path, so `base + relativePath`
// is a stable raw-markdown URL — both for this button and for AI crawlers.
import { ref, computed } from 'vue'
import { useData } from 'vitepress'

const { page, site, lang } = useData()

const rawUrl = computed(() => {
  const base = site.value.base.endsWith('/') ? site.value.base : site.value.base + '/'
  return base + page.value.relativePath
})

const siteUrl = computed(() => {
  const base = site.value.base.endsWith('/') ? site.value.base : site.value.base + '/'
  return 'https://ibloc.github.io' + base + page.value.relativePath.replace(/\.md$/, '')
})

const isZh = computed(() => lang.value.startsWith('zh'))
const copyLabel = computed(() => (isZh.value ? '复制本页 Markdown' : 'Copy page as Markdown'))
const rawLabel = computed(() => (isZh.value ? '查看原始 .md' : 'View raw .md'))

const state = ref<'idle' | 'ok' | 'fail'>('idle')

async function copyMarkdown() {
  try {
    const res = await fetch(rawUrl.value)
    if (!res.ok) throw new Error(String(res.status))
    const text = await res.text()
    // In `vitepress dev` the .md path is served as a compiled JS module, not
    // raw markdown — detect that and refuse instead of copying garbage.
    if (text.trimStart().startsWith('<') || /^\s*import[\s{(]/m.test(text.slice(0, 200))) {
      throw new Error('not raw markdown (dev server?)')
    }
    await navigator.clipboard.writeText(text)
    state.value = 'ok'
  } catch {
    state.value = 'fail'
  }
  setTimeout(() => (state.value = 'idle'), 2000)
}
</script>

<template>
  <div class="doc-actions">
    <button class="doc-action-btn" type="button" @click="copyMarkdown">
      <span v-if="state === 'idle'">⧉ {{ copyLabel }}</span>
      <span v-else-if="state === 'ok'">✓ {{ isZh ? '已复制' : 'Copied!' }}</span>
      <span v-else>✕ {{ isZh ? '复制失败' : 'Copy failed' }}</span>
    </button>
    <a class="doc-action-link" :href="rawUrl" target="_blank" rel="noopener">{{ rawLabel }}</a>
    <span class="doc-action-ai" :title="siteUrl">🤖 {{ isZh ? 'AI 友好：每页均有 .md 原文与 llms.txt' : 'AI-friendly: every page has a raw .md URL; llms.txt at site root' }}</span>
  </div>
</template>

<style scoped>
.doc-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--vp-c-divider);
  font-size: 13px;
  flex-wrap: wrap;
}
.doc-action-btn {
  border: 1px solid var(--vp-c-divider);
  border-radius: 8px;
  padding: 3px 10px;
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-1);
  cursor: pointer;
  font-size: 13px;
}
.doc-action-btn:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}
.doc-action-link {
  color: var(--vp-c-text-2);
  text-decoration: none;
}
.doc-action-link:hover {
  color: var(--vp-c-brand-1);
}
.doc-action-ai {
  margin-left: auto;
  color: var(--vp-c-text-3);
}
</style>
