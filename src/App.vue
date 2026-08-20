<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRimeStore } from './stores/rimeStore'
import { useTheme } from './composables/useTheme'

import TitleBar from './components/TitleBar.vue'
import Toaster from './components/ui/Toaster.vue'
import ConfirmDialog from './components/ui/ConfirmDialog.vue'
import LanguagePicker from './components/ui/LanguagePicker.vue'
import ThemeToggle from './components/ui/ThemeToggle.vue'
import HomeView from './views/Home.vue'
import DictionaryView from './views/Dictionary.vue'
import ThemeView from './views/Theme.vue'
import SettingsView from './views/Settings.vue'
import StatusIconsView from './views/StatusIcons.vue'
import BackupView from './views/Backup.vue'
import PreferencesView from './views/Preferences.vue'

const currentView = ref('Home')

/** 构建时由 vite 注入，唯一真相是 package.json */
const version = __APP_VERSION__

const menuItems = computed(() => [
  { id: 'Home', label: 'nav.home', icon: 'icon-[lucide--home]' },
  { id: 'Dictionary', label: 'nav.schemas', icon: 'icon-[lucide--book-open]' },
  { id: 'Settings', label: 'nav.settings', icon: 'icon-[lucide--sliders-horizontal]' },
  { id: 'Theme', label: 'nav.theme', icon: 'icon-[lucide--palette]' },
  { id: 'StatusIcons', label: 'nav.icons', icon: 'icon-[lucide--app-window]' },
  { id: 'Backup', label: 'nav.backup', icon: 'icon-[lucide--archive]' },
  { id: 'Preferences', label: 'nav.preferences', icon: 'icon-[lucide--settings]' },
])

// 挂上去就够了，深浅色由 html 上的 class 驱动
useTheme()

const rimeStore = useRimeStore()

onMounted(() => {
  rimeStore.detect()
})
</script>

<template>
  <div class="h-full flex flex-col bg-background">
    <TitleBar />
    <div class="flex flex-1 overflow-hidden">
      <!-- 侧边栏 -->
      <aside class="w-48 shrink-0 bg-card/50 flex flex-col pt-3 pb-4">
        <nav class="flex-1 px-3 space-y-1">
          <button
            v-for="item in menuItems"
            :key="item.id"
            class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-[15px] transition-all duration-150"
            :class="
              currentView === item.id
                ? 'bg-primary/10 text-foreground font-medium'
                : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
            "
            @click="currentView = item.id"
          >
            <span
              :class="item.icon"
              class="size-[18px] shrink-0"
              :style="currentView === item.id ? 'opacity: 1' : 'opacity: 0.6'"
            />
            <span>{{ $t(item.label) }}</span>
          </button>
        </nav>

        <!-- 底部：外观 + 语言 + 版本 -->
        <div class="px-3 mt-auto space-y-1.5">
          <ThemeToggle />
          <LanguagePicker />
          <p class="px-2 text-[11px] text-muted-foreground/50">XGRime v{{ version }}</p>
        </div>
      </aside>

      <!-- 主内容区 -->
      <main class="flex-1 overflow-auto">
        <!--
          内容统一收在一根居中的列里：窗口拉多大都不会散到左边去。
          宽度归这里管，各个页面就别再自己写 max-w 了，写了反而会在这根列里再偏一次。
        -->
        <div class="p-8 h-full mx-auto w-full max-w-3xl">
          <HomeView v-if="currentView === 'Home'" />
          <DictionaryView v-else-if="currentView === 'Dictionary'" />
          <SettingsView v-else-if="currentView === 'Settings'" />
          <ThemeView v-else-if="currentView === 'Theme'" />
          <StatusIconsView v-else-if="currentView === 'StatusIcons'" />
          <BackupView v-else-if="currentView === 'Backup'" />
          <PreferencesView v-else-if="currentView === 'Preferences'" />
        </div>
      </main>
    </div>

    <!-- 应用自己的提示与确认，不用浏览器原生弹窗 -->
    <Toaster />
    <ConfirmDialog />
  </div>
</template>
