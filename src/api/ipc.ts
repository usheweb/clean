// 所有与 Rust 通信的唯一入口。
// 约定：Rust command 参数是 snake_case，前端传 camelCase，Tauri 会自动转换。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppInfo,
  BigFile,
  CleanItemMeta,
  CleanTarget,
  DataDone,
  DataRow,
  InstalledApp,
  LedgerEntry,
  MigrateItem,
  MigrateRow,
  Settings,
  TaskDone,
  TaskLog,
  TaskProgress,
} from "./types";

/** 连通性自检 */
export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("get_app_info");
}

// ---------- 清理：白名单元信息 + 扫描 + 执行 ----------

/** 清理白名单的静态元信息（未扫描，用于列表骨架） */
export async function listCleanItems(): Promise<CleanItemMeta[]> {
  return invoke<CleanItemMeta[]>("list_clean_items");
}

/** 启动清理扫描（后台任务，结果通过 data://row 增量推送）。返回 taskId。 */
export async function startCleanScan(): Promise<string> {
  return invoke<string>("start_clean_scan");
}

/** 执行清理。targets 为勾选的扫描结果；permanent=true 直接删除，否则进回收站/废纸篓。 */
export async function startCleanRun(
  targets: CleanTarget[],
  permanent: boolean,
): Promise<string> {
  return invoke<string>("start_clean_run", { targets, permanent });
}

// ---------- 大文件 ----------

export async function startBigFileScan(
  includeAppData: boolean,
  minBytes: number,
): Promise<string> {
  return invoke<string>("start_bigfile_scan", { includeAppData, minBytes });
}

// ---------- 已装软件 ----------

export async function startAppsScan(): Promise<string> {
  return invoke<string>("start_apps_scan");
}

// ---------- 高级迁移（Windows） ----------

export async function listMigrateItems(): Promise<MigrateItem[]> {
  return invoke<MigrateItem[]>("list_migrate_items");
}

export async function listDrives(): Promise<string[]> {
  return invoke<string[]>("list_drives");
}

export async function startMigrateScan(
  drive: string,
  rootName: string,
  includeCandidates: boolean,
): Promise<string> {
  return invoke<string>("start_migrate_scan", { drive, rootName, includeCandidates });
}

/** 执行迁移。传整行（候选 key 是有损变换，后端无法从 key 反推路径）。 */
export async function startMigrateRun(
  rows: MigrateRow[],
  drive: string,
  rootName: string,
): Promise<string> {
  return invoke<string>("start_migrate_run", { rows, drive, rootName });
}

export async function listLedger(): Promise<LedgerEntry[]> {
  return invoke<LedgerEntry[]>("list_ledger");
}

export async function startRestore(srcs: string[]): Promise<string> {
  return invoke<string>("start_restore", { srcs });
}

export async function startReorganize(drive: string, newRoot: string): Promise<string> {
  return invoke<string>("start_reorganize", { drive, newRoot });
}

export async function startRebuildLedger(): Promise<string> {
  return invoke<string>("start_rebuild_ledger");
}

// ---------- 任务控制 ----------

export async function cancelTask(): Promise<void> {
  return invoke<void>("cancel_task");
}

export async function pauseTask(): Promise<void> {
  return invoke<void>("pause_task");
}

export async function resumeTask(): Promise<void> {
  return invoke<void>("resume_task");
}

// ---------- 系统交互 ----------

/** 用系统默认程序打开 */
export async function openPath(path: string): Promise<void> {
  return invoke<void>("open_path", { path });
}

/** 在文件管理器中定位（Windows 选中 / macOS 在 Finder 中显示） */
export async function revealPath(path: string): Promise<void> {
  return invoke<void>("reveal_path", { path });
}

// ---------- 设置 ----------

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

// ---------- 事件订阅 ----------

export function onTaskProgress(cb: (p: TaskProgress) => void): Promise<UnlistenFn> {
  return listen<TaskProgress>("task://progress", (e) => cb(e.payload));
}

export function onTaskLog(cb: (l: TaskLog) => void): Promise<UnlistenFn> {
  return listen<TaskLog>("task://log", (e) => cb(e.payload));
}

export function onTaskDone(cb: (d: TaskDone) => void): Promise<UnlistenFn> {
  return listen<TaskDone>("task://done", (e) => cb(e.payload));
}

export function onDataRow<T>(kind: string, cb: (r: DataRow<T>) => void): Promise<UnlistenFn> {
  return listen<DataRow<T>>("data://row", (e) => {
    if (e.payload.kind === kind) cb(e.payload);
  });
}

export function onDataDone(kind: string, cb: (d: DataDone) => void): Promise<UnlistenFn> {
  return listen<DataDone>("data://done", (e) => {
    if (e.payload.kind === kind) cb(e.payload);
  });
}

export type {
  BigFile,
  InstalledApp,
  MigrateRow,
  MigrateItem,
  LedgerEntry,
  CleanTarget,
  CleanItemMeta,
};
