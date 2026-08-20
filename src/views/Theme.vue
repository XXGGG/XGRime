<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRimeStore } from '@/stores/rimeStore'
import { useFeedback } from '@/composables/useFeedback'
import { useI18n } from 'vue-i18n'
import { saveThemeConfig, readThemeConfig } from '@/types/commands'
import { useInstallStore } from '@/stores/installStore'
import type { ThemeConfig, LayoutConfig, StyleConfig, NotifyConfig } from '@/types/rime'

import ColorPicker from '@/components/theme/ColorPicker.vue'
import LayoutOptions from '@/components/theme/LayoutOptions.vue'
import PresetGallery from '@/components/theme/PresetGallery.vue'
import UserPresets from '@/components/theme/UserPresets.vue'
import type { ThemePreset } from '@/components/theme/PresetGallery.vue'
import { readSchemaOptions } from '@/types/commands'
import type { UserPreset } from '@/types/rime'
import ToggleSwitch from '@/components/ui/ToggleSwitch.vue'

const rimeStore = useRimeStore()
const { toast } = useFeedback()
const job = useInstallStore()
const { t } = useI18n()
const tab = ref<'presets' | 'custom'>('presets')
/**
 * 真实配置读回来了没有
 *
 * theme / layout 的初始值就是出厂那套预设，首帧算出来就是「用着」，
 * 等 handleLoad 把用户的真配置换进来才对不上 —— 于是徽标闪一下就没了。
 * 读完之前不下结论。
 */
const loaded = ref(false)
const saving = ref(false)
const saved = ref(false)

// 出厂默认就是「XGRime 浅色」那套预设，跟 PresetGallery 里第一个保持一致
const theme = ref<ThemeConfig>({
  name: 'xgrime_custom',
  backColor: '#ffffff',
  borderColor: '#f2f2f2',
  textColor: '#000000',
  hilitedTextColor: '#000000',
  hilitedBackColor: '#e8e8e8',
  candidateTextColor: '#000000',
  commentTextColor: '#888888',
  labelColor: '#666666',
  hilitedCandidateTextColor: '#000000',
  hilitedCandidateBackColor: '#fafafa',
  hilitedCandidateLabelColor: '#000000',
  hilitedCommentTextColor: '#555555',
  hilitedMarkColor: '',
})


const notify = ref<NotifyConfig>({ mode: 'always', durationMs: 1200 })
/** 一页几个候选，读自输入法实际生效的配置 */
const pageSize = ref(5)

const layout = ref<LayoutConfig>({
  fontFace: '',
  labelFontFace: '',
  horizontal: true,
  inlinePreedit: true,
  fontSize: 12,
  cornerRadius: 4,
  borderWidth: 1,
  marginX: 12,
  marginY: 8,
  hilitePaddingX: 8,
  hilitePaddingY: 4,
  candidateSpacing: 14,
  hiliteSpacing: 4,
  spacing: 10,
  roundCorner: 4,
  shadowRadius: 4,
  labelFontSize: 0,
  minWidth: 0,
  maxWidth: 720,
  labelFormat: '%s',
  markText: '',
})


const colorGroups = [
  {
    titleKey: 'background',
    items: [
      { key: 'backColor' as const },
      { key: 'borderColor' as const },
    ]
  },
  {
    titleKey: 'preedit',
    items: [
      { key: 'textColor' as const },
      { key: 'hilitedTextColor' as const },
      { key: 'hilitedBackColor' as const },
    ]
  },
  {
    titleKey: 'candidate',
    items: [
      { key: 'candidateTextColor' as const },
      { key: 'commentTextColor' as const },
      { key: 'labelColor' as const },
    ]
  },
  {
    titleKey: 'hilited',
    items: [
      { key: 'hilitedCandidateBackColor' as const },
      { key: 'hilitedCandidateTextColor' as const },
      { key: 'hilitedCandidateLabelColor' as const },
      { key: 'hilitedCommentTextColor' as const },
    ]
  },
]

/**
 * 套用预设
 *
 * 之前这里只抄了颜色，预设里写好的 layout 完全没生效 ——
 * 于是六个预设换来换去只有配色变，形状一模一样。
 * 字体是用户自己挑的，不该被预设带走。
 */
function applyPreset(preset: ThemePreset) {
  theme.value = { ...preset.theme }
  if (preset.layout) {
    layout.value = {
      ...layout.value,
      ...preset.layout,
      fontFace: layout.value.fontFace,
      labelFontFace: layout.value.labelFontFace,
    }
  }
}

/** 自己存的那套是完整的，字体也一起用上 —— 那本来就是他当时的设置 */
function applyUserPreset(preset: UserPreset) {
  theme.value = { ...preset.theme }
  layout.value = { ...preset.layout }
}

