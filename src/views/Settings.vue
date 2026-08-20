<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRimeStore } from '@/stores/rimeStore'
import { useInstallStore } from '@/stores/installStore'
import {
  readInputSettings,
  saveInputSettings,
  readSchemaOptions,
  saveSchemaSwitch,
  saveFuzzy,
} from '@/types/commands'
import type { InputSettings, SchemaOptions, SchemaSwitch } from '@/types/rime'
import { useFeedback } from '@/composables/useFeedback'

const rimeStore = useRimeStore()
const job = useInstallStore()
const { toast, confirmAction } = useFeedback()
const { t } = useI18n()

const saving = ref(false)
const saved = ref(false)
const loading = ref(false)

const settings = ref<InputSettings>({
  pageSize: 5,
  shiftLBehavior: 'commit_text',
  shiftRBehavior: 'noop',
  pageKeys: 'minus_equal',
})

const schema = ref<SchemaOptions | null>(null)
/**
 * 读进来时的样子
 *
 * 保存时拿它来 diff：只写真正改过的项。全写一遍的话，那些用户根本没碰过的
 * 开关会被写进 custom.yaml，从「跟方案默认」变成「用户指定」，方案将来改了
 * 默认值也跟不上了。
 */
const original = ref<{ settings: InputSettings; switches: Record<number, number>; fuzzy: string[] } | null>(null)
/** 用户在选择器里挑的方案；null 表示跟着方案选单的第一个走 */
const picked = ref<string | null>(null)
const configDir = computed(() => rimeStore.installInfo?.configDir ?? null)


// 模糊音的例字本身就是内容，不随界面语言变
const fuzzyOptions = [
  { id: 'zh_z', label: 'zh ↔ z', desc: '知 ↔ 资' },
  { id: 'ch_c', label: 'ch ↔ c', desc: '吃 ↔ 词' },
  { id: 'sh_s', label: 'sh ↔ s', desc: '师 ↔ 思' },
  { id: 'n_l', label: 'n ↔ l', desc: '你 ↔ 里' },
  { id: 'r_l', label: 'r ↔ l', desc: '人 ↔ 论' },
  { id: 'f_h', label: 'f ↔ h', desc: '发 ↔ 花' },
  { id: 'an_ang', label: 'an ↔ ang', desc: '安 ↔ 昂' },
  { id: 'en_eng', label: 'en ↔ eng', desc: '恩 ↔ 鞥' },
  { id: 'in_ing', label: 'in ↔ ing', desc: '因 ↔ 英' },
  { id: 'ian_iang', label: 'ian ↔ iang', desc: '烟 ↔ 央' },
  { id: 'uan_uang', label: 'uan ↔ uang', desc: '弯 ↔ 汪' },
]

const shiftOptions = ['commit_text', 'inline_ascii', 'noop']

const pageKeyOptions = [
  { value: 'minus_equal', label: '- / =' },
  { value: 'bracket', label: '[ / ]' },
  { value: 'tab', label: 'Tab / Shift+Tab' },
  { value: 'comma_period', label: ', / .' },
]

/** 认得的开关翻成人话，认不出的退回方案里写的原名 */
/**
 * 开关的选项名
 *
 * 这些名字是方案自己写在 `switches` 里的，多数能看懂（简体/繁体），但雾凇的
 * Emoji 开关给的是两个表情符号，摆在按钮上根本分不出哪个是开哪个是关。
 * 认不出字的就换成我们自己的文字。
 */
function stateLabel(sw: SchemaSwitch, i: number) {
  const raw = sw.states[i] ?? ''
  if (sw.states.length === 2 && sw.labelKey === 'emoji') {
    return t(`switchStates.emoji.${i === 0 ? 'off' : 'on'}`)
  }
  const readable = /[\p{Script=Han}\p{Letter}\p{Number}]/u.test(raw)
  if (readable) return raw
  return sw.states.length === 2
    ? t(`switchStates.generic.${i === 0 ? 'off' : 'on'}`)
    : raw
}

function switchLabel(sw: SchemaSwitch) {
  if (sw.labelKey !== 'other') return t(`switches.${sw.labelKey}`)
  return sw.rawName ? sw.rawName.replace(/_/g, ' ') : t('switches.other')
}

