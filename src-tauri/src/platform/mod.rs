// 平台差异封装：管理员检测、打开/定位路径、平台名
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
pub mod windows;
#[cfg(target_os = "macos")]
pub mod macos;

pub fn platform_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[cfg(windows)]
pub fn is_admin() -> bool {
    #[link(name = "shell32")]
    extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    unsafe { IsUserAnAdmin() != 0 }
}

#[cfg(target_os = "macos")]
pub fn is_admin() -> bool {
    extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn is_admin() -> bool {
    false
}

/// 用系统默认程序打开该路径
pub fn open_path(path: &Path) -> Result<(), String> {
    let s = path.display().to_string();
    #[cfg(windows)]
    let (cmd, args) = ("explorer", vec![s]);
    #[cfg(target_os = "macos")]
    let (cmd, args) = ("open", vec![s]);
    #[cfg(not(any(windows, target_os = "macos")))]
    let (cmd, args) = ("xdg-open", vec![s]);
    Command::new(cmd)
        .args(&args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("无法打开: {}", e))
}

/// 在文件管理器中定位该路径（Windows 选中该项 / macOS 在 Finder 中显示）
pub fn reveal_path(path: &Path) -> Result<(), String> {
    let s = path.display().to_string();
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", s))
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("无法定位: {}", e))
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &s])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("无法定位: {}", e))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        open_path(path)
    }
}
