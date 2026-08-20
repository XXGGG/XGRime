<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRimeStore } from '@/stores/rimeStore'
import { useFeedback } from '@/composables/useFeedback'
import { getAutostart, setAutostart, openSystemSetting, openConfigDir } from '@/types/commands'
import ToggleSwitch from '@/components/ui/ToggleSwitch.vue'

/**
 * 设置
 *
 * 这一页装的是 XGRime 自己的事（开机自启、托盘、配置目录），跟输入法怎么打字
 * 无关 —— 那些在「输入设置」。没装小狼毫也该能进来调。
 */
const rimeStore = useRimeStore()
const { t } = useI18n()
const { toast } = useFeedback()

const version = __APP_VERSION__
/** 「高级键盘设置」是 Windows 的页面，macOS 上不该出现这个入口 */
const isWindows = computed(() => rimeStore.installInfo?.platform === 'windows')

const autostart = ref(false)
const autostartBusy = ref(false)

async function toggleAutostart() {
  autostartBusy.value = true
  try {
    autostart.value = await setAutostart(!autostart.value)
  } catch (e) {
    toast.error(t('prefs.autostartFail'), e)
  } finally {
    autostartBusy.value = false
  }
}

async function openKeyboardSettings() {
  try {
    await openSystemSetting('keyboard-advanced')
  } catch (e) {
    toast.error(t('prefs.openSettingFail'), e)
  }
}

function openFolder(dir: string) {
  openConfigDir(dir).catch((e) => toast.error(t('prefs.openSettingFail'), e))
}

onMounted(() => {
  getAutostart()
    .then((v) => (autostart.value = v))
    .catch(() => {})
})
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="mb-6">
      <h1 class="text-xl font-semibold tracking-tight">{{ $t('prefs.title') }}</h1>
      <p class="text-[15px] text-muted-foreground mt-0.5">{{ $t('prefs.subtitle') }}</p>
    </div>

    <div class="flex-1 overflow-auto min-h-0 pb-8 max-w-2xl space-y-4">
      <section class="rounded-xl bg-card p-4 space-y-4">
        <h2 class="text-[15px] font-semibold text-foreground/90">{{ $t('prefs.startup') }}</h2>

        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0 pr-3">
            <span class="text-[14px] text-foreground/70">{{ $t('prefs.autostart') }}</span>
            <p class="text-[11px] text-muted-foreground/50 mt-0.5">{{ $t('prefs.autostartHint') }}</p>
          </div>
          <ToggleSwitch
            :model-value="autostart"
            :disabled="autostartBusy"
            @update:model-value="toggleAutostart"
          />
        </div>

        <p class="text-[11px] text-muted-foreground/50 leading-relaxed">
          {{ $t('prefs.trayHint') }}
        </p>
      </section>

      <section v-if="isWindows" class="rounded-xl bg-card p-4 space-y-4">
        <h2 class="text-[15px] font-semibold text-foreground/90">{{ $t('prefs.system') }}</h2>

        <div class="flex items-center justify-between gap-3">
          <div class="min-w-0 pr-3">
            <span class="text-[14px] text-foreground/70">{{ $t('prefs.keyboardSettings') }}</span>
            <p class="text-[11px] text-muted-foreground/50 mt-0.5">
              {{ $t('prefs.keyboardSettingsHint') }}
            </p>
          </div>
          <button
            class="shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-muted/50 text-[13px] text-foreground/80 hover:text-foreground transition-colors"
            @click="openKeyboardSettings"
          >
            <span class="icon-[lucide--external-link] size-3.5 opacity-60" />
            {{ $t('prefs.openIt') }}
          </button>
        </div>
      </section>

      <section class="rounded-xl bg-card p-4 space-y-4">
        <h2 class="text-[15px] font-semibold text-foreground/90">{{ $t('prefs.about') }}</h2>

        <div class="flex items-center justify-between gap-3 text-[14px]">
          <span class="text-muted-foreground">{{ $t('prefs.version') }}</span>
          <span class="text-foreground/80 tabular-nums">v{{ version }}</span>
        </div>

        <!-- 只给入口，不显示路径：路径里夹着用户名，截图和贴日志时会带出去 -->
        <div
          v-if="rimeStore.installInfo?.installDir"
          class="flex items-center justify-between gap-3"
        >
          <span class="text-[14px] text-foreground/70">{{ $t('prefs.programDir') }}</span>
          <button
            class="shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-muted/50 text-[13px] text-foreground/80 hover:text-foreground transition-colors"
            @click="openFolder(rimeStore.installInfo.installDir)"
          >
            <span class="icon-[lucide--folder-open] size-3.5 opacity-60" />
            {{ $t('prefs.openIt') }}
          </button>
        </div>

        <div
          v-if="rimeStore.installInfo?.configDir"
          class="flex items-center justify-between gap-3"
        >
          <span class="text-[14px] text-foreground/70">{{ $t('prefs.configDir') }}</span>
          <button
            class="shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-muted/50 text-[13px] text-foreground/80 hover:text-foreground transition-colors"
            @click="openFolder(rimeStore.installInfo.configDir)"
          >
            <span class="icon-[lucide--folder-open] size-3.5 opacity-60" />
            {{ $t('prefs.openIt') }}
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
