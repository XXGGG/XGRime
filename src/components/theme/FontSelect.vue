<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { getSystemFonts } from '@/types/commands'

const props = defineProps<{
  modelValue: string
  label: string
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const fonts = ref<string[]>([])
const search = ref(props.modelValue)
const open = ref(false)
const loading = ref(false)

const filtered = computed(() => {
  if (!search.value) return fonts.value
  const q = search.value.toLowerCase()
  return fonts.value.filter(f => f.toLowerCase().includes(q))
})

function select(font: string) {
  search.value = font
  emit('update:modelValue', font)
  open.value = false
}

function handleInput(e: Event) {
  search.value = (e.target as HTMLInputElement).value
  open.value = true
}

function handleBlur() {
  // 延迟关闭，让点击事件先触发
  setTimeout(() => { open.value = false }, 150)
}

function clear() {
  search.value = ''
  emit('update:modelValue', '')
}

watch(() => props.modelValue, (v) => {
  search.value = v
})

onMounted(async () => {
  loading.value = true
  try {
    fonts.value = await getSystemFonts()
  } catch {
    fonts.value = []
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="space-y-1.5">
    <span class="text-[14px] text-foreground/70">{{ label }}</span>
    <div class="relative">
      <input
        type="text"
        :value="search"
        @input="handleInput"
        @focus="open = true"
        @blur="handleBlur"
        :placeholder="placeholder || $t('theme.font.search')"
        class="w-full px-3 py-1.5 text-[14px] rounded-lg bg-muted/30 border border-transparent focus:border-primary/30 text-foreground placeholder:text-muted-foreground/40 outline-none pr-8"
      />
      <button
        v-if="search"
        @mousedown.prevent="clear"
        class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground/40 hover:text-muted-foreground"
      >
        <span class="icon-[lucide--x] size-3.5" />
      </button>

      <!-- 下拉列表 -->
      <div
        v-if="open && !loading"
        class="absolute top-full left-0 right-0 mt-1 rounded-lg bg-card border border-border/30 shadow-lg max-h-48 overflow-auto z-50"
      >
        <div v-if="filtered.length === 0" class="px-3 py-2 text-[13px] text-muted-foreground">
          {{ $t('theme.font.notFound') }}
        </div>
        <button
          v-for="font in filtered.slice(0, 50)"
          :key="font"
          @mousedown.prevent="select(font)"
          class="w-full text-left px-3 py-1.5 text-[14px] hover:bg-muted/50 transition-colors truncate"
          :style="{ fontFamily: font }"
        >
          {{ font }}
        </button>
      </div>
    </div>
  </div>
</template>