async function load() {
  if (!configDir.value) return
  loading.value = true
  try {
    const [s, opts] = await Promise.all([
      readInputSettings(configDir.value),
      readSchemaOptions(configDir.value, picked.value ?? undefined),
    ])
    settings.value = s
    schema.value = opts
    picked.value = opts.schemaId
    snapshot()
  } catch (e) {
    toast.error(t('settings.loadFail'), e)
  } finally {
    loading.value = false
  }
}

function snapshot() {
  const sw: Record<number, number> = {}
  schema.value?.switches.forEach((x) => (sw[x.index] = x.current))
  original.value = {
    settings: { ...settings.value },
    switches: sw,
    fuzzy: [...(schema.value?.fuzzyPairs ?? [])],
  }
}

/** 改过还没存的项 */
const changedSwitches = computed(() => {
  const base = original.value?.switches
  if (!base || !schema.value) return []
  return schema.value.switches.filter((x) => base[x.index] !== x.current)
})
const fuzzyChanged = computed(() => {
  const base = original.value?.fuzzy
  const now = schema.value?.fuzzyPairs
  if (!base || !now) return false
  return base.length !== now.length || base.some((x) => !now.includes(x))
})
const settingsChanged = computed(() => {
  const base = original.value?.settings
  if (!base) return false
  return (Object.keys(base) as (keyof InputSettings)[]).some((k) => base[k] !== settings.value[k])
})
const dirty = computed(
  () => settingsChanged.value || fuzzyChanged.value || changedSwitches.value.length > 0,
)

/** 只重读方案那一块，全局设置不动 —— 换方案时用 */
async function reloadSchema(target: string) {
  if (!configDir.value || target === picked.value) return
  // 换方案会把这个方案上没保存的改动丢掉，先问一句
  if (changedSwitches.value.length > 0 || fuzzyChanged.value) {
    const ok = await confirmAction({
      title: t('settings.discardTitle'),
      description: t('settings.discardHint'),
      confirmText: t('settings.discard'),
      danger: true,
    })
    if (!ok) return
  }
  picked.value = target
  try {
    schema.value = await readSchemaOptions(configDir.value, target)
    snapshot()
  } catch (e) {
    toast.error(t('settings.loadFail'), e)
  }
}

/**
 * 一次把改过的全写进去
 *
 * 以前是点一下开关就写一次盘、部署一次 —— 调五项就编译五遍词库，
 * 每次几十秒。现在全部攒到这里，最后只部署一次。
 */
async function handleSave() {
  if (!configDir.value) return
  saving.value = true
  saved.value = false
  try {
    if (settingsChanged.value) {
      await saveInputSettings(configDir.value, settings.value)
    }
    const schemaId = schema.value?.schemaId
    if (schemaId) {
      for (const sw of changedSwitches.value) {
        await saveSchemaSwitch(configDir.value, schemaId, sw.index, sw.current)
        sw.configured = true
      }
      if (fuzzyChanged.value) {
        await saveFuzzy(configDir.value, schemaId, schema.value!.fuzzyPairs)
      }
    }
    snapshot()
    saved.value = true
    setTimeout(() => (saved.value = false), 3000)
    toast.success(t('settings.saveOk'))
    // 部署要几十秒，不等它 —— 横幅会显示进度
    job.redeploy().catch((e) => toast.error(t('settings.saveFail'), e))
  } catch (e) {
    toast.error(t('settings.saveFail'), e)
  } finally {
    saving.value = false
  }
}

/** 只改本地，等用户点「保存」再一起写 */
function pickSwitch(index: number, value: number) {
  const target = schema.value?.switches.find((s) => s.index === index)
  if (target) target.current = value
}

/** 同样只改本地 */
function toggleFuzzy(id: string) {
  if (!schema.value) return
  const current = schema.value.fuzzyPairs
  schema.value.fuzzyPairs = current.includes(id)
    ? current.filter((x) => x !== id)
    : [...current, id]
}

