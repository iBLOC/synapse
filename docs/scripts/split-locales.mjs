#!/usr/bin/env node
// One-off migration: split the interleaved bilingual pages (English prose +
// Chinese translation lines) into two clean locale trees:
//   - English-only page stays at its current path (root locale)
//   - Chinese-only page is written to docs/zh/<same-relative-path>
//
// Handles all three bilingual line shapes found in the repo:
//   a) English paragraph followed by an italic Chinese line  (`*中文…*`)
//   b) inline `English / 中文` pairs (table cells, list items, bold labels)
//   c) legacy pages where the Chinese line is plain / bold-wrapped
//      (`***加粗** —— …*`) — treated as "Chinese-only line" on split.
import { readdirSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const docsDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const CJK = /[\u4e00-\u9fff\u3400-\u4dbf\uf900-\ufaff]/;
const EXCLUDE_DIRS = new Set(['.vitepress', 'node_modules', 'public', 'superpowers', 'zh']);
// Files whose bilingual format predates the "EN prose + italic ZH" convention
// (legacy Chinese-primary guides, plan docs): leave interleaved, don't split.
const SKIP_FILES = new Set([
  'CHANGELOG.md', 'SECURITY.md', 'CONTRIBUTING.md',
  'framework-alignment.md', 'lcel-pipe.md', 'memory.md', 'observability.md',
  'opentelemetry-evaluation.md', 'opentelemetry-integration.md',
  'structured-output.md', 'typed-lambda.md',
  'synapse-eino-alignment-plan.md', 'synapse-skills-a2a-plan.md',
]);

const hasCJK = (s) => CJK.test(s);

function walk(dir, rel = '') {
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (EXCLUDE_DIRS.has(entry.name)) continue;
      out.push(...walk(join(dir, entry.name), join(rel, entry.name)));
    } else if (entry.name.endsWith('.md') && !SKIP_FILES.has(entry.name)) {
      out.push(join(rel, entry.name).split('\\').join('/'));
    }
  }
  return out;
}

// Split a piece of text at the RIGHTMOST ` / ` whose right side has CJK and
// whose left side doesn't. Returns {en, zh} or null.
function splitBilingual(text) {
  let idx = text.lastIndexOf(' / ');
  while (idx !== -1) {
    const left = text.slice(0, idx);
    const right = text.slice(idx + 3);
    if (hasCJK(right) && !hasCJK(left)) return { en: left, zh: right };
    idx = text.lastIndexOf(' / ', idx - 1);
  }
  return null;
}

// Capture leading indent + optional list marker, and the leading/trailing
// emphasis wraps (***, **, *) around the body.
const LINE_RE = /^(\s*(?:\d+[.)]|[-*+])\s+)?(\s*)([\s\S]*?)(\s*)$/;
function parseLine(line) {
  const m = line.match(LINE_RE);
  let head = m[1] || '';
  let body = m[3];
  let pre = '';
  let post = '';
  const bm = body.match(/^(\*+)(.*?)(\*+)$/s);
  if (bm && bm[2].trim()) {
    pre = bm[1];
    body = bm[2].trim();
    post = bm[3];
  }
  return { head, indent: m[2], body, pre, post };
}

function rebuild(parts, pickedBody) {
  const wrap = parts.pre && parts.post ? (parts.pre.length <= parts.post.length ? parts.pre : parts.post) : parts.pre || parts.post;
  const inner = wrap.length ? pickedBody : pickedBody;
  return parts.head + parts.indent + wrap + inner + (wrap ? wrap : '') + '\n';
}

// Italic-only Chinese line (`*中文*` — bold `**…**` handled as emphasis wrap).
const isChineseItalic = (line) => {
  const t = line.trim();
  if (t.startsWith('**')) return false;
  return t.length > 2 && t.startsWith('*') && t.endsWith('*') && hasCJK(t.slice(1, -1));
};
const isQuoteChineseItalic = (line) => {
  const t = line.trim();
  if (!t.startsWith('>')) return false;
  return isChineseItalic(t.replace(/^>\s*/, ''));
};
const isFence = (line) => line.trim().startsWith('```');

