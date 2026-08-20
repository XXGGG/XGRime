<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRimeStore } from '@/stores/rimeStore'
import { useInstallStore } from '@/stores/installStore'
import { listAvailableDicts, listInstalledDicts, checkDictUpdates } from '@/types/commands'
import type { DictInfo } from '@/types/rime'
import { useFeedback } from '@/composables/useFeedback'
import SchemaCard from '@/components/schema/SchemaCard.vue'
import ActiveSchemaBar from '@/components/schema/ActiveSchemaBar.vue'

const rimeStore = useRimeStore()
const job = useInstallStore()
const { toast, confirmAction } = useFeedback()
const { t } = useI18n()

const dicts = ref<DictInfo[]>([])
const loading = ref(false)
const busyId = ref<string | null>(null)
const checking = ref(false)

// 推荐当成第一个分组，不再在上面单独摆一块 —— 那样推荐的方案会出现两次
const GROUPS = ['recommended', 'sound', 'shape', 'extra'] as const
type Group = (typeof GROUPS)[number]
const activeGroup = ref<Group>('recommended')

const configDir = computed(() => rimeStore.installInfo?.configDir ?? null)
const anyBusy = computed(() => busyId.value !== null || job.busy)

const recommended = computed(() => dicts.value.filter((d) => d.recommended))
const installedCount = computed(() => dicts.value.filter((d) => d.installed).length)

/** 组内再按小类分段，免得二十几个方案糊成一片 */
const sections = computed(() => {
  // 推荐那一组不分小类：就三四个，再切段反而碎
  if (activeGroup.value === 'recommended') {
    return recommended.value.length ? [{ category: '', items: recommended.value }] : []
  }
  const order: string[] = []
  const bucket = new Map<string, DictInfo[]>()
  for (const d of dicts.value) {
    if (d.group !== activeGroup.value) continue
    if (!bucket.has(d.category)) {
      bucket.set(d.category, [])
      order.push(d.category)
    }
    bucket.get(d.category)!.push(d)
  }
  return order.map((category) => ({ category, items: bucket.get(category)! }))
})

const nameOf = (dict: DictInfo) => t(`dicts.${dict.id}.name`)

/**
 * 状态一律从后端重读，不在前端猜 —— 猜出来的状态和磁盘对不上
 *
 * 带个序号：装方案时切走再切回来会再发一次请求，那次读到的是「还没装好」，
 * 万一它比装完那次晚回来，界面就会被旧数据盖回去，显示成没装。
 */
let loadSeq = 0
async function loadDicts() {
  const seq = ++loadSeq
  loading.value = true
  try {
    const next = configDir.value
      ? await listInstalledDicts(configDir.value)
      : await listAvailableDicts()
    if (seq !== loadSeq) return // 已经有更新的一次在跑了，这次的结果作废
    dicts.value = next
  } catch (e) {
    if (seq === loadSeq) toast.error(t('schemas.loadFail'), e)
  } finally {
    if (seq === loadSeq) loading.value = false
  }
}

/**
 * 触发部署但**不等它**
 *
 * 部署器要编译几十秒。等它跑完再弹「装好了」，用户点完会先干瞪眼半分钟 ——
 * 结果反馈立刻给，部署的进度交给上面那条横幅。
 */
function redeploy() {
  job.redeploy().catch((e) => toast.error(t('schemas.deployWarn'), e))
}

async function handleInstall(dict: DictInfo) {
  if (!configDir.value) return
  try {
    await job.install(dict.id, configDir.value)
    await loadDicts()
    toast.success(t('schemas.installOk', { name: nameOf(dict) }))
    redeploy()
  } catch (e) {
    await loadDicts()
    toast.error(t('schemas.installFail', { name: nameOf(dict) }), e)
  }
}

async function handleToggle(dict: DictInfo) {
  if (!configDir.value) return
  const wasActive = dict.active
  busyId.value = dict.id
  try {
    await job.toggle(dict.id, configDir.value, !wasActive)
    await loadDicts()
    toast.success(
      wasActive
        ? t('schemas.disabled', { name: nameOf(dict) })
        : t('schemas.enabled', { name: nameOf(dict) }),
    )
    redeploy()
  } catch (e) {
    await loadDicts()
    toast.error(t('schemas.toggleFail'), e)
  } finally {
    busyId.value = null
  }
}

async function handleRemove(dict: DictInfo) {
  if (!configDir.value) return

  const ok = await confirmAction({
    title: t('schemas.removeTitle', { name: nameOf(dict) }),
    description: t('schemas.removeDesc'),
    confirmText: t('common.uninstall'),
    danger: true,
  })
  if (!ok) return

  busyId.value = dict.id
  try {
    await job.remove(dict.id, configDir.value)
    await loadDicts()
    toast.success(t('schemas.removeOk', { name: nameOf(dict) }))
    redeploy()
  } catch (e) {
    await loadDicts()
    toast.error(t('schemas.removeFail'), e)
  } finally {
    busyId.value = null
  }
}