onMounted(() => {
  if (rimeStore.installInfo?.installed) load()
})
watch(
  () => [configDir.value, rimeStore.installInfo?.installed],
  () => {
    if (rimeStore.installInfo?.installed) load()
  },
)
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-semibold tracking-tight">{{ $t('settings.title') }}</h1>
        <p class="text-[15px] text-muted-foreground mt-0.5">{{ $t('settings.subtitle') }}</p>
      </div>
      <div class="flex items-center gap-3">
        <span v-if="dirty && !saving" class="text-[13px] text-amber-500/80">
          {{ $t('settings.unsaved') }}
        </span>
        <button
          class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-primary text-primary-foreground text-[15px] hover:bg-primary/90 transition-colors disabled:opacity-40"
          :disabled="saving || !dirty || !rimeStore.installInfo?.installed"
          @click="handleSave"
        >
          <span
            :class="saving ? 'icon-[lucide--loader-2] animate-spin' : saved ? 'icon-[lucide--check]' : 'icon-[lucide--save]'"
            class="size-3.5"
          />
          {{ saving ? $t('common.saving') : saved ? $t('common.saved') : $t('common.save') }}
        </button>
      </div>
    </div>

    <div
      v-if="!rimeStore.installInfo?.installed"
      class="rounded-xl bg-amber-500/5 px-4 py-3 text-[14px] text-amber-500/80 mb-5"
    >
      {{ $t('settings.needRime') }}
    </div>

    <div v-else-if="loading" class="flex items-center gap-2.5 text-muted-foreground text-[15px]">
      <span class="icon-[lucide--loader-2] size-4 animate-spin" />
      <span>{{ $t('settings.loading') }}</span>
    </div>

    <div v-else class="flex-1 overflow-auto space-y-6">
      <div
        v-if="job.deploying"
        class="rounded-xl bg-sky-500/5 px-4 py-2.5 flex items-center gap-2.5 text-[13px] text-sky-600 dark:text-sky-400"
      >
        <span class="icon-[lucide--loader-2] size-3.5 animate-spin shrink-0" />
        {{ $t('schemas.deployingBody') }}
      </div>
      <div
        v-else-if="job.justDeployed"
        class="rounded-xl bg-emerald-500/5 px-4 py-2.5 flex items-center gap-2.5 text-[13px] text-emerald-600 dark:text-emerald-400"
      >
        <span class="icon-[lucide--check-circle-2] size-3.5 shrink-0" />
        {{ $t('schemas.deployedTitle') }}
      </div>

      <!-- ═══ 按方案配置 ═══ -->
      <section v-if="schema && !schema.missing" class="rounded-xl bg-card p-5 space-y-5">
        <div>
          <h2 class="text-[13px] font-medium text-muted-foreground uppercase tracking-wider">
            {{ $t('settings.perSchema') }}
          </h2>
          <p class="text-[12px] text-muted-foreground/60 mt-1">
            {{ $t('settings.schemaSectionHint') }}
          </p>
        </div>

        <!--
          装了两个以上方案才需要选。只有一个时摆个选择器纯属噪音，
          但两个以上不给入口的话，第二个方案的开关就永远调不到。
        -->
        <div v-if="schema.available.length > 1" class="space-y-2">
          <span class="text-[14px] text-foreground/70">{{ $t('settings.whichSchema') }}</span>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="s in schema.available"
              :key="s.schemaId"
              class="px-3 py-1.5 rounded-lg text-[13px] transition-colors"
              :class="schema.schemaId === s.schemaId
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted/30 text-muted-foreground hover:text-foreground'"
              @click="reloadSchema(s.schemaId)"
            >
              {{ s.name }}
            </button>
          </div>
        </div>
        <p v-else class="text-[14px] text-foreground/70">{{ schema.schemaName }}</p>

        <div v-for="sw in schema.switches" :key="sw.index" class="space-y-2">
          <div class="flex items-center gap-2">
            <span class="text-[14px] text-foreground/70">{{ switchLabel(sw) }}</span>
            <span v-if="!sw.configured" class="text-[11px] text-muted-foreground/50">
              {{ $t('settings.schemaDefault') }}
            </span>
          </div>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="(_state, i) in sw.states"
              :key="i"
              class="px-3 py-1.5 rounded-lg text-[13px] transition-colors disabled:opacity-50"
              :class="sw.current === i
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted/30 text-muted-foreground hover:text-foreground'"
              @click="pickSwitch(sw.index, i)"
            >
              {{ stateLabel(sw, i) }}
            </button>
          </div>
        </div>

        <!-- 模糊音只对拼音类方案有意义 -->
        <div v-if="schema.supportsFuzzy" class="space-y-3 pt-1">
          <div>
            <span class="text-[14px] text-foreground/70">{{ $t('settings.fuzzy') }}</span>
            <p class="text-[12px] text-muted-foreground/60 mt-0.5">{{ $t('settings.fuzzyHint') }}</p>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <button
              v-for="opt in fuzzyOptions"
              :key="opt.id"
              class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-all disabled:opacity-50"
              :class="schema.fuzzyPairs.includes(opt.id)
                ? 'bg-primary/10 inset-ring-1 inset-ring-primary/20'
                : 'bg-muted/30 hover:bg-muted/50'"
              @click="toggleFuzzy(opt.id)"
            >
              <div
                class="size-4 rounded border flex items-center justify-center shrink-0 transition-colors"
                :class="schema.fuzzyPairs.includes(opt.id)
                  ? 'bg-primary border-primary'
                  : 'border-muted-foreground/30'"
              >
                <span
                  v-if="schema.fuzzyPairs.includes(opt.id)"
                  class="icon-[lucide--check] size-3 text-primary-foreground"
                />
              </div>
              <div class="min-w-0">
                <span class="text-[14px] font-mono">{{ opt.label }}</span>
                <span class="text-[12px] text-muted-foreground ml-2">{{ opt.desc }}</span>
              </div>
            </button>
          </div>
        </div>
      </section>

      <section v-else class="rounded-xl bg-card p-5 text-[14px] text-muted-foreground">
        {{ $t('settings.noSchema') }}
      </section>

      <!-- ═══ 全局设置：不分方案 ═══ -->
      <section class="rounded-xl bg-card p-5 space-y-5">
        <div>
          <h2 class="text-[13px] font-medium text-muted-foreground uppercase tracking-wider">
            {{ $t('settings.basic') }}
          </h2>
          <p class="text-[12px] text-muted-foreground/60 mt-1">{{ $t('settings.basicHint') }}</p>
        </div>

        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-[14px] text-foreground/70">{{ $t('settings.pageSize') }}</span>
            <span class="text-[15px] font-medium tabular-nums w-6 text-center">{{ settings.pageSize }}</span>
          </div>
          <input
            v-model.number="settings.pageSize"
            type="range"
            min="3"
            max="10"
            step="1"
            class="w-full h-1 rounded-full bg-muted appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary [&::-webkit-slider-thumb]:shadow-sm"
          />
          <div class="flex justify-between text-[11px] text-muted-foreground/50">
            <span>3</span>
            <span>10</span>
          </div>
        </div>

        <div class="space-y-2">
          <span class="text-[14px] text-foreground/70">{{ $t('settings.shiftL') }}</span>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="opt in shiftOptions"
              :key="opt"
              class="px-3 py-1.5 rounded-lg text-[13px] transition-colors"
              :class="settings.shiftLBehavior === opt
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted/30 text-muted-foreground hover:text-foreground'"
              @click="settings.shiftLBehavior = opt"
            >
              {{ $t(`settings.shift.${opt}`) }}
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <span class="text-[14px] text-foreground/70">{{ $t('settings.shiftR') }}</span>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="opt in shiftOptions"
              :key="opt"
              class="px-3 py-1.5 rounded-lg text-[13px] transition-colors"
              :class="settings.shiftRBehavior === opt
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted/30 text-muted-foreground hover:text-foreground'"
              @click="settings.shiftRBehavior = opt"
            >
              {{ $t(`settings.shift.${opt}`) }}
            </button>
          </div>
        </div>

        <div class="space-y-2">
          <span class="text-[14px] text-foreground/70">{{ $t('settings.pageKeys') }}</span>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="opt in pageKeyOptions"
              :key="opt.value"
              class="px-3 py-1.5 rounded-lg text-[13px] font-mono transition-colors"
              :class="settings.pageKeys === opt.value
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted/30 text-muted-foreground hover:text-foreground'"
              @click="settings.pageKeys = opt.value"
            >
              {{ opt.label }}
            </button>
          </div>
        </div>
      </section>
    </div>

  </div>
</template>
