<script setup lang="ts">
defineProps<{
  label: string
  modelValue: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()
</script>

<template>
  <div class="flex items-center justify-between gap-3 py-0.5">
    <label class="text-[14px] text-foreground/70 shrink-0">{{ label }}</label>
    <div class="flex items-center gap-1.5">
      <div class="relative">
        <input
          type="color"
          :value="modelValue"
          @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
          class="absolute inset-0 opacity-0 cursor-pointer"
        />
        <div
          class="w-7 h-7 rounded-md border border-border/50 shadow-sm"
          :style="{ backgroundColor: modelValue }"
        />
      </div>
      <input
        type="text"
        :value="modelValue"
        @change="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        class="w-[72px] px-2 py-1 text-[12px] font-mono rounded-md bg-card border border-transparent focus:border-primary/30 text-foreground/80 text-center outline-none"
        maxlength="7"
      />
    </div>
  </div>
</template>
