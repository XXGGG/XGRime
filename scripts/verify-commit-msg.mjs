#!/usr/bin/env node
// 提交信息守门员 —— 复制到项目的 scripts/verify-commit-msg.mjs
//
// package.json 里挂上（配 simple-git-hooks）：
//   "simple-git-hooks": { "commit-msg": "node scripts/verify-commit-msg.mjs $1" }
//   然后跑一次 npx simple-git-hooks 装进 .git/hooks
//
// 挡住三种：格式不对、类型不在表里、说明是空的/敷衍的。
// 粒度太粗只警告不拦（拆不拆你自己判断）。

import { readFileSync } from 'node:fs'

// 跟 VSCode 的 gitmoji.addCustomEmoji 逐字一致
const TYPES = {
  init: '🎉', feat: '✨', fix: '🐛', ui: '💄', style: '🎨',
  refactor: '♻️', chore: '🧹', perf: '⚡️', docs: '📝', test: '🧪',
  deps: '📦', build: '🛠', ci: '🚨', wip: '🚧', prune: '🔥', revert: '⏪',
}

// 变体选择符 U+FE0F 有没有都算对
const norm = s => s.replace(/\uFE0F/g, '')

const die = (...lines) => {
  console.error(`\n  ✗ 提交被拦下\n`)
  lines.forEach(l => console.error(`    ${l}`))
  console.error(`\n    格式：<emoji> <类型>: <中文说明>`)
  console.error(`    例子：✨ feat: 参考图可以一次挑多张`)
  console.error(`    类型：${Object.entries(TYPES).map(([t, e]) => `${e} ${t}`).join('  ')}\n`)
  process.exit(1)
}

const file = process.argv[2]
if (!file) die('没收到提交信息文件路径')

const title = readFileSync(file, 'utf8')
  .split('\n')
  .filter(l => !l.startsWith('#'))[0]
  ?.trim() ?? ''

// 这些是 git 自己生成的，放行
if (/^(Merge |Revert "|fixup!|squash!)/.test(title)) process.exit(0)

const m = title.match(/^(\S+)\s+([a-z]+):\s*(.*)$/)
if (!m) {
  die(`现在是：${title || '(空)'}`, `emoji、类型、冒号，三样缺一不可。`)
}

const [, emoji, type, descRaw] = m
const desc = descRaw.trim()

if (!(type in TYPES)) {
  die(`「${type}」不在类型表里。`)
}

if (norm(emoji) !== norm(TYPES[type])) {
  die(`${type} 配的 emoji 是 ${TYPES[type]}，你写的是 ${emoji}。`)
}

if (!desc) {
  die(`「${type}:」后面是空的。`, `空说明等于没写，半年后你自己也看不懂这条改了啥。`)
}

// 敷衍的说明：只有日期、只有 null/wip、纯占位词
const LAZY = /^(null|none|test|tmp|临时|更新|修改|提交|优化|小改|小修|一波更新|wip)$/i
if (LAZY.test(desc) || /^\d{4}[-/.]\d{1,2}([-/.]\d{1,2})?$/.test(desc)) {
  die(`说明「${desc}」等于没说。`, `写清楚改了什么对象、结果是什么。日期 git 自己有。`)
}

if (desc.length < 4) {
  die(`说明「${desc}」太短了，写句人话。`)
}

// ---- 下面只警告，不拦 ----

// 中文按 2 个宽度算
const width = [...desc].reduce((n, c) => n + (c.charCodeAt(0) > 0x2E80 ? 2 : 1), 0)
if (width > 60) {
  console.warn(`\n  ⚠ 标题有点长（${width} 宽），细节挪到正文里去更好读。\n`)
}

// 一条提交塞了好几件事
const topics = desc.split(/\s*[+＋、；;]\s*/).filter(Boolean)
if (topics.length >= 3) {
  console.warn(`\n  ⚠ 这条标题串了 ${topics.length} 件事：`)
  topics.forEach(t => console.warn(`      · ${t}`))
  console.warn(`    一条提交只干一件事，方便以后单独回滚 / 查这一件事怎么改的。`)
  console.warn(`    真要拆：git reset HEAD~1 然后 git add -p 分批提交。\n`)
}

process.exit(0)
