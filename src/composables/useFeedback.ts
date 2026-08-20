import { ref } from 'vue'
import { i18n } from '@/i18n'

export type ToastKind = 'success' | 'error' | 'info'

export interface Toast {
  id: number
  kind: ToastKind
  message: string
  /** 展开后的细节，通常是后端返回的原始报错 */
  detail?: string
}

export interface ConfirmRequest {
  title: string
  description?: string
  confirmText?: string
  cancelText?: string
  /** 破坏性操作用红色确认按钮 */
  danger?: boolean
}

const toasts = ref<Toast[]>([])
const pending = ref<(ConfirmRequest & { id: number }) | null>(null)

let seq = 0
let resolvePending: ((ok: boolean) => void) | null = null

function push(kind: ToastKind, message: string, detail?: string) {
  const id = ++seq
  toasts.value.push({ id, kind, message, detail })
  // 报错留久一点，用户要有时间看清楚
  const ttl = kind === 'error' ? 8000 : 3500
  setTimeout(() => dismiss(id), ttl)
  return id
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

/** 后端抛上来的结构化错误：只有码和技术细节，没有人话 */
interface BackendError {
  code: string
  detail?: string
}

function isBackendError(e: unknown): e is BackendError {
  return !!e && typeof e === 'object' && typeof (e as BackendError).code === 'string'
}

/**
 * 把后端抛上来的东西整理成给人看的两行。
 *
 * Rust 那边给的是 `{ code, detail }` —— 码在这里按界面语言翻成人话，
 * detail 是系统报的原文（路径、HTTP 状态码这类），本来也翻不了，原样带上。
 */
export function describeError(e: unknown): string {
  if (isBackendError(e)) {
    const { t, te } = i18n.global
    const key = `errors.${e.code}`
    // 万一漏了某个码的翻译，宁可露出码也别显示空白
    const head = te(key) ? t(key) : e.code
    return e.detail ? `${head}（${e.detail}）` : head
  }
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message)
  return String(e)
}

export function useFeedback() {
  const toast = {
    success: (message: string) => push('success', message),
    info: (message: string) => push('info', message),
    error: (message: string, detail?: unknown) =>
      push('error', message, detail === undefined ? undefined : describeError(detail)),
  }

  /** 弹出应用自己的确认框，返回用户点了确认没有 */
  function confirmAction(request: ConfirmRequest): Promise<boolean> {
    // 同一时间只允许一个确认框，后来的直接顶掉前一个（前一个当作取消）
    resolvePending?.(false)
    pending.value = { ...request, id: ++seq }
    return new Promise<boolean>((resolve) => {
      resolvePending = resolve
    })
  }

  function settleConfirm(ok: boolean) {
    pending.value = null
    resolvePending?.(ok)
    resolvePending = null
  }

  return { toasts, dismiss, toast, confirmAction, pending, settleConfirm }
}
