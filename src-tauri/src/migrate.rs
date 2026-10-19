// 高级迁移（Windows 专属）：把大目录整体搬到别的盘，原位置留一个 junction 联接，
// 系统和软件完全无感知。任何一步失败都会自动回滚，绝不丢数据。
//
// 结构：
//   * 顶部是跨平台共享类型 + 命令壳（非 Windows 返回"不支持"，前端在 macOS 隐藏该页）
//   * Windows 实现全部放在 #[cfg(windows)] mod imp 里，macOS 不编译，避免 unused 警告
//
// 与旧 Python 版的对应关系（参考 clean_c.py）：
//   白名单 600–732 / 候选发现 754–886 / 账本 905–1165 / 搬运核心 1277–1559 / 辅助 173–410
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::state::AppState;

// ---------------------------------------------------------------- 共享类型

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MigrateItem {
    pub key: String,
    pub name: String,
    pub src: String,
    pub category: String,
    pub procs: Vec<String>,
    pub subdir: String,
    pub desc: String,
}

/// 扫描出来的一行。需要反序列化：迁移时前端把整行回传（候选 key 是有损变换，无法反推路径）。
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MigrateRow {
    pub key: String,
    pub name: String,
    pub src: String,
    pub category: String,
    pub desc: String,
    pub bytes: u64,
    pub files: u64,
    pub running: Vec<String>,
    pub candidate: bool,
    pub migrated: bool,
}

/// 账本条目。需要反序列化：从磁盘文件 / 日志里读回账本。
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub key: String,
    pub name: String,
    pub src: String,
    pub dst: String,
    pub bytes: u64,
    pub time: String,
    pub candidate: bool,
}

#[cfg(not(windows))]
const UNSUPPORTED: &str = "高级迁移仅支持 Windows";

// ---------------------------------------------------------------- 命令壳

#[cfg(windows)]
#[tauri::command]
pub fn list_migrate_items() -> Vec<MigrateItem> {
    imp::list_items()
}

#[cfg(not(windows))]
#[tauri::command]
pub fn list_migrate_items() -> Vec<MigrateItem> {
    Vec::new()
}

#[cfg(windows)]
#[tauri::command]
pub fn list_drives() -> Vec<String> {
    crate::platform::windows::list_fixed_drives()
}

