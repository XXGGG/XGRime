import { invoke } from '@tauri-apps/api/core'
import type {
  RimeInstallInfo,
  StyleConfig,
  DictInfo,
  InputSettings,
  SchemaOptions,
  RimeUpdate,
  DeployOutcome,
  UserPreset,
  SchemaIcon,
  ActiveSchema,
  IconPref,
  BackupParts,
  BackupManifest,
  ExportSummary,
  ImportSummary,
  IconSet,
} from './rime'

/** 检测 RIME 安装 */
export function detectRime() {
  return invoke<RimeInstallInfo>('detect_rime')
}

/** 启动官方卸载程序 */
export function uninstallRime() {
  return invoke<void>('uninstall_rime')
}

/** 把卸载后残留的旧配置目录改名备份，返回备份路径 */
export function backupLeftoverConfig(configDir: string) {
  return invoke<string>('backup_leftover_config', { configDir })
}

/** 读取当前主题配置 */
export function readThemeConfig(configDir: string) {
  return invoke<StyleConfig | null>('read_theme_config', { configDir })
}

/** 保存主题配置 */
export function saveThemeConfig(configDir: string, config: StyleConfig) {
  return invoke<void>('save_theme_config', { configDir, config })
}

/** 触发 RIME 重新部署；在 Windows 上会一直等到词库编译完 */
export function deployRime() {
  return invoke<DeployOutcome>('deploy_rime')
}

/** 获取可用方案列表 */
export function listAvailableDicts() {
  return invoke<DictInfo[]>('list_available_dicts')
}

/** 获取方案列表（带已安装 / 已启用状态） */
export function listInstalledDicts(configDir: string) {
  return invoke<DictInfo[]>('list_installed_dicts', { configDir })
}

/** 安装方案（含其依赖的全部仓库），装完自动挂进方案列表 */
export function installDict(dictId: string, configDir: string) {
  return invoke<void>('install_dict', { dictId, configDir })
}

/** 启用 / 停用方案（不删文件，只改 schema_list） */
export function toggleDict(dictId: string, configDir: string, enable: boolean) {
  return invoke<void>('toggle_dict', { dictId, configDir, enable })
}

/** 卸载方案（按安装清单删文件，共用文件会保留） */
export function removeDict(dictId: string, configDir: string) {
  return invoke<void>('remove_dict', { dictId, configDir })
}

/** 下载 RIME 安装包 */
export function downloadRime() {
  return invoke<string>('download_rime')
}

/** 打开配置目录 */
export function openConfigDir(configDir: string) {
  return invoke<void>('open_config_dir', { configDir })
}

/** 获取系统字体列表 */
export function getSystemFonts() {
  return invoke<string[]>('get_system_fonts')
}

/** 读取输入设置 */
export function readInputSettings(configDir: string) {
  return invoke<InputSettings>('read_input_settings', { configDir })
}

/** 保存输入设置 */
export function saveInputSettings(configDir: string, settings: InputSettings) {
  return invoke<void>('save_input_settings', { configDir, settings })
}

/** 读某个方案能调的开关；不传 schema 就用方案选单里的第一个 */
export function readSchemaOptions(configDir: string, schema?: string) {
  return invoke<SchemaOptions>('read_schema_options', { configDir, schema: schema ?? null })
}

/** 设置某个方案开关的默认档位 */
export function saveSchemaSwitch(
  configDir: string,
  schema: string,
  index: number,
  value: number,
) {
  return invoke<void>('save_schema_switch', { configDir, schema, index, value })
}

/** 存某个方案的模糊音 */
export function saveFuzzy(configDir: string, schema: string, pairs: string[]) {
  return invoke<void>('save_fuzzy', { configDir, schema, pairs })
}

/** 查小狼毫 / 鼠须管有没有新版本 */
export function checkRimeUpdate(installed: string | null) {
  return invoke<RimeUpdate>('check_rime_update', { installed })
}

/** 查哪些已装方案的词库有更新，返回方案 id */
export function checkDictUpdates(configDir: string) {
  return invoke<string[]>('check_dict_updates', { configDir })
}

