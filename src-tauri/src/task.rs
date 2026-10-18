// 统一任务模型：节流推送 task://progress / task://log / task://done
// 核心目的：让前端永远有反馈（spinner + 进度），且任何时刻都能暂停/停止。
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::AppHandle;
use tauri::Emitter as _;

use crate::state::TaskCtl;

/// 进度事件最小间隔：再快也没必要刷（UI 100ms 一次足够平滑）
const THROTTLE: Duration = Duration::from_millis(100);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub task_id: String,
    pub kind: String,
    pub phase: String,
    pub title: String,
    pub current: Option<String>,
    pub items_done: u64,
    pub items_total: u64,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub ratio: Option<f64>,
    pub indeterminate: bool,
    pub paused: bool,
    pub can_pause: bool,
    pub elapsed_ms: u64,
    pub speed_bps: f64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskLog {
    pub task_id: String,
    pub line: String,
    pub level: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaskDone {
    pub task_id: String,
    pub kind: String,
    pub ok: u64,
    pub skipped: u64,
    pub failed: u64,
    pub bytes: u64,
    pub stopped: bool,
    pub message: String,
    pub report_path: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataRow<T: Serialize + Clone> {
    pub kind: String,
    pub row: T,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataDone {
    pub kind: String,
    pub total: u64,
}

/// 任务进度发射器。持有全部计数，按节流对外发射事件。
pub struct Emitter {
    app: AppHandle,
    pub ctl: TaskCtl,
    kind: String,
    phase: String,
    title: String,
    current: Option<String>,
    items_done: u64,
    items_total: u64,
    files_done: u64,
    files_total: u64,
    bytes_done: u64,
    bytes_total: u64,
    can_pause: bool,
    indeterminate: bool,
    start: Instant,
    last: Instant,
}

impl Emitter {
    pub fn new(app: AppHandle, ctl: TaskCtl, title: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            app,
            kind: ctl.kind.to_string(),
            ctl,
            phase: "准备".into(),
            title: title.into(),
            current: None,
            items_done: 0,
            items_total: 0,
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            can_pause: true,
            indeterminate: true,
            start: now,
            last: now - THROTTLE,
        }
    }

    // ---------- 更新计数 ----------

    pub fn set_phase(&mut self, phase: &str, title: impl Into<String>) {
        self.phase = phase.to_string();
        self.title = title.into();
        self.flush();
    }

    pub fn set_current(&mut self, path: impl AsRef<std::path::Path>) {
        self.current = Some(crate::fsutil::short_path(path.as_ref()));
    }

    pub fn set_totals(&mut self, items: u64, files: u64, bytes: u64) {
        self.items_total = items;
        self.files_total = files;
        self.bytes_total = bytes;
        self.indeterminate = self.bytes_total == 0 && self.items_total == 0;
    }

    pub fn set_items_total(&mut self, items: u64) {
        self.items_total = items;
    }

    /// 只设置字节总量（不动条目/文件计数）：搬运类任务用它驱动字节级进度条，
    /// 同时保留"第几项 / 共几项"的显示。目前仅 Windows 的高级迁移用得到。
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn set_bytes_total(&mut self, bytes: u64) {
        self.bytes_total = bytes;
        self.files_total = 0;
        self.indeterminate = self.bytes_total == 0 && self.items_total == 0;
    }

    /// 枚举阶段：已发现的总量（绝对值，不是增量），配合不定的总量显示
    pub fn set_discovered(&mut self, bytes: u64, files: u64) {
        self.bytes_done = bytes;
        self.files_done = files;
        self.tick();
    }

    /// 已完成一个处理单元（文件/条目）
    pub fn add_files(&mut self, files: u64, bytes: u64) {
        self.files_done += files;
        self.bytes_done += bytes;
        self.tick();
    }

    /// 文件级进度（永久删除时每个文件调用一次）
    pub fn add_file(&mut self, bytes: u64) {
        self.files_done += 1;
        self.bytes_done += bytes;
        self.tick();
    }

    /// 已完成一个顶层条目
    pub fn inc_items(&mut self) {
        self.items_done += 1;
        self.tick();
    }

    pub fn set_can_pause(&mut self, v: bool) {
        self.can_pause = v;
    }

    /// 每个处理单元后调用：节流发射，绝不空转。
    pub fn tick(&mut self) {
        if self.last.elapsed() >= THROTTLE {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        self.last = Instant::now();
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        let ratio = self.ratio();
        let secs = self.start.elapsed().as_secs_f64().max(0.001);
        let p = TaskProgress {
            task_id: self.ctl.id.clone(),
            kind: self.kind.clone(),
            phase: self.phase.clone(),
            title: self.title.clone(),
            current: self.current.clone(),
            items_done: self.items_done,
            items_total: self.items_total,
            files_done: self.files_done,
            files_total: self.files_total,
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            ratio,
            indeterminate: self.indeterminate || ratio.is_none(),
            paused: self.ctl.is_paused(),
            can_pause: self.can_pause,
            elapsed_ms,
            speed_bps: self.bytes_done as f64 / secs,
        };
        let _ = self.app.emit("task://progress", p);
    }

    fn ratio(&self) -> Option<f64> {
        if self.bytes_total > 0 {
            return Some((self.bytes_done as f64 / self.bytes_total as f64).clamp(0.0, 1.0));
        }
        if self.items_total > 0 {
            return Some((self.items_done as f64 / self.items_total as f64).clamp(0.0, 1.0));
        }
        None
    }

    // ---------- 日志 / 数据行 ----------

    pub fn log(&self, line: impl Into<String>, level: &str) {
        let _ = self.app.emit(
            "task://log",
            TaskLog {
                task_id: self.ctl.id.clone(),
                line: line.into(),
                level: level.to_string(),
            },
        );
    }

    pub fn row<T: Serialize + Clone>(&self, kind: &str, row: T) {
        let _ = self.app.emit(
            "data://row",
            DataRow {
                kind: kind.to_string(),
                row,
            },
        );
    }

    pub fn rows_done(&self, kind: &str, total: u64) {
        let _ = self.app.emit(
            "data://done",
            DataDone {
                kind: kind.to_string(),
                total,
            },
        );
    }

    // ---------- 结束 ----------

    #[allow(clippy::too_many_arguments)]
    pub fn finish(
        &self,
        ok: u64,
        skipped: u64,
        failed: u64,
        bytes: u64,
        stopped: bool,
        message: impl Into<String>,
        report_path: Option<String>,
    ) {
        let d = TaskDone {
            task_id: self.ctl.id.clone(),
            kind: self.kind.clone(),
            ok,
            skipped,
            failed,
            bytes,
            stopped,
            message: message.into(),
            report_path,
        };
        let _ = self.app.emit("task://done", d);
        self.ctl.mark_finished();
    }
}
