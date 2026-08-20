<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRimeStore } from '@/stores/rimeStore'
import { useFeedback } from '@/composables/useFeedback'
import { readActiveSchema, switchActiveSchema } from '@/types/commands'
import type { SchemaBrief } from '@/types/rime'

const rimeStore = useRimeStore()
const { t } = useI18n()
const { toast } = useFeedback()

const current = ref('')
const available = ref<SchemaBrief[]>([])
const switching = ref<string | null>(null)

async function load() {
  const dir = rimeStore.installInfo?.configDir
  if (!dir) {
    available.value = []
    return
  }
  try {
    const info = await readActiveSchema(dir)
    current.value = info.current
    available.value = info.available
  } catch {
    available.value = []
  }
}

async function pick(schema: string) {
  const dir = rimeStore.installInfo?.configDir
  if (!dir || schema === current.value || switching.value) return
  switching.value = schema
  try {
    // 这一条里面就带了停服和启动 —— 光改文件不够，服务把当前方案缓在内存里
    const info = await switchActiveSchema(dir, schema)
    current.value = info.current
    available.value = info.available
    toast.success(t('schemas.switched', { name: nameOf(schema) }))
  } catch (e) {
    toast.error(t('schemas.switchFailed'), e)
    await load()
  } finally {
    switching.value = null
  }
}

const nameOf = (id: string) => available.value.find((s) => s.schemaId === id)?.name || id

watch(() => rimeStore.installInfo?.configDir, load, { immediate: true })
defineExpose({ load })
</script>

<template>
  <div v-if="available.length > 1" class="rounded-xl bg-card p-4 space-y-3">
    <div>
      <h3 class="text-[15px] font-semibold text-foreground/90">{{ $t('schemas.active') }}</h3>
      <p class="text-[12px] text-muted-foreground/60 mt-1">{{ $t('schemas.activeHint') }}</p>
    </div>

    <div class="flex flex-wrap gap-2">
      <button
        v-for="s in available"
        :key="s.schemaId"
        class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-[13px] transition-colors disabled:opacity-60"
        :class="s.schemaId === current
          ? 'bg-primary text-primary-foreground'
          : 'bg-muted/50 text-foreground/80 hover:text-foreground'"
        :disabled="switching !== null"
        @click="pick(s.schemaId)"
      >
        <span
          v-if="switching === s.schemaId"
          class="icon-[lucide--loader-2] animate-spin size-3.5"
        />
        <span v-else-if="s.schemaId === current" class="icon-[lucide--check] size-3.5" />
        {{ s.name }}
      </button>
    </div>
  </div>
</template>
