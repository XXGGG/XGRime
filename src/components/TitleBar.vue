<script setup lang="ts">
import { Window } from '@tauri-apps/api/window'
import { ref, onMounted } from 'vue'

const appWindow = Window.getCurrent()
const isMaximized = ref(false)

const minimize = () => appWindow.minimize()
const toggleMaximize = async () => {
  await appWindow.toggleMaximize()
  isMaximized.value = await appWindow.isMaximized()
}
const close = () => appWindow.close()

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized()
})
</script>

<template>
  <div data-tauri-drag-region
    class="h-9 shrink-0 flex items-center justify-between bg-card/50 select-none z-50">
    <div class="pl-4 flex items-center gap-2" data-tauri-drag-region>
      <span class="text-[13px] font-semibold tracking-wide text-foreground/80">XGRime</span>
    </div>
    <div class="flex h-full">
      <button @click="minimize"
        class="w-11 h-full flex items-center justify-center hover:bg-muted/60 transition-colors text-muted-foreground hover:text-foreground">
        <span class="icon-[lucide--minus] size-3.5" />
      </button>
      <button @click="toggleMaximize"
        class="w-11 h-full flex items-center justify-center hover:bg-muted/60 transition-colors text-muted-foreground hover:text-foreground">
        <span class="icon-[lucide--square] size-3" />
      </button>
      <button @click="close"
        class="w-11 h-full flex items-center justify-center hover:bg-red-500/90 hover:text-white transition-colors text-muted-foreground">
        <span class="icon-[lucide--x] size-3.5" />
      </button>
    </div>
  </div>
</template>
