<script setup lang="ts">
import { ref, computed } from 'vue'
import type { ThemeConfig, LayoutConfig } from '@/types/rime'
import presetData from '@/data/presets.json'

export interface ThemePreset {
  /** 显示名由界面语言决定，这里只存键 */
  key: string
  theme: ThemeConfig
  layout: Partial<LayoutConfig>
}

const presets = presetData as ThemePreset[]

const props = defineProps<{
  theme: ThemeConfig
  layout: LayoutConfig
  /** 真实配置读回来了没有。没读回来之前别标「用着」—— 那时候摆的还是出厂默认值 */
  ready: boolean
}>()

/**
 * 哪一套正被用着
 *
 * 不记「上次点了哪个」而是拿当前配置跟每套比对：这样重开应用、或者点了
 * 「读取当前配置」之后也认得出来。只比预设自己写了的那些项 —— 字体是用户
 * 自己挑的，预设不带，拿它比会永远对不上。
 */
const selected = computed(() => {
  if (!props.ready) return ''
  const same = (p: ThemePreset) => {
    const t = p.theme as unknown as Record<string, unknown>
    const cur = props.theme as unknown as Record<string, unknown>
    for (const k of Object.keys(t)) {
      if (k === 'name') continue
      if (t[k] !== cur[k]) return false
    }
    const l = p.layout as unknown as Record<string, unknown>
    const curL = props.layout as unknown as Record<string, unknown>
    return Object.keys(l).every((k) => l[k] === curL[k])
  }
  return presets.find(same)?.key ?? ''
})

/**
 * 缩略图是小狼毫自己画出来的真实候选框，不是前端仿的
 *
 * 见 scripts/shoot-presets.py：逐套写进配置、部署、敲字、抓窗口。
 * 图上的间距、圆角、高亮块宽度就是部署完看到的样子。
 */
const shots = import.meta.glob<string>('@/assets/presets/*.png', {
  eager: true,
  import: 'default',
})
const shotOf = (key: string) =>
  shots[Object.keys(shots).find((p) => p.endsWith(`/${key}.png`)) ?? ''] ?? ''

// 前两套是出厂默认，剩下的收起来 —— 一次摊开二十套，第一眼全是图，反而不好挑
const featured = presets.slice(0, 2)
const others = presets.slice(2)
// 竖排的图又窄又高，跟横排堆在同一列排不整齐，分开摆
const horizontal = computed(() => others.filter((p) => p.layout.horizontal !== false))
const vertical = computed(() => others.filter((p) => p.layout.horizontal === false))

const expanded = ref(false)

const emit = defineEmits<{ apply: [preset: ThemePreset] }>()
</script>

<template>
  <div class="space-y-3">
    <div class="space-y-2">
      <button
        v-for="preset in featured"
        :key="preset.key"
        class="w-full rounded-xl bg-card p-2 border transition-all group"
        :class="selected === preset.key
          ? 'border-primary inset-ring-1 inset-ring-primary/40'
          : 'border-transparent hover:border-primary/30'"
        @click="emit('apply', preset)"
      >
        <img
          :src="shotOf(preset.key)"
          :alt="$t(`theme.presetNames.${preset.key}`)"
          class="max-w-full h-auto mx-auto"
        />
        <span
          class="block mt-2 text-[12px] transition-colors"
          :class="selected === preset.key ? 'text-primary font-medium' : 'text-muted-foreground group-hover:text-foreground'"
        >
          {{ $t(`theme.presetNames.${preset.key}`) }}
          <span v-if="selected === preset.key">· {{ $t('theme.presetInUse') }}</span>
        </span>
      </button>
    </div>

    <button
      class="w-full flex items-center justify-center gap-1.5 py-1.5 rounded-lg text-[12px] text-muted-foreground hover:text-foreground transition-colors"
      @click="expanded = !expanded"
    >
      <span
        class="icon-[lucide--chevron-down] size-3.5 transition-transform"
        :class="expanded && 'rotate-180'"
      />
      {{ $t('theme.morePresets', { n: others.length }) }}
    </button>

    <div v-if="expanded" class="space-y-5">
      <div class="space-y-2">
        <h4 class="text-[12px] text-muted-foreground/60">
          {{ $t('theme.orientation.horizontal') }} · {{ horizontal.length }}
        </h4>
        <button
          v-for="preset in horizontal"
          :key="preset.key"
          class="w-full rounded-xl bg-card p-2 border transition-all group"
          :class="selected === preset.key
            ? 'border-primary inset-ring-1 inset-ring-primary/40'
            : 'border-transparent hover:border-primary/30'"
          @click="emit('apply', preset)"
        >
          <img
            :src="shotOf(preset.key)"
            :alt="$t(`theme.presetNames.${preset.key}`)"
            class="max-w-full h-auto mx-auto"
            loading="lazy"
          />
          <span
            class="block mt-2 text-[12px] transition-colors"
            :class="selected === preset.key ? 'text-primary font-medium' : 'text-muted-foreground group-hover:text-foreground'"
          >
            {{ $t(`theme.presetNames.${preset.key}`) }}
            <span v-if="selected === preset.key">· {{ $t('theme.presetInUse') }}</span>
          </span>
        </button>
      </div>

      <div v-if="vertical.length" class="space-y-2">
        <h4 class="text-[12px] text-muted-foreground/60">
          {{ $t('theme.orientation.vertical') }} · {{ vertical.length }}
        </h4>
        <!-- 竖排的图本来就窄，并排放才不浪费一整行；按原尺寸居中，别横向拉伸 -->
        <div class="grid grid-cols-3 gap-2">
          <button
            v-for="preset in vertical"
            :key="preset.key"
            class="rounded-xl bg-card p-2 border transition-all group flex flex-col items-center"
            :class="selected === preset.key
              ? 'border-primary inset-ring-1 inset-ring-primary/40'
              : 'border-transparent hover:border-primary/30'"
            @click="emit('apply', preset)"
          >
            <img
              :src="shotOf(preset.key)"
              :alt="$t(`theme.presetNames.${preset.key}`)"
              class="max-w-full h-auto"
              loading="lazy"
            />
            <span
              class="block mt-2 text-[12px] transition-colors"
              :class="selected === preset.key ? 'text-primary font-medium' : 'text-muted-foreground group-hover:text-foreground'"
            >
              {{ $t(`theme.presetNames.${preset.key}`) }}
              <span v-if="selected === preset.key">· {{ $t('theme.presetInUse') }}</span>
            </span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
