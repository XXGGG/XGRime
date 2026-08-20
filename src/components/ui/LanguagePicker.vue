<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { LOCALES, setLocale, currentLocale, type LocaleCode } from '@/i18n'

const open = ref(false)
const root = ref<HTMLElement | null>(null)
const active = computed(() => LOCALES.find((l) => l.code === currentLocale()) ?? LOCALES[0])

function pick(code: LocaleCode) {
  setLocale(code)
  open.value = false
}

// 点外面 / 按 Esc 收起来。用不了原生 select，得自己收。
function onDocClick(e: MouseEvent) {
  if (open.value && root.value && !root.value.contains(e.target as Node)) open.value = false
}
function onEsc(e: KeyboardEvent) {
  if (e.key === 'Escape') open.value = false
}
onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onEsc)
})
onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onEsc)
})
</script>

<template>
  <div ref="root" class="relative">
    <button
      class="w-full flex items-center justify-between gap-1.5 px-2 py-1.5 rounded-md text-[12px] text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
      :title="$t('common.language')"
      @click="open = !open"
    >
      <span class="flex items-center gap-1.5 min-w-0">
        <span class="icon-[lucide--languages] size-3.5 shrink-0" />
        <span class="truncate">{{ active.label }}</span>
      </span>
      <span
        class="icon-[lucide--chevron-up] size-3 shrink-0 transition-transform"
        :class="open ? '' : 'rotate-180'"
      />
    </button>

    <!--
      往上弹。这个控件贴在侧边栏最底下，向下弹会被窗口底边裁掉，
      只剩第一项露在外面。
    -->
    <Transition
      enter-active-class="transition duration-100 ease-out"
      enter-from-class="opacity-0 translate-y-1"
      leave-active-class="transition duration-75 ease-in"
      leave-to-class="opacity-0"
    >
      <div
        v-if="open"
        class="absolute bottom-full left-0 right-0 mb-1 rounded-lg bg-card shadow-lg ring-1 ring-border/60 p-1 z-50"
      >
        <button
          v-for="l in LOCALES"
          :key="l.code"
          class="w-full text-left px-2.5 py-1.5 rounded-md text-[13px] transition-colors flex items-center justify-between gap-2"
          :class="l.code === active.code
            ? 'bg-primary/10 text-foreground'
            : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'"
          @click="pick(l.code)"
        >
          <span class="truncate">{{ l.label }}</span>
          <span v-if="l.code === active.code" class="icon-[lucide--check] size-3 text-primary shrink-0" />
        </button>
      </div>
    </Transition>
  </div>
</template>
