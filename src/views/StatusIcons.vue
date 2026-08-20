<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { open } from '@tauri-apps/plugin-dialog'
import { useRimeStore } from '@/stores/rimeStore'
import { useInstallStore } from '@/stores/installStore'
import { useFeedback } from '@/composables/useFeedback'
import {
  readSchemaIcons,
  setSchemaIcon,
  clearSchemaIcon,
  clearAllSchemaIcons,
  listBuiltinIconSets,
  applyBuiltinIconSet,
  readSchemaOptions,
  readIconPref,
  syncStatusIcons,
} from '@/types/commands'
import type { SchemaIcon, IconSet } from '@/types/rime'

const rimeStore = useRimeStore()
const job = useInstallStore()
const { t } = useI18n()
const { toast, confirmAction } = useFeedback()

const KINDS = ['zhung', 'ascii', 'full', 'half'] as const

/**
 * 内置图标的预览图
 *
 * 装进输入法的是 .ico（小狼毫画的是 Win32 图标），网页显示不了多尺寸 ico，
 * 所以生成脚本同时出一份 png 给界面用。两边同源，见 scripts/gen-status-icons.py。
 */
const previews = import.meta.glob<string>('@/assets/status-icons/*.png', {
  eager: true,
  import: 'default',
})
const previewOf = (set: string, kind: string) =>
  previews[Object.keys(previews).find((p) => p.endsWith(`/${set}-${kind}.png`)) ?? ''] ?? ''

/**
 * 每套图标底下垫什么色
 *
 * 这块底不是界面的一部分，是在**模拟任务栏**：黑字那套永远配浅底、白字那套
 * 永远配深底，跟应用是浅色还是深色主题无关。跟着主题走的话，深色模式下
 * 黑字配深底，四个图标直接看不见。
 */
const BACKDROP: Record<string, string> = {
  plain_light: 'bg-neutral-700',
}
const backdropOf = (id: string) => BACKDROP[id] ?? 'bg-neutral-100'

/** 「跟着任务栏自动换」不是某一套图标，是个选择 */
const AUTO = 'auto'

const sets = ref<IconSet[]>([])
/** 当前选的是哪一套（可能是 auto） */
const mode = ref('')
const icons = ref<SchemaIcon[]>([])
const schemaId = ref<string | null>(null)
const busy = ref<string | null>(null)

const configDir = () => rimeStore.installInfo?.configDir ?? null

async function load() {
  const dir = configDir()
  if (!dir) {
    icons.value = []
    return
  }
  try {
    schemaId.value = (await readSchemaOptions(dir)).schemaId
  } catch {
    schemaId.value = null
  }
  try {
    sets.value = (await listBuiltinIconSets()).filter((s) => s.complete)
  } catch {
    sets.value = []
  }
  try {
    mode.value = (await readIconPref()).mode
  } catch {
    mode.value = ''
  }
  // 选了「自动」的话，开机换了任务栏深浅这里对一次，换了就顺手重新部署
  try {
    if (await syncStatusIcons(dir)) {
      job.redeploy().catch(() => {})
    }
  } catch {
    /* 同步失败不该挡住整页 */
  }
  try {
    icons.value = await readSchemaIcons(dir)
  } catch {
    icons.value = []
  }
}

const iconOf = (kind: string) => icons.value.find((i) => i.kind === kind)
/** 图标是不是这一套装的 —— 装进去的文件名带方案和用途，认不出是哪一套，所以只记状态 */
const anySet = () => icons.value.some((i) => i.path)

async function useSet(id: string) {
  const dir = configDir()
  if (!dir) return
  busy.value = id
  try {
    icons.value = await applyBuiltinIconSet(dir, id)
    mode.value = id
    toast.success(t('icons.applied'))
    job.redeploy().catch((e) => toast.error(t('icons.failed'), e))
  } catch (e) {
    toast.error(t('icons.failed'), e)
  } finally {
    busy.value = null
  }
}

async function pick(kind: string) {
  const dir = configDir()
  if (!dir) return
  const chosen = await open({
    multiple: false,
    filters: [{ name: 'Icon', extensions: ['ico', 'png'] }],
  })
  if (typeof chosen !== 'string') return

  busy.value = kind
  try {
    icons.value = await setSchemaIcon(dir, kind, chosen)
    toast.success(t('icons.changed'))
    job.redeploy().catch((e) => toast.error(t('icons.failed'), e))
  } catch (e) {
    toast.error(t('icons.failed'), e)
  } finally {
    busy.value = null
  }
}

async function reset(kind: string) {
  const dir = configDir()
  if (!dir) return
  busy.value = kind
  try {
    icons.value = await clearSchemaIcon(dir, kind)
    toast.success(t('icons.reset'))
    job.redeploy().catch((e) => toast.error(t('icons.failed'), e))
  } catch (e) {
    toast.error(t('icons.failed'), e)
  } finally {
    busy.value = null
  }
}

async function resetAll() {
  const dir = configDir()
  if (!dir) return
  const ok = await confirmAction({
    title: t('icons.resetAllConfirm'),
    confirmText: t('icons.resetAll'),
    danger: true,
  })
  if (!ok) return
  busy.value = 'all'
  try {
    icons.value = await clearAllSchemaIcons(dir)
    mode.value = ''
    toast.success(t('icons.reset'))
    job.redeploy().catch((e) => toast.error(t('icons.failed'), e))
  } catch (e) {
    toast.error(t('icons.failed'), e)
  } finally {
    busy.value = null
  }
}

