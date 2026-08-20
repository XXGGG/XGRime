import { useColorMode } from '@vueuse/core'

export const THEME_MODES = ['auto', 'light', 'dark'] as const
export type ThemeMode = (typeof THEME_MODES)[number]

/**
 * 深浅色
 *
 * Tailwind 这边认的是 html 上的 `.dark`，所以 auto 也要落到具体那个 class 上，
 * 不能只留个 `auto` 让 CSS 自己猜。
 */
const mode = useColorMode({
  storageKey: 'xgrime.theme',
  emitAuto: true,
  modes: { auto: '', light: 'light', dark: 'dark' },
})

export function useTheme() {
  return {
    /** 用户选的：auto / light / dark */
    mode,
    /** 实际生效的深浅，auto 时跟系统 */
    isDark: () => document.documentElement.classList.contains('dark'),
    set(next: ThemeMode) {
      mode.value = next
    },
  }
}
