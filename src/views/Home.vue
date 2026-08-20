<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRimeStore } from '@/stores/rimeStore'
import { useInstallStore } from '@/stores/installStore'
import {
  openConfigDir,
  downloadRime,
  uninstallRime,
  backupLeftoverConfig,
  checkRimeUpdate,
} from '@/types/commands'
import { listen } from '@tauri-apps/api/event'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { DownloadProgress, RimeUpdate } from '@/types/rime'
import { useFeedback } from '@/composables/useFeedback'

const rimeStore = useRimeStore()
const job = useInstallStore()
const { toast, confirmAction } = useFeedback()
const { t } = useI18n()

const downloading = ref(false)
const downloadDone = ref(false)
const cleaning = ref(false)
const progress = ref<DownloadProgress | null>(null)
const update = ref<RimeUpdate | null>(null)

const info = computed(() => rimeStore.installInfo)
/** 平台不同叫法不同，文案里统一用这个变量替换 */
const ime = computed(() =>
  info.value?.platform === 'macos' ? t('ime.squirrel') : t('ime.weasel'),
)

async function handleDeploy() {
  try {
    // 部署器编译完才退出，所以这一句会一直等到真的好了
    const outcome = await job.redeploy()
    toast.success(
      outcome.confirmed
        ? t('home.deployDone', { ime: ime.value })
        : t('home.deployOk', { ime: ime.value }),
    )
  } catch (e) {
    toast.error(t('home.deployFail'), e)
  }
}

/**
 * 在文件管理器里打开某个目录
 *
 * 界面上不显示路径了 —— 里头夹着 Windows 用户名，截图和贴日志时会带出去。
 * 要看就直接开文件管理器，比抄一串路径还方便。
 */
async function openFolder(dir: string) {
  try {
    await openConfigDir(dir)
  } catch (e) {
    toast.error(t('home.openConfigFail'), e)
  }
}

function handleOpenConfig() {
  if (info.value?.configDir) openFolder(info.value.configDir)
}

async function handleDownload() {
  downloading.value = true
  downloadDone.value = false
  progress.value = null

  const unlisten = await listen<DownloadProgress>('rime-download-progress', (event) => {
    progress.value = event.payload
  })

  try {
    await downloadRime()
    downloadDone.value = true
    toast.info(t('home.installerToast'))
  } catch (e) {
    toast.error(t('home.downloadFail'), e)
  } finally {
    downloading.value = false
    unlisten()
  }
}

async function handleRedetect() {
  downloadDone.value = false
  await rimeStore.detect()
  if (info.value?.installed) {
    toast.success(t('home.detectOk', { ime: ime.value }))
  } else {
    toast.info(t('home.detectStill'))
  }
}

async function handleUninstall() {
  const ok = await confirmAction({
    title: t('home.uninstallTitle', { ime: ime.value }),
    description: t('home.uninstallDesc'),
    confirmText: t('home.uninstallCta'),
    danger: true,
  })
  if (!ok) return

  try {
    await uninstallRime()
    toast.info(t('home.uninstallStarted'))
  } catch (e) {
    toast.error(t('home.uninstallFail'), e)
  }
}

async function handleCleanLeftover() {
  if (!info.value?.configDir) return
  const ok = await confirmAction({
    title: t('home.leftoverConfirmTitle'),
    description: t('home.leftoverConfirmDesc'),
    confirmText: t('home.leftoverConfirmCta'),
  })
  if (!ok) return

  cleaning.value = true
  try {
    const dest = await backupLeftoverConfig(info.value.configDir)
    await rimeStore.detect()
    toast.success(t('home.leftoverDone', { dir: dest }))
  } catch (e) {
    toast.error(t('home.leftoverFail'), e)
  } finally {
    cleaning.value = false
  }
}

/** 装好了才去问有没有新版；查不到就安静跳过，别拿网络问题烦用户 */
watch(
  () => [info.value?.installed, info.value?.version],
  async ([installed]) => {
    if (!installed) {
      update.value = null
      return
    }
    try {
      update.value = await checkRimeUpdate(info.value?.version ?? null)
    } catch {
      update.value = null
    }
  },
  { immediate: true },
)

async function openRelease() {
  if (!update.value?.releaseUrl) return
  try {
    await openUrl(update.value.releaseUrl)
  } catch (e) {
    toast.error(t('home.openConfigFail'), e)
  }
}
</script>

