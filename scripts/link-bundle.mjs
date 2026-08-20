#!/usr/bin/env node
/**
 * 在仓库根目录放一个指向安装包的快捷方式
 *
 * `pnpm tauri build` 把包吐在 src-tauri/target/release/bundle/ 下面，路径深得离谱。
 * 克隆源码自己编译的人编译完常常找不到包在哪 —— 根目录放个链接，双击就到。
 *
 * Windows 用目录联接（junction），不需要管理员权限也不需要开开发者模式；
 * 其余系统用普通符号链接。链接本身进 .gitignore，不提交。
 */
import { existsSync, lstatSync, mkdirSync, rmSync, symlinkSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const target = join(root, 'src-tauri', 'target', 'release', 'bundle')
const link = join(root, '安装包')

if (!existsSync(target)) {
  console.log('还没有安装包 —— 先跑 pnpm tauri build，编译完再执行这个命令。')
  process.exit(0)
}

// 已经有了就先拆掉重建，免得指向旧路径
if (existsSync(link) || safeLstat(link)) {
  rmSync(link, { recursive: true, force: true })
}

mkdirSync(dirname(link), { recursive: true })
symlinkSync(target, link, process.platform === 'win32' ? 'junction' : 'dir')

console.log(`安装包 -> ${relative(root, target)}`)

/** 断掉的链接 existsSync 会返回 false，但它确实占着这个名字 */
function safeLstat(p) {
  try {
    return lstatSync(p)
  } catch {
    return null
  }
}
