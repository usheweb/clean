// 唐三浪清理 —— 跨平台系统垃圾清理与磁盘空间管理
//
// 分层：
//   commands（各模块里的 #[tauri::command]）只做参数校验 + 起后台线程
//   *_engine（clean/migrate/bigfile/apps）在后台线程里跑真正的活，通过事件回报
//   fsutil/platform 提供可复用的底层能力
mod apps;
mod bigfile;
mod clean;
mod fsutil;
mod migrate;
mod platform;
mod settings;
mod state;
mod task;

use serde::Serialize;
use tauri::{Manager, State};

use state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    product_name: String,
    version: String,
    platform: String,
    is_admin: bool,
    home_dir: String,
}

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        product_name: "唐三浪清理".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        platform: platform::platform_name().into(),
        is_admin: platform::is_admin(),
        home_dir: dirs::home_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
fn cancel_task(state: State<'_, AppState>) -> Result<(), String> {
    match state.current_task() {
        Some(t) => {
            t.cancel();
            Ok(())
        }
        None => Err("当前没有正在进行的任务".into()),
    }
}

#[tauri::command]
fn pause_task(state: State<'_, AppState>) -> Result<(), String> {
    match state.current_task() {
        Some(t) => {
            t.set_pause(true);
            Ok(())
        }
        None => Err("当前没有正在进行的任务".into()),
    }
}

#[tauri::command]
fn resume_task(state: State<'_, AppState>) -> Result<(), String> {
    match state.current_task() {
        Some(t) => {
            t.set_pause(false);
            Ok(())
        }
        None => Err("当前没有正在进行的任务".into()),
    }
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    platform::open_path(std::path::Path::new(&path))
}

#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    platform::reveal_path(std::path::Path::new(&path))
}

#[tauri::command]
fn relaunch_as_admin() -> Result<(), String> {
    #[cfg(windows)]
    {
        platform::windows::relaunch_as_admin()
    }
    #[cfg(not(windows))]
    {
        Err("当前平台不支持提权".into())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let cfg = settings::Settings::load(&data_dir.join("settings.json"));
            app.manage(AppState::new(data_dir, cfg));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            get_app_info,
            cancel_task,
            pause_task,
            resume_task,
            open_path,
            reveal_path,
            relaunch_as_admin,
            // 清理
            clean::list_clean_items,
            clean::start_clean_scan,
            clean::start_clean_run,
            // 大文件
            bigfile::start_bigfile_scan,
            // 已装软件
            apps::start_apps_scan,
            // 高级迁移（Windows）
            migrate::list_migrate_items,
            migrate::list_drives,
            migrate::start_migrate_scan,
            migrate::start_migrate_run,
            migrate::list_ledger,
            migrate::start_restore,
            migrate::start_reorganize,
            migrate::start_rebuild_ledger,
            // 设置
            settings::get_settings,
            settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