<template>
  <div class="max-w-xl space-y-8">
    <div>
      <h1 class="text-xl font-semibold tracking-tight">XGRime</h1>
      <p class="text-[15px] text-muted-foreground mt-1">{{ $t('home.tagline') }}</p>
    </div>

    <div v-if="rimeStore.loading" class="flex items-center gap-2.5 text-muted-foreground text-[15px]">
      <span class="icon-[lucide--loader-2] size-4 animate-spin" />
      <span>{{ $t('home.detecting') }}</span>
    </div>

    <div v-else-if="info" class="space-y-5">
      <!-- ═══ 已安装 ═══ -->
      <template v-if="info.installed">
        <div class="rounded-xl bg-card p-5 space-y-4">
          <div class="flex items-center gap-2.5">
            <div class="size-8 rounded-full flex items-center justify-center bg-green-500/10">
              <span class="icon-[lucide--check] text-green-500 size-4" />
            </div>
            <span class="font-medium text-[15px]">{{ $t('home.installed', { ime }) }}</span>
          </div>

          <div class="space-y-2.5 text-[14px]">
            <div v-if="info.version" class="flex gap-3">
              <span class="text-muted-foreground w-20 shrink-0">{{ $t('home.version') }}</span>
              <span class="text-foreground/90">{{ info.version }}</span>
            </div>
          </div>
        </div>

        <!-- 有新版才出现，没新版不占地方 -->
        <div v-if="update?.updateAvailable" class="rounded-xl bg-sky-500/5 p-5 space-y-3">
          <div class="flex items-center gap-2.5">
            <span class="icon-[lucide--arrow-up-circle] size-4 text-sky-500" />
            <h3 class="font-medium text-[15px]">{{ $t('home.updateTitle', { ime }) }}</h3>
          </div>
          <p class="text-[14px] text-muted-foreground">
            {{ $t('home.updateBody', { installed: update.installed, latest: update.latest }) }}
          </p>
          <button
            class="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-muted/50 text-[15px] text-foreground/80 hover:text-foreground transition-colors"
            @click="openRelease"
          >
            <span class="icon-[lucide--external-link] size-3.5 opacity-60" />
            {{ $t('home.updateCta') }}
          </button>
        </div>

        <div class="flex flex-wrap gap-2.5">
          <button
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-[15px] hover:bg-primary/90 transition-colors disabled:opacity-50"
            :disabled="job.deploying"
            @click="handleDeploy"
          >
            <span
              :class="job.deploying ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--rocket]'"
              class="size-4"
            />
            {{ job.deploying ? $t('common.deploying') : $t('common.redeploy') }}
          </button>
          <button
            v-if="info.installDir"
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-card text-[15px] text-foreground/80 hover:text-foreground hover:bg-card/80 transition-colors"
            @click="openFolder(info.installDir)"
          >
            <span class="icon-[lucide--folder-open] size-4 opacity-60" />
            {{ $t('common.openProgramDir') }}
          </button>
          <button
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-card text-[15px] text-foreground/80 hover:text-foreground hover:bg-card/80 transition-colors"
            @click="handleOpenConfig"
          >
            <span class="icon-[lucide--folder-open] size-4 opacity-60" />
            {{ $t('common.openFolder') }}
          </button>
          <button
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-card text-[15px] text-foreground/80 hover:text-foreground hover:bg-card/80 transition-colors"
            @click="rimeStore.detect()"
          >
            <span class="icon-[lucide--refresh-cw] size-4 opacity-60" />
            {{ $t('common.recheck') }}
          </button>
          <button
            v-if="info.canUninstall"
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-[15px] text-muted-foreground hover:text-red-500 transition-colors ml-auto"
            @click="handleUninstall"
          >
            <span class="icon-[lucide--trash-2] size-4" />
            {{ $t('home.uninstallButton', { ime }) }}
          </button>
        </div>
      </template>

      <!-- ═══ 未安装 ═══ -->
      <template v-else>
        <div v-if="downloadDone" class="rounded-xl bg-card p-5 space-y-4">
          <div class="flex items-center gap-2.5">
            <div class="size-8 rounded-full flex items-center justify-center bg-blue-500/10">
              <span class="icon-[lucide--package-check] text-blue-500 size-4" />
            </div>
            <div>
              <span class="font-medium text-[15px]">{{ $t('home.installerLaunched') }}</span>
              <p class="text-[13px] text-muted-foreground mt-0.5">{{ $t('home.installerHint') }}</p>
              <p class="text-[13px] text-muted-foreground/70 mt-1.5 leading-relaxed">
                {{ $t('home.setupNoteBody') }}
              </p>
            </div>
          </div>
          <button
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-[15px] hover:bg-primary/90 transition-colors"
            @click="handleRedetect"
          >
            <span class="icon-[lucide--refresh-cw] size-4" />
            {{ $t('home.installerDone') }}
          </button>
        </div>

        <div v-else class="rounded-xl bg-card p-5 space-y-4">
          <div class="flex items-center gap-2.5">
            <div class="size-8 rounded-full flex items-center justify-center bg-amber-500/10">
              <span class="icon-[lucide--alert-triangle] text-amber-500 size-4" />
            </div>
            <div>
              <span class="font-medium text-[15px]">{{ $t('home.notInstalled', { ime }) }}</span>
              <p class="text-[13px] text-muted-foreground mt-0.5">{{ $t('home.notInstalledHint') }}</p>
            </div>
          </div>

          <!--
            安装程序最后会弹一个「请勾选所需的输入方案」，那个对话框写的
            其实就是 default.custom.yaml 的 schema_list —— 跟本应用管的是同一份。
            不先说清楚，用户会对着一堆没听过的方案名发懵。
          -->
          <div class="rounded-lg bg-muted/40 px-3.5 py-3 space-y-1.5">
            <div class="flex items-center gap-2">
              <span class="icon-[lucide--info] size-3.5 text-muted-foreground shrink-0" />
              <span class="text-[13px] font-medium text-foreground/80">
                {{ $t('home.setupNoteTitle') }}
              </span>
            </div>
            <p class="text-[13px] text-muted-foreground leading-relaxed">
              {{ $t('home.setupNoteBody') }}
            </p>
          </div>

          <button
            class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-primary text-primary-foreground text-[15px] hover:bg-primary/90 transition-colors disabled:opacity-50"
            :disabled="downloading"
            @click="handleDownload"
          >
            <span
              :class="downloading ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--download]'"
              class="size-4"
            />
            {{ downloading ? $t('home.downloading') : $t('home.downloadCta') }}
          </button>

          <div v-if="progress && downloading" class="space-y-2">
            <div class="w-full bg-muted rounded-full h-1.5 overflow-hidden">
              <div
                class="bg-primary h-1.5 rounded-full transition-all duration-300"
                :style="{ width: `${progress.percentage}%` }"
              />
            </div>
            <div class="flex justify-between text-[12px] text-muted-foreground">
              <span>
                {{ (progress.downloaded / 1024 / 1024).toFixed(1) }} /
                {{ (progress.total / 1024 / 1024).toFixed(1) }} MB
              </span>
              <span>{{ progress.percentage.toFixed(0) }}%</span>
            </div>
          </div>
        </div>

        <!-- 卸载残留：目录还在但程序没了 -->
        <div v-if="info.hasLeftover" class="rounded-xl bg-card p-5 space-y-3">
          <div class="flex items-center gap-2.5">
            <span class="icon-[lucide--archive] size-4 text-muted-foreground" />
            <h3 class="font-medium text-[15px]">{{ $t('home.leftoverTitle') }}</h3>
          </div>
          <p class="text-[14px] text-muted-foreground leading-relaxed">
            {{ $t('home.leftoverBody') }}
          </p>
          <button
            class="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-muted/50 text-[15px] text-foreground/80 hover:text-foreground transition-colors disabled:opacity-50"
            :disabled="cleaning"
            @click="handleCleanLeftover"
          >
            <span
              :class="cleaning ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--archive-restore]'"
              class="size-3.5 opacity-60"
            />
            {{ cleaning ? $t('home.leftoverBusy') : $t('home.leftoverCta') }}
          </button>
        </div>

        <div class="rounded-xl bg-card p-5 space-y-3">
          <h3 class="font-medium text-[15px]">{{ $t('home.notFoundTitle') }}</h3>
          <p class="text-[14px] text-muted-foreground">{{ $t('home.notFoundBody') }}</p>
          <button
            class="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-lg bg-muted/50 text-[15px] text-foreground/80 hover:text-foreground transition-colors"
            @click="rimeStore.detect()"
          >
            <span class="icon-[lucide--refresh-cw] size-3.5 opacity-60" />
            {{ $t('common.recheck') }}
          </button>
        </div>
      </template>
    </div>

    <div v-if="rimeStore.error" class="rounded-xl bg-destructive/5 p-4 text-[15px] text-destructive">
      {{ rimeStore.error }}
    </div>
  </div>
</template>
