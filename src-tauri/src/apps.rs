// 已装软件盘点（只读）：Windows 读注册表，macOS 扫 /Applications
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::fsutil;
use crate::state::AppState;
use crate::task::Emitter;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledApp {
    pub name: String,
    pub version: String,
    pub location: String,
    pub bytes: u64,
}

struct Raw {
    name: String,
    version: String,
    loc: String,
}

#[cfg(windows)]
fn collect_raw() -> Vec<Raw> {
    crate::platform::windows::read_installed_apps()
        .into_iter()
        .map(|a| Raw {
            name: a.name,
            version: a.version,
            loc: a.loc,
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn collect_raw() -> Vec<Raw> {
    crate::platform::macos::list_app_dirs()
        .into_iter()
        .map(|a| Raw {
            name: a.name,
            version: a.version,
            loc: a.loc,
        })
        .collect()
}

#[cfg(not(any(windows, target_os = "macos")))]
fn collect_raw() -> Vec<Raw> {
    Vec::new()
}

#[tauri::command]
pub fn start_apps_scan(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let ctl = state.begin_task("apps_scan")?;
    let id = ctl.id.clone();
    let mut em = Emitter::new(app, ctl, "盘点已装软件");
    std::thread::spawn(move || {
        run_scan(&mut em);
    });
    Ok(id)
}

fn run_scan(em: &mut Emitter) {
    em.set_phase("扫描", "正在读取已安装软件…");
    let raw = collect_raw();
    if raw.is_empty() {
        em.rows_done("installed_apps", 0);
        em.finish(0, 0, 0, 0, false, "没有找到可盘点的软件", None);
        return;
    }
    em.set_items_total(raw.len() as u64);
    em.set_totals(raw.len() as u64, 0, 0);
    em.flush();

    let mut out: Vec<InstalledApp> = Vec::new();
    let mut stopped = false;
    let mut total: u64 = 0;
    for r in &raw {
        if em.ctl.wait_gate() {
            stopped = true;
            break;
        }
        em.set_current(&r.loc);
        let loc = std::path::PathBuf::from(&r.loc);
        if !loc.is_dir() || fsutil::is_reparse(&loc) {
            em.inc_items();
            continue; // 目录不存在或是联接：位置未知，不统计
        }
        let ctl = em.ctl.clone();
        let st = fsutil::count_tree(&loc, &ctl, &mut |b, f, cur| {
            em.set_current(cur);
            em.set_discovered(b + total, f);
        });
        if st.cancelled {
            stopped = true;
            break;
        }
        total += st.bytes;
        let app = InstalledApp {
            name: r.name.clone(),
            version: r.version.clone(),
            location: r.loc.clone(),
            bytes: st.bytes,
        };
        em.row("installed_apps", app.clone());
        out.push(app);
        em.inc_items();
    }

    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    let sum: u64 = out.iter().map(|a| a.bytes).sum();
    em.rows_done("installed_apps", out.len() as u64);
    let msg = format!(
        "{}共 {} 个软件，合计 {}",
        if stopped { "已中断，" } else { "" },
        out.len(),
        fsutil::fmt_size(sum)
    );
    em.finish(out.len() as u64, 0, 0, sum, stopped, msg, None);
}
