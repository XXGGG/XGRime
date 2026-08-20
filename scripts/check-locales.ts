// 两件事：
//   1. 四份界面语言的键一一对应 —— 少一个，那个语言下界面就直接露出 key
//   2. Rust 里定义的每个错误码，四份语言都有对应的 errors.<码>
//      后端只抛码不抛人话，漏一个用户就会看到一串 camelCase
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import cmn_CN from '../src/i18n/locales/cmn_CN.ts'
import cmn_TW from '../src/i18n/locales/cmn_TW.ts'
import en from '../src/i18n/locales/en.ts'
import jyut from '../src/i18n/locales/jyut.ts'

type Tree = Record<string, unknown>

function keysOf(obj: Tree, prefix = ''): string[] {
  return Object.entries(obj).flatMap(([k, v]) => {
    const path = prefix ? `${prefix}.${k}` : k
    return v && typeof v === 'object' && !Array.isArray(v) ? keysOf(v as Tree, path) : [path]
  })
}

const base = keysOf(cmn_CN as Tree)
const others: Array<[string, Tree]> = [
  ['cmn_TW', cmn_TW as Tree],
  ['en', en as Tree],
  ['jyut', jyut as Tree],
]

let bad = 0
for (const [name, tree] of others) {
  const have = new Set(keysOf(tree))
  const missing = base.filter((k) => !have.has(k))
  const extra = [...have].filter((k) => !base.includes(k))
  if (missing.length) {
    console.error(`${name} 缺 ${missing.length} 个键：`, missing.slice(0, 12).join(', '))
    bad++
  }
  if (extra.length) {
    console.error(`${name} 多出 ${extra.length} 个键：`, extra.slice(0, 12).join(', '))
    bad++
  }
}

// ── Rust 的错误码 ──
const here = dirname(fileURLToPath(import.meta.url))
const errorRs = readFileSync(join(here, '..', 'src-tauri', 'src', 'error.rs'), 'utf8')
const codes = [...errorRs.matchAll(/pub const [A-Z_]+: &str = "(\w+)";/g)].map((m) => m[1])

if (codes.length === 0) {
  console.error('没从 src-tauri/src/error.rs 里读到任何错误码，是不是改了写法？')
  process.exit(1)
}

const all: Array<[string, Tree]> = [['cmn_CN', cmn_CN as Tree], ...others]
for (const [name, tree] of all) {
  const errs = (tree as { errors?: Record<string, string> }).errors ?? {}
  const missing = codes.filter((c) => !(c in errs))
  const stray = Object.keys(errs).filter((k) => !codes.includes(k))
  if (missing.length) {
    console.error(`${name} 缺错误码翻译：`, missing.join(', '))
    bad++
  }
  if (stray.length) {
    console.error(`${name} 有多余的错误码（Rust 里已经没有了）：`, stray.join(', '))
    bad++
  }
}

if (bad) process.exit(1)
console.log(`四份语言各 ${base.length} 个键、${codes.length} 个错误码，完全对齐`)
