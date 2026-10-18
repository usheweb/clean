// Windows 专属能力：固定盘枚举、盘剩余空间、注册表盘点已装软件、UAC 提权重启
use std::ffi::c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetDriveTypeW(lp_root_path_name: *const u16) -> u32;
    fn GetDiskFreeSpaceExW(
        lp_directory_name: *const u16,
        lp_free_bytes_available_to_caller: *mut u64,
        lp_total_number_of_bytes: *mut u64,
        lp_total_number_of_free_bytes: *mut u64,
    ) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: *mut c_void,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> *mut c_void;
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

const DRIVE_FIXED: u32 = 3;

/// 枚举本机固定盘（C:、D: …），不含移动盘/光驱
pub fn list_fixed_drives() -> Vec<String> {
    let mut out = Vec::new();
    for c in b'A'..=b'Z' {
        let root = format!("{}:\\", c as char);
        let w = wide(&root);
        let t = unsafe { GetDriveTypeW(w.as_ptr()) };
        if t == DRIVE_FIXED {
            out.push(format!("{}:", c as char));
        }
    }
    out
}

/// 某盘的 (可用空间, 总空间)
pub fn drive_space(letter: &str) -> Option<(u64, u64)> {
    let root = if letter.ends_with('\\') {
        letter.to_string()
    } else {
        format!("{}\\", letter)
    };
    let w = wide(&root);
    let (mut free, mut total, mut total_free) = (0u64, 0u64, 0u64);
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            w.as_ptr(),
            &mut free,
            &mut total,
            &mut total_free,
        )
    };
    if ok == 0 {
        None
    } else {
        Some((free, total))
    }
}

/// 以管理员身份重启本程序（用于需要 admin 的清理项）
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = wide(&exe.display().to_string());
    let op = wide("runas");
    let ret = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1,
        )
    };
    // ShellExecuteW 返回值 <= 32 表示失败
    if (ret as isize) <= 32 {
        Err("提权被拒绝或失败".into())
    } else {
        Ok(())
    }
}

pub struct RawApp {
    pub name: String,
    pub version: String,
    pub loc: String,
}

const APP_BAD_LOCS: [&str; 6] = [
    "c:",
    "c:\\program files",
    "c:\\program files (x86)",
    "c:\\windows",
    "c:\\programdata",
    "c:\\program files\\common files",
];

/// 只读读取注册表 Uninstall 项，盘点装在 C 盘的程序本体。
pub fn read_installed_apps() -> Vec<RawApp> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let roots = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (
            HKEY_CURRENT_USER,
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ),
    ];

    let mut seen: Vec<(String, String)> = Vec::new();
    let mut apps: Vec<RawApp> = Vec::new();

    for (hive, sub) in roots {
        let root = RegKey::predef(hive);
        let Ok(top) = root.open_subkey_with_flags(sub, KEY_READ) else {
            continue;
        };
        for key_name in top.enum_keys().flatten() {
            let Ok(k) = top.open_subkey_with_flags(&key_name, KEY_READ) else {
                continue;
            };
            let disp: Option<String> = k.get_value("DisplayName").ok();
            let Some(disp) = disp else { continue };
            let name = disp.trim().to_string();
            if name.is_empty() {
                continue;
            }
            let sysc: u32 = k.get_value("SystemComponent").unwrap_or(0);
            if sysc == 1 {
                continue;
            }
            let loc_raw: String = k.get_value("InstallLocation").unwrap_or_default();
            let loc_raw = loc_raw.trim().trim_matches('"').to_string();
            if loc_raw.is_empty() {
                continue;
            }
            let loc = crate::fsutil::expand_env(&loc_raw);
            let low = loc.to_lowercase().replace('/', "\\");
            let low = low.trim_end_matches('\\').to_string();
            if !low.starts_with("c:\\") || APP_BAD_LOCS.contains(&low.as_str()) {
                continue;
            }
            if low.contains("windowsapps") {
                continue;
            }
            let mark = (name.to_lowercase(), low.clone());
            if seen.contains(&mark) {
                continue;
            }
            seen.push(mark);
            apps.push(RawApp {
                name,
                version: k
                    .get_value::<String, _>("DisplayVersion")
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                loc,
            });
        }
    }
    apps
}
