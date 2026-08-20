import { createI18n } from 'vue-i18n'
import cmn_CN from './locales/cmn_CN'
import cmn_TW from './locales/cmn_TW'
import en from './locales/en'
import jyut from './locales/jyut'

/**
 * 界面语言
 *
 * 代码跟 README 的四份文件对齐：cmn_CN / cmn_TW / en / jyut。
 * 用 BCP 47 那套（zh-Hans 之类）反而说不清「粤语书面繁体」这种组合，
 * 干脆沿用项目自己的命名。
 */
export const LOCALES = [
  { code: 'cmn_CN', label: '简体中文' },
  { code: 'cmn_TW', label: '繁體中文' },
  { code: 'jyut', label: '廣東話' },
  { code: 'en', label: 'English' },
] as const

export type LocaleCode = (typeof LOCALES)[number]['code']

const STORAGE_KEY = 'xgrime.locale'

/** 猜一个初始语言：先看用户存过的，再看系统语言，最后退回简体 */
function initialLocale(): LocaleCode {
  const saved = localStorage.getItem(STORAGE_KEY)
  if (saved && LOCALES.some((l) => l.code === saved)) return saved as LocaleCode

  const langs = navigator.languages?.length ? navigator.languages : [navigator.language]
  for (const raw of langs) {
    const tag = (raw || '').toLowerCase()
    if (!tag) continue
    // 粤语的标记有 yue / zh-yue / zh-hk 好几种写法
    if (tag.startsWith('yue') || tag.startsWith('zh-yue')) return 'jyut'
    if (tag.startsWith('zh')) {
      // 港澳台和 Hant 都给繁体
      return /hant|tw|hk|mo/.test(tag) ? 'cmn_TW' : 'cmn_CN'
    }
    if (tag.startsWith('en')) return 'en'
  }
  return 'cmn_CN'
}

export const i18n = createI18n({
  legacy: false,
  locale: initialLocale(),
  // 缺键时退回简体，而不是把 key 本身显示给用户
  fallbackLocale: 'cmn_CN',
  messages: { cmn_CN, cmn_TW, en, jyut },
})

export function setLocale(code: LocaleCode) {
  i18n.global.locale.value = code
  localStorage.setItem(STORAGE_KEY, code)
  document.documentElement.lang = code === 'en' ? 'en' : 'zh'
}

export function currentLocale(): LocaleCode {
  return i18n.global.locale.value as LocaleCode
}