watch(() => rimeStore.installInfo?.configDir, load, { immediate: true })
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-semibold tracking-tight">{{ $t('icons.title') }}</h1>
        <p class="text-[15px] text-muted-foreground mt-0.5">{{ $t('icons.subtitle') }}</p>
      </div>
      <button
        v-if="anySet()"
        class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-card text-[15px] text-foreground/80 hover:text-destructive transition-colors"
        @click="resetAll"
      >
        <span class="icon-[lucide--rotate-ccw] size-3.5 opacity-60" />
        {{ $t('icons.resetAll') }}
      </button>
    </div>

    <div v-if="!rimeStore.installInfo?.installed" class="rounded-xl bg-amber-500/5 px-4 py-3 text-[14px] mb-5">
      <p class="text-amber-500/80">{{ $t('theme.needRime') }}</p>
    </div>

    <div class="flex-1 overflow-auto min-h-0 pb-8 space-y-8">
      <p class="text-[13px] text-muted-foreground/70 leading-relaxed">
        {{ $t('icons.hint') }}
      </p>

      <!-- 内置图标 -->
      <div class="space-y-3">
        <h3 class="text-[15px] font-semibold text-foreground/90">{{ $t('icons.builtin') }}</h3>
        <div class="grid grid-cols-1 min-[820px]:grid-cols-2 gap-3">
          <!-- 自动：不是某一套图标，是个选择，所以单独一张卡 -->
          <button
            class="rounded-xl bg-card p-4 border transition-all text-left disabled:opacity-40 min-[820px]:col-span-2"
            :class="mode === AUTO
              ? 'border-primary inset-ring-1 inset-ring-primary/40'
              : 'border-transparent hover:border-primary/30'"
            :disabled="busy !== null"
            @click="useSet(AUTO)"
          >
            <div class="flex items-center gap-3">
              <span
                :class="busy === AUTO ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--contrast]'"
                class="size-4 opacity-70 shrink-0"
              />
              <span class="min-w-0">
                <span class="text-[13px] text-foreground/80">{{ $t('icons.sets.auto') }}</span>
                <span class="block text-[11px] text-muted-foreground/60 mt-0.5">
                  {{ $t('icons.autoHint') }}
                </span>
              </span>
            </div>
          </button>

          <button
            v-for="set in sets"
            :key="set.id"
            class="rounded-xl bg-card p-4 border transition-all text-left disabled:opacity-40"
            :class="mode === set.id
              ? 'border-primary inset-ring-1 inset-ring-primary/40'
              : 'border-transparent hover:border-primary/30'"
            :disabled="busy !== null"
            @click="useSet(set.id)"
          >
            <div class="rounded-lg px-3 py-2.5 flex items-center gap-3" :class="backdropOf(set.id)">
              <img
                v-for="kind in KINDS"
                :key="kind"
                :src="previewOf(set.id, kind)"
                :alt="kind"
                class="size-6"
              />
            </div>
            <span
              class="block mt-2.5 text-[13px]"
              :class="mode === set.id ? 'text-primary font-medium' : 'text-foreground/80'"
            >
              {{ $t(`icons.sets.${set.id}`) }}
              <span v-if="mode === set.id">· {{ $t('theme.presetInUse') }}</span>
            </span>
          </button>
        </div>

        <p class="text-[11px] text-muted-foreground/50 leading-relaxed">
          {{ $t('icons.bootHint') }}
        </p>
      </div>

      <!-- 一个个换 -->
      <div class="space-y-3">
        <h3 class="text-[15px] font-semibold text-foreground/90">{{ $t('icons.custom') }}</h3>
        <div v-for="kind in KINDS" :key="kind" class="flex items-center justify-between gap-3 py-1">
          <div class="min-w-0">
            <span class="text-[14px] text-foreground/70">{{ $t(`icons.kind.${kind}`) }}</span>
            <p
              v-if="iconOf(kind)?.path"
              class="text-[11px] mt-0.5 truncate"
              :class="iconOf(kind)?.exists ? 'text-muted-foreground/60' : 'text-red-500/80'"
            >
              {{ iconOf(kind)?.exists ? iconOf(kind)?.path : $t('icons.missing') }}
            </p>
            <p v-else class="text-[11px] text-muted-foreground/40 mt-0.5">
              {{ $t('icons.unset') }}
            </p>
          </div>

          <div class="flex items-center gap-1.5 shrink-0">
            <button
              class="px-2.5 py-1 rounded-lg bg-muted/50 text-[12px] text-foreground/80 hover:text-foreground transition-colors disabled:opacity-40"
              :disabled="busy !== null"
              @click="pick(kind)"
            >
              {{ $t('icons.choose') }}
            </button>
            <button
              v-if="iconOf(kind)?.path"
              class="px-2 py-1 rounded-lg text-[12px] text-muted-foreground/60 hover:text-destructive transition-colors disabled:opacity-40"
              :disabled="busy !== null"
              @click="reset(kind)"
            >
              {{ $t('icons.clear') }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