/** 读用户自己存的外观预设 */
export function listUserPresets() {
  return invoke<UserPreset[]>('list_user_presets')
}

/** 存一套外观；同 id 视为覆盖 */
export function saveUserPreset(preset: UserPreset) {
  return invoke<UserPreset[]>('save_user_preset', { preset })
}

export function deleteUserPreset(id: string) {
  return invoke<UserPreset[]>('delete_user_preset', { id })
}

/** 读某个方案配了哪些状态图标 */
export function readSchemaIcons(configDir: string, schema: string) {
  return invoke<SchemaIcon[]>('read_schema_icons', { configDir, schema })
}

/** 把选中的图片设成某个状态的图标 */
export function setSchemaIcon(configDir: string, schema: string, kind: string, source: string) {
  return invoke<SchemaIcon[]>('set_schema_icon', { configDir, schema, kind, source })
}

export function clearSchemaIcon(configDir: string, schema: string, kind: string) {
  return invoke<SchemaIcon[]>('clear_schema_icon', { configDir, schema, kind })
}

/** 四个状态一起清掉 */
export function clearAllSchemaIcons(configDir: string, schema: string) {
  return invoke<SchemaIcon[]>('clear_all_schema_icons', { configDir, schema })
}

/** 跟应用一起打包的那几套图标 */
export function listBuiltinIconSets() {
  return invoke<IconSet[]>('list_builtin_icon_sets')
}

/** 一次把整套内置图标装上 */
export function applyBuiltinIconSet(configDir: string, schema: string, set: string) {
  return invoke<SchemaIcon[]>('apply_builtin_icon_set', { configDir, schema, set })
}

/** 现在用的是哪个输入方案，以及选单里有哪些 */
export function readActiveSchema(configDir: string) {
  return invoke<ActiveSchema>('read_active_schema', { configDir })
}

/**
 * 换一个输入方案
 *
 * 停服 → 改文件 → 启动，三步在 Rust 那边一次做完。别拆开自己拼顺序：
 * 先改文件再停服的话，服务退出时会把旧方案写回去，等于没切。
 */
export function switchActiveSchema(configDir: string, schema: string) {
  return invoke<ActiveSchema>('switch_schema_and_restart', { configDir, schema })
}

/** 把设置打包成 zip */
export function exportSettings(configDir: string, target: string, parts: BackupParts) {
  return invoke<ExportSummary>('export_settings', { configDir, target, parts })
}

/** 先看看这个备份包里有什么 */
export function inspectBackup(path: string) {
  return invoke<BackupManifest>('inspect_backup', { path })
}

/** 从备份包装回来。会覆盖现有设置，覆盖前自动留一份 */
export function importSettings(configDir: string, path: string, parts: BackupParts) {
  return invoke<ImportSummary>('import_settings', { configDir, path, parts })
}

/** 状态图标选的是哪一套 */
export function readIconPref() {
  return invoke<IconPref>('read_icon_pref')
}

/** 选了「自动」而任务栏深浅变了就换过来，返回 true 表示真换了 */
export function syncStatusIcons(configDir: string, schema: string) {
  return invoke<boolean>('sync_status_icons', { configDir, schema })
}

/** 开机自启 */
export function getAutostart() {
  return invoke<boolean>('get_autostart')
}

export function setAutostart(enabled: boolean) {
  return invoke<boolean>('set_autostart', { enabled })
}

/** 打开系统设置里的某一页（只有 Windows 有） */
export function openSystemSetting(which: string) {
  return invoke<void>('open_system_setting', { which })
}

/** 从托盘菜单里点「打开」 */
export function showMainWindow() {
  return invoke<void>('show_main_window')
}

export function hideTrayMenu() {
  return invoke<void>('hide_tray_menu')
}

/** 按内容改完高度之后叫一次，让菜单按新高度重新贴到鼠标旁边 */
export function anchorTrayMenu() {
  return invoke<void>('anchor_tray_menu')
}

/** 真退出。关窗口只是收进托盘，退出只能走这里 */
export function quitApp() {
  return invoke<void>('quit_app')
}