/** 图标是按方案配的，得知道当前是哪个方案 */
const schemaId = ref<string | null>(null)
async function loadSchemaId() {
  const dir = rimeStore.installInfo?.configDir
  if (!dir) return
  try {
    schemaId.value = (await readSchemaOptions(dir)).schemaId
  } catch {
    schemaId.value = null
  }
}

/** 标记开关：关掉就是把颜色清空，小狼毫看到透明色就不画 */
function toggleMark() {
  theme.value.hilitedMarkColor = theme.value.hilitedMarkColor ? '' : '#0067c0'
}

async function handleSave() {
  if (!rimeStore.installInfo?.configDir) return
  saving.value = true
  saved.value = false
  try {
    const config: StyleConfig = {
      theme: theme.value,
      layout: layout.value,
      notify: notify.value,
      pageSize: pageSize.value,
    }
    await saveThemeConfig(rimeStore.installInfo.configDir, config)
    saved.value = true
    setTimeout(() => { saved.value = false }, 3000)
    toast.success(t('theme.saveOk'))
    // 部署要编译几十秒，不等它 —— 等的话这个按钮会僵住半分钟
    job.redeploy().catch((e) => toast.error(t('theme.saveFail'), e))
  } catch (e) {
    toast.error(t('theme.saveFail'), e)
  } finally {
    saving.value = false
  }
}

async function handleLoad() {
  if (!rimeStore.installInfo?.configDir) {
    loaded.value = true
    return
  }
  try {
    const config = await readThemeConfig(rimeStore.installInfo.configDir)
    if (config) {
      theme.value = config.theme
      layout.value = config.layout
      notify.value = config.notify
      pageSize.value = config.pageSize
    }
  } catch (e) {
    toast.error(t('theme.loadFail'), e)
  } finally {
    loaded.value = true
  }
}

