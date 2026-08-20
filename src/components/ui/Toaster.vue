<script setup lang="ts">
import { useFeedback } from '@/composables/useFeedback'

const { toasts, dismiss } = useFeedback()

const style = {
  success: { icon: 'icon-[lucide--check-circle-2]', tone: 'text-emerald-500' },
  error: { icon: 'icon-[lucide--alert-circle]', tone: 'text-red-500' },
  info: { icon: 'icon-[lucide--info]', tone: 'text-sky-500' },
} as const
</script>

<template>
  <div class="fixed bottom-5 right-5 z-50 flex flex-col-reverse gap-2 w-[360px] pointer-events-none">
    <TransitionGroup
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 translate-y-2"
      leave-active-class="transition duration-150 ease-in"
      leave-to-class="opacity-0 translate-y-1"
    >
      <div
        v-for="t in toasts"
        :key="t.id"
        class="pointer-events-auto rounded-xl bg-card shadow-lg ring-1 ring-border/60 px-4 py-3 flex gap-3 items-start"
      >
        <span :class="[style[t.kind].icon, style[t.kind].tone]" class="size-[18px] shrink-0 mt-px" />
        <div class="min-w-0 flex-1">
          <p class="text-[14px] leading-snug text-foreground">{{ t.message }}</p>
          <p v-if="t.detail" class="text-[12px] leading-snug text-muted-foreground mt-1 break-words">
            {{ t.detail }}
          </p>
        </div>
        <button
          class="shrink-0 text-muted-foreground/60 hover:text-foreground transition-colors"
          @click="dismiss(t.id)"
        >
          <span class="icon-[lucide--x] size-3.5" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
