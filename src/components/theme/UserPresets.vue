<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { listUserPresets, saveUserPreset, deleteUserPreset } from '@/types/commands'
import type { ThemeConfig, LayoutConfig, UserPreset } from '@/types/rime'
import { useFeedback } from '@/composables/useFeedback'

const props = defineProps<{
  theme: ThemeConfig
  layout: LayoutConfig
}>()

const emit = defineEmits<{ apply: [preset: UserPreset] }>()

const { t } = useI18n()
const { toast, confirmAction } = useFeedback()

const presets = ref<UserPreset[]>([])
const naming = ref(false)
const draftName = ref('')
const busy = ref(false)

async function load() {
  try {
    presets.value = await listUserPresets()
  } catch (e) {
    toast.error(t('theme.mine.loadFail'), e)
  }
}

function startNaming() {
  draftName.value = t('theme.mine.defaultName', { n: presets.value.length + 1 })
  naming.value = true
}

async function commit() {
  const name = draftName.value.trim()
  if (!name) return
  busy.value = true
  try {
    // 用时间戳当 id：够唯一，也天然按创建顺序排
    presets.value = await saveUserPreset({
      id: String(Date.now()),
      name,
      theme: { ...props.theme },
      layout: { ...props.layout },
    })
    naming.value = false
    toast.success(t('theme.mine.saved', { name }))
  } catch (e) {
    toast.error(t('theme.mine.saveFail'), e)
  } finally {
    busy.value = false
  }
}

async function remove(preset: UserPreset) {
  const ok = await confirmAction({
    title: t('theme.mine.removeTitle', { name: preset.name }),
    confirmText: t('common.uninstall'),
    danger: true,
  })
  if (!ok) return
  try {
    presets.value = await deleteUserPreset(preset.id)
  } catch (e) {
    toast.error(t('theme.mine.removeFail'), e)
  }
}

onMounted(load)
</script>

<template>
  <div class="space-y-3">
    <div class="flex items-center justify-between gap-3">
      <p class="text-[12px] text-muted-foreground/60">
        {{ presets.length ? $t('theme.mine.pick') : $t('theme.mine.empty') }}
      </p>
      <button
        v-if="!naming"
        class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-muted/50 text-[12px] text-foreground/80 hover:text-foreground transition-colors"
        @click="startNaming"
      >
        <span class="icon-[lucide--bookmark-plus] size-3.5 opacity-70" />
        {{ $t('theme.mine.save') }}
      </button>
    </div>

    <!-- 起名字 -->
    <!-- 两个按钮要 shrink-0，不然名字一长就把它们挤出容器 -->
    <div v-if="naming" class="flex items-center gap-2 w-full max-w-full">
      <input
        v-model="draftName"
        class="flex-1 min-w-0 px-3 py-2 rounded-lg bg-muted/40 text-[13px] outline-none focus:inset-ring-1 focus:inset-ring-primary/50"
        :placeholder="$t('theme.mine.namePlaceholder')"
        maxlength="24"
        @keydown.enter="commit"
        @keydown.esc="naming = false"
      />
      <button
        class="shrink-0 px-3 py-2 rounded-lg bg-primary text-primary-foreground text-[12px] disabled:opacity-40"
        :disabled="busy || !draftName.trim()"
        @click="commit"
      >
        {{ $t('common.confirm') }}
      </button>
      <button
        class="shrink-0 px-3 py-2 rounded-lg text-[12px] text-muted-foreground hover:text-foreground"
        @click="naming = false"
      >
        {{ $t('common.cancel') }}
      </button>
    </div>

    <div v-if="presets.length" class="grid grid-cols-3 gap-2.5">
      <div v-for="p in presets" :key="p.id" class="group relative">
        <button
          class="w-full rounded-xl bg-card p-2 border border-transparent hover:border-primary/30 transition-all text-left"
          @click="emit('apply', p)"
        >
          <div
            class="rounded-lg h-9 mb-2 flex items-center px-2 gap-1"
            :style="{
              backgroundColor: p.theme.backColor,
              border: `1px solid ${p.theme.borderColor}`,
            }"
          >
            <span
              class="text-[9px] px-1 rounded"
              :style="{
                color: p.theme.hilitedCandidateTextColor,
                backgroundColor: p.theme.hilitedCandidateBackColor,
              }"
            >中</span>
            <span class="text-[9px]" :style="{ color: p.theme.candidateTextColor }">文</span>
            <span class="text-[9px]" :style="{ color: p.theme.candidateTextColor }">字</span>
          </div>
          <span class="text-[12px] text-foreground/80 truncate block">{{ p.name }}</span>
        </button>
        <button
          class="absolute top-1.5 right-1.5 p-1 rounded-md bg-card/90 text-muted-foreground/60 opacity-0 group-hover:opacity-100 hover:text-destructive transition-all"
          :title="$t('common.uninstall')"
          @click.stop="remove(p)"
        >
          <span class="icon-[lucide--trash-2] size-3" />
        </button>
      </div>
    </div>
  </div>
</template>
