<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window'
import {
  detectRime,
  readActiveSchema,
  switchActiveSchema,
  deployRime,
  showMainWindow,
  hideTrayMenu,
  anchorTrayMenu,
  quitApp,
} from '@/types/commands'
import type { SchemaBrief } from '@/types/rime'

/**
 * 托盘菜单
 *
 * 单独一个无边框小窗口，不是系统原生菜单 —— 原生那套的字号、圆角、配色归
 * 系统管，跟应用里对不上。这里就是普通的 Vue 组件，样式跟主界面同一套 token。
 *
 * 里面只放「不打开主界面也想干」的三件事：换方案、重新部署、打开/退出。
 */

const configDir = ref('')
const current = ref('')
const schemas = ref<SchemaBrief[]>([])
const busy = ref('')

const card = ref<HTMLElement | null>(null)

/**
 * 窗口贴着内容高
 *
 * 配置里那个高度只是个初值。装了几个方案决定菜单多高，写死的话要么下面空
 * 一大截、要么装多了被切掉。量完再把窗口调到刚好。
 */
async function fit() {
  await nextTick()
  // 12 是四周留给阴影的透明边（下面那层 p-3），上下各一份。
  // 这个数在 src-tauri/src/tray.rs 的 MENU_PADDING 也有一份，定位要用，改就一起改。
  const h = Math.ceil((card.value?.getBoundingClientRect().height ?? 0) + 24)
  if (h <= 24) return
  try {
    await getCurrentWindow().setSize(new LogicalSize(220, h))
    // 窗口是左上角定位的：高度一改下边缘就跑了，得按新高度重新贴一次
    await anchorTrayMenu()
  } catch {
    /* 改不了尺寸就维持原样，总比整个菜单不出来强 */
  }
}

async function load() {
  try {
    const info = await detectRime()
    if (info.installed) {
      configDir.value = info.configDir
      const active = await readActiveSchema(info.configDir)
      current.value = active.current
      schemas.value = active.available
    }
  } catch {
    /* 托盘菜单读不到就少显示两项，别弹错误框 —— 这个窗口没地方放 */
  }
  fit()
}

onMounted(() => {
  load()
  // 这扇窗在应用启动时就建好了（藏着），里面的方案列表到用户点开时早就旧了。
  // 每次被显示出来（拿到焦点）重读一次。
  getCurrentWindow().onFocusChanged(({ payload }) => {
    if (payload) load()
  })
})

async function pick(id: string) {
  if (!configDir.value || id === current.value || busy.value) return
  busy.value = id
  try {
    const info = await switchActiveSchema(configDir.value, id)
    current.value = info.current
  } finally {
    busy.value = ''
    hideTrayMenu()
  }
}

async function redeploy() {
  busy.value = 'deploy'
  // 部署要几十秒，不等它 —— 菜单该收就收
  deployRime().catch(() => {})
  busy.value = ''
  hideTrayMenu()
}
</script>

<template>
  <!-- 窗口是透明的，圆角和阴影由这一层画 -->
  <div class="w-screen p-3 select-none">
    <div ref="card" class="rounded-xl bg-popover text-popover-foreground shadow-2xl inset-ring-1 inset-ring-border/60 p-1.5 flex flex-col gap-0.5 overflow-hidden">
      <template v-if="schemas.length > 1">
        <p class="px-2.5 pt-1.5 pb-1 text-[11px] text-muted-foreground/60">
          {{ $t('tray.schema') }}
        </p>
        <button
          v-for="s in schemas"
          :key="s.schemaId"
          class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] text-left transition-colors hover:bg-muted/60 disabled:opacity-50"
          :disabled="busy !== ''"
          @click="pick(s.schemaId)"
        >
          <span
            v-if="busy === s.schemaId"
            class="icon-[lucide--loader-2] animate-spin size-3.5 shrink-0"
          />
          <span
            v-else
            class="icon-[lucide--check] size-3.5 shrink-0"
            :class="s.schemaId === current ? 'opacity-100' : 'opacity-0'"
          />
          <span class="truncate">{{ s.name }}</span>
        </button>
        <div class="h-px bg-border/60 my-1 mx-1.5" />
      </template>

      <button
        class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] text-left transition-colors hover:bg-muted/60"
        @click="showMainWindow()"
      >
        <span class="icon-[lucide--panel-top] size-3.5 shrink-0 opacity-70" />
        {{ $t('tray.open') }}
      </button>
      <button
        class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] text-left transition-colors hover:bg-muted/60"
        @click="redeploy"
      >
        <span class="icon-[lucide--rocket] size-3.5 shrink-0 opacity-70" />
        {{ $t('tray.redeploy') }}
      </button>

      <div class="h-px bg-border/60 my-1 mx-1.5" />

      <button
        class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[13px] text-left transition-colors hover:bg-destructive/10 hover:text-destructive"
        @click="quitApp()"
      >
        <span class="icon-[lucide--log-out] size-3.5 shrink-0 opacity-70" />
        {{ $t('tray.quit') }}
      </button>
    </div>
  </div>
</template>