// 自动读取当前配置
//
// 盯的是 installInfo 本身而不是 installed：冷启动那会儿检测还没跑完，
// installInfo 是 null，这时候既不能读配置、也不能当「没装」放行 ——
// 放行了就会先按出厂默认标一下「用着」，等真配置到了又缩回去，闪一下。
onMounted(loadSchemaId)
watch(
  () => rimeStore.installInfo,
  (info) => {
    if (info?.installed) handleLoad()
    // 检测跑完了确认没装：界面上摆的就是出厂那套，直接认
    else if (info) loaded.value = true
  },
  { immediate: true },
)
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- 顶部栏 -->
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-xl font-semibold tracking-tight">{{ $t('theme.title') }}</h1>
        <p class="text-[15px] text-muted-foreground mt-0.5">{{ $t('theme.subtitle') }}</p>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="handleLoad"
          :disabled="!rimeStore.installInfo?.installed"
          class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-card text-[15px] text-foreground/80 hover:text-foreground transition-colors disabled:opacity-40"
        >
          <span class="icon-[lucide--upload] size-3.5 opacity-60" />
          {{ $t('theme.reload') }}
        </button>
        <button
          @click="handleSave"
          :disabled="saving || !rimeStore.installInfo?.installed"
          class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-primary text-primary-foreground text-[15px] hover:bg-primary/90 transition-colors disabled:opacity-40"
        >
          <span :class="saving ? 'icon-[lucide--loader-2] animate-spin' : saved ? 'icon-[lucide--check]' : 'icon-[lucide--save]'" class="size-3.5" />
          {{ saving ? $t('common.saving') : saved ? $t('common.saved') : $t('common.save') }}
        </button>
      </div>
    </div>

    <!-- 未安装提示 -->
    <div v-if="!rimeStore.installInfo?.installed" class="rounded-xl bg-amber-500/5 px-4 py-3 text-[14px] mb-5">
      <p class="text-amber-500/80">{{ $t('theme.needRime') }}</p>
    </div>

    <!-- 两个大分区：现成的一套套挑，或者自己一项项调 -->
    <div class="flex items-center gap-1 p-1 rounded-xl bg-muted/40 mb-5">
      <button
        v-for="name in (['presets', 'custom'] as const)"
        :key="name"
        class="flex-1 px-3.5 py-1.5 rounded-lg text-[14px] transition-colors"
        :class="tab === name ? 'bg-card text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'"
        @click="tab = name"
      >
        {{ name === 'presets' ? $t('theme.presets') : $t('theme.customTitle') }}
      </button>
    </div>

    <div class="flex-1 overflow-auto min-h-0 pb-8">
      <PresetGallery
          v-show="tab === 'presets'"
          :theme="theme"
          :layout="layout"
          :ready="loaded"
          @apply="applyPreset"
        />
      <div v-show="tab === 'custom'" class="space-y-4">
        <div class="rounded-xl bg-card p-4">
          <UserPresets :theme="theme" :layout="layout" @apply="applyUserPreset" />
        </div>

        <!--
          调色的控件本来挤在 280px 的窄边栏里，一路滚到底才看得全。
          现在这一整页都是它的，按宽度铺成两到三栏。
        -->
        <div class="grid grid-cols-1 min-[640px]:grid-cols-2 gap-x-8 gap-y-7 items-start">
          <div class="space-y-6">
            <!--
              「隐藏拼音」开着的时候，拼音根本不画在候选框里，预编辑那组颜色调了也看不见。
              与其让用户对着没用的三个色块试半天，不如直接收起来并说清楚原因。
            -->
            <div
              v-for="group in colorGroups"
              v-show="group.titleKey !== 'preedit' || !layout.inlinePreedit"
              :key="group.titleKey"
              class="space-y-2.5"
            >
              <h3 class="text-[15px] font-semibold text-foreground/90">{{ $t(`theme.colorGroups.${group.titleKey}`) }}</h3>
              <div class="space-y-1.5">
                <ColorPicker
                  v-for="item in group.items"
                  :key="item.key"
                  :label="$t(`theme.colors.${item.key}`)"
                  v-model="theme[item.key]"
                />
              </div>
            </div>

            <p v-if="layout.inlinePreedit" class="text-[12px] text-muted-foreground/60 leading-relaxed">
              {{ $t('theme.preeditHidden') }}
            </p>
          </div>

          <div class="space-y-4">
            <div class="rounded-xl bg-card p-4">
              <LayoutOptions v-model:layout="layout" />
            </div>

            <!--
              选中标记：小狼毫在选中项前画的那条竖杠（Windows 11 输入法那种）。
              它靠配色里的 hilited_mark_color 有没有不透明值来决定画不画。
            -->
            <div class="rounded-xl bg-card p-4 space-y-3">
              <h3 class="text-[15px] font-semibold text-foreground/90">
                {{ $t('theme.mark.title') }}
              </h3>

              <div class="flex items-center justify-between">
                <div class="min-w-0 pr-3">
                  <span class="text-[14px] text-foreground/70">{{ $t('theme.mark.show') }}</span>
                  <p class="text-[11px] text-muted-foreground/50 mt-0.5">{{ $t('theme.mark.hint') }}</p>
                </div>
                <ToggleSwitch
                  :model-value="!!theme.hilitedMarkColor"
                  @update:model-value="toggleMark"
                />
              </div>

              <ColorPicker
                v-if="theme.hilitedMarkColor"
                v-model="theme.hilitedMarkColor"
                :label="$t('theme.mark.color')"
              />

              <div class="space-y-1.5">
                <span class="text-[14px] text-foreground/70">{{ $t('theme.mark.text') }}</span>
                <input
                  v-model="layout.markText"
                  class="w-full px-3 py-1.5 rounded-lg bg-muted/40 text-[13px] outline-none focus:inset-ring-1 focus:inset-ring-primary/50"
                  :placeholder="$t('theme.mark.textHint')"
                  maxlength="4"
                />
              </div>
            </div>
          </div>

          <div class="space-y-7">
            <!--
              切中英文时小狼毫会弹一个小提示框。图标是它内建画的，改不了，
              但可以关掉或者缩短 —— 嫌它难看的话这是唯一的出路。
            -->
            <div class="rounded-xl bg-card p-4 space-y-4">
              <h3 class="text-[15px] font-semibold text-foreground/90">
                {{ $t('theme.notify.title') }}
              </h3>

              <div class="flex items-center justify-between">
                <div class="min-w-0 pr-3">
                  <span class="text-[14px] text-foreground/70">{{ $t('theme.notify.show') }}</span>
                  <p class="text-[11px] text-muted-foreground/50 mt-0.5">{{ $t('theme.notify.hint') }}</p>
                </div>
                <ToggleSwitch
                  :model-value="notify.mode === 'always'"
                  @update:model-value="notify.mode = $event ? 'always' : 'never'"
                />
              </div>

              <div v-if="notify.mode === 'always'" class="space-y-1.5">
                <div class="flex items-center justify-between">
                  <span class="text-[14px] text-foreground/70">{{ $t('theme.notify.duration') }}</span>
                  <span class="text-[12px] text-muted-foreground tabular-nums">{{ notify.durationMs }} ms</span>
                </div>
                <input
                  v-model.number="notify.durationMs"
                  type="range"
                  min="200"
                  max="3000"
                  step="100"
                  class="w-full h-1 rounded-full bg-muted/30 appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary [&::-webkit-slider-thumb]:shadow-sm"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
