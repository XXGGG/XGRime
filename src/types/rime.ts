/** RIME 安装信息 */
export interface RimeInstallInfo {
  installed: boolean
  version: string | null
  installDir: string | null
  configDir: string
  platform: 'windows' | 'macos' | 'linux'
  /** 没装程序，但配置目录里有上一次卸载留下的东西 */
  hasLeftover: boolean
  /** 找得到卸载入口 */
  canUninstall: boolean
}

/** RIME 主题配置（颜色使用 CSS #RRGGBB 格式，保存时转为 BGR） */
export interface ThemeConfig {
  name: string
  // 背景与边框
  backColor: string
  borderColor: string
  // 预编辑区
  textColor: string
  hilitedTextColor: string
  hilitedBackColor: string
  // 候选词
  candidateTextColor: string
  commentTextColor: string
  labelColor: string
  // 高亮候选词
  hilitedCandidateTextColor: string
  hilitedCandidateBackColor: string
  hilitedCandidateLabelColor: string
  hilitedCommentTextColor: string
  /** 选中项前那个标记的颜色；空字符串 = 不显示 */
  hilitedMarkColor: string
}

/** RIME 布局配置 */
export interface LayoutConfig {
  // 常规
  horizontal: boolean
  inlinePreedit: boolean
  fontFace: string
  fontSize: number
  cornerRadius: number
  borderWidth: number
  // 高级
  marginX: number
  marginY: number
  hilitePaddingX: number
  hilitePaddingY: number
  candidateSpacing: number
  hiliteSpacing: number
  spacing: number
  roundCorner: number
  shadowRadius: number
  labelFontFace: string
  labelFontSize: number
  /** 候选框最小宽度；小狼毫默认 160，框显得比系统输入法宽就是它撑的。0 = 不设下限 */
  minWidth: number
  /** 0 = 不限 */
  maxWidth: number
  /** 序号怎么显示，小狼毫默认 "%s."（出来是 1. 2. 3.） */
  labelFormat: string
  /** 标记字符；空 = 画成竖杠（Win11 风格） */
  markText: string
}

/** 切换中英文 / 切换方案时弹的提示 */
export interface NotifyConfig {
  /** always = 都弹，never = 都不弹 */
  mode: 'always' | 'never'
  /** 弹多久（毫秒） */
  durationMs: number
}

/** 完整的样式配置 */
export interface StyleConfig {
  theme: ThemeConfig
  layout: LayoutConfig
  notify: NotifyConfig
  /** 一页几个候选；预览照这个数画才跟真的一致 */
  pageSize: number
}

/** 一个方案要下载的仓库来源 */
export interface DictSource {
  repo: string
  branch: string
}

/** 输入方案信息（名字和说明由界面按语言翻译，后端只给键） */
export interface DictInfo {
  id: string
  schemaId: string
  /** sound = 按读音打，shape = 按字形打，extra = 进阶 */
  group: 'sound' | 'shape' | 'extra'
  /** 小类键，对应 i18n 的 schemas.category.* */
  category: string
  recommended: boolean
  homepage: string
  /** 装这个方案要拉的全部仓库，含依赖 */
  sources: DictSource[]
  /** 全部来源加起来的下载体积 */
  totalBytes: number
  installed: boolean
  /** 卸得掉吗；输入法自带的方案在程序目录里，只给停用不给卸载 */
  removable: boolean
  active: boolean
}

/** 模糊音对 */
export interface FuzzyPair {
  id: string
  label: string
  description: string
  enabled: boolean
}

/** 输入设置 */
export interface InputSettings {
  /** 候选词个数 */
  pageSize: number
  /** 左Shift行为: commit_text / inline_ascii / noop */
  shiftLBehavior: string
  /** 右Shift行为 */
  shiftRBehavior: string
  /** 翻页键: minus_equal / bracket / tab / comma_period */
  pageKeys: string
}

/**
 * 方案自带的一个开关。简繁、中英标点、Emoji 这些不是我们造的功能，
 * 是方案 schema.yaml 里的 switches，读出来有几个显示几个。
 */
export interface SchemaSwitch {
  index: number
  /** 对应 i18n 的 switches.*；认不出来的是 'other' */
  labelKey: string
  /** labelKey 为 other 时用它兜底显示 */
  rawName: string | null
  states: string[]
  current: number
  configured: boolean
}

/** 方案选单里的一项 */
export interface SchemaBrief {
  schemaId: string
  name: string
}

/** 某个方案能调的东西 */
export interface SchemaOptions {
  schemaId: string
  schemaName: string
  /** 找不到方案文件，通常是还没装任何输入方案 */
  missing: boolean
  supportsFuzzy: boolean
  switches: SchemaSwitch[]
  /** 这个方案自己的模糊音 */
  fuzzyPairs: string[]
  /** 全部启用中的方案，装了两个以上时给选择器用 */
  available: SchemaBrief[]
}

/** 下载进度事件 */
export interface DownloadProgress {
  downloaded: number
  total: number
  percentage: number
  /** 装方案时正在下第几个仓库（安装 RIME 本体时不带这几个字段） */
  step?: number
  stepTotal?: number
  stepName?: string
}

/** 一次重新部署的结果 */
export interface DeployOutcome {
  /** 确实等到部署跑完了；false 表示只发了指令，不知道什么时候结束 */
  confirmed: boolean
}

/** 小狼毫 / 鼠须管本身的版本情况 */
export interface RimeUpdate {
  installed: string | null
  latest: string | null
  updateAvailable: boolean
  releaseUrl: string
}

/** 用户自己存下来的一套外观 */
export interface UserPreset {
  id: string
  name: string
  theme: ThemeConfig
  layout: LayoutConfig
}

/** 输入法状态图标（中文 / 英文 / 全角 / 半角） */
/** 备份里分哪几类 */
export interface BackupParts {
  theme: boolean
  schemas: boolean
  phrases: boolean
  userdb: boolean
}

export interface BackupManifest {
  format: number
  app: string
  parts: BackupParts
  files: string[]
}

export interface ExportSummary {
  path: string
  files: number
  bytes: number
}

export interface ImportSummary {
  files: number
  /** 覆盖前把原文件挪去了这里，空表示没有东西被覆盖 */
  backupDir: string
}

/** 现在用的是哪个输入方案 */
export interface ActiveSchema {
  current: string
  available: SchemaBrief[]
}

/** 状态图标选的是哪一套 */
export interface IconPref {
  /** `auto` = 跟着任务栏深浅换；其余就是某一套的 id；空 = 没设过 */
  mode: string
  /** 上次实际装进去的是哪一套 */
  applied: string
}

/** 跟应用一起打包的一套状态图标，见 scripts/gen-status-icons.py */
export interface IconSet {
  id: string
  /** 四个状态是不是都齐了 */
  complete: boolean
}

export interface SchemaIcon {
  kind: 'zhung' | 'ascii' | 'full' | 'half'
  /** 配置里写的相对路径，空 = 没设 */
  path: string
  exists: boolean
}
