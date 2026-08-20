<script setup lang="ts">
import { useTheme, THEME_MODES, type ThemeMode } from '@/composables/useTheme'

const { mode, set } = useTheme()

// 分段控件而不是下拉：三个选项而已，摊开比展开一层菜单省事，
// 也不会像下拉那样被窗口底边裁掉
const icons: Record<ThemeMode, string> = {
  auto: 'icon-[lucide--monitor]',
  light: 'icon-[lucide--sun]',
  dark: 'icon-[lucide--moon]',
}
</script>

<template>
  <div class="flex rounded-md bg-muted/40 p-0.5 gap-0.5">
    <button
      v-for="m in THEME_MODES"
      :key="m"
      class="flex-1 flex items-center justify-center py-1 rounded transition-colors"
      :class="mode === m
        ? 'bg-card text-foreground shadow-sm'
        : 'text-muted-foreground hover:text-foreground'"
      :title="$t(`theme.mode.${m}`)"
      :aria-label="$t(`theme.mode.${m}`)"
      @click="set(m)"
    >
      <span :class="icons[m]" class="size-3.5" />
    </button>
  </div>
</template>
