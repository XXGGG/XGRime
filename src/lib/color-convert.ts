/**
 * RIME 使用 BGR 格式 0xBBGGRR，CSS 使用 #RRGGBB
 * 这个模块提供两者之间的转换
 */

/** BGR 数字 (0xBBGGRR) → CSS hex (#RRGGBB) */
export function bgrToHex(bgr: number): string {
  const r = bgr & 0xff
  const g = (bgr >> 8) & 0xff
  const b = (bgr >> 16) & 0xff
  return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
}

/** CSS hex (#RRGGBB) → BGR 数字 (0xBBGGRR) */
export function hexToBgr(hex: string): number {
  const clean = hex.replace('#', '')
  const r = parseInt(clean.substring(0, 2), 16)
  const g = parseInt(clean.substring(2, 4), 16)
  const b = parseInt(clean.substring(4, 6), 16)
  return (b << 16) | (g << 8) | r
}

/** BGR 数字 → RIME YAML 格式字符串 "0xBBGGRR" */
export function bgrToYaml(bgr: number): string {
  return `0x${bgr.toString(16).padStart(6, '0').toUpperCase()}`
}

/** RIME YAML 格式字符串 "0xBBGGRR" → CSS hex */
export function yamlColorToHex(yamlColor: string): string {
  const num = parseInt(yamlColor.replace('0x', ''), 16)
  return bgrToHex(num)
}
