#!/usr/bin/env node
// Rewrites the `base:` line in docs/.vitepress/config.ts so vitepress
// build resolves assets under the GitHub Pages subpath. Default target
// is '/synapse/', matching project pages at /<repo>/.
//
// We rewrite by reading the file as bytes, locating the `base:` line
// in any line-ending convention (LF / CRLF / CR), and patching in place.
// This avoids regex pitfalls with mixed line endings on Windows.

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const raw = process.env.BASE_PATH || process.env.GITHUB_REPOSITORY?.split('/')[1] || 'synapse';
// Normalize whatever shape it came in as. On Linux runners this is just
// `/synapse` or `synapse`; on Git Bash (Windows) MSYS rewrites `/synapse`
// into `/Program Files/Git/synapse`, so we keep only the last segment.
const repo = raw
  .replace(/^[A-Za-z]:.*?\//, '')
  .replace(/^\/+|\/+$/g, '')
  .split('/')
  .pop()
  .replace(/[^A-Za-z0-9._-]/g, '');
const base = `/${repo}/`;

const configPath = resolve(here, '..', '.vitepress', 'config.ts');
const buf = readFileSync(configPath);

// Split on CRLF, CR, or LF so we tolerate any line ending on disk.
const lines = buf.toString('utf8').split(/\r\n|\r|\n/);
const idx = lines.findIndex((l) => /^\s*base\s*:/.test(l));
if (idx === -1) {
  console.error('write-base: no base: line found in ' + configPath);
  process.exit(1);
}
lines[idx] = `  base: ${JSON.stringify(base)},`;

writeFileSync(configPath, lines.join('\n'));
console.log('write-base: base = ' + base);