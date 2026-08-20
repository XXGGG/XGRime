<script setup lang="ts">
import { useFeedback } from '@/composables/useFeedback'

const { pending, settleConfirm } = useFeedback()
</script>

<template>
  <Transition
    enter-active-class="transition duration-150 ease-out"
    enter-from-class="opacity-0"
    leave-active-class="transition duration-100 ease-in"
    leave-to-class="opacity-0"
  >
    <div
      v-if="pending"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-[2px]"
      @click.self="settleConfirm(false)"
    >
      <div class="w-[380px] rounded-2xl bg-card shadow-2xl ring-1 ring-border/60 p-6">
        <h2 class="text-[15px] font-semibold text-foreground">{{ pending.title }}</h2>
        <p v-if="pending.description" class="text-[14px] text-muted-foreground mt-2 leading-relaxed">
          {{ pending.description }}
        </p>

        <div class="flex justify-end gap-2 mt-6">
          <button
            class="px-4 py-2 rounded-lg text-[14px] text-muted-foreground hover:bg-muted/60 transition-colors"
            @click="settleConfirm(false)"
          >
            {{ pending.cancelText || $t('common.cancel') }}
          </button>
          <button
            class="px-4 py-2 rounded-lg text-[14px] font-medium transition-colors"
            :class="pending.danger
              ? 'bg-red-500 hover:bg-red-600 text-white'
              : 'bg-primary hover:bg-primary/90 text-primary-foreground'"
            @click="settleConfirm(true)"
          >
            {{ pending.confirmText || $t('common.confirm') }}
          </button>
        </div>
      </div>
    </div>
  </Transition>
</template>
