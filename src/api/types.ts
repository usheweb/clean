// 前后端共享的数据类型定义

export type Platform = "windows" | "macos" | "linux";

/** 清理项风险等级：green=一定安全（默认勾选），yellow=建议先关软件 */
export type ItemLevel = "green" | "yellow";

/** 任务种类 */
export type TaskKind =
  | "clean_scan"
  | "clean_run"
  | "bigfile_scan"
  | "migrate_scan"
  | "migrate_run"
  | "migrate_restore"
  | "migrate_reorganize"
  | "migrate_rebuild"
  | "apps_scan";

export interface AppInfo {
  productName: string;
  version: string;
  platform: Platform;
  isAdmin: boolean;
  /** macOS 用 ~，Windows 用 %VAR%，仅用于界面文案 */
  homeDir: string;
}

/** 清理白名单条目的静态元信息（不含体积，未扫描） */
export interface CleanItemMeta {
  key: string;
  name: string;
  level: ItemLevel;
  desc: string;
  admin: boolean;
  category: string;
  /** 该条目是否需要目标盘支持（如 Windows 专属），为 false 时 macOS 也提供 */
  pathHint: string[];
}

/** 扫描后得到的一个具体可清理目标 */
export interface CleanTarget {
  id: string;
  itemKey: string;
  itemName: string;
  level: ItemLevel;
  desc: string;
  admin: boolean;
  category: string;
  path: string;
  bytes: number;
  files: number;
  /** dir = 清空目录内容（保留目录）；file = 删除该文件 */
  kind: "dir" | "file";
  /** 该条目是否可直接删除（特殊逻辑条目标记为 false，只能回收站方式处理） */
  deletable: boolean;
}

/** 单条目标的清理结果：清干净的条目要当场从列表里摘掉 */
export interface CleanDoneRow {
  id: string;
  bytes: number;
  /** 整条都清理干净了才为 true；有跳过/失败的条目留在列表上，提示还有残留 */
  removed: boolean;
}

export interface BigFile {
  path: string;
  name: string;
  bytes: number;
  mtime: number;
  kind: string;
}

export interface InstalledApp {
  name: string;
  version: string;
  location: string;
  bytes: number;
}

export interface MigrateItem {
  key: string;
  name: string;
  src: string;
  category: string;
  procs: string[];
  /** 迁移后落在目标盘的子目录名 */
  subdir: string;
  desc: string;
}

export interface MigrateRow {
  key: string;
  name: string;
  src: string;
  category: string;
  desc: string;
  bytes: number;
  files: number;
  /** 有进程在运行时给出进程名 */
  running: string[];
  /** 白名单之外发现的未验证候选 */
  candidate: boolean;
  /** 已被迁移过（src 已是 junction） */
  migrated: boolean;
}

/** 单条迁移结果：迁移成功的条目要当场标成「已迁移」并取消勾选 */
export interface MigrateDoneRow {
  key: string;
}

export interface LedgerEntry {
  key: string;
  name: string;
  src: string;
  dst: string;
  bytes: number;
  time: string;
  candidate: boolean;
}

export interface Settings {
  theme: "light" | "dark";
  targetRootName: string;
  targetDrive: string;
  includeCandidates: boolean;
  lastPromoDay: string;
}

/** 任务进度（节流后推送） */
export interface TaskProgress {
  taskId: string;
  kind: TaskKind;
  phase: string;
  title: string;
  current: string | null;
  itemsDone: number;
  itemsTotal: number;
  filesDone: number;
  filesTotal: number;
  bytesDone: number;
  bytesTotal: number;
  ratio: number | null;
  indeterminate: boolean;
  paused: boolean;
  canPause: boolean;
  elapsedMs: number;
  speedBps: number;
}

export interface TaskDone {
  taskId: string;
  kind: TaskKind;
  ok: number;
  skipped: number;
  failed: number;
  bytes: number;
  stopped: boolean;
  message: string;
  reportPath?: string | null;
}

export interface TaskLog {
  taskId: string;
  line: string;
  level: "info" | "warn" | "error" | "ok";
}

export interface DataRow<T = unknown> {
  kind: string;
  row: T;
}

export interface DataDone {
  kind: string;
  total: number;
}
