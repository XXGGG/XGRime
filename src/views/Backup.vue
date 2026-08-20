<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useRimeStore } from '@/stores/rimeStore'
import { useInstallStore } from '@/stores/installStore'
import { useFeedback } from '@/composables/useFeedback'
import { exportSettings, inspectBackup, importSettings } from '@/types/commands'
import type { BackupParts } from '@/types/rime'
import { shortPath } from '@/lib/utils'

const rimeStore = useRimeStore()
const job = useInstallStore()
const { t } = useI18n()
const { toast, confirmAction } = useFeedback()

// 词频那一项默认不勾：几十 MB，而且换台机器不一定还认得
// 自造词 XGRime 还没做，键先留着但不给勾；词频默认不勾 —— 几十 MB，
// 而且换台机器不一定还认得
const parts = ref<BackupParts>({ theme: true, schemas: true, phrases: false, userdb: false })
/** 还没做的项：位置先占着，做完了从这里删掉就能用 */
const PENDING: readonly string[] = ['phrases']
const KEYS = ['theme', 'schemas', 'phrases', 'userdb'] as const
const busy = ref<'export' | 'import' | null>(null)

const dir = () => rimeStore.installInfo?.configDir ?? null

function human(bytes: number) {
  return bytes > 1024 * 1024
    ? `${(bytes / 1024 / 1024).toFixed(1)} MB`
    : `${Math.max(1, Math.round(bytes / 1024))} KB`
}

async function doExport() {
  const configDir = dir()
  if (!configDir) return
  const target = await save({
    defaultPath: 'XGRime-设置备份.zip',
    filters: [{ name: 'Zip', extensions: ['zip'] }],
  })
  if (!target) return

  busy.value = 'export'
  try {
    const r = await exportSettings(configDir, target, parts.value)
    toast.success(t('backup.exported', { n: r.files, size: human(r.bytes) }))
  } catch (e) {
    toast.error(t('backup.exportFailed'), e)
  } finally {
    busy.value = null
  }
}

async function doImport() {
  const configDir = dir()
  if (!configDir) return
  const picked = await open({ multiple: false, filters: [{ name: 'Zip', extensions: ['zip'] }] })
  if (typeof picked !== 'string') return

  busy.value = 'import'
  try {
    // 先看包里有什么，再问用户确认 —— 这一步会覆盖他现在的设置
    const info = await inspectBackup(picked)
    const has = KEYS.filter((k) => info.parts[k]).map((k) => t(`backup.parts.${k}`))
    const ok = await confirmAction({
      title: t('backup.importConfirm'),
      description: t('backup.importDetail', { list: has.join('、'), app: info.app }),
      confirmText: t('backup.import'),
    })
    if (!ok) return

    const r = await importSettings(configDir, picked, parts.value)
    toast.success(
      r.backupDir
        ? t('backup.importedWithBackup', { n: r.files, dir: shortPath(r.backupDir) })
        : t('backup.imported', { n: r.files }),
    )
    job.redeploy().catch((e) => toast.error(t('backup.importFailed'), e))
  } catch (e) {
    toast.error(t('backup.importFailed'), e)
  } finally {
    busy.value = null
  }
}
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="mb-6">
      <h1 class="text-xl font-semibold tracking-tight">{{ $t('backup.title') }}</h1>
      <p class="text-[15px] text-muted-foreground mt-0.5">{{ $t('backup.subtitle') }}</p>
    </div>

    <div v-if="!rimeStore.installInfo?.installed" class="rounded-xl bg-amber-500/5 px-4 py-3 text-[14px] mb-5">
      <p class="text-amber-500/80">{{ $t('theme.needRime') }}</p>
    </div>

    <div class="space-y-5">
      <p class="text-[13px] text-muted-foreground/70 leading-relaxed">
        {{ $t('backup.hint') }}
      </p>

      <div class="space-y-2.5">
        <label
          v-for="k in KEYS"
          :key="k"
          class="flex items-start gap-2.5 select-none"
          :class="PENDING.includes(k) ? 'opacity-50' : 'cursor-pointer'"
        >
          <input
            v-model="parts[k]"
            type="checkbox"
            :disabled="PENDING.includes(k)"
            class="mt-1 size-3.5 rounded accent-[--color-primary]"
          />
          <span class="min-w-0">
            <span class="text-[14px] text-foreground/80">{{ $t(`backup.parts.${k}`) }}</span>
            <span v-if="PENDING.includes(k)" class="ml-1.5 text-[11px] text-muted-foreground/50">
              {{ $t('backup.pending') }}
            </span>
            <span class="block text-[11px] text-muted-foreground/50 mt-0.5">
              {{ $t(`backup.partHints.${k}`) }}
            </span>
          </span>
        </label>
      </div>

      <div class="flex items-center gap-2 pt-1">
      <button
        class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-primary text-primary-foreground text-[14px] hover:bg-primary/90 transition-colors disabled:opacity-40"
        :disabled="busy !== null || !rimeStore.installInfo?.installed"
        @click="doExport"
      >
        <span
          :class="busy === 'export' ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--download]'"
          class="size-3.5"
        />
        {{ $t('backup.export') }}
      </button>
      <button
        class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg bg-muted/50 text-[14px] text-foreground/80 hover:text-foreground transition-colors disabled:opacity-40"
        :disabled="busy !== null || !rimeStore.installInfo?.installed"
        @click="doImport"
      >
        <span
          :class="busy === 'import' ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--upload]'"
          class="size-3.5"
        />
          {{ $t('backup.import') }}
        </button>
      </div>
    </div>
  </div>
</template>
