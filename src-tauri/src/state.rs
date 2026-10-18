// 全局状态：同一时刻只允许一个后台任务；任务可取消/暂停。
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

static TASK_SEQ: AtomicU64 = AtomicU64::new(1);

/// 一个后台任务的控制器。所有字段都是 Arc，克隆后各线程共享同一份标志。
#[derive(Clone)]
pub struct TaskCtl {
    pub id: String,
    pub kind: &'static str,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
}

impl TaskCtl {
    fn new(kind: &'static str) -> Self {
        let n = TASK_SEQ.fetch_add(1, Ordering::Relaxed);
        Self {
            id: format!("t{}", n),
            kind,
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 一个永不取消的空控制块，供无任务上下文的只读统计使用。
    pub fn noop() -> Self {
        Self::new("noop")
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }

    pub fn set_pause(&self, v: bool) {
        self.pause.store(v, Ordering::Relaxed);
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.pause.store(false, Ordering::Relaxed);
    }

    pub fn mark_finished(&self) {
        self.finished.store(true, Ordering::Relaxed);
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    /// 在每个可中断的边界调用：暂停时阻塞等待，取消时返回 true（调用方应立刻收尾退出）。
    pub fn wait_gate(&self) -> bool {
        loop {
            if self.is_cancelled() {
                return true;
            }
            if !self.is_paused() {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
    }
}

pub struct AppState {
    current: Mutex<Option<TaskCtl>>,
    pub data_dir: PathBuf,
    settings: Mutex<Settings>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, settings: Settings) -> Self {
        Self {
            current: Mutex::new(None),
            data_dir,
            settings: Mutex::new(settings),
        }
    }

    /// 开始一个任务；已有任务在跑时拒绝。
    pub fn begin_task(&self, kind: &'static str) -> Result<TaskCtl, String> {
        let mut cur = self.current.lock().unwrap();
        if let Some(t) = cur.as_ref() {
            if !t.is_finished() {
                return Err("已有任务正在进行，请先停止或等待其完成".into());
            }
        }
        let ctl = TaskCtl::new(kind);
        *cur = Some(ctl.clone());
        Ok(ctl)
    }

    pub fn current_task(&self) -> Option<TaskCtl> {
        self.current.lock().unwrap().clone()
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn update_settings(&self, s: Settings) {
        *self.settings.lock().unwrap() = s;
    }
}