async function handleCheckUpdates() {
  if (!configDir.value) return
  checking.value = true
  try {
    job.stale = await checkDictUpdates(configDir.value)
    if (job.stale.length === 0) toast.success(t('schemas.noUpdates'))
  } catch (e) {
    toast.error(t('schemas.checkUpdatesFail'), e)
  } finally {
    checking.value = false
  }
}

onMounted(() => {
  if (configDir.value) loadDicts()
})
watch(configDir, (dir) => {
  if (dir) loadDicts()
})
</script>

<template>
  <div class="max-w-2xl space-y-6">
    <div class="flex items-start justify-between gap-4">
      <div>
        <h1 class="text-xl font-semibold tracking-tight">{{ $t('schemas.title') }}</h1>
        <p class="text-[15px] text-muted-foreground mt-1">{{ $t('schemas.subtitle') }}</p>
      </div>
      <button
        v-if="rimeStore.installInfo?.installed && installedCount > 0"
        class="shrink-0 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-card text-[13px] text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
        :disabled="checking"
        @click="handleCheckUpdates"
      >
        <span
          :class="checking ? 'icon-[lucide--loader-2] animate-spin' : 'icon-[lucide--refresh-cw]'"
          class="size-3.5"
        />
        {{ checking ? $t('schemas.checkingUpdates') : $t('schemas.checkUpdates') }}
      </button>
    </div>

    <ActiveSchemaBar />

    <div
      v-if="!rimeStore.installInfo?.installed"
      class="rounded-xl bg-amber-500/5 px-4 py-3 text-[14px] text-amber-500/80"
    >
      {{ $t('schemas.needRime') }}
    </div>

    <div v-else-if="loading" class="flex items-center gap-2.5 text-muted-foreground text-[15px]">
      <span class="icon-[lucide--loader-2] size-4 animate-spin" />
      <span>{{ $t('schemas.loading') }}</span>
    </div>

    <template v-else>
      <!-- 编译大词库要几十秒。部署器跑完才退出，所以这里能给出确切的结束 -->
      <div
        v-if="job.deploying"
        class="rounded-xl bg-sky-500/5 px-4 py-3 flex items-start gap-3"
      >
        <span class="icon-[lucide--loader-2] size-4 animate-spin text-sky-500 shrink-0 mt-0.5" />
        <div class="min-w-0 flex-1">
          <p class="text-[14px] text-sky-600 dark:text-sky-400 font-medium">
            {{ $t('schemas.deployingTitle') }}
          </p>
          <p class="text-[13px] text-muted-foreground mt-0.5 leading-relaxed">
            {{ $t('schemas.deployingBody') }}
          </p>
        </div>
      </div>

      <div
        v-else-if="job.justDeployed"
        class="rounded-xl bg-emerald-500/5 px-4 py-3 flex items-center gap-3"
      >
        <span class="icon-[lucide--check-circle-2] size-4 text-emerald-500 shrink-0" />
        <p class="text-[14px] text-emerald-600 dark:text-emerald-400 flex-1">
          {{ $t('schemas.deployedTitle') }}
        </p>
        <button
          class="shrink-0 text-muted-foreground/60 hover:text-foreground transition-colors"
          @click="job.dismissDeployHint()"
        >
          <span class="icon-[lucide--x] size-3.5" />
        </button>
      </div>

      <div
        v-if="job.stale.length"
        class="rounded-xl bg-sky-500/5 px-4 py-3 text-[14px] text-sky-600 dark:text-sky-400"
      >
        {{ $t('schemas.updateHint', { count: job.stale.length }) }}
      </div>

      <!-- 推荐 / 按读音打 / 按字形打 / 进阶 -->
      <div class="space-y-3">
        <div class="flex gap-2">
          <button
            v-for="g in GROUPS"
            :key="g"
            class="px-3.5 py-1.5 rounded-lg text-[14px] transition-colors"
            :class="activeGroup === g
              ? 'bg-primary text-primary-foreground'
              : 'bg-muted/40 text-muted-foreground hover:text-foreground hover:bg-muted'"
            @click="activeGroup = g"
          >
            {{ $t(`schemas.tabs.${g}`) }}
          </button>
        </div>
        <p class="text-[12px] text-muted-foreground/60">{{ $t(`schemas.tabHint.${activeGroup}`) }}</p>

        <section v-for="sec in sections" :key="sec.category" class="space-y-2.5 pt-2">
          <h3
            v-if="sec.category"
            class="text-[12px] font-medium text-muted-foreground/70 uppercase tracking-wider"
          >
            {{ $t(`schemas.category.${sec.category}`) }}
          </h3>
          <SchemaCard
            v-for="dict in sec.items"
            :key="dict.id"
            :dict="dict"
            :highlight="activeGroup === 'recommended'"
            :installing="job.installingId === dict.id"
            :busy="anyBusy"
            :progress="job.progress"
            :stale="job.stale.includes(dict.id)"
            @install="handleInstall"
            @toggle="handleToggle"
            @remove="handleRemove"
          />
        </section>
      </div>
    </template>
  </div>
</template>
