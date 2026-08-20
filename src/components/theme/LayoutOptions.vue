<script setup lang="ts">
import { ref } from 'vue'
import type { LayoutConfig } from '@/types/rime'
import FontSelect from './FontSelect.vue'
import ToggleSwitch from '@/components/ui/ToggleSwitch.vue'

const props = defineProps<{
  layout: LayoutConfig
}>()

const emit = defineEmits<{
  'update:layout': [value: LayoutConfig]
}>()

const showAdvanced = ref(false)

function update<K extends keyof LayoutConfig>(key: K, value: LayoutConfig[K]) {
  emit('update:layout', { ...props.layout, [key]: value })
}

const sliderClass = "w-full h-1 rounded-full bg-muted/30 appearance-none cursor-pointer [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:w-3 [&::-webkit-slider-thumb]:h-3 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-primary [&::-webkit-slider-thumb]:shadow-sm"
</script>

<template>
  <div class="space-y-4">
    <h3 class="text-[15px] font-semibold text-foreground/90 pt-1">{{ $t('theme.layout.title') }}</h3>

    <!-- ═══ 常规设置 ═══ -->

    <!-- 候选词方向 -->
    <div class="flex items-center justify-between">
      <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.direction') }}</span>
      <div class="flex rounded-lg bg-muted/30 overflow-hidden">
        <button @click="update('horizontal', true)" class="px-3 py-1 text-[13px] transition-colors"
          :class="layout.horizontal ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'">{{ $t('theme.layout.horizontal') }}</button>
        <button @click="update('horizontal', false)" class="px-3 py-1 text-[13px] transition-colors"
          :class="!layout.horizontal ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'">{{ $t('theme.layout.vertical') }}</button>
      </div>
    </div>

    <!-- 隐藏拼音 -->
    <div class="flex items-center justify-between">
      <div>
        <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.hidePinyin') }}</span>
        <p class="text-[11px] text-muted-foreground/50 mt-0.5">{{ $t('theme.layout.hidePinyinHint') }}</p>
      </div>
      <ToggleSwitch
        class="ml-3"
        :model-value="layout.inlinePreedit"
        @update:model-value="update('inlinePreedit', $event)"
      />
    </div>

    <!-- 字号 -->
    <div class="space-y-1.5">
      <div class="flex items-center justify-between">
        <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.fontSize') }}</span>
        <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.fontSize }}pt</span>
      </div>
      <input type="range" :value="layout.fontSize" @input="update('fontSize', Number(($event.target as HTMLInputElement).value))"
        min="8" max="28" step="1" :class="sliderClass" />
    </div>

    <!-- 圆角 -->
    <div class="space-y-1.5">
      <div class="flex items-center justify-between">
        <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.cornerRadius') }}</span>
        <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.cornerRadius }}px</span>
      </div>
      <input type="range" :value="layout.cornerRadius" @input="update('cornerRadius', Number(($event.target as HTMLInputElement).value))"
        min="0" max="20" step="1" :class="sliderClass" />
    </div>

    <!-- 边框宽度 -->
    <div class="space-y-1.5">
      <div class="flex items-center justify-between">
        <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.borderWidth') }}</span>
        <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.borderWidth }}px</span>
      </div>
      <input type="range" :value="layout.borderWidth" @input="update('borderWidth', Number(($event.target as HTMLInputElement).value))"
        min="0" max="10" step="1" :class="sliderClass" />
    </div>

    <!-- 字体 -->
    <FontSelect
      :label="$t('theme.layout.font')"
      :model-value="layout.fontFace"
      @update:model-value="update('fontFace', $event)"
    />

    <!-- ═══ 高级设置折叠 ═══ -->
    <button @click="showAdvanced = !showAdvanced"
      class="flex items-center gap-1.5 text-[13px] text-muted-foreground hover:text-foreground transition-colors mt-2">
      <span :class="showAdvanced ? 'icon-[lucide--chevron-down]' : 'icon-[lucide--chevron-right]'" class="size-3.5" />
      {{ $t('theme.layout.advanced') }}
    </button>

    <template v-if="showAdvanced">
      <!-- 左右边距 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.marginX') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.marginX }}px</span>
        </div>
        <input type="range" :value="layout.marginX" @input="update('marginX', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 上下边距 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.marginY') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.marginY }}px</span>
        </div>
        <input type="range" :value="layout.marginY" @input="update('marginY', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 高亮内边距：小狼毫本来就分左右和上下两个键 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.hilitePaddingX') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.hilitePaddingX }}px</span>
        </div>
        <input type="range" :value="layout.hilitePaddingX" @input="update('hilitePaddingX', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.hilitePaddingY') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.hilitePaddingY }}px</span>
        </div>
        <input type="range" :value="layout.hilitePaddingY" @input="update('hilitePaddingY', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 候选项间距 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.candidateSpacing') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.candidateSpacing }}px</span>
        </div>
        <input type="range" :value="layout.candidateSpacing" @input="update('candidateSpacing', Number(($event.target as HTMLInputElement).value))"
          min="0" max="40" step="1" :class="sliderClass" />
      </div>

      <!-- 标签与候选间距 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.hiliteSpacing') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.hiliteSpacing }}px</span>
        </div>
        <input type="range" :value="layout.hiliteSpacing" @input="update('hiliteSpacing', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 编码区与候选区间距 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.spacing') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.spacing }}px</span>
        </div>
        <input type="range" :value="layout.spacing" @input="update('spacing', Number(($event.target as HTMLInputElement).value))"
          min="0" max="30" step="1" :class="sliderClass" />
      </div>

      <!-- 高亮圆角 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.roundCorner') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.roundCorner }}px</span>
        </div>
        <input type="range" :value="layout.roundCorner" @input="update('roundCorner', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 阴影半径 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.shadowRadius') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.shadowRadius }}px</span>
        </div>
        <input type="range" :value="layout.shadowRadius" @input="update('shadowRadius', Number(($event.target as HTMLInputElement).value))"
          min="0" max="20" step="1" :class="sliderClass" />
      </div>

      <!-- 标签字号 -->
      <div class="space-y-2">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.labelFontSize') }}</span>
          <div class="flex items-center gap-2">
            <span class="text-[12px] text-muted-foreground">{{ $t('theme.layout.followMainSize') }}</span>
            <ToggleSwitch
              :model-value="layout.labelFontSize === 0"
              @update:model-value="update('labelFontSize', $event ? 0 : layout.fontSize)"
            />
          </div>
        </div>
        <template v-if="layout.labelFontSize !== 0">
          <div class="flex items-center justify-between">
            <span class="text-[12px] text-muted-foreground/60">{{ $t('theme.layout.customSize') }}</span>
            <span class="text-[12px] text-muted-foreground tabular-nums">{{ layout.labelFontSize }}pt</span>
          </div>
          <input type="range" :value="layout.labelFontSize"
            @input="update('labelFontSize', Number(($event.target as HTMLInputElement).value))"
            min="8" max="28" step="1" :class="sliderClass" />
        </template>
      </div>

      <!-- 最小宽度：小狼毫默认 160，框显得宽就是它撑的 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.minWidth') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">
            {{ layout.minWidth === 0 ? $t('theme.layout.autoWidth') : `${layout.minWidth}px` }}
          </span>
        </div>
        <input type="range" :value="layout.minWidth"
          @input="update('minWidth', Number(($event.target as HTMLInputElement).value))"
          min="0" max="400" step="10" :class="sliderClass" />
        <p class="text-[11px] text-muted-foreground/50">{{ $t('theme.layout.minWidthHint') }}</p>
      </div>

      <!-- 最大宽度 -->
      <div class="space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[14px] text-foreground/70">{{ $t('theme.layout.maxWidth') }}</span>
          <span class="text-[12px] text-muted-foreground tabular-nums">
            {{ layout.maxWidth === 0 ? $t('theme.layout.noLimit') : `${layout.maxWidth}px` }}
          </span>
        </div>
        <input type="range" :value="layout.maxWidth"
          @input="update('maxWidth', Number(($event.target as HTMLInputElement).value))"
          min="0" max="1200" step="20" :class="sliderClass" />
      </div>

      <!-- 标签字体 -->
      <FontSelect
        :label="$t('theme.layout.labelFont')"
        :placeholder="$t('theme.layout.labelFontHint')"
        :model-value="layout.labelFontFace"
        @update:model-value="update('labelFontFace', $event)"
      />
    </template>
  </div>
</template>
