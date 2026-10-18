// 大文件扫描（跨平台，只读，不提供任何删除入口）
use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::fsutil;
use crate::state::AppState;
use crate::task::Emitter;

const DEFAULT_MIN: u64 = 100 * 1024 * 1024;
const TOPN: usize = 200;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BigFile {
    pub path: String,
    pub name: String,
    pub bytes: u64,
    pub mtime: u64,
    pub kind: String,
}

/// 从路径层面就硬排除的目录名（小写）
const EXCLUDE_DIRS: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "$recycle.bin",
    "system volume information",
    "recovery",
    "perflogs",
    "config.msi",
    "$winreagent",
    "$sysreset",
    "msocache",
    // macOS：系统与库目录（用户数据区之外的都不碰）
    "system",
    "volumes",
    "cores",
    "trash",
    ".trash",
];

/// 不含 AppData 时额外硬排除（这些是软件数据目录，体积大但扫起来很慢）
#[cfg(windows)]
const EXCLUDE_WHEN_SHALLOW: &[&str] = &["appdata", "application data", "local settings"];
#[cfg(not(windows))]
const EXCLUDE_WHEN_SHALLOW: &[&str] = &["library"];

const EXCLUDE_FILES: &[&str] = &[
    "pagefile.sys",
    "hiberfil.sys",
    "swapfile.sys",
    "dumpstack.log.tmp",
    "ntuser.dat",
    "ntuser.dat.log1",
    "ntuser.dat.log2",
    "bootmgr",
    "bootnxt",
    "winre.wim",
    "install.wim",
];

const KINDS: &[(&str, &[&str])] = &[
    ("视频", &[".mp4", ".mkv", ".avi", ".mov", ".flv", ".wmv", ".ts", ".m4v", ".rmvb", ".webm", ".mpg", ".mpeg"]),
    ("镜像/虚拟磁盘", &[".iso", ".vhd", ".vhdx", ".vmdk", ".qcow2", ".img", ".sparseimage"]),
    ("压缩包", &[".zip", ".rar", ".7z", ".tar", ".gz", ".xz", ".bz2", ".tgz"]),
    ("安装包", &[".exe", ".msi", ".msix", ".appx", ".apk", ".dmg", ".pkg", ".deb", ".rpm"]),
    ("数据库/备份", &[".db", ".db3", ".sqlite", ".sqlite3", ".mdb", ".accdb", ".bak", ".dump", ".sql"]),
    ("日志/转储", &[".log", ".dmp", ".etl", ".evtx", ".ips", ".crash"]),
    ("游戏/素材包", &[".pak", ".vpk", ".assets", ".bundle", ".cab"]),
    ("设计/素材", &[".psd", ".ai", ".sketch", ".fig", ".blend", ".fbx"]),
];

fn kind_of(path: &str) -> String {
    let lower = path.to_lowercase();
    let ext = match lower.rfind('.') {
        Some(i) => &lower[i..],
        None => return "其它".into(),
    };
    for (name, exts) in KINDS {
        if exts.contains(&ext) {
            return (*name).to_string();
        }
    }
    if [".sys", ".dll", ".dat", ".app", ".framework"].contains(&ext) {
        return "系统/程序文件".into();
    }
    "其它".into()
}

fn mtime_of(md: &std::fs::Metadata) -> u64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[tauri::command]
pub fn start_bigfile_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    include_app_data: bool,
    min_bytes: u64,
) -> Result<String, String> {
    let ctl = state.begin_task("bigfile_scan")?;
    let id = ctl.id.clone();
    let min = if min_bytes == 0 { DEFAULT_MIN } else { min_bytes };
    let mut em = Emitter::new(app, ctl, "扫描大文件");
    std::thread::spawn(move || {
        run_scan(&mut em, include_app_data, min);
    });
    Ok(id)
}

fn run_scan(em: &mut Emitter, include_app_data: bool, min_bytes: u64) {
    em.set_phase("扫描", "正在扫描大文件…");
    let Some(home) = dirs::home_dir() else {
        em.finish(0, 0, 0, 0, true, "无法定位用户目录", None);
        return;
    };

    let mut found: Vec<BigFile> = Vec::new();
    let mut seen_dirs: u64 = 0;
    let mut stack: Vec<PathBuf> = vec![home];
    let mut stopped = false;
    let mut last_tick = std::time::Instant::now();

    while let Some(cur) = stack.pop() {
        if em.ctl.wait_gate() {
            stopped = true;
            break;
        }
        seen_dirs += 1;
        em.set_current(&cur);
        if last_tick.elapsed() >= std::time::Duration::from_millis(150) {
            last_tick = std::time::Instant::now();
            em.set_discovered(0, seen_dirs);
            em.flush();
        }
        let rd = match std::fs::read_dir(&cur) {
            Ok(rd) => rd,
            Err(_) => continue, // 权限不足/已被删：静默跳过
        };
        for e in rd.flatten() {
            if em.ctl.is_cancelled() {
                stopped = true;
                break;
            }
            let p = e.path();
            let md = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.file_type().is_symlink() || fsutil::is_reparse(&p) {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_lowercase();
            if md.is_dir() {
                if EXCLUDE_DIRS.contains(&name.as_str()) {
                    continue;
                }
                if !include_app_data && EXCLUDE_WHEN_SHALLOW.contains(&name.as_str()) {
                    continue;
                }
                stack.push(p);
            } else if !EXCLUDE_FILES.contains(&name.as_str()) && md.len() >= min_bytes {
                let display = p.display().to_string();
                let bf = BigFile {
                    kind: kind_of(&display),
                    name: e.file_name().to_string_lossy().to_string(),
                    bytes: md.len(),
                    mtime: mtime_of(&md),
                    path: display,
                };
                em.row("big_files", bf.clone());
                found.push(bf);
            }
        }
    }

    found.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    found.truncate(TOPN);
    em.rows_done("big_files", found.len() as u64);
    let total_bytes: u64 = found.iter().map(|f| f.bytes).sum();
    let msg = format!(
        "{}大文件 {} 个，合计 {}（已扫描 {} 个目录）",
        if stopped { "已中断，" } else { "" },
        found.len(),
        fsutil::fmt_size(total_bytes),
        seen_dirs
    );
    em.finish(0, 0, 0, total_bytes, stopped, msg, None);
}
