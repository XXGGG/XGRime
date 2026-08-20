<script setup lang="ts">
/**
 * 开关
 *
 * 原来这段 markup 在四个地方各抄了一份，改一处忘三处 —— 旋钮跑出轨道、
 * 深色模式下白轨道配白旋钮看不见，都是这么来的。收成一个组件。
 *
 * 颜色一律走 token 对：轨道 `bg-primary` 就配 `bg-primary-foreground` 的旋钮。
 * 深色模式下 `--primary` 是近白色，写死 `bg-white` 就是白配白。
 */
defineProps<{ disabled?: boolean }>()
const model = defineModel<boolean>({ required: true })
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="model"
    class="relative w-9 h-5 rounded-full shrink-0 transition-colors disabled:opacity-50"
    :class="model ? 'bg-primary' : 'bg-muted-foreground/30'"
    :disabled="disabled"
    @click="model = !model"
  >
    <!-- 旋钮钉死在轨道左边，开的时候只平移：不写 left 的话位置靠静态流推算，会飘出轨道 -->
    <span
      class="absolute top-0.5 left-0.5 size-4 rounded-full shadow transition-transform"
      :class="model ? 'bg-primary-foreground translate-x-4' : 'bg-background translate-x-0'"
    />
  </button>
</template>
