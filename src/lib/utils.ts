import type { ClassValue } from "clsx"
import { clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Windows 上的用户目录，抓的是盘符和用户名那一段 */
const WIN_HOME = /^[A-Za-z]:\\Users\\[^\\]+/i

/**
 * 显示用的路径：把用户目录收成环境变量写法
 *
 * 全路径里带着 Windows 用户名，截图发到网上、贴报错日志的时候就跟着出去了。
 * 界面上没必要显示全路径 —— 要真路径旁边就有「打开配置目录」。
 * 落盘和传给后端的一律还是原路径，只有画到屏幕上的时候才收。
 */
export function shortPath(p: string): string {
  if (!p) return p
  const win = p.match(WIN_HOME)
  if (win) {
    const rest = p.slice(win[0].length)
    if (/^\\AppData\\Roaming/i.test(rest)) {
      return "%APPDATA%" + rest.slice("\\AppData\\Roaming".length)
    }
    if (/^\\AppData\\Local/i.test(rest)) {
      return "%LOCALAPPDATA%" + rest.slice("\\AppData\\Local".length)
    }
    return "%USERPROFILE%" + rest
  }
  // macOS / Linux
  return p.replace(/^\/(Users|home)\/[^/]+/, "~")
}