#[cfg(not(windows))]
#[tauri::command]
pub fn list_drives() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
#[tauri::command]
pub fn start_migrate_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    drive: String,
    root_name: String,
    include_candidates: bool,
) -> Result<String, String> {
    if !imp::valid_root_name(&root_name) {
        return Err("目标文件夹名不能为空, 且不能包含 \\ / : * ? \" < > | 等字符。".into());
    }
    if imp::drive_letter(&drive).is_empty() {
        return Err("没有可用的目标盘".into());
    }
    let ctl = state.begin_task("migrate_scan")?;
    imp::run_scan(
        app,
        ctl.clone(),
        state.data_dir.clone(),
        include_candidates,
    );
    Ok(ctl.id)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn start_migrate_scan(
    _app: AppHandle,
    _state: State<'_, AppState>,
    _drive: String,
    _root_name: String,
    _include_candidates: bool,
) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

#[cfg(windows)]
#[tauri::command]
pub fn start_migrate_run(
    app: AppHandle,
    state: State<'_, AppState>,
    rows: Vec<MigrateRow>,
    drive: String,
    root_name: String,
) -> Result<String, String> {
    if rows.is_empty() {
        return Err("请先勾选要迁移的目录".into());
    }
    if !imp::valid_root_name(&root_name) {
        return Err("目标文件夹名不能为空, 且不能包含 \\ / : * ? \" < > | 等字符。".into());
    }
    if imp::drive_letter(&drive).is_empty() {
        return Err("没有可用的目标盘".into());
    }
    let ctl = state.begin_task("migrate_run")?;
    imp::run_migration(
        app,
        ctl.clone(),
        state.data_dir.clone(),
        rows,
        drive,
        root_name,
    );
    Ok(ctl.id)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn start_migrate_run(
    _app: AppHandle,
    _state: State<'_, AppState>,
    _rows: Vec<MigrateRow>,
    _drive: String,
    _root_name: String,
) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

#[cfg(windows)]
#[tauri::command]
pub fn list_ledger(state: State<'_, AppState>) -> Vec<LedgerEntry> {
    imp::ledger_entries(&state.data_dir)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn list_ledger(_state: State<'_, AppState>) -> Vec<LedgerEntry> {
    Vec::new()
}

#[cfg(windows)]
#[tauri::command]
pub fn start_restore(
    app: AppHandle,
    state: State<'_, AppState>,
    srcs: Vec<String>,
) -> Result<String, String> {
    if srcs.is_empty() {
        return Err("请先勾选要还原的账本记录".into());
    }
    let ctl = state.begin_task("migrate_restore")?;
    imp::run_restore(app, ctl.clone(), state.data_dir.clone(), srcs);
    Ok(ctl.id)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn start_restore(
    _app: AppHandle,
    _state: State<'_, AppState>,
    _srcs: Vec<String>,
) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

#[cfg(windows)]
#[tauri::command]
pub fn start_reorganize(
    app: AppHandle,
    state: State<'_, AppState>,
    drive: String,
    new_root: String,
) -> Result<String, String> {
    if !imp::valid_root_name(&new_root) {
        return Err("目标文件夹名不能为空, 且不能包含 \\ / : * ? \" < > | 等字符。".into());
    }
    if imp::drive_letter(&drive).is_empty() {
        return Err("没有可用的目标盘".into());
    }
    let ctl = state.begin_task("migrate_reorganize")?;
    imp::run_reorganize(app, ctl.clone(), state.data_dir.clone(), drive, new_root);
    Ok(ctl.id)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn start_reorganize(
    _app: AppHandle,
    _state: State<'_, AppState>,
    _drive: String,
    _new_root: String,
) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

#[cfg(windows)]
#[tauri::command]
pub fn start_rebuild_ledger(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ctl = state.begin_task("migrate_rebuild")?;
    imp::run_rebuild(app, ctl.clone(), state.data_dir.clone());
    Ok(ctl.id)
}

#[cfg(not(windows))]
#[tauri::command]
pub fn start_rebuild_ledger(
    _app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<String, String> {
    Err(UNSUPPORTED.into())
}

// ================================================================ Windows 实现

#[cfg(windows)]
mod imp {
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::time::Duration;

    use tauri::AppHandle;

    use super::{LedgerEntry, MigrateItem, MigrateRow};
    use crate::fsutil;
    use crate::state::TaskCtl;
    use crate::task::Emitter;

    const MB: u64 = 1024 * 1024;

    // ---------------------------------------------------------------- 进程常量

    fn jb_procs() -> Vec<String> {
        strs(&[
            "idea64.exe", "idea.exe", "webstorm64.exe", "phpstorm64.exe",
            "goland64.exe", "pycharm64.exe", "clion64.exe", "rider64.exe",
            "datagrip64.exe", "rubymine64.exe", "studio64.exe",
            "jetbrains-toolbox.exe",
        ])
    }

    fn java_procs() -> Vec<String> {
        let mut v = jb_procs();
        v.push("java.exe".into());
        v.push("javaw.exe".into());
        v
    }

    fn wps_procs() -> Vec<String> {
        strs(&[
            "wps.exe", "et.exe", "wpp.exe", "wpscloudsvr.exe",
            "ksomisc.exe", "wpscenter.exe", "wpsupdate.exe",
        ])
    }

    fn tencent_procs() -> Vec<String> {
        strs(&[
            "QQ.exe", "QQScLauncher.exe", "QQProtect.exe", "QQBrowser.exe",
            "WeChat.exe", "Weixin.exe", "weixin.exe", "wechat.exe", "TencentDL.exe",
        ])
    }

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ---------------------------------------------------------------- 白名单条目

    struct MigDef {
        key: String,
        name: String,
        cat: String,
        src: String,
        procs: Vec<String>,
        wsl: bool,
        desc: String,
    }

    fn item(key: &str, name: &str, cat: &str, src: &str, procs: Vec<String>, desc: &str) -> MigDef {
        MigDef {
            key: key.into(),
            name: name.into(),
            cat: cat.into(),
            src: src.into(),
            procs,
            wsl: false,
            desc: desc.into(),
        }
    }

    fn p1(a: &str) -> Vec<String> {
        vec![a.to_string()]
    }

    /// 37 条白名单：开发 21 / 聊天 5 / 办公 2 / 浏览器 4 / 工具 3 / 系统 2
    fn defs() -> Vec<MigDef> {
        vec![
            // ---------------- 开发 / 编程 ----------------
            item("jb_local", "JetBrains 本地缓存(Local)", "开发", r"%LOCALAPPDATA%\JetBrains", jb_procs(),
                 "索引/编译缓存, 迁移后首次打开重建索引"),
            item("jb_roam", "JetBrains 配置(Roaming)", "开发", r"%APPDATA%\JetBrains", jb_procs(),
                 "IDE 配置与插件"),
            item("vscode_ext", "VS Code 扩展", "开发", r"%USERPROFILE%\.vscode",
                 strs(&["Code.exe", "code.exe"]), "扩展本体与 argv.json"),
            item("vscode_roam", "VS Code 数据(Roaming)", "开发", r"%APPDATA%\Code",
                 strs(&["Code.exe", "code.exe"]), "设置/快捷键与缓存, 迁移前必须完全退出 VS Code"),
            item("cursor", "Cursor 数据", "开发", r"%APPDATA%\Cursor",
                 strs(&["Cursor.exe", "cursor.exe"]), "Cursor 编辑器设置与缓存"),
            item("trae", "Trae 扩展与数据", "开发", r"%USERPROFILE%\.trae-cn",
                 strs(&["Trae.exe", "Trae CN.exe"]), "Trae 的扩展与本地数据"),
            item("trae_cn", "Trae CN 应用数据", "开发", r"%APPDATA%\Trae CN",
                 strs(&["Trae CN.exe", "Trae.exe"]), "Trae 国内版的设置与缓存(体积常常很大)"),
            item("marscode", "MarsCode 数据", "开发", r"%USERPROFILE%\.marscode",
                 strs(&["Trae.exe", "Trae CN.exe", "MarsCode.exe"]),
                 "MarsCode/Trae 旧数据, 含 AI 对话历史"),
            item("codeium", "Codeium/Windsurf 数据", "开发", r"%USERPROFILE%\.codeium",
                 strs(&["Windsurf.exe", "language_server.exe"]), "Windsurf 插件数据"),
            item("postman", "Postman 应用数据", "开发", r"%LOCALAPPDATA%\Postman",
                 strs(&["Postman.exe", "PostmanAgent.exe"]), "Postman 本体与数据"),
            item("android_sdk", "Android SDK", "开发", r"%LOCALAPPDATA%\Android\Sdk",
                 strs(&["studio64.exe", "adb.exe", "emulator.exe", "qemu-system-x86_64.exe"]),
                 "安卓开发工具包(体积很大), 迁移后 Android Studio 照常用"),
            item("gradle_home", "Gradle 缓存", "开发", r"%USERPROFILE%\.gradle", java_procs(),
                 "Gradle 依赖缓存与构建缓存"),
            item("maven_repo", "Maven 本地仓库", "开发", r"%USERPROFILE%\.m2", java_procs(),
                 "Maven 依赖仓库(体积常常很大)"),
            item("nuget_global", "NuGet 包缓存", "开发", r"%USERPROFILE%\.nuget\packages",
                 strs(&["dotnet.exe", "msbuild.exe", "devenv.exe", "VBCSCompiler.exe"]),
                 ".NET 包缓存, 迁移后 Visual Studio 照常用"),
            item("go_mod", "Go 模块缓存", "开发", r"%USERPROFILE%\go\pkg\mod",
                 strs(&["go.exe", "gopls.exe", "dlv.exe"]), "go mod 下载的模块缓存"),
            item("npm_cache", "npm 缓存", "开发", r"%LOCALAPPDATA%\npm-cache",
                 strs(&["node.exe", "npm.exe"]), "npm 包缓存, 删了/搬了都会自动重建"),
            item("pip_cache", "pip 缓存", "开发", r"%LOCALAPPDATA%\pip\Cache",
                 strs(&["python.exe", "pythonw.exe"]), "pip 下载的 wheel 缓存, 可放心迁移"),
            item("pub_cache", "Dart/Flutter 包缓存", "开发", r"%LOCALAPPDATA%\Pub\Cache",
                 strs(&["dart.exe", "flutter.exe"]), "Flutter/Dart 依赖缓存"),
            item("unity", "Unity 缓存", "开发", r"%LOCALAPPDATA%\Unity",
                 strs(&["Unity.exe", "UnityHub.exe", "UnityShaderCompiler.exe"]),
                 "Unity 编辑器缓存与包缓存"),
            item("zig", "zig 工具链", "开发", r"%LOCALAPPDATA%\zig", p1("zig.exe"),
                 "zig 编译器本体(按路径使用, 迁移后照常可用)"),
            item("gitkraken_cli", "GitKraken 命令行工具", "开发", r"%LOCALAPPDATA%\GitKrakenCLI",
                 strs(&["GitKraken.exe", "gitkraken.exe", "GitKrakenCLI.exe"]),
                 "GitKraken 附带的 git 命令行, 迁移前请确认已退出 GitKraken"),

            // ---------------- 聊天 / 办公 ----------------
            item("dingtalk", "钉钉数据", "聊天", r"%APPDATA%\DingTalk",
                 strs(&["DingTalk.exe", "DingtalkLauncher.exe", "DingTalkUpdater.exe", "dingtalk.exe"]),
                 "钉钉聊天文件与缓存"),
            item("larkshell", "飞书数据", "聊天", r"%APPDATA%\LarkShell",
                 strs(&["LarkShell.exe", "Lark.exe", "Feishu.exe", "feishu.exe"]),
                 "飞书聊天文件与缓存"),
            item("xwechat", "微信(新版)运行数据", "聊天", r"%APPDATA%\Tencent\xwechat",
                 strs(&["WeChat.exe", "Weixin.exe", "weixin.exe", "wechat.exe"]),
                 "新版微信插件/日志/运行数据(聊天记录文件在设置的保存路径, 不受影响)"),
            item("wxwork", "企业微信数据", "聊天", r"%APPDATA%\Tencent\WXWork",
                 strs(&["WXWork.exe", "WXWorkWeb.exe", "WXWorkUpdater.exe"]),
                 "企业微信聊天文件与缓存"),
            item("tencent_local", "腾讯系列本地数据(QQ等)", "聊天", r"%LOCALAPPDATA%\Tencent",
                 tencent_procs(), "QQ/QQ浏览器等腾讯软件的本地缓存, 迁移前请先退出这些软件"),
            item("wps_local", "WPS 本地缓存", "办公", r"%LOCALAPPDATA%\Kingsoft", wps_procs(),
                 "WPS 的缓存与本地数据"),
            item("wps_roam", "WPS 配置(Roaming)", "办公", r"%APPDATA%\kingsoft", wps_procs(),
                 "WPS 配置、模板与云文档缓存"),

            // ---------------- 浏览器 ----------------
            item("chrome_data", "Chrome 用户数据", "浏览器", r"%LOCALAPPDATA%\Google\Chrome",
                 p1("chrome.exe"), "含缓存与配置, 迁移前必须完全退出 Chrome"),
            item("edge_data", "Edge 用户数据", "浏览器", r"%LOCALAPPDATA%\Microsoft\Edge",
                 strs(&["msedge.exe", "msedgewebview2.exe", "MicrosoftEdgeUpdate.exe"]),
                 "含缓存与配置, 迁移前必须完全退出 Edge"),
            item("firefox", "Firefox 用户数据", "浏览器", r"%LOCALAPPDATA%\Mozilla",
                 p1("firefox.exe"), "含缓存与配置, 迁移前必须完全退出 Firefox"),
            item("brave", "Brave 用户数据", "浏览器", r"%LOCALAPPDATA%\BraveSoftware",
                 p1("brave.exe"), "含缓存与配置, 迁移前必须完全退出 Brave"),

            // ---------------- 工具 / 网盘 ----------------
            item("utools", "uTools 数据", "工具", r"%APPDATA%\uTools", p1("uTools.exe"),
                 "uTools 插件与本地数据"),
            item("oss_browser", "OSS 浏览器数据", "工具", r"%APPDATA%\oss-browser",
                 p1("oss-browser.exe"), "阿里云 OSS 浏览器客户端数据"),
            item("thunder", "迅雷数据", "工具", r"%APPDATA%\Thunder Network",
                 strs(&["Thunder.exe", "XLServicePlatform.exe", "DownloadSDKServer.exe"]),
                 "迅雷配置与缓存(下载文件本体在设置里的下载目录, 不受影响)"),

            // ---------------- 系统 / 容器 ----------------
            MigDef {
                key: "wsl".into(),
                name: "WSL 子系统数据".into(),
                cat: "系统".into(),
                src: r"%LOCALAPPDATA%\wsl".into(),
                procs: Vec::new(),
                wsl: true,
                desc: "WSL 发行版虚拟磁盘, 迁移/还原前会自动执行 wsl --shutdown".into(),
            },
            item("docker", "Docker Desktop 数据", "系统", r"%LOCALAPPDATA%\Docker",
                 strs(&["Docker Desktop.exe", "com.docker.backend.exe", "docker.exe"]),
                 "镜像与容器虚拟磁盘(通常体积很大), 迁移前必须完全退出 Docker"),
        ]
    }

    fn def_by_key(key: &str) -> Option<MigDef> {
        defs().into_iter().find(|d| d.key == key)
    }

    /// 按源路径反查白名单条目（候选目录、没存 key 的老记录用它）
    fn def_by_src(src: &str) -> Option<MigDef> {
        let low = normcase(src);
        defs().into_iter().find(|d| normcase(&expand(&d.src)) == low)
    }

    // ---------------------------------------------------------------- 路径工具

    /// 统一分隔符为 `\`、去掉结尾多余分隔符（保留 `X:\` 形式）
    fn norm(p: &str) -> String {
        let mut s = p.replace('/', "\\");
        while s.len() > 3 && s.ends_with('\\') {
            s.pop();
        }
        s
    }

    fn normcase(p: &str) -> String {
        norm(p).to_lowercase()
    }

    fn expand(p: &str) -> String {
        fsutil::expand_env(p)
    }

    fn strip_extended(p: &str) -> &str {
        p.strip_prefix(r"\\?\").unwrap_or(p)
    }

    fn drive_of(p: &str) -> String {
        let p = strip_extended(p);
        let b = p.as_bytes();
        if b.len() >= 2 && b[1] == b':' {
            p[..2].to_uppercase()
        } else {
            String::new()
        }
    }

    fn same_volume(a: &str, b: &str) -> bool {
        let da = drive_of(a);
        !da.is_empty() && da == drive_of(b)
    }

    /// 前端传来的盘符可能是 "D:"，统一成 "D"
    pub(crate) fn drive_letter(drive: &str) -> String {
        let s = drive.trim().trim_end_matches('\\').trim_end_matches(':');
        match s.chars().next() {
            Some(c) if c.is_ascii_alphabetic() => c.to_ascii_uppercase().to_string(),
            _ => String::new(),
        }
    }

    fn target_root(drive: &str, root_name: &str) -> String {
        norm(&format!("{}:\\{}", drive_letter(drive), root_name))
    }

    fn target_path(drive: &str, root_name: &str, key: &str) -> String {
        format!("{}\\{}", target_root(drive, root_name), key)
    }

    pub(crate) fn valid_root_name(s: &str) -> bool {
        !s.is_empty()
            && !s.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|'])
            && s != "."
            && s != ".."
    }

    fn list_names(dir: &str) -> Vec<String> {
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn rel_to(path: &str, root: &str) -> String {
        let p = norm(path);
        let r = norm(root);
        if let Some(prefix) = p.get(..r.len()) {
            if prefix.eq_ignore_ascii_case(&r) {
                return p[r.len()..].trim_start_matches('\\').to_string();
            }
        }
        p.rsplit('\\').next().unwrap_or(&p).to_string()
    }

    fn now_str() -> String {
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn dir_size(p: &str) -> (u64, u64) {
        fsutil::path_size(Path::new(p))
    }

    fn real_target(link: &Path) -> Option<String> {
        let p = std::fs::canonicalize(link).ok()?;
        let s = p.display().to_string();
        Some(strip_extended(&s).to_string())
    }

    // ---------------------------------------------------------------- 控制台解码

    fn decode_utf16le(b: &[u8]) -> String {
        let units: Vec<u16> = b
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c.get(1).copied().unwrap_or(0)]))
            .collect();
        String::from_utf16_lossy(&units)
    }

    fn decode_console(b: &[u8]) -> String {
        match std::str::from_utf8(b) {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(b).into_owned(),
        }
    }

    // ---------------------------------------------------------------- WSL

    fn wsl_running_distros() -> Vec<String> {
        let mut cmd = Command::new("wsl");
        cmd.args(["--list", "--quiet", "--running"]);
        fsutil::no_window(&mut cmd);
        match cmd.output() {
            Ok(o) => decode_utf16le(&o.stdout)
                .split(['\r', '\n'])
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect(),
            Err(_) => vec!["<无法检测WSL状态>".into()],
        }
    }

    fn wsl_shutdown(em: &mut Emitter) {
        em.log("  执行 wsl --shutdown 关闭 WSL（未保存的终端工作会丢失）…", "warn");
        let mut cmd = Command::new("wsl");
        cmd.arg("--shutdown");
        fsutil::no_window(&mut cmd);
        let _ = cmd.output();
        std::thread::sleep(Duration::from_secs(3));
    }

    // ---------------------------------------------------------------- 候选发现层

    const CAND_MIN: u64 = 500 * MB;
    const CAND_ROOTS: [(&str, &str); 3] = [
        (r"%LOCALAPPDATA%", "Local"),
        (r"%APPDATA%", "Roaming"),
        (r"%ProgramData%", "ProgramData"),
    ];

    /// 这些一级目录一律不看（系统组件混居 / 商店应用 / 程序本体等）
    const CAND_DENY: [&str; 26] = [
        "microsoft", "packages", "programs", "windows", "windowsapps", "windowsapp",
        "onedrive", "temp", "tmp", "crashdumps", "d3dscache", "comms", "history",
        "connecteddevicesplatform", "publisher", "virtualstore", "package cache",
        "application data", "local settings", "regid.1991-06.com.microsoft",
        "nvidia corporation", "amd", "intel", "$recycle.bin",
        "system volume information", "clean_c_migrated",
    ];

    fn is_denied(name: &str) -> bool {
        let n = name.to_lowercase();
        CAND_DENY.contains(&n.as_str())
    }

    fn cand_roots() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (pat, tag) in CAND_ROOTS {
            let p = norm(&expand(pat));
            if Path::new(&p).is_dir() && !fsutil::is_reparse(Path::new(&p)) {
                out.push((p, tag.to_string()));
            }
        }
        out
    }

    /// 候选目录在目标盘的文件夹名：cand_<Local|Roaming|ProgramData>_<相对路径>
    fn cand_key(tag: &str, rel: &str) -> String {
        let mut safe = String::new();
        for c in rel.chars() {
            let keep = c.is_ascii_alphanumeric()
                || matches!(c, '.' | '_' | '-')
                || ('\u{4e00}'..='\u{9fff}').contains(&c);
            safe.push(if keep { c } else { '_' });
        }
        let safe = safe.trim_matches('_');
        let safe = if safe.is_empty() { "dir" } else { safe };
        format!("cand_{}_{}", tag, safe)
    }

    fn whitelist_sources() -> Vec<String> {
        defs().iter().map(|d| normcase(&expand(&d.src))).collect()
    }

    fn wl_has(wl: &[String], path: &str) -> bool {
        wl.contains(&normcase(path))
    }

    /// 白名单里有没有条目的源路径等于 path 或位于 path 之下
    fn wl_covers(wl: &[String], path: &str) -> bool {
        let p = normcase(path);
        let prefix = format!("{p}\\");
        wl.iter().any(|w| *w == p || w.starts_with(&prefix))
    }

    /// 列出根下的一级、二级目录：[(绝对路径, 相对路径)]
    fn cand_walk(root: &str) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let Ok(rd) = std::fs::read_dir(root) else {
            return out;
        };
        let mut level1: Vec<(String, String)> = Vec::new();
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if !ft.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if is_denied(&name) {
                continue;
            }
            level1.push((e.path().display().to_string(), name));
        }
        for (p1, n1) in &level1 {
            out.push((p1.clone(), n1.clone()));
            if fsutil::is_reparse(Path::new(p1)) {
                continue; // 一级本身是联接：不再往下（否则认出来的是假路径）
            }
            let Ok(rd2) = std::fs::read_dir(p1) else { continue };
            for e2 in rd2.flatten() {
                let Ok(ft2) = e2.file_type() else { continue };
                if !ft2.is_dir() {
                    continue;
                }
                let n2 = e2.file_name().to_string_lossy().to_string();
                if is_denied(&n2) {
                    continue;
                }
                out.push((e2.path().display().to_string(), format!("{n1}\\{n2}")));
            }
        }
        out
    }

    /// 找出"白名单没覆盖、但体积很大"的目录，按体积降序返回 (体积, 绝对路径, 相对路径, 标签)
    fn scan_candidates(ctl: &TaskCtl, em: &mut Emitter) -> Vec<(u64, String, String, String)> {
        let wl = whitelist_sources();
        let mut out: Vec<(u64, String, String, String)> = Vec::new();
        for (root, tag) in cand_roots() {
            let mut level1: Vec<PathBuf> = match std::fs::read_dir(&root) {
                Ok(rd) => rd
                    .flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.path())
                    .collect(),
                Err(_) => continue,
            };
            level1.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
            for e1 in level1 {
                if ctl.is_cancelled() {
                    out.sort_by(|a, b| b.0.cmp(&a.0));
                    return out;
                }
                let p1 = norm(&e1.display().to_string());
                let n1 = e1
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if is_denied(&n1) || fsutil::is_reparse(&e1) || wl_has(&wl, &p1) {
                    continue;
                }
                em.set_current(&p1);
                em.tick();
                let mut kids: Vec<(u64, String)> = Vec::new();
                let sub: Vec<PathBuf> = std::fs::read_dir(&p1)
                    .map(|rd| {
                        rd.flatten()
                            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                            .map(|e| e.path())
                            .collect()
                    })
                    .unwrap_or_default();
                for e2 in sub {
                    if ctl.is_cancelled() {
                        out.sort_by(|a, b| b.0.cmp(&a.0));
                        return out;
                    }
                    let p2 = norm(&e2.display().to_string());
                    let n2 = e2
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if is_denied(&n2)
                        || fsutil::is_reparse(&e2)
                        || wl_has(&wl, &p2)
                        || wl_covers(&wl, &p2)
                    {
                        continue;
                    }
                    let size = dir_size(&p2).0;
                    if size >= CAND_MIN {
                        kids.push((size, p2));
                    }
                }
                if !kids.is_empty() {
                    for (size, path) in kids {
                        let rel = rel_to(&path, &root);
                        out.push((size, path, rel, tag.clone()));
                    }
                    continue;
                }
                // 二级都不够大：退回一级目录本身；下面若套着白名单项就不能整块搬
                if wl_covers(&wl, &p1) {
                    continue;
                }
                let total = dir_size(&p1).0;
                if total >= CAND_MIN {
                    out.push((total, p1, n1, tag.clone()));
                }
            }
        }
        out.sort_by(|a, b| b.0.cmp(&a.0));
        out
    }

    // ---------------------------------------------------------------- 迁移账本

    const LEDGER_FILE_NAME: &str = "migrations.json";
    const LEDGER_MIRROR_NAME: &str = "clean_c_ledger_backup.json";
    const LEDGER_JOURNAL_NAME: &str = "ledger_journal.jsonl";
    const LEDGER_JOURNAL_MAX: usize = 200;

    fn ledger_file(data_dir: &Path) -> PathBuf {
        data_dir.join(LEDGER_FILE_NAME)
    }

    fn ledger_bak(data_dir: &Path) -> PathBuf {
        data_dir.join(format!("{LEDGER_FILE_NAME}.bak"))
    }

    fn journal_file(data_dir: &Path) -> PathBuf {
        data_dir.join(LEDGER_JOURNAL_NAME)
    }

    /// 读一个账本文件：读不到 / 解析失败 → None；空数组是 Some([])，与"空账本"一致
    fn read_ledger_file(path: &Path) -> Option<Vec<LedgerEntry>> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<Vec<LedgerEntry>>(&text).ok()
    }

    /// 镜像位置 = 数据所在那层的迁移根目录（如 D:\clean_c_migrated）
    fn ledger_mirrors(entries: &[LedgerEntry]) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = Vec::new();
        for e in entries {
            let dst = norm(&e.dst);
            if dst.is_empty() {
                continue;
            }
            let Some(root) = Path::new(&dst).parent() else {
                continue;
            };
            if root.display().to_string().len() < 4 {
                continue; // 只落到盘根（如 "D:\"）的不放
            }
            let p = root.join(LEDGER_MIRROR_NAME);
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    fn find_mirror_ledger() -> Option<Vec<LedgerEntry>> {
        for d in crate::platform::windows::list_fixed_drives() {
            let root = format!("{d}\\");
            let Ok(rd) = std::fs::read_dir(&root) else {
                continue;
            };
            for e in rd.flatten() {
                let sub = e.path();
                if !sub.is_dir() {
                    continue;
                }
                if let Some(data) = read_ledger_file(&sub.join(LEDGER_MIRROR_NAME)) {
                    if !data.is_empty() {
                        return Some(data);
                    }
                }
            }
        }
        None
    }

    fn read_journal_entries(data_dir: &Path) -> Option<Vec<LedgerEntry>> {
        let text = std::fs::read_to_string(journal_file(data_dir)).ok()?;
        for line in text.lines().rev() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // 结尾残缺的半行（写入过程中被强杀）直接跳过，继续往前找
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let Some(arr) = v.get("entries") else { continue };
            if let Ok(list) = serde_json::from_value::<Vec<LedgerEntry>>(arr.clone()) {
                return Some(list);
            }
        }
        None
    }

    fn append_journal(data_dir: &Path, entries: &[LedgerEntry], op: &str) {
        let rec = serde_json::json!({
            "time": now_str(),
            "op": op,
            "count": entries.len(),
            "entries": entries,
        });
        let Ok(line) = serde_json::to_string(&rec) else {
            return;
        };
        let path = journal_file(data_dir);
        use std::io::Write;
        let appended = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| f.write_all(format!("{line}\n").as_bytes()));
        if appended.is_err() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if text.lines().count() > LEDGER_JOURNAL_MAX {
                let _ = std::fs::write(&path, format!("{line}\n"));
            }
        }
    }

    /// 三份写同一份最新内容（主文件 / .bak / 数据旁边镜像），再追加一行操作日志。
    /// 返回主文件是否成功落盘：调用方据此把失败暴露到日志里，
    /// 免得"账本没建起来"这种情况被静默吞掉，无从排查。
    fn save_ledger(data_dir: &Path, entries: &[LedgerEntry], op: &str) -> bool {
        let _ = std::fs::create_dir_all(data_dir);
        let prev = read_ledger_file(&ledger_file(data_dir)).unwrap_or_default();
        let Ok(data) = serde_json::to_string_pretty(entries) else {
            return false;
        };
        // 主文件：写临时文件再 rename，避免写一半被强杀留下坏文件
        let main = ledger_file(data_dir);
        let tmp = data_dir.join(format!("{LEDGER_FILE_NAME}.tmp"));
        let mut ok = false;
        if std::fs::write(&tmp, &data).is_ok() {
            ok = std::fs::rename(&tmp, &main).is_ok();
        }
        let mut targets = vec![ledger_bak(data_dir)];
        let mut all: Vec<LedgerEntry> = entries.to_vec();
        all.extend(prev);
        targets.extend(ledger_mirrors(&all));
        for p in targets {
            let Some(dir) = p.parent() else { continue };
            if !dir.is_dir() {
                continue;
            }
            let tmp2 = PathBuf::from(format!("{}.tmp", p.display()));
            if std::fs::write(&tmp2, &data).is_ok() {
                let _ = std::fs::rename(&tmp2, &p);
            }
        }
        append_journal(data_dir, entries, op);
        ok
    }

    /// 读账本，一路兜底：主文件 → .bak → 日志末条 → 数据旁边镜像；救回后立刻回填主文件
    fn load_ledger(data_dir: &Path) -> Vec<LedgerEntry> {
        let mut recovered = false;
        let mut data = read_ledger_file(&ledger_file(data_dir));
        if data.is_none() {
            data = read_ledger_file(&ledger_bak(data_dir));
            recovered = true;
        }
        if data.is_none() {
            data = read_journal_entries(data_dir);
        }
        if data.is_none() {
            data = find_mirror_ledger();
        }
        match data {
            None => Vec::new(),
            Some(d) => {
                if recovered {
                    save_ledger(data_dir, &d, "recover");
                }
                d
            }
        }
    }

    fn ledger_has(data_dir: &Path, src: &str) -> bool {
        let n = normcase(src);
        load_ledger(data_dir).iter().any(|e| normcase(&e.src) == n)
    }

    /// 缺备份就先补一份（只在缺的时候写一次）
    fn ensure_ledger_backups(data_dir: &Path) {
        let entries = load_ledger(data_dir);
        if entries.is_empty() {
            return;
        }
        let bak = ledger_bak(data_dir);
        let mirrors = ledger_mirrors(&entries);
        if !bak.is_file() || mirrors.iter().any(|p| !p.is_file()) {
            save_ledger(data_dir, &entries, "backup-init");
        }
    }

    /// 按 C 盘原位的联接反推账本。返回 (找回条数, 原有条数)
    fn rebuild_ledger(data_dir: &Path) -> (usize, usize) {
        let mut entries = load_ledger(data_dir);
        let before = entries.len();
        let mut known: Vec<String> = entries.iter().map(|e| normcase(&e.src)).collect();

        for d in defs() {
            let src = norm(&expand(&d.src));
            if known.contains(&normcase(&src)) {
                continue;
            }
            let p = Path::new(&src);
            if !p.is_dir() || !fsutil::is_reparse(p) {
                continue;
            }
            let Some(target) = real_target(p) else { continue };
            if !Path::new(&target).is_dir() {
                continue;
            }
            let (size, _) = dir_size(&target);
            entries.push(LedgerEntry {
                key: d.key.clone(),
                name: d.name.clone(),
                src: src.clone(),
                dst: target,
                bytes: size,
                time: now_str(),
                candidate: false,
            });
            known.push(normcase(&src));
        }

        // 候选 / 白名单之外的目录：只认"指向别的盘"的联接（同盘兼容性联接是假的）
        for (root, tag) in cand_roots() {
            for (path, rel) in cand_walk(&root) {
                if known.contains(&normcase(&path)) {
                    continue;
                }
                let p = Path::new(&path);
                if !fsutil::is_reparse(p) {
                    continue;
                }
                let Some(target) = real_target(p) else { continue };
                if !Path::new(&target).is_dir() {
                    continue;
                }
                if drive_of(&target) == drive_of(&path) {
                    continue;
                }
                let (size, _) = dir_size(&target);
                entries.push(LedgerEntry {
                    key: cand_key(&tag, &rel),
                    name: rel.clone(),
                    src: path.clone(),
                    dst: target,
                    bytes: size,
                    time: now_str(),
                    candidate: true,
                });
                known.push(normcase(&path));
            }
        }

        let added = entries.len() - before;
        if added != 0 {
            save_ledger(data_dir, &entries, "rebuild");
        }
        (added, before)
    }

    // ---------------------------------------------------------------- 子进程：robocopy / mklink

    /// robocopy 返回码的含义：0-7 都算成功（>=8 才是真失败）
    fn rc_hint(rc: i32) -> &'static str {
        match rc {
            0 => "（无变化）",
            1 => "（复制成功）",
            2 => "（有额外文件/目录）",
            3 => "（复制成功 + 有额外文件）",
            4..=7 => "（成功 + 有额外/不匹配项）",
            8 => "（有文件复制失败）",
            9..=15 => "（部分失败）",
            16 => "（严重错误：多半是权限不足或路径不可访问）",
            112 => "（目标盘空间不足）",
            _ => "",
        }
    }

    /// 搬运目录。move_files=true 用 /MOVE（搬完即删源）。返回 (返回码, 日志首页)。
    /// 轮询 try_wait 上报进度；被取消时 kill 子进程（随后由调用方安全回搬）。
    fn robocopy(
        src: &str,
        dst: &str,
        move_files: bool,
        total: u64,
        ctl: &TaskCtl,
        em: &mut Emitter,
    ) -> (i32, String) {
        let mut args: Vec<&str> = vec![src, dst, "/E", "/R:2", "/W:1", "/NFL", "/NDL", "/NJH", "/NP"];
        if move_files {
            args.push("/MOVE");
        }
        // 把实际执行的命令行记下来：排查"搬到一半又回退"时，先要确认两边路径到底是什么
        em.log(
            format!(
                "  robocopy {} -> {}{}",
                src,
                dst,
                if move_files { " (/MOVE)" } else { "" }
            ),
            "info",
        );
        let mut cmd = Command::new("robocopy");
        cmd.args(&args);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        fsutil::no_window(&mut cmd);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return (16, format!("robocopy 启动失败: {e}")),
        };
        // 输出必须被读走，否则管道写满会把子进程卡住
        let stdout = child.stdout.take();
        let reader = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut o) = stdout {
                let _ = o.read_to_end(&mut buf);
            }
            buf
        });
        if total > 0 {
            em.set_bytes_total(total);
            em.flush();
        }
        let mut rc: i32 = 16;
        let mut killed = false;
        loop {
            match child.try_wait() {
                Ok(Some(st)) => {
                    rc = st.code().unwrap_or(0);
                    break;
                }
                Ok(None) => {
                    if ctl.is_cancelled() {
                        let _ = child.kill();
                        let _ = child.wait();
                        killed = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(800));
                    if total > 0 {
                        let (b, f) = dir_size(dst);
                        em.set_discovered(b.min(total), f);
                    }
                }
                Err(_) => break,
            }
        }
        let out_b = reader.join().unwrap_or_default();
        if killed {
            return (16, "已停止".into());
        }
        if total > 0 {
            em.set_discovered(total, 0);
        }
        let text = decode_console(&out_b);
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        // 头尾都要看：robocopy 的失败原因基本都在最前面几行（「拒绝访问」「磁盘空间不足」
        // 之类），只截尾巴会正好把原因漏掉，只留一堆 "New Dir" 之类的统计行。
        let head = lines.iter().take(4).cloned().collect::<Vec<_>>().join(" | ");
        let tail = lines
            .iter()
            .rev()
            .take(3)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ");
        em.log(
            format!(
                "  robocopy 返回码 {rc}{} {}",
                rc_hint(rc),
                if head.is_empty() { &tail } else { &head }
            ),
            if rc >= 8 { "error" } else { "info" },
        );
        (rc, if head.is_empty() { tail } else { head })
    }

    /// 安全回退：把已经搬到 dst 的数据搬回 src
    fn move_back(dst: &str, src: &str, ctl: &TaskCtl, em: &mut Emitter) -> bool {
        let total = dir_size(dst).0;
        let (rc, _) = robocopy(dst, src, true, total, ctl, em);
        if rc >= 8 {
            em.log(
                format!(
                    "  !!! 回搬也失败（返回码 {rc}）。两侧都可能有数据，请人工核对:\n      源: {src}\n      目标: {dst}"
                ),
                "error",
            );
            return false;
        }
        em.log("  已把数据搬回源位置", "ok");
        // robocopy /MOVE 只清空目标目录、不会删掉目录本身。留着它的话，下次再迁这条会被
        // "目标路径已存在" 直接挡掉 —— 等于这条永远搬不动，账本自然一直建不起来。
        if let Err(e) = std::fs::remove_dir(dst) {
            em.log(
                format!("  目标空目录 {dst} 未能删除({e})，下次迁移前请手动删除"),
                "warn",
            );
        }
        true
    }

    /// 目标路径所在磁盘的可用空间。空间不够就没必要"先搬一半再回退" ——
    /// 那正是「搬到一半又回退、来回循环」最容易出现的一种场景，提前判掉最省事。
    /// 直接用 kernel32!GetDiskFreeSpaceExW，不引新依赖。
    fn drive_free_space(path: &str) -> Option<u64> {
        use std::os::windows::ffi::OsStrExt;

        #[link(name = "kernel32")]
        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpdirectoryname: *const u16,
                lpfreebytesavailabletocaller: *mut u64,
                lptotalnumberofbytes: *mut u64,
                lptotalnumberoffreebytes: *mut u64,
            ) -> i32;
        }

        let wide: Vec<u16> = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let (mut avail, mut total, mut free) = (0u64, 0u64, 0u64);
        let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), &mut avail, &mut total, &mut free) };
        if ok == 0 {
            None
        } else {
            Some(free)
        }
    }

    /// 建 junction（cmd 自行解析整行，故显式加引号）。
    /// 失败时把 cmd 实际吐出来的话带回去 —— 只报一句"建联接失败"的话，
    /// 日志里就看不出到底是权限不够还是路径有问题，等于没法排查。
    fn mklink(src: &str, dst: &str) -> Result<(), String> {
        let line = format!("mklink /J \"{src}\" \"{dst}\"");
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", &line]);
        fsutil::no_window(&mut cmd);
        let out = cmd
            .output()
            .map_err(|e| format!("无法启动 cmd 执行 mklink: {e}"))?;
        let msg = decode_console(&out.stdout).trim().to_string();
        if out.status.success() && fsutil::is_reparse(Path::new(src)) && Path::new(src).exists() {
            return Ok(());
        }
        Err(format!(
            "mklink 退出码 {:?}, 输出: {}",
            out.status.code(),
            if msg.is_empty() { "<空>" } else { &msg }
        ))
    }

    // ---------------------------------------------------------------- 搬运核心

    /// 把 src 目录迁移到 dst 并建 junction。成功返回记账条目。
    fn migrate_core(
        em: &mut Emitter,
        data_dir: &Path,
        src_in: &str,
        dst_in: &str,
        name: &str,
        procs: &[String],
        key: &str,
        cand: bool,
    ) -> Result<LedgerEntry, String> {
        let ctl = em.ctl.clone();
        em.log(format!("== 迁移: {name}"), "info");
        let src = norm(&expand(src_in));
        let dst = norm(dst_in);
        if !Path::new(&src).is_dir() {
            return Err(format!("目录不存在: {src}"));
        }
        if fsutil::is_reparse(Path::new(&src)) {
            return Err("该位置已经是联接(junction), 无需迁移".into());
        }
        if ledger_has(data_dir, &src) {
            return Err("账本里已有这条迁移记录, 请先还原或忽略".into());
        }

        em.set_phase("统计", name);
        // 统计阶段只滚动"当前文件"，不推字节进度：这一段和随后的搬运覆盖的是同一段字节，
        // 两边都推的话搬运一启动就会把刚走过的进度打回去（看起来像重新开始）。
        let st = {
            let mut tick = |_b: u64, _f: u64, p: &Path| {
                em.set_current(p);
                em.tick();
            };
            fsutil::count_tree(Path::new(&src), &ctl, &mut tick)
        };
        if st.cancelled {
            return Err("已停止".into());
        }
        let size = st.bytes;
        em.log(
            format!("  体积 {} / {} 个文件", fsutil::fmt_size(size), st.files),
            "info",
        );
        if let Some(n) = st.nested_reparse {
            return Err(format!(
                "源目录里还套着联接（多半是已迁移的子目录）:\n  {}\n为避免数据被重复搬运, 请改用更精确的目录, 或先把子项还原。",
                n.display()
            ));
        }

        // 1. 进程检测：在运行就跳过，绝不强杀
        let running = fsutil::running_processes(procs);
        if !running.is_empty() {
            return Err(format!(
                "检测到相关程序在运行: {}\n请先退出程序再迁移(本工具不会强杀进程)",
                running.join(", ")
            ));
        }

        // 2. 目标体检
        if Path::new(&dst).exists() {
            return Err(format!("目标路径已存在: {dst}"));
        }
        em.log(format!("  计划: {src}\n     -> {dst}"), "info");
        em.set_current(format!("{src} → {dst}"));
        // 空间不够就提前停手：否则会搬一部分、失败、再回退，来回折腾还可能留下两侧都有数据的
        // 中间状态。这里判掉，源目录一动没动。
        if let Some(free) = drive_free_space(&dst) {
            em.log(format!("  目标盘剩余空间 {}", fsutil::fmt_size(free)), "info");
            if free < size {
                return Err(format!(
                    "目标盘空间不足: 需要 {}, 剩余 {}（未搬动任何数据）",
                    fsutil::fmt_size(size),
                    fsutil::fmt_size(free)
                ));
            }
        }
        std::fs::create_dir_all(&dst).map_err(|e| format!("创建目标目录失败: {e}"))?;

        // 3. robocopy + 源清零校验（最多 3 轮）
        em.set_can_pause(false);
        let mut leftover: Vec<String> = Vec::new();
        let mut emptied = false;
        for attempt in 1..=3 {
            let (rc, _) = robocopy(&src, &dst, true, size, &ctl, em);
            if rc >= 8 {
                em.log("  robocopy 失败, 开始安全回退…", "warn");
                move_back(&dst, &src, &ctl, em);
                em.set_can_pause(true);
                return Err(format!("robocopy 失败(返回码 {rc}), 数据已回搬, 未做任何删除"));
            }
            leftover = list_names(&src);
            if leftover.is_empty() {
                emptied = true;
                break;
            }
            if ctl.is_cancelled() {
                move_back(&dst, &src, &ctl, em);
                em.set_can_pause(true);
                return Err("已停止, 数据已回搬".into());
            }
            em.log(
                format!("  第 {attempt} 轮: 源目录还有 {} 个条目(可能被占用), 重试…", leftover.len()),
                "warn",
            );
        }
        if !emptied {
            let names = leftover.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
            em.log(format!("  源目录未搬空(剩余示例: {names}), 开始安全回退…"), "warn");
            move_back(&dst, &src, &ctl, em);
            em.set_can_pause(true);
            return Err("源目录有文件被占用未搬空, 已回退, 未建联接".into());
        }

        // 4. 源已空 -> 删空目录 -> 建联接
        if Path::new(&src).is_dir() {
            if let Err(e) = std::fs::remove_dir(&src) {
                move_back(&dst, &src, &ctl, em);
                em.set_can_pause(true);
                return Err(format!("删除空源目录失败: {e}"));
            }
        }
        if let Err(e) = mklink(&src, &dst) {
            em.log(
                format!("  建联接失败({e}), 把数据搬回源位置…"),
                "warn",
            );
            move_back(&dst, &src, &ctl, em);
            em.set_can_pause(true);
            return Err(format!("创建 junction 失败: {e}；（数据已搬回原位置）"));
        }
        em.set_can_pause(true);

        // 5. 校验 + 记账
        let (_, files_now) = dir_size(&dst);
        em.log(format!("  完成: junction 已建立并校验({files_now} 个文件), 已记入账本"), "ok");
        let entry = LedgerEntry {
            key: key.to_string(),
            name: name.to_string(),
            src: src.clone(),
            dst: dst.clone(),
            bytes: size,
            time: now_str(),
            candidate: cand,
        };
        let mut entries = load_ledger(data_dir);
        entries.push(entry.clone());
        if !save_ledger(data_dir, &entries, "migrate") {
            em.log(
                "  ! 账本主文件写入失败, 本条记录可能丢失, 请检查磁盘空间与权限",
                "error",
            );
        }
        Ok(entry)
    }

    /// 一键还原：删联接 -> 把数据从目标盘复制回 C 盘 -> 校验 -> 删目标盘数据
    fn restore_core(em: &mut Emitter, data_dir: &Path, entry: &LedgerEntry) -> Result<(), String> {
        let ctl = em.ctl.clone();
        let src = norm(&entry.src);
        let dst = norm(&entry.dst);
        em.log(format!("== 还原: {}", entry.name), "info");
        if !Path::new(&dst).is_dir() {
            return Err(format!("目标数据目录丢失({dst}), 中止, 原联接未动"));
        }
        if !fsutil::is_reparse(Path::new(&src)) {
            return Err("源位置不是联接(可能被手动改过), 拒绝自动还原, 请人工确认".into());
        }
        let def = def_by_key(&entry.key).or_else(|| def_by_src(&src));
        let is_wsl = def.as_ref().map(|d| d.wsl).unwrap_or(false);
        if is_wsl {
            if !wsl_running_distros().is_empty() {
                wsl_shutdown(em);
            }
        } else {
            let procs = def.as_ref().map(|d| d.procs.clone()).unwrap_or_default();
            let running = fsutil::running_processes(&procs);
            if !running.is_empty() {
                return Err(format!("检测到相关程序在运行: {}, 请先退出", running.join(", ")));
            }
            if entry.candidate {
                em.log("  注意: 这是未验证候选, 无法检测其程序是否退出, 请自行确认已退出", "warn");
            }
        }

        let (size, _) = dir_size(&dst);
        let free_c = crate::platform::windows::drive_space("C")
            .map(|(f, _)| f)
            .unwrap_or(0);
        let need = (size as f64 * 1.15) as u64 + 256 * MB;
        if free_c < need {
            return Err(format!(
                "C 盘空间不足(需要约 {}), 请先清理再还原",
                fsutil::fmt_size(need)
            ));
        }

        // 1. 删联接（只删联接本身，不碰目标盘数据）
        fsutil::remove_link(Path::new(&src));
        if fsutil::is_reparse(Path::new(&src)) {
            return Err("删除联接失败, 中止, 未做任何改动".into());
        }
        std::fs::create_dir_all(&src).map_err(|e| format!("创建源目录失败: {e}"))?;

        // 2. 复制回 C 盘
        em.set_phase("还原", format!("{dst} → {src}"));
        em.set_current(format!("{dst} → {src}"));
        em.set_can_pause(false);
        let (rc, _) = robocopy(&dst, &src, false, size, &ctl, em);
        em.set_can_pause(true);
        if rc >= 8 {
            return Err(format!(
                "复制回 C 盘失败, 两侧数据都保留:\n  源(不完整副本): {src}\n  完整数据仍在: {dst}\n可删除源处不完整副本后重试还原"
            ));
        }

        // 3. 文件数一致性校验
        let (_, f_src) = dir_size(&src);
        let (_, f_dst) = dir_size(&dst);
        if f_src != f_dst {
            return Err(format!(
                "文件数不一致(源 {f_src} / 目标 {f_dst}), 两侧数据都保留, 账本记录未删, 可重试"
            ));
        }

        // 4. 删目标盘副本（逐文件进度、可中断）
        em.set_phase("清理", "删除目标盘副本");
        let mut files = 0u64;
        let mut bytes = 0u64;
        if !crate::clean::remove_recursive(Path::new(&dst), &ctl, em, &mut files, &mut bytes) {
            return Err(format!(
                "目标目录删除失败({dst}), 数据已回到 C 盘, 可稍后手动删除"
            ));
        }

        let entries: Vec<LedgerEntry> = load_ledger(data_dir)
            .into_iter()
            .filter(|e| normcase(&e.src) != normcase(&src))
            .collect();
        save_ledger(data_dir, &entries, "restore");
        em.log("  还原完成: 数据已回到 C 盘, 目标盘副本已删除", "ok");
        Ok(())
    }

    /// 把一条已迁移记录搬到 new_root 下。同盘 = 瞬间改名；跨盘 = robocopy。失败自动回滚。
    fn reorganize_core(
        em: &mut Emitter,
        data_dir: &Path,
        entry: &LedgerEntry,
        new_root: &str,
    ) -> Result<LedgerEntry, String> {
        let ctl = em.ctl.clone();
        let src = norm(&entry.src);
        let old_dst = norm(&entry.dst);
        let base = Path::new(&old_dst)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_dst = norm(&format!("{}\\{}", new_root.trim_end_matches('\\'), base));
        if normcase(&old_dst) == normcase(&new_dst) {
            return Err("已在该目录下".into());
        }
        if Path::new(&new_dst).exists() {
            return Err(format!("目标已存在: {new_dst}"));
        }
        if entry.candidate {
            return Err(
                "未验证候选(无法确认程序是否退出), 已跳过; 要换目录请先还原再重新迁移".into(),
            );
        }
        let def = def_by_key(&entry.key).or_else(|| def_by_src(&src));
        if let Some(d) = &def {
            if d.wsl {
                wsl_shutdown(em);
            }
            let running = fsutil::running_processes(&d.procs);
            if !running.is_empty() {
                let names: Vec<String> = running.iter().take(2).cloned().collect();
                return Err(format!("程序运行中({}), 跳过", names.join(", ")));
            }
        }
        em.log(format!("整理: {}", entry.name), "info");
        em.log(format!("  {old_dst}"), "info");
        em.log(format!("  → {new_dst}"), "info");
        em.set_current(format!("{old_dst} → {new_dst}"));

        // 1. 删旧联接（只删联接本身）
        fsutil::remove_link(Path::new(&src));
        if fsutil::is_reparse(Path::new(&src)) {
            return Err("删旧联接失败".into());
        }
        // 2. 搬数据
        if let Some(par) = Path::new(&new_dst).parent() {
            let _ = std::fs::create_dir_all(par);
        }
        let moved: Result<(), String> = if same_volume(&old_dst, &new_dst) {
            std::fs::rename(&old_dst, &new_dst)
                .map(|_| em.log("  同盘改名, 瞬间完成", "info"))
                .map_err(|e| e.to_string())
        } else {
            let size = dir_size(&old_dst).0;
            em.set_can_pause(false);
            let (rc, _) = robocopy(&old_dst, &new_dst, true, size, &ctl, em);
            em.set_can_pause(true);
            if rc >= 8 {
                Err(format!("robocopy 返回码 {rc}"))
            } else {
                Ok(())
            }
        };
        if let Err(e) = moved {
            em.log(format!("  搬运失败({e}), 重建旧联接恢复原状…"), "warn");
            if let Err(le) = mklink(&src, &old_dst) {
                em.log(format!("  !!! 旧联接也重建失败: {le}"), "error");
            }
            return Err("搬运失败, 已恢复原状".into());
        }
        // 3. 建新联接
        if let Err(e) = mklink(&src, &new_dst) {
            em.log(format!("  建新联接失败({e}), 数据搬回老位置…"), "warn");
            let size = dir_size(&new_dst).0;
            let (rc, _) = robocopy(&new_dst, &old_dst, true, size, &ctl, em);
            if rc >= 8 {
                em.log(
                    format!(
                        "  !!! 回搬失败, 两侧都可能有数据, 请人工核对:\n      老: {old_dst}\n      新: {new_dst}"
                    ),
                    "error",
                );
            } else if let Err(le) = mklink(&src, &old_dst) {
                em.log(format!("  !!! 旧联接重建失败: {le}"), "error");
            }
            return Err(format!("创建 junction 失败({e}), 数据已搬回老位置"));
        }
        // 4. 更新账本
        let new_entry = LedgerEntry {
            dst: new_dst.clone(),
            time: now_str(),
            ..entry.clone()
        };
        let mut entries = load_ledger(data_dir);
        let k = normcase(&src);
        for e in entries.iter_mut() {
            if normcase(&e.src) == k {
                *e = new_entry.clone();
                break;
            }
        }
        save_ledger(data_dir, &entries, "reorganize");
        em.log("  完成: 联接已指向新位置, 账本已更新", "ok");

        // 5. 老根目录空了就顺手删掉
        if let Some(old_root) = Path::new(&old_dst).parent() {
            let ors = old_root.display().to_string();
            if normcase(&ors) != normcase(&new_root.trim_end_matches('\\').to_string())
                && old_root.is_dir()
                && list_names(&ors).is_empty()
            {
                if std::fs::remove_dir(old_root).is_ok() {
                    em.log(format!("  老目录已空, 已删除: {ors}"), "info");
                }
            }
        }
        Ok(new_entry)
    }

    // ---------------------------------------------------------------- 后台任务

    pub(crate) fn list_items() -> Vec<MigrateItem> {
        defs()
            .into_iter()
            .map(|d| {
                let src = expand(&d.src);
                MigrateItem {
                    key: d.key,
                    name: d.name,
                    src,
                    category: d.cat,
                    procs: d.procs,
                    subdir: String::new(),
                    desc: d.desc,
                }
            })
            .collect()
    }

    pub(crate) fn ledger_entries(data_dir: &Path) -> Vec<LedgerEntry> {
        ensure_ledger_backups(data_dir);
        load_ledger(data_dir)
    }

    pub(crate) fn run_scan(
        app: AppHandle,
        ctl: TaskCtl,
        data_dir: PathBuf,
        include_candidates: bool,
    ) {
        std::thread::spawn(move || {
            let mut em = Emitter::new(app, ctl.clone(), "扫描可迁移目录");
            ensure_ledger_backups(&data_dir);
            let ledger = load_ledger(&data_dir);
            let known: Vec<String> = ledger.iter().map(|e| normcase(&e.src)).collect();
            let defs = defs();
            em.set_can_pause(true);
            em.set_phase("扫描", "扫描白名单目录");
            em.set_items_total(defs.len() as u64);
            let mut total = 0u64;

            for d in &defs {
                if ctl.wait_gate() {
                    break;
                }
                let src = norm(&expand(&d.src));
                em.set_current(&src);
                em.tick();
                let p = Path::new(&src);
                // 未安装/不存在：前端没有对应状态，不列（避免"可迁移 0 B"的误导行）
                if !p.is_dir() {
                    em.inc_items();
                    continue;
                }
                let migrated = fsutil::is_reparse(p) || known.contains(&normcase(&src));
                if migrated {
                    em.row(
                        "migrate_rows",
                        MigrateRow {
                            key: d.key.clone(),
                            name: d.name.clone(),
                            src: src.clone(),
                            category: d.cat.clone(),
                            desc: d.desc.clone(),
                            bytes: 0,
                            files: 0,
                            running: Vec::new(),
                            candidate: false,
                            migrated: true,
                        },
                    );
                    total += 1;
                    em.inc_items();
                    continue;
                }
                let running = if d.wsl {
                    wsl_running_distros()
                } else {
                    fsutil::running_processes(&d.procs)
                };
                let st = {
                    let mut tick = |b: u64, f: u64, _p: &Path| em.set_discovered(b, f);
                    fsutil::count_tree(p, &ctl, &mut tick)
                };
                if st.cancelled {
                    break;
                }
                em.row(
                    "migrate_rows",
                    MigrateRow {
                        key: d.key.clone(),
                        name: d.name.clone(),
                        src: src.clone(),
                        category: d.cat.clone(),
                        desc: d.desc.clone(),
                        bytes: st.bytes,
                        files: st.files,
                        running,
                        candidate: false,
                        migrated: false,
                    },
                );
                total += 1;
                em.inc_items();
            }

            if include_candidates && !ctl.is_cancelled() {
                em.set_phase("发现候选", "发现未验证候选目录(要统计体积, 可能较慢)");
                em.set_items_total(0);
                em.set_current("");
                let cands = scan_candidates(&ctl, &mut em);
                let mut seen: Vec<String> = defs.iter().map(|d| d.key.clone()).collect();
                for (size, path, rel, tag) in cands {
                    if ctl.is_cancelled() {
                        break;
                    }
                    let key = cand_key(&tag, &rel);
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key.clone());
                    em.row(
                        "migrate_rows",
                        MigrateRow {
                            key,
                            name: rel.clone(),
                            src: norm(&path),
                            category: "候选".into(),
                            desc: "未验证候选 · 迁移前请自行确认已退出该软件".into(),
                            bytes: size,
                            files: 0,
                            running: Vec::new(),
                            candidate: true,
                            migrated: false,
                        },
                    );
                    total += 1;
                }
            }

            let stopped = ctl.is_cancelled();
            em.rows_done("migrate_rows", total);
            em.finish(
                total,
                0,
                0,
                0,
                stopped,
                format!("扫描完成: {total} 个可迁移目录"),
                None,
            );
        });
    }

    pub(crate) fn run_migration(
        app: AppHandle,
        ctl: TaskCtl,
        data_dir: PathBuf,
        rows: Vec<MigrateRow>,
        drive: String,
        root_name: String,
    ) {
        std::thread::spawn(move || {
            let mut em = Emitter::new(app, ctl.clone(), "高级迁移");
            em.set_can_pause(true);
            em.set_phase("迁移", "开始迁移");
            em.set_items_total(rows.len() as u64);
            // 整批总量一次性定死并锁定：搬运单元内部再调 set_bytes_total 也不会改写它。
            // 进度条因此只在整批范围内单调前进，不会每搬一项就被打回原点。
            em.set_bytes_total(rows.iter().map(|r| r.bytes).sum());
            em.lock_bytes_total();
            let defs = defs();
            let mut ok = 0u64;
            let mut failed = 0u64;
            let mut bytes = 0u64;
            let mut done_bytes = 0u64;
            // 逐条留痕：迁移失败时用户看到的就是界面上一行 warn，光靠它没法判断到底是
            // 权限、空间、被占用还是联接建不起来。这里把每条的结果攒成一份报告落盘，
            // 出问题可以直接把文件发回来。
            let mut failures: Vec<String> = Vec::new();
            let mut report: Vec<String> = vec![
                "唐三浪清理 · 高级迁移报告".into(),
                format!("时间: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
                format!("目标盘: {}  目标根目录: {}", drive, root_name),
                format!("整批条目: {} 项, 计划搬运 {}", rows.len(), fsutil::fmt_size(rows.iter().map(|r| r.bytes).sum::<u64>())),
                format!("当前是否管理员: {}", if crate::platform::is_admin() { "是" } else { "否（部分系统目录会因权限不足失败）" }),
                String::new(),
            ];

            for row in &rows {
                if ctl.wait_gate() {
                    break;
                }
                em.set_current(&row.src);
                em.set_bytes_base(done_bytes);
                let def = defs.iter().find(|d| d.key == row.key);
                if def.map(|d| d.wsl).unwrap_or(false) {
                    wsl_shutdown(&mut em);
                }
                let procs: Vec<String> = def.map(|d| d.procs.clone()).unwrap_or_default();
                let dst = target_path(&drive, &root_name, &row.key);
                match migrate_core(
                    &mut em,
                    &data_dir,
                    &row.src,
                    &dst,
                    &row.name,
                    &procs,
                    &row.key,
                    row.candidate,
                ) {
                    Ok(entry) => {
                        ok += 1;
                        bytes += entry.bytes;
                        done_bytes += entry.bytes;
                        em.row("migrate_ledger", entry);
                        report.push(format!(
                            "[成功] {} {} -> {} ({})",
                            row.name,
                            row.src,
                            dst,
                            fsutil::fmt_size(row.bytes)
                        ));
                    }
                    Err(msg) => {
                        failed += 1;
                        em.log(format!("{} 迁移失败: {msg}", row.name), "warn");
                        failures.push(format!("{}（{}）: {msg}", row.name, row.src));
                        report.push(format!("[失败] {} {} -> {}\n       原因: {msg}", row.name, row.src, dst));
                    }
                }
                em.inc_items();
            }

            // 失败明细在结束时兜底再打一遍：中间那些 warn 会被后续日志刷走，
            // 用户回头找原因时往往只看到最后一行。
            if !failures.is_empty() {
                em.log(format!("—— 失败条目共 {} 项 ——", failures.len()), "error");
                for f in &failures {
                    em.log(format!("  {f}"), "error");
                }
            }

            let stopped = ctl.is_cancelled();
            report.push(String::new());
            report.push(format!("结果: 成功 {ok}, 失败 {failed}, 搬运 {}", fsutil::fmt_size(bytes)));
            if stopped {
                report.push("（任务被用户中断，未处理完的条目不在统计内）".into());
            }
            let report_path = fsutil::write_report(&data_dir, "迁移报告", &report);
            if let Some(p) = &report_path {
                em.log(format!("本次迁移报告: {p}"), "info");
            }
            em.finish(
                ok,
                0,
                failed,
                bytes,
                stopped,
                format!("迁移完成: 成功 {ok}, 失败 {failed}"),
                report_path,
            );
        });
    }

    pub(crate) fn run_restore(
        app: AppHandle,
        ctl: TaskCtl,
        data_dir: PathBuf,
        srcs: Vec<String>,
    ) {
        std::thread::spawn(move || {
            let mut em = Emitter::new(app, ctl.clone(), "还原迁移数据");
            em.set_can_pause(true);
            em.set_phase("还原", "准备还原");
            let todo: Vec<LedgerEntry> = load_ledger(&data_dir)
                .into_iter()
                .filter(|e| srcs.iter().any(|s| normcase(s) == normcase(&e.src)))
                .collect();
            em.set_items_total(todo.len() as u64);
            let mut ok = 0u64;
            let mut failed = 0u64;
            let mut bytes = 0u64;

            for e in &todo {
                if ctl.wait_gate() {
                    break;
                }
                match restore_core(&mut em, &data_dir, e) {
                    Ok(()) => {
                        ok += 1;
                        bytes += e.bytes;
                    }
                    Err(msg) => {
                        failed += 1;
                        em.log(msg, "warn");
                    }
                }
                em.inc_items();
            }

            let stopped = ctl.is_cancelled();
            for e in load_ledger(&data_dir) {
                em.row("migrate_ledger", e);
            }
            em.rows_done("migrate_ledger", 0);
            em.finish(
                ok,
                0,
                failed,
                bytes,
                stopped,
                format!("还原完成: 成功 {ok}, 失败 {failed}"),
                None,
            );
        });
    }

    pub(crate) fn run_reorganize(
        app: AppHandle,
        ctl: TaskCtl,
        data_dir: PathBuf,
        drive: String,
        root_name: String,
    ) {
        std::thread::spawn(move || {
            let mut em = Emitter::new(app, ctl.clone(), "整理迁移数据");
            em.set_can_pause(true);
            let new_root = target_root(&drive, &root_name);
            em.set_phase("整理", format!("归拢到 {new_root}"));
            let todo: Vec<LedgerEntry> = load_ledger(&data_dir)
                .into_iter()
                .filter(|e| {
                    let cur = Path::new(&norm(&e.dst))
                        .parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    normcase(&cur) != normcase(&new_root)
                })
                .collect();
            em.set_items_total(todo.len() as u64);
            let mut ok = 0u64;
            let mut skipped = 0u64;

            for e in &todo {
                if ctl.wait_gate() {
                    break;
                }
                match reorganize_core(&mut em, &data_dir, e, &new_root) {
                    Ok(ne) => {
                        ok += 1;
                        em.row("migrate_ledger", ne);
                    }
                    Err(msg) => {
                        skipped += 1;
                        em.log(format!("  {}: {msg}", e.name), "warn");
                    }
                }
                em.inc_items();
            }

            let stopped = ctl.is_cancelled();
            for e in load_ledger(&data_dir) {
                em.row("migrate_ledger", e);
            }
            em.rows_done("migrate_ledger", 0);
            em.finish(
                ok,
                skipped,
                0,
                0,
                stopped,
                format!("整理完成: 成功 {ok}, 跳过 {skipped}"),
                None,
            );
        });
    }

    pub(crate) fn run_rebuild(app: AppHandle, ctl: TaskCtl, data_dir: PathBuf) {
        std::thread::spawn(move || {
            let mut em = Emitter::new(app, ctl.clone(), "重建迁移账本");
            em.set_can_pause(true);
            em.set_phase("重建", "按 C 盘原位联接反推账本");
            let (added, kept) = rebuild_ledger(&data_dir);
            for e in load_ledger(&data_dir) {
                em.row("migrate_ledger", e);
            }
            em.rows_done("migrate_ledger", 0);
            em.finish(
                added as u64,
                kept as u64,
                0,
                0,
                false,
                format!("账本重建完成: 找回 {added} 条, 原有 {kept} 条"),
                None,
            );
        });
    }
}
