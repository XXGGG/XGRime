import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { RimeInstallInfo } from '@/types/rime'
import { detectRime } from '@/types/commands'

export const useRimeStore = defineStore('rime', () => {
  const installInfo = ref<RimeInstallInfo | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function detect() {
    loading.value = true
    error.value = null
    try {
      installInfo.value = await detectRime()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { installInfo, loading, error, detect }
})