function transform(lines, pick) {
  const out = [];
  let inFence = false;
  let fmState = 0;
  let pendingMarker = null; // list marker of a dropped EN item, for ZH

  for (const line of lines) {
    if (fmState < 2) {
      if (line.trim() === '---') {
        fmState = fmState === 0 ? 1 : 2;
        out.push(line);
        continue;
      }
      if (fmState === 1) { out.push(line); continue; }
    }
    if (isFence(line)) { inFence = !inFence; out.push(line); pendingMarker = null; continue; }
    if (inFence) { out.push(line); continue; }

    if (!line.trim()) { out.push(line); continue; }

    // Italic Chinese translation line.
    if (isChineseItalic(line)) {
      if (pick === 'zh') {
        const indent = line.match(/^\s*/)[0];
        const inner = line.trim().replace(/^\*+/, '').replace(/\*+$/, '').trim();
        out.push(pendingMarker ? `${indent}${pendingMarker} ${inner}` : `${indent}${inner}`);
      }
      pendingMarker = null;
      continue;
    }
    if (isQuoteChineseItalic(line)) {
      if (pick === 'zh') {
        const gt = line.match(/^(\s*>\s*)/)[0];
        const inner = line.trim().replace(/^>\s*/, '').trim().replace(/^\*+/, '').replace(/\*+$/, '').trim();
        out.push(`${gt}${inner}`);
      }
      pendingMarker = null;
      continue;
    }

    const trimmed = line.trim();

    // Heading `English · 中文`
    if (/^#{1,6}\s/.test(trimmed)) {
      const m = trimmed.match(/^(#{1,6}\s+)(.*)$/);
      const split = splitBilingual(m[2].replace(/ · /g, ' / ')); // headings use ` · `
      if (split) out.push(line.match(/^\s*/)[0] + m[1] + split[pick]);
      else out.push(line);
      pendingMarker = null;
      continue;
    }

    // Table row → per-cell split
    if (trimmed.startsWith('|')) {
      out.push(line.split('|').map((cell) => {
        const lead = cell.match(/^\s*/)[0];
        const trail = /\s*$/.exec(cell)[0];
        const body = cell.slice(lead.length, cell.length - trail.length);
        if (!body) return cell;
        const split = splitBilingual(body);
        if (!split) return cell;
        return lead + split[pick] + trail;
      }).join('|'));
      pendingMarker = null;
      continue;
    }

    // Blockquote
    if (trimmed.startsWith('>')) {
      const gt = line.match(/^(\s*>\s?)/)[0];
      const body = trimmed.replace(/^>\s?/, '');
      // Leading bold bilingual label with an English tail (`**X / Y** rest…`)
      // → EN keeps the label line; ZH drops it (full Chinese quote follows).
      const lbl = body.match(/^\*\*(.+?)\s\/\s([^*]+?)\*\*([\s\S]*)$/);
      if (lbl && !hasCJK(lbl[3])) {
        if (pick === 'en') out.push(gt + `**${lbl[1]}**${lbl[3]}`);
        pendingMarker = null;
        continue;
      }
      const split = splitBilingual(body);
      if (split) out.push(gt + split[pick]);
      else if (hasCJK(body) && pick === 'en') { /* pure Chinese quote → drop */ }
      else out.push(line);
      pendingMarker = null;
      continue;
    }

    // Leading bold bilingual label with English tail in prose/list lines.
    const boldLbl = trimmed.match(/^(\s*(?:\d+[.)]|[-*+])\s+)?\*\*(.+?)\s\/\s([^*]+?)\*\*([\s\S]*)$/);
    if (boldLbl && !hasCJK(boldLbl[4])) {
      if (pick === 'en') out.push(`${boldLbl[1] || ''}**${boldLbl[2]}**${boldLbl[4]}`);
      pendingMarker = null;
      continue;
    }

    // General prose / list line, possibly `English / 中文` inline.
    const parts = parseLine(line);
    const split = splitBilingual(parts.body);
    if (split) {
      const wrap = parts.pre && parts.pre.length && parts.post && parts.post.length
        ? (parts.pre.length <= parts.post.length ? parts.pre : parts.post)
        : '';
      out.push(parts.head + parts.indent + wrap + split[pick] + (wrap || ''));
      pendingMarker = null;
      continue;
    }
    if (hasCJK(parts.body)) {
      // Pure Chinese line (legacy plain / bold-wrapped).
      if (pick === 'zh') {
        let body = parts.body;
        // Uneven unwrap (`***加粗** text*` → pre=`***`, post=`*`): the extra
        // leading asterisks are the inner bold — put them back.
        if (parts.pre.length && parts.post.length && parts.pre.length > parts.post.length) {
          body = parts.pre.slice(0, parts.pre.length - parts.post.length) + body;
        }
        if (body.startsWith('*') && body.endsWith('*')) body = body.slice(1, -1).trim();
        out.push(parts.head + parts.indent + body);
      }
      // EN: drop.
      pendingMarker = null;
      continue;
    }
    // English-only line.
    if (pick === 'en') {
      out.push(line);
      pendingMarker = null;
      continue;
    }
    const li = trimmed.match(/^(\d+)[.)]\s/);
    const ul = trimmed.match(/^[-*+]\s/);
    pendingMarker = li ? `${li[1]}.` : ul ? '-' : null;
  }

  const collapsed = [];
  for (const l of out) {
    if (!l.trim() && collapsed.length && !collapsed[collapsed.length - 1].trim()) continue;
    collapsed.push(l);
  }
  while (collapsed.length && !collapsed[collapsed.length - 1].trim()) collapsed.pop();
  return collapsed.join('\n') + '\n';
}

// Site-internal links for the zh tree: /guide/x → /zh/guide/x (not /api/*).
function zhLinks(text) {
  return text.replace(/\]\((\/[^)\s]*)\)/g, (m, path) => {
    if (path.startsWith('/api') || path.startsWith('//')) return m;
    return `](/zh${path})`;
  });
}

const files = walk(docsDir);
let enCount = 0, zhCount = 0;
for (const rel of files) {
  const src = join(docsDir, rel);
  const lines = readFileSync(src, 'utf8').split(/\r?\n/);
  if (lines[lines.length - 1] === '') lines.pop();

  writeFileSync(src, transform(lines, 'en'));
  enCount++;

  const zhPath = join(docsDir, 'zh', rel);
  mkdirSync(dirname(zhPath), { recursive: true });
  writeFileSync(zhPath, zhLinks(transform(lines, 'zh')));
  zhCount++;
}
console.log(`done: ${enCount} EN pages stripped, ${zhCount} ZH pages written`);
