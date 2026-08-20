import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { DownloadProgress } from '@/types/rime'
import { installDict, removeDict, toggleDict, deployRime } from '@/types/commands'

/**
 * 安装任务的状态放在 store 里，不放页面里
 *
 * 放页面里的话，用户点了安装再切去别的页，组件一卸载 ref 就没了，
 * 切回来进度条凭空消失 —— 后台其实还在下，界面却装作什么都没发生。
 */
export const useInstallStore = defineStore('install', () => {
  const installingId = ref<string | null>(null)
  const progress = ref<DownloadProgress | null>(null)
  /** 已知有更新的方案 id */
  const stale = ref<string[]>([])
  const busy = computed(() => installingId.value !== null)

  let unlisten: UnlistenFn | null = null

  /** 进度事件只订一次，全局有效，跟哪个页面开着无关 */
  async function ensureListening() {
    if (unlisten) return
    unlisten = await listen<DownloadProgress>('dict-download-progress', (event) => {
      progress.value = event.payload
    })
  }

  /**
   * 部署的进行中 / 刚完成
   *
   * Windows 上 `WeaselDeployer.exe /deploy` 编译完词库才退出，所以我们能等到
   * 真结果；macOS 的 `--reload` 发完通知就返回，等不到，那种情况只提示不报完成。
   */
  const deploying = ref(false)
  /** 刚部署完，绿条显示几秒就收 */
  const justDeployed = ref(false)
  let doneTimer: ReturnType<typeof setTimeout> | null = null

  function dismissDeployHint() {
    deploying.value = false
    justDeployed.value = false
    if (doneTimer) clearTimeout(doneTimer)
  }

  async function redeploy() {
    if (doneTimer) clearTimeout(doneTimer)
    justDeployed.value = false
    deploying.value = true
    try {
      const outcome = await deployRime()
      if (outcome.confirmed) {
        justDeployed.value = true
        doneTimer = setTimeout(() => (justDeployed.value = false), 6000)
      }
      return outcome
    } finally {
      deploying.value = false
    }
  }

  async function install(dictId: string, configDir: string) {
    await ensureListening()
    installingId.value = dictId
    progress.value = null
    try {
      await installDict(dictId, configDir)
      stale.value = stale.value.filter((id) => id !== dictId)
    } finally {
      installingId.value = null
      progress.value = null
    }
  }

  async function toggle(dictId: string, configDir: string, enable: boolean) {
    await toggleDict(dictId, configDir, enable)
  }

  async function remove(dictId: string, configDir: string) {
    await removeDict(dictId, configDir)
    stale.value = stale.value.filter((id) => id !== dictId)
  }

  return {
    installingId,
    progress,
    stale,
    busy,
    deploying,
    justDeployed,
    install,
    toggle,
    remove,
    redeploy,
    dismissDeployHint,
  }
})
