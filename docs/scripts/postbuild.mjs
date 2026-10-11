#!/usr/bin/env node
// Post-build steps for AI-friendliness:
//   1. Copy every source .md (excluding internals) into dist/ at its
//      original relative path, so each page has a stable raw-markdown URL
//      (https://ibloc.github.io/synapse/<path>.md) — used by the in-page
//      "Copy page as Markdown" button and by AI crawlers.
//   2. Generate llms.txt (curated index) and llms-full.txt (full corpus)
//      per the llmstxt.org convention.

import { readdirSync, readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const docsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const distDir = resolve(docsDir, '.vitepress', 'dist');

// Same base normalization as write-base.mjs (MSYS-safe).
const raw = process.env.BASE_PATH || process.env.GITHUB_REPOSITORY?.split('/')[1] || 'synapse';
const repo = raw
  .replace(/^[A-Za-z]:.*?\//, '')
  .replace(/^\/+|\/+$/g, '')
  .split('/')
  .pop()
  .replace(/[^A-Za-z0-9._-]/g, '');
const base = `/${repo}/`;
const siteRoot = `https://ibloc.github.io${base}`;

const EXCLUDE_DIRS = new Set(['.vitepress', 'node_modules', 'public', 'superpowers']);

// Collect relative .md paths, with a crude section order for llms-full.txt.
function walk(dir, rel = '') {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (EXCLUDE_DIRS.has(entry.name)) continue;
      out.push(...walk(join(dir, entry.name), join(rel, entry.name)));
    } else if (entry.name.endsWith('.md')) {
      out.push(join(rel, entry.name).split('\\').join('/'));
    }
  }
  return out;
}

const allMd = walk(docsDir);

// Section weight for ordering the full corpus: landing first, then
// guide, concepts, howto, reference, then everything else alphabetically.
function sectionWeight(rel) {
  if (rel === 'index.md') return 0;
  if (rel.startsWith('guide/')) return 1;
  if (rel.startsWith('zh/guide/')) return 2;
  if (rel.startsWith('concepts/')) return 3;
  if (rel.startsWith('zh/concepts/')) return 4;
  if (rel.startsWith('howto/')) return 5;
  if (rel.startsWith('zh/howto/')) return 6;
  if (rel.startsWith('reference/')) return 7;
  if (rel.startsWith('zh/reference/')) return 8;
  if (rel.startsWith('zh/')) return 10;
  return 9;
}
const ordered = [...allMd].sort(
  (a, b) => sectionWeight(a) - sectionWeight(b) || a.localeCompare(b),
);

// --- 1. copy sources into dist --------------------------------------------
let copied = 0;
for (const rel of allMd) {
  const src = join(docsDir, rel);
  const dst = join(distDir, rel);
  mkdirSync(dirname(dst), { recursive: true });
  writeFileSync(dst, readFileSync(src));
  copied++;
}
console.log(`postbuild: copied ${copied} source .md files into dist (raw-markdown URLs)`);

// --- helpers ---------------------------------------------------------------
function stripFrontmatter(text) {
  if (text.startsWith('---')) {
    const end = text.indexOf('\n---', 3);
    if (end !== -1) return text.slice(text.indexOf('\n', end + 1) + 1);
  }
  return text;
}

function pageTitle(rel, text) {
  // frontmatter title wins; else first H1; else path.
  const fm = text.match(/^---\n([\s\S]*?)\n---/);
  if (fm) {
    const t = fm[1].match(/^title:\s*(.+)$/m);
    if (t) return t[1].trim();
  }
  const h1 = text.match(/^#\s+(.+)$/m);
  if (h1) return h1[1].trim();
  return rel;
}

function firstSentence(text) {
  const body = stripFrontmatter(text).trim();
  const para = body
    .split('\n\n')
    .find((p) => {
      const t = p.trim();
      return t && !t.startsWith('#') && !t.startsWith('```') && !t.startsWith('|') && !t.startsWith('>');
    });
  if (!para) return '';
  return para.replace(/\s+/g, ' ').slice(0, 160);
}

// --- 2. llms.txt (index) ---------------------------------------------------
const SECTION_TITLES = [
  [0, 'Home'],
  [1, 'Guide (English)'],
  [2, '指南 (中文)'],
  [3, 'Concepts (English)'],
  [4, '概念 (中文)'],
  [5, 'How-to (English)'],
  [6, '实操 (中文)'],
  [7, 'API Reference (English)'],
  [8, 'API 参考 (中文)'],
  [9, 'Project pages (English)'],
  [10, '项目页 (中文)'],
];

let llms = `# Synapse\n\n`;
llms += `> Synapse is a Rust-based intelligent agent development framework: agents, tools, RAG, MultiAgent, MCP, A2A, ACP, and WASM sandboxing. This file is generated at build time; every page below also has a raw-markdown URL (append .md to the page path).\n\n`;
llms += `The docs are bilingual: English at the site root, 简体中文 under /zh/. A full corpus is at ${siteRoot}llms-full.txt.\n\n`;

let lastWeight = -1;
for (const rel of ordered) {
  const text = readFileSync(join(docsDir, rel), 'utf8');
  const weight = sectionWeight(rel);
  if (weight !== lastWeight) {
    const title = SECTION_TITLES.find(([w]) => w === weight)?.[1] ?? 'Other';
    llms += `## ${title}\n\n`;
    lastWeight = weight;
  }
  const url = siteRoot + rel.replace(/\.md$/, '').replace(/index$/, '');
  const name = pageTitle(rel, text);
  const desc = firstSentence(text);
  llms += `- [${name}](${url})${desc ? ': ' + desc : ''}\n`;
}
writeFileSync(join(distDir, 'llms.txt'), llms);

// --- 3. llms-full.txt (corpus) ---------------------------------------------
let full = `# Synapse — full documentation corpus\n\n`;
full += `> Bilingual (English + 简体中文). Generated at build time from source markdown.\n\n`;
for (const rel of ordered) {
  const text = readFileSync(join(docsDir, rel), 'utf8');
  const url = siteRoot + rel.replace(/\.md$/, '').replace(/index$/, '');
  full += `\n---\n\n<!-- ${rel} -->\n<!-- URL: ${url} -->\n\n`;
  full += stripFrontmatter(text).trim() + '\n';
}
writeFileSync(join(distDir, 'llms-full.txt'), full);

console.log(`postbuild: wrote llms.txt (${ordered.length} pages indexed) + llms-full.txt`);
