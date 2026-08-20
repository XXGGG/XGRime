<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { DictInfo, DownloadProgress } from '@/types/rime'

const props = defineProps<{
  dict: DictInfo
  /** 正在装的是不是它 */
  installing: boolean
  /** 有别的方案在忙，按钮要禁掉 */
  busy: boolean
  progress: DownloadProgress | null
  /** 词库有更新 */
  stale: boolean
  /** 推荐区里的卡片描一圈边 */
  highlight?: boolean
}>()

defineEmits<{
  install: [dict: DictInfo]
  toggle: [dict: DictInfo]
  remove: [dict: DictInfo]
}>()

const { t } = useI18n()

const name = computed(() => t(`dicts.${props.dict.id}.name`))
const desc = computed(() => t(`dicts.${props.dict.id}.desc`))

const size = computed(() => {
  const b = props.dict.totalBytes
  return b >= 1024 * 1024
    ? `${(b / 1024 / 1024).toFixed(1)} MB`
    : `${Math.max(1, Math.round(b / 1024))} KB`
})

const mb = (n: number) => (n / 1024 / 1024).toFixed(1)
</script>

<template>
  <div
    class="rounded-xl bg-card p-4 flex items-start justify-between gap-4"
    :class="[
      dict.active ? 'inset-ring-1 inset-ring-primary/20' : '',
      highlight && !dict.active ? 'inset-ring-1 inset-ring-border/60' : '',
    ]"
  >
    <div class="space-y-1 min-w-0">
      <div class="flex items-center gap-2 flex-wrap">
        <h3 class="font-medium text-[15px]">{{ name }}</h3>
        <span class="text-[11px] text-muted-foreground/50">{{ $t('schemas.downloadSize', { size }) }}</span>
        <span v-if="dict.active" class="text-[11px] px-1.5 py-0.5 rounded-full bg-primary/10 text-primary">
          {{ $t('schemas.inUse') }}
        </span>
        <span
          v-else-if="dict.installed"
          class="text-[11px] px-1.5 py-0.5 rounded-full bg-muted/50 text-muted-foreground"
        >
          {{ $t('schemas.installedIdle') }}
        </span>
        <!-- 独立一条，跟上面的启用状态不冲突：自带的方案用着也是自带的 -->
        <span
          v-if="dict.installed && !dict.removable"
          class="text-[11px] text-muted-foreground/50"
        >
          {{ $t('schemas.bundled') }}
        </span>
        <span
          v-if="stale"
          class="text-[11px] px-1.5 py-0.5 rounded-full bg-sky-500/10 text-sky-600 dark:text-sky-400"
        >
          {{ $t('schemas.updateBadge') }}
        </span>
      </div>

      <p class="text-[14px] text-muted-foreground leading-relaxed">{{ desc }}</p>

      <p v-if="dict.sources.length > 1 && !dict.installed" class="text-[12px] text-muted-foreground/60">
        {{ $t('schemas.depsNote', { count: dict.sources.length }) }}
      </p>

      <!-- 一个方案要下好几个包，得让用户看见在下哪个 -->
      <div v-if="installing" class="pt-2 space-y-1">
        <div class="w-full bg-muted rounded-full h-1">
          <div
            class="bg-primary h-1 rounded-full transition-all"
            :style="{ width: `${progress?.percentage ?? 0}%` }"
          />
        </div>
        <p class="text-[11px] text-muted-foreground">
          <template v-if="progress?.stepTotal">
            {{ $t('schemas.stepOf', { step: progress.step, total: progress.stepTotal, name: progress.stepName }) }}
          </template>
          <template v-if="progress?.total">
            · {{ mb(progress.downloaded) }} / {{ mb(progress.total) }} MB
          </template>
          <template v-else>{{ $t('schemas.connecting') }}</template>
        </p>
      </div>
    </div>

    <div class="shrink-0 pt-0.5 flex flex-col items-end gap-1.5">
      <button
        v-if="!dict.installed"
        class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary text-primary-foreground text-[13px] hover:bg-primary/90 transition-colors disabled:opacity-40"
        :disabled="busy"
        @click="$emit('install', dict)"
      >
        <span
          :class="installing ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--download]'"
          class="size-3"
        />
        {{ installing ? $t('common.installing') : $t('common.install') }}
      </button>

      <template v-else>
        <button
          v-if="stale"
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-sky-500/10 text-sky-600 dark:text-sky-400 text-[13px] hover:bg-sky-500/20 transition-colors disabled:opacity-40"
          :disabled="busy"
          @click="$emit('install', dict)"
        >
          <span
            :class="installing ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--download-cloud]'"
            class="size-3"
          />
          {{ installing ? $t('common.installing') : $t('schemas.updateNow') }}
        </button>

        <button
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[13px] transition-colors disabled:opacity-40"
          :class="dict.active
            ? 'bg-primary/10 text-primary hover:bg-primary/20'
            : 'bg-muted/50 text-muted-foreground hover:text-foreground hover:bg-muted'"
          :disabled="busy"
          @click="$emit('toggle', dict)"
        >
          <span :class="dict.active ? 'icon-[lucide--pause]' : 'icon-[lucide--play]'" class="size-3" />
          {{ dict.active ? $t('common.disable') : $t('common.enable') }}
        </button>

        <!--
          自带的方案文件在输入法自己的安装目录里，删不得也不该删。
          与其摆个按不动的按钮，不如不摆 —— 要它从选单消失，用「停用」。
        -->
        <button
          v-if="dict.removable"
          class="inline-flex items-center gap-1.5 px-2 py-1 rounded-lg text-[12px] text-muted-foreground/50 hover:text-destructive hover:bg-destructive/5 transition-colors disabled:opacity-40"
          :disabled="busy"
          @click="$emit('remove', dict)"
        >
          <span class="icon-[lucide--trash-2] size-3" />
          {{ $t('common.uninstall') }}
        </button>
      </template>
    </div>
  </div>
</template>
