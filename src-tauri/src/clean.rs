// 清理引擎：白名单（Windows / macOS 各一份）+ 扫描 + 执行
//
// 关键修复：不再"先整树统计、再一次性 rmtree"。
//   * 扫描阶段：逐个目标统计体积，全程节流上报（有反馈、可中断）
//   * 执行阶段：直接删除时逐文件删（文件级进度）；回收站/废纸篓按顶层条目处理
//   * 任何时刻都能暂停/停止
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::fsutil::{self, TreeStat};
use crate::state::{AppState, TaskCtl};
use crate::task::Emitter;

// ---------------------------------------------------------------- 白名单条目定义

pub struct CleanItem {
    pub key: &'static str,
    pub name: &'static str,
    /// green = 一定安全（默认勾选）；yellow = 建议先关闭对应软件
    pub level: &'static str,
    pub desc: &'static str,
    pub admin: bool,
    pub category: &'static str,
    pub paths: &'static [&'static str],
    /// 特殊条目：postman_old（保留最新版，清其余）
    pub special: Option<&'static str>,
    /// 该条目只能直接删除（例如废纸篓不能"移进废纸篓"）
    pub force_permanent: bool,
}

macro_rules! item {
    ($key:literal, $name:literal, $level:literal, $cat:literal, $admin:literal, $desc:literal, $paths:expr) => {
        CleanItem {
            key: $key,
            name: $name,
            level: $level,
            desc: $desc,
            admin: $admin,
            category: $cat,
            paths: $paths,
            special: None,
            force_permanent: false,
        }
    };
}

// ---------------------------------------------------------------- Windows 白名单

#[cfg(windows)]
static ITEMS: &[CleanItem] = &[
    item!("temp", "用户临时文件", "green", "系统", false, "未被占用的临时文件, 占用中的自动跳过", &[r"%LOCALAPPDATA%\Temp"]),
    item!("wtemp", "系统临时文件", "green", "系统", true, "系统级临时文件, 需要管理员权限", &[r"C:\Windows\Temp"]),
    item!("wer", "Windows 错误报告", "green", "系统", false, "程序崩溃报告存档", &[
        r"%LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive",
        r"%LOCALAPPDATA%\Microsoft\Windows\WER\ReportQueue"]),
    item!("thumb", "缩略图/图标缓存", "green", "系统", false, "资源管理器自动重建; 被占用的个别文件跳过", &[
        r"%LOCALAPPDATA%\Microsoft\Windows\Explorer\thumbcache_*.db",
        r"%LOCALAPPDATA%\Microsoft\Windows\Explorer\iconcache_*.db"]),
    item!("crashdumps", "程序崩溃转储", "yellow", "系统", false, "崩溃内存转储, 一般只在排查问题时有用", &[r"%LOCALAPPDATA%\CrashDumps"]),
    item!("d3dscache", "DirectX 着色器缓存", "green", "系统", false, "游戏/应用着色器缓存, 首次运行时自动重建", &[r"%LOCALAPPDATA%\D3DSCache"]),
    item!("inetcache", "Windows INet 缓存", "yellow", "系统", false, "IE/系统组件网络缓存, 被占用的自动跳过", &[r"%LOCALAPPDATA%\Microsoft\Windows\INetCache"]),
    item!("wud", "Windows 更新缓存", "green", "系统", true, "已下载的更新安装包, 需要管理员权限", &[r"C:\Windows\SoftwareDistribution\Download"]),
    item!("cbs", "Windows 组件日志", "yellow", "系统", true, "系统组件安装日志, 需要管理员权限", &[r"C:\Windows\Logs\CBS\CbsPersist_*.cab"]),
    item!("do_cache", "传递优化缓存", "green", "系统", true, "Windows 更新分发光缓存, 需要管理员权限", &[
        r"C:\Windows\ServiceProfiles\NetworkService\AppData\Local\Microsoft\Windows\DeliveryOptimization\Cache"]),
    item!("steam_html", "Steam 内嵌浏览器缓存", "green", "系统", false, "Steam 商店/社区页面的浏览器缓存", &[r"%LOCALAPPDATA%\Steam\htmlcache"]),

    item!("npm", "npm 下载缓存", "green", "开发工具", false, "npm 包缓存, 下次安装自动重新下载", &[r"%LOCALAPPDATA%\npm-cache"]),
    item!("yarn", "yarn 下载缓存", "green", "开发工具", false, "yarn 包缓存, 下次安装自动重新下载", &[r"%LOCALAPPDATA%\Yarn\Cache"]),
    item!("pnpm", "pnpm 下载缓存", "green", "开发工具", false, "pnpm 包存储, 下次安装自动重新下载", &[r"%LOCALAPPDATA%\pnpm\store"]),
    item!("pip", "pip 下载缓存", "green", "开发工具", false, "pip 包缓存", &[r"%LOCALAPPDATA%\pip\cache"]),
    item!("go", "Go 模块缓存", "green", "开发工具", false, "go get 下载的模块, 编译时自动重新下载", &[r"%USERPROFILE%\go\pkg"]),
    item!("cargo", "Rust cargo 下载缓存", "green", "开发工具", false, "crates.io 下载缓存(不含工具链本体), 编译时自动重新下载", &[r"%USERPROFILE%\.cargo\registry"]),
    item!("gradle", "Gradle 构建缓存", "yellow", "开发工具", false, "依赖与构建缓存, 下次构建自动重新下载", &[r"%USERPROFILE%\.gradle\caches"]),
    item!("maven", "Maven 本地仓库", "yellow", "开发工具", false, "依赖仓库, 下次构建自动重新下载", &[r"%USERPROFILE%\.m2\repository"]),
    item!("nuget", "NuGet 包缓存", "yellow", "开发工具", false, "dotnet 依赖包, 下次还原自动重新下载", &[r"%USERPROFILE%\.nuget\packages"]),
    item!("dotcache", "通用工具缓存(~/.cache)", "yellow", "开发工具", false, "pip/matplotlib/torch 等工具缓存, 需要时自动重新下载", &[r"%USERPROFILE%\.cache"]),
    item!("postman_old", "Postman 旧版本", "yellow", "开发工具", false, "保留最新版, 清理其余 app-* 旧版本", &[]),
    item!("apifox_pending", "Apifox 更新残留", "yellow", "开发工具", false, "自动更新下载的安装包", &[r"%APPDATA%\apifox-updater\pending"]),

    item!("chrome", "Chrome 浏览器缓存", "green", "浏览器", false, "网页缓存, 不碰密码/历史/书签", &[
        r"%LOCALAPPDATA%\Google\Chrome\User Data\*\Cache",
        r"%LOCALAPPDATA%\Google\Chrome\User Data\*\Code Cache",
        r"%LOCALAPPDATA%\Google\Chrome\User Data\*\GPUCache"]),
    item!("edge", "Edge 浏览器缓存", "green", "浏览器", false, "网页缓存, 不碰密码/历史/书签", &[
        r"%LOCALAPPDATA%\Microsoft\Edge\User Data\*\Cache",
        r"%LOCALAPPDATA%\Microsoft\Edge\User Data\*\Code Cache",
        r"%LOCALAPPDATA%\Microsoft\Edge\User Data\*\GPUCache"]),
    item!("firefox", "Firefox 浏览器缓存", "green", "浏览器", false, "浏览器缓存, 删除后自动重建", &[
        r"%LOCALAPPDATA%\Mozilla\Firefox\Profiles\*\cache2"]),
    item!("brave", "Brave 浏览器缓存", "green", "浏览器", false, "浏览器缓存, 删除后自动重建", &[
        r"%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data\*\Cache",
        r"%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data\*\Code Cache",
        r"%LOCALAPPDATA%\BraveSoftware\Brave-Browser\User Data\*\GPUCache"]),

    item!("idecache", "VS Code / Trae 缓存与日志", "yellow", "编辑器", false, "IDE 缓存与日志, 重启后自动重建", &[
        r"%APPDATA%\Code\Cache", r"%APPDATA%\Code\CachedData", r"%APPDATA%\Code\GPUCache", r"%APPDATA%\Code\logs",
        r"%APPDATA%\Trae CN\Cache", r"%APPDATA%\Trae CN\CachedData", r"%APPDATA%\Trae CN\GPUCache", r"%APPDATA%\Trae CN\logs"]),
    item!("cursor", "Cursor 缓存与日志", "yellow", "编辑器", false, "AI 编辑器缓存与日志, 与 VS Code 同结构", &[
        r"%APPDATA%\Cursor\Cache", r"%APPDATA%\Cursor\CachedData", r"%APPDATA%\Cursor\GPUCache", r"%APPDATA%\Cursor\logs"]),
    item!("jetbrains_log", "JetBrains IDE 日志", "yellow", "编辑器", false, "PyCharm/GoLand 等各版本 IDE 的日志, 自动重建", &[r"%LOCALAPPDATA%\JetBrains\*\log"]),

    item!("xwechat_log", "微信(新版)日志", "yellow", "聊天办公", false, "微信运行日志", &[r"%APPDATA%\Tencent\xwechat\log"]),
    item!("xwechat_old_log", "微信(旧版)日志", "yellow", "聊天办公", false, "旧版微信运行日志", &[r"%APPDATA%\Tencent\WeChat\log"]),
    item!("qq_log", "QQ 日志", "yellow", "聊天办公", false, "QQ 运行日志", &[r"%APPDATA%\Tencent\QQ\log", r"%APPDATA%\Tencent\QQ\Logs"]),
    item!("wxwork_log", "企业微信日志", "yellow", "聊天办公", false, "企业微信运行日志", &[r"%APPDATA%\Tencent\WXWork\log"]),
    item!("lark_log", "飞书内嵌浏览器缓存", "green", "聊天办公", false, "飞书内置浏览器/着色器缓存, 删除后自动重建", &[
        r"%APPDATA%\LarkShell\CodeCache", r"%APPDATA%\LarkShell\GrShaderCache", r"%APPDATA%\LarkShell\ShaderCache",
        r"%APPDATA%\LarkShell\GraphiteDawnCache", r"%APPDATA%\LarkShell\Default\Cache",
        r"%APPDATA%\LarkShell\Default\Code Cache", r"%APPDATA%\LarkShell\Default\GPUCache"]),
    item!("discord", "Discord 缓存", "green", "聊天办公", false, "Electron 缓存, 删除后自动重建", &[
        r"%APPDATA%\discord\Cache", r"%APPDATA%\discord\Code Cache", r"%APPDATA%\discord\GPUCache"]),
    item!("netease_music", "网易云音乐缓存", "yellow", "聊天办公", false, "在线播放歌曲的缓存, 删除后下次播放重新缓冲", &[r"%APPDATA%\NetEase\CloudMusic\Cache"]),
];

// ---------------------------------------------------------------- macOS 白名单

#[cfg(not(windows))]
static ITEMS: &[CleanItem] = &[
    item!("mac_caches", "应用缓存(~/Library/Caches)", "yellow", "系统", false, "各 App 的缓存, 建议先退出正在使用的软件再清理", &["~/Library/Caches"]),
    item!("mac_logs", "应用日志(~/Library/Logs)", "green", "系统", false, "各 App 的运行日志, 删除后自动重建", &["~/Library/Logs"]),
    item!("mac_savedstate", "窗口恢复状态", "green", "系统", false, "App 退出时的窗口状态, 删除后下次打开重新布局", &["~/Library/Saved Application State"]),
    item!("mac_trash", "废纸篓", "yellow", "系统", false, "废纸篓里的内容, 不可恢复（该条目始终直接删除）", &["~/.Trash"]),
    item!("sys_caches", "系统级缓存(/Library/Caches)", "yellow", "系统", true, "系统范围的缓存, 需要管理员权限", &["/Library/Caches"]),
    item!("sys_logs", "系统级日志(/Library/Logs)", "yellow", "系统", true, "系统范围的日志, 需要管理员权限", &["/Library/Logs"]),

    item!("mac_npm", "npm 下载缓存", "green", "开发工具", false, "npm 包缓存, 下次安装自动重新下载", &["~/.npm/_cacache"]),
    item!("mac_yarn", "yarn 下载缓存", "green", "开发工具", false, "yarn 包缓存, 下次安装自动重新下载", &["~/Library/Caches/Yarn", "~/.cache/yarn"]),
    item!("mac_pnpm", "pnpm 存储", "green", "开发工具", false, "pnpm 包存储, 下次安装自动重新下载", &["~/Library/pnpm/store", "~/Library/Caches/pnpm"]),
    item!("mac_pip", "pip 下载缓存", "green", "开发工具", false, "pip 包缓存", &["~/Library/Caches/pip"]),
    item!("mac_go_build", "Go 构建缓存", "green", "开发工具", false, "go build 编译缓存, 编译时自动重建", &["~/Library/Caches/go-build"]),
    item!("mac_cargo", "Rust cargo 下载缓存", "green", "开发工具", false, "crates.io 下载缓存, 编译时自动重新下载", &["~/.cargo/registry/cache"]),
    item!("mac_gradle", "Gradle 构建缓存", "yellow", "开发工具", false, "依赖与构建缓存, 下次构建自动重新下载", &["~/.gradle/caches"]),
    item!("mac_maven", "Maven 本地仓库", "yellow", "开发工具", false, "依赖仓库, 下次构建自动重新下载", &["~/.m2/repository"]),
    item!("mac_nuget", "NuGet 包缓存", "yellow", "开发工具", false, "dotnet 依赖包, 下次还原自动重新下载", &["~/.nuget/packages"]),
    item!("mac_homebrew", "Homebrew 下载缓存", "green", "开发工具", false, "brew 下载的安装包与源码, 需要时自动重新下载", &["~/Library/Caches/Homebrew"]),
    item!("mac_dotcache", "通用工具缓存(~/.cache)", "yellow", "开发工具", false, "pip/matplotlib/torch 等工具缓存, 需要时自动重新下载", &["~/.cache"]),
    item!("mac_xcode_derived", "Xcode DerivedData", "yellow", "开发工具", false, "Xcode 编译中间产物, 重新编译时自动生成", &["~/Library/Developer/Xcode/DerivedData"]),
    item!("mac_xcode_devicesupport", "Xcode 旧设备支持文件", "yellow", "开发工具", false, "旧 iOS 版本的调试符号, 连接对应设备时会重新下载", &["~/Library/Developer/Xcode/iOS DeviceSupport"]),
    item!("mac_simulator", "模拟器缓存", "yellow", "开发工具", false, "CoreSimulator 运行时缓存, 按需自动重建", &["~/Library/Developer/CoreSimulator/Caches"]),

    item!("mac_safari", "Safari 浏览器缓存", "green", "浏览器", false, "网页缓存, 不碰书签/历史", &["~/Library/Caches/com.apple.Safari"]),
    item!("mac_chrome", "Chrome 浏览器缓存", "green", "浏览器", false, "网页缓存, 不碰密码/历史/书签", &[
        "~/Library/Caches/Google/Chrome",
        "~/Library/Application Support/Google/Chrome/*/Cache",
        "~/Library/Application Support/Google/Chrome/*/Code Cache",
        "~/Library/Application Support/Google/Chrome/*/GPUCache"]),
    item!("mac_edge", "Edge 浏览器缓存", "green", "浏览器", false, "网页缓存, 不碰密码/历史/书签", &[
        "~/Library/Caches/Microsoft Edge",
        "~/Library/Application Support/Microsoft Edge/*/Cache",
        "~/Library/Application Support/Microsoft Edge/*/GPUCache"]),
    item!("mac_firefox", "Firefox 浏览器缓存", "green", "浏览器", false, "浏览器缓存, 删除后自动重建", &[
        "~/Library/Caches/Firefox",
        "~/Library/Application Support/Firefox/Profiles/*/cache2"]),
    item!("mac_brave", "Brave 浏览器缓存", "green", "浏览器", false, "浏览器缓存, 删除后自动重建", &[
        "~/Library/Caches/BraveSoftware"]),

    item!("mac_vscode", "VS Code 缓存与日志", "yellow", "编辑器", false, "IDE 缓存与日志, 重启后自动重建", &[
        "~/Library/Application Support/Code/Cache", "~/Library/Application Support/Code/CachedData",
        "~/Library/Application Support/Code/GPUCache", "~/Library/Application Support/Code/logs"]),
    item!("mac_cursor", "Cursor 缓存与日志", "yellow", "编辑器", false, "AI 编辑器缓存与日志, 与 VS Code 同结构", &[
        "~/Library/Application Support/Cursor/Cache", "~/Library/Application Support/Cursor/CachedData",
        "~/Library/Application Support/Cursor/GPUCache", "~/Library/Application Support/Cursor/logs"]),
    item!("mac_trae", "Trae 缓存与日志", "yellow", "编辑器", false, "IDE 缓存与日志, 重启后自动重建", &[
        "~/Library/Application Support/Trae CN/Cache", "~/Library/Application Support/Trae CN/CachedData",
        "~/Library/Application Support/Trae CN/GPUCache", "~/Library/Application Support/Trae CN/logs"]),
    item!("mac_jetbrains", "JetBrains IDE 缓存与日志", "yellow", "编辑器", false, "PyCharm/GoLand 等各版本 IDE 的缓存与日志, 自动重建", &[
        "~/Library/Caches/JetBrains", "~/Library/Logs/JetBrains"]),

    item!("mac_lark", "飞书缓存", "green", "聊天办公", false, "飞书内置浏览器/着色器缓存, 删除后自动重建", &["~/Library/Caches/com.bytedance.lark"]),
    item!("mac_wechat", "微信缓存", "yellow", "聊天办公", false, "微信缓存（不含聊天记录）", &["~/Library/Caches/com.tencent.xinWeChat"]),
    item!("mac_qq", "QQ 缓存", "yellow", "聊天办公", false, "QQ 缓存（不含聊天记录）", &["~/Library/Caches/com.tencent.qq"]),
    item!("mac_wxwork", "企业微信缓存", "yellow", "聊天办公", false, "企业微信缓存（不含聊天记录）", &["~/Library/Caches/com.tencent.WeWorkMac"]),
    item!("mac_discord", "Discord 缓存", "green", "聊天办公", false, "Electron 缓存, 删除后自动重建", &[
        "~/Library/Application Support/discord/Cache", "~/Library/Application Support/discord/Code Cache",
        "~/Library/Application Support/discord/GPUCache"]),
    item!("mac_slack", "Slack 缓存", "green", "聊天办公", false, "Electron 缓存, 删除后自动重建", &[
        "~/Library/Application Support/Slack/Cache", "~/Library/Application Support/Slack/Code Cache",
        "~/Library/Application Support/Slack/GPUCache"]),

    // 对标 Windows 白名单里对应的跨平台软件项（Mac 上目录结构不同，单独列出）
    item!("mac_netease", "网易云音乐缓存", "yellow", "聊天办公", false, "在线播放的歌曲缓存, 删除后下次播放重新缓冲", &[
        "~/Library/Containers/com.netease.163music/Data/Caches"]),
    item!("mac_steam", "Steam 内嵌浏览器缓存", "green", "系统", false, "Steam 商店/社区页面的浏览器缓存, 删除后自动重建", &[
        "~/Library/Application Support/Steam/htmlcache",
        "~/Library/Application Support/Steam/appcache/httpcache"]),
    item!("mac_postman", "Postman 缓存与日志", "green", "开发工具", false, "内置浏览器缓存与日志, 删除后自动重建", &[
        "~/Library/Application Support/Postman/Cache",
        "~/Library/Application Support/Postman/Code Cache",
        "~/Library/Application Support/Postman/GPUCache",
        "~/Library/Application Support/Postman/logs"]),
    item!("mac_apifox", "Apifox 缓存与日志", "green", "开发工具", false, "内置浏览器缓存与日志, 删除后自动重建", &[
        "~/Library/Application Support/Apifox/Cache",
        "~/Library/Application Support/Apifox/Code Cache",
        "~/Library/Application Support/Apifox/GPUCache",
        "~/Library/Application Support/Apifox/logs"]),
];

fn items() -> &'static [CleanItem] {
    ITEMS
}

fn item_by_key(key: &str) -> Option<&'static CleanItem> {
    ITEMS.iter().find(|i| i.key == key)
}

// ---------------------------------------------------------------- 序列化结构

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanItemMeta {
    pub key: String,
    pub name: String,
    pub level: String,
    pub desc: String,
    pub admin: bool,
    pub category: String,
    pub path_hint: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanTarget {
    pub id: String,
    pub item_key: String,
    pub item_name: String,
    pub level: String,
    pub desc: String,
    pub admin: bool,
    pub category: String,
    pub path: String,
    pub bytes: u64,
    pub files: u64,
    /// dir = 清空目录内容（保留目录）；file = 删除该文件；remove = 整个删除该路径
    pub kind: String,
    pub deletable: bool,
}

// ---------------------------------------------------------------- 目标解析

#[cfg(windows)]
fn postman_targets() -> Vec<PathBuf> {
    let base = match dirs::data_local_dir() {
        Some(d) => d.join("Postman"),
        None => return Vec::new(),
    };
    let Ok(rd) = std::fs::read_dir(&base) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("app-"))
                    .unwrap_or(false)
        })
        .collect();
    if dirs.len() <= 1 {
        return Vec::new();
    }
    let ver = |p: &PathBuf| -> Vec<u32> {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        name.trim_start_matches("app-")
            .split('.')
            .map(|x| x.parse::<u32>().unwrap_or(0))
            .collect()
    };
    dirs.sort_by_key(ver);
    dirs.pop();
    dirs
}

fn resolve_item(it: &CleanItem) -> Vec<PathBuf> {
    #[cfg(windows)]
    if it.special == Some("postman_old") {
        return postman_targets();
    }
    let pats: Vec<String> = it.paths.iter().map(|s| s.to_string()).collect();
    fsutil::expand_paths(&pats)
}

// ---------------------------------------------------------------- 命令

#[tauri::command]
pub fn list_clean_items() -> Vec<CleanItemMeta> {
    items()
        .iter()
        .map(|i| CleanItemMeta {
            key: i.key.to_string(),
            name: i.name.to_string(),
            level: i.level.to_string(),
            desc: i.desc.to_string(),
            admin: i.admin,
            category: i.category.to_string(),
            path_hint: i.paths.iter().map(|s| s.to_string()).collect(),
        })
        .collect()
}

#[tauri::command]
pub fn start_clean_scan(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let ctl = state.begin_task("clean_scan")?;
    let id = ctl.id.clone();
    let mut em = Emitter::new(app, ctl, "扫描可清理项");
    std::thread::spawn(move || {
        run_scan(&mut em);
    });
    Ok(id)
}

#[tauri::command]
pub fn start_clean_run(
    app: AppHandle,
    state: State<'_, AppState>,
    targets: Vec<CleanTarget>,
    permanent: bool,
) -> Result<String, String> {
    if targets.is_empty() {
        return Err("没有选中任何清理项".into());
    }
    let ctl = state.begin_task("clean_run")?;
    let id = ctl.id.clone();
    let data_dir = state.data_dir.clone();
    let mut em = Emitter::new(app, ctl, "正在清理");
    std::thread::spawn(move || {
        run_clean(&mut em, targets, permanent, &data_dir);
    });
    Ok(id)
}

// ---------------------------------------------------------------- 扫描

fn run_scan(em: &mut Emitter) {
    em.set_phase("枚举", "正在扫描可清理项…");
    em.set_can_pause(true);
    let mut rows: u64 = 0;
    let mut stopped = false;
    // 已扫完条目的累计量。「已发现」显示的应是本次扫描到目前为止的总量：
    // 不累加的话，每换一个白名单条目数字就掉回 0 再重新往上爬，界面看着一直在跳。
    let mut acc_bytes: u64 = 0;
    let mut acc_files: u64 = 0;

    for it in items() {
        if em.ctl.wait_gate() {
            stopped = true;
            break;
        }
        let paths = resolve_item(it);
        if paths.is_empty() {
            continue;
        }
        let ctl = em.ctl.clone();
        for p in paths {
            if em.ctl.wait_gate() {
                stopped = true;
                break;
            }
            if fsutil::is_reparse(&p) || !p.exists() {
                continue;
            }
            em.set_current(&p);
            let stat: TreeStat = {
                // 把已扫完条目的量作为基线，叠上当前这条正在统计的部分：
                // 进度条上的「已发现 / 文件」因此只增不减，不会每条目重来一遍
                let base_bytes = acc_bytes;
                let base_files = acc_files;
                fsutil::count_tree(&p, &ctl, &mut |b, f, cur| {
                    em.set_current(cur);
                    em.set_discovered(base_bytes + b, base_files + f);
                })
            };
            acc_bytes += stat.bytes;
            acc_files += stat.files;
            if stat.cancelled {
                stopped = true;
                break;
            }
            let is_dir = p.is_dir();
            let kind = if it.special.is_some() {
                "remove"
            } else if is_dir {
                "dir"
            } else {
                "file"
            };
            em.row(
                "clean_targets",
                CleanTarget {
                    id: format!("{}#{}", it.key, rows),
                    item_key: it.key.to_string(),
                    item_name: it.name.to_string(),
                    level: it.level.to_string(),
                    desc: it.desc.to_string(),
                    admin: it.admin,
                    category: it.category.to_string(),
                    path: p.display().to_string(),
                    bytes: stat.bytes,
                    files: stat.files,
                    kind: kind.to_string(),
                    deletable: !it.force_permanent,
                },
            );
            rows += 1;
            em.inc_items();
        }
        if stopped {
            break;
        }
    }

    em.rows_done("clean_targets", rows);
    if stopped {
        em.finish(rows, 0, 0, 0, true, "扫描已中断", None);
    } else {
        em.finish(rows, 0, 0, 0, false, format!("扫描完成，发现 {} 个可清理目标", rows), None);
    }
}

// ---------------------------------------------------------------- 执行

/// 单条目标的清理结果。前端据此把已经清干净的条目当场从列表里摘掉，
/// 于是清理过程中列表和合计体积会一起往下减，不用等结束再手动重扫一次。
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CleanDone {
    id: String,
    bytes: u64,
    /// 整条都处理干净了才为 true；有跳过/失败的条目留在列表上，提示磁盘上还有残留
    removed: bool,
}

struct Counter {
    ok: u64,
    skipped: u64,
    failed: u64,
    bytes: u64,
}

fn run_clean(em: &mut Emitter, targets: Vec<CleanTarget>, permanent: bool, data_dir: &Path) {
    let files_total: u64 = targets.iter().map(|t| t.files).sum();
    let bytes_total: u64 = targets.iter().map(|t| t.bytes).sum();
    em.set_phase("清理", "正在清理选中项…");
    em.set_items_total(targets.len() as u64);
    em.set_totals(targets.len() as u64, files_total, bytes_total);
    em.flush();

    let mut c = Counter { ok: 0, skipped: 0, failed: 0, bytes: 0 };
    let mut report: Vec<String> = vec![
        format!(
            "唐三浪清理 —— 清理报告   {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
        format!(
            "模式: {}",
            if permanent { "直接删除（不可恢复）" } else { "移入回收站/废纸篓" }
        ),
        format!("选中 {} 个目标, 预计体积 {}", targets.len(), fsutil::fmt_size(bytes_total)),
        "".into(),
    ];
    let mut stopped = false;

    for t in &targets {
        if em.ctl.wait_gate() {
            stopped = true;
            break;
        }
        let path = PathBuf::from(&t.path);
        em.set_current(&path);
        let force_perm = item_by_key(&t.item_key).map(|i| i.force_permanent).unwrap_or(false);
        let use_perm = permanent || force_perm;

        let before = (c.ok, c.skipped, c.failed, c.bytes);
        match t.kind.as_str() {
            "file" => process_file(&path, use_perm, em, &mut c, &mut report),
            "remove" => process_remove(&path, use_perm, em, &mut c, &mut report),
            _ => process_dir(&path, use_perm, em, &mut c, &mut report),
        }
        if em.ctl.is_cancelled() {
            stopped = true;
            break;
        }
        // 整个目标一个条目都没处理成功时，算作失败条目
        if c.ok == before.0 && c.skipped == before.1 && c.failed == before.2 {
            c.failed += 1;
        }
        report.push(format!(
            "  [{}] {}  {}{}",
            t.item_name,
            path.display(),
            if c.bytes > before.3 {
                format!("释放 {}", fsutil::fmt_size(c.bytes - before.3))
            } else {
                "无变化".to_string()
            },
            if use_perm { "  (直接删除)" } else { "" }
        ));
        // 每处理完一条就报一次结果：清干净的条目前端会立刻从列表里摘掉。
        // removed 要求这一条既没跳过也没失败 —— 有残留的条目留在列表上，
        // 免得列表看着已经清空、磁盘上其实还留着东西。
        em.row(
            "clean_done",
            CleanDone {
                id: t.id.clone(),
                bytes: c.bytes - before.3,
                removed: c.failed == before.2 && c.skipped == before.1,
            },
        );
        em.inc_items();
    }

    report.push("".into());
    report.push(format!(
        "合计: 成功 {} 项 / 跳过 {} 项 / 失败 {} 项, 共释放 {}",
        c.ok,
        c.skipped,
        c.failed,
        fsutil::fmt_size(c.bytes)
    ));
    if stopped {
        report.push("（用户中途停止，已完成的部分保留）".into());
    }
    let report_path = fsutil::write_report(data_dir, "清理报告", &report);

    let msg = if stopped {
        format!("已停止：释放 {}，成功 {} 项", fsutil::fmt_size(c.bytes), c.ok)
    } else {
        format!("清理完成：释放 {}，成功 {} 项", fsutil::fmt_size(c.bytes), c.ok)
    };
    em.log(msg.clone(), if stopped { "warn" } else { "ok" });
    em.finish(c.ok, c.skipped, c.failed, c.bytes, stopped, msg, report_path);
}

fn process_file(
    path: &Path,
    permanent: bool,
    em: &mut Emitter,
    c: &mut Counter,
    report: &mut Vec<String>,
) {
    let (bytes, _) = fsutil::path_size(path);
    if permanent {
        if fsutil::force_remove_file(path) {
            c.ok += 1;
            c.bytes += bytes;
            em.add_files(1, bytes);
            em.log(format!("已删除 {}", path.display()), "ok");
        } else {
            c.skipped += 1;
            em.log(format!("跳过（被占用或权限不足）{}", path.display()), "warn");
            report.push(format!("  ! 跳过: {}", path.display()));
        }
        return;
    }
    match fsutil::move_to_trash(path) {
        Ok(_) => {
            c.ok += 1;
            c.bytes += bytes;
            em.add_files(1, bytes);
            em.log(format!("已移入回收站 {}", path.display()), "ok");
        }
        Err(e) => {
            fsutil::clear_readonly(path);
            if fsutil::move_to_trash(path).is_ok() {
                c.ok += 1;
                c.bytes += bytes;
                em.add_files(1, bytes);
                em.log(format!("已移入回收站 {}", path.display()), "ok");
            } else {
                c.skipped += 1;
                em.log(format!("跳过（{}）{}", e, path.display()), "warn");
            }
        }
    }
}

/// kind = "remove"：把整个路径（目录树或文件）一次处理掉
fn process_remove(
    path: &Path,
    permanent: bool,
    em: &mut Emitter,
    c: &mut Counter,
    report: &mut Vec<String>,
) {
    if !path.exists() {
        c.skipped += 1;
        return;
    }
    if permanent {
        let ctl = em.ctl.clone();
        let mut removed: u64 = 0;
        let mut bytes: u64 = 0;
        let ok = remove_recursive(path, &ctl, em, &mut removed, &mut bytes);
        if em.ctl.is_cancelled() {
            return;
        }
        if ok {
            c.ok += 1;
            c.bytes += bytes;
        } else {
            c.failed += 1;
            report.push(format!("  ! 失败: {}", path.display()));
            em.log(format!("删除失败 {}", path.display()), "error");
        }
        return;
    }
    // 回收站模式：先统计体积（有反馈），再整目录移入回收站
    let ctl = em.ctl.clone();
    let stat = fsutil::count_tree(path, &ctl, &mut |_b, _f, cur| {
        em.set_current(cur);
        em.tick();
    });
    if stat.cancelled {
        return;
    }
    match fsutil::move_to_trash(path) {
        Ok(_) => {
            c.ok += 1;
            c.bytes += stat.bytes;
            em.add_files(stat.files, stat.bytes);
            em.log(
                format!("已移入回收站 {}（{}）", path.display(), fsutil::fmt_size(stat.bytes)),
                "ok",
            );
        }
        Err(e) => {
            c.skipped += 1;
            em.log(format!("跳过（{}）{}", e, path.display()), "warn");
            report.push(format!("  ! 跳过: {}  ({})", path.display(), e));
        }
    }
}

/// 清理一个目录的内容（保留目录本身），逐顶层条目处理
fn process_dir(
    path: &Path,
    permanent: bool,
    em: &mut Emitter,
    c: &mut Counter,
    report: &mut Vec<String>,
) {
    let children: Vec<PathBuf> = match std::fs::read_dir(path) {
        Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
        Err(e) => {
            c.skipped += 1;
            em.log(format!("无法读取（{}）{}", e, path.display()), "warn");
            return;
        }
    };

    for child in children {
        if em.ctl.wait_gate() {
            return;
        }
        em.set_current(&child);
        if fsutil::is_reparse(&child) {
            c.skipped += 1;
            em.log(format!("跳过联接 {}", child.display()), "warn");
            continue;
        }
        let md = match std::fs::symlink_metadata(&child) {
            Ok(m) => m,
            Err(_) => {
                c.failed += 1;
                continue;
            }
        };
        if permanent {
            let ctl = em.ctl.clone();
            let mut removed: u64 = 0;
            let mut bytes: u64 = 0;
            let ok = remove_recursive(&child, &ctl, em, &mut removed, &mut bytes);
            if em.ctl.is_cancelled() {
                return;
            }
            if ok {
                c.ok += 1;
                c.bytes += bytes;
                em.log(
                    format!("已删除 {}（{}）", child.display(), fsutil::fmt_size(bytes)),
                    "ok",
                );
            } else {
                c.skipped += 1;
                em.log(format!("跳过（被占用或权限不足）{}", child.display()), "warn");
                report.push(format!("  ! 跳过: {}", child.display()));
            }
            if md.is_dir() {
                // 目录已清空则一并移除
                let _ = std::fs::remove_dir(&child);
            }
        } else {
            // 回收站模式：统计体积用于进度显示，随后整条目移入回收站/废纸篓
            let ctl = em.ctl.clone();
            let stat = fsutil::count_tree(&child, &ctl, &mut |_b, _f, cur| {
                em.set_current(cur);
                em.tick();
            });
            if stat.cancelled {
                return;
            }
            match fsutil::move_to_trash(&child) {
                Ok(_) => {
                    c.ok += 1;
                    c.bytes += stat.bytes;
                    em.add_files(stat.files, stat.bytes);
                    em.log(
                        format!(
                            "已移入回收站 {}（{}）",
                            child.display(),
                            fsutil::fmt_size(stat.bytes)
                        ),
                        "ok",
                    );
                }
                Err(e) => {
                    fsutil::clear_readonly(&child);
                    if fsutil::move_to_trash(&child).is_ok() {
                        c.ok += 1;
                        c.bytes += stat.bytes;
                        em.add_files(stat.files, stat.bytes);
                        em.log(format!("已移入回收站 {}", child.display()), "ok");
                    } else {
                        c.skipped += 1;
                        em.log(format!("跳过（{}）{}", e, child.display()), "warn");
                        report.push(format!("  ! 跳过: {}  ({})", child.display(), e));
                    }
                }
            }
        }
    }
}

/// 递归直接删除：逐文件上报进度，每个文件之间检查暂停/取消
/// （迁移还原时要删目标盘副本，故对 crate 内可见）
pub(crate) fn remove_recursive(
    path: &Path,
    ctl: &TaskCtl,
    em: &mut Emitter,
    files: &mut u64,
    bytes: &mut u64,
) -> bool {
    if ctl.is_cancelled() {
        return false;
    }
    let md = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return true,
    };
    if md.file_type().is_symlink() || fsutil::is_reparse(path) {
        // 只删联接本身，绝不触碰它指向的数据
        return fsutil::remove_link(path);
    }
    if md.is_dir() {
        let children: Vec<PathBuf> = match std::fs::read_dir(path) {
            Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
            Err(_) => {
                fsutil::clear_readonly(path);
                match std::fs::read_dir(path) {
                    Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
                    Err(_) => return false,
                }
            }
        };
        let mut all_ok = true;
        for ch in children {
            if ctl.wait_gate() {
                return false;
            }
            if !remove_recursive(&ch, ctl, em, files, bytes) {
                if ctl.is_cancelled() {
                    return false;
                }
                all_ok = false;
            }
        }
        return match std::fs::remove_dir(path) {
            Ok(_) => all_ok,
            Err(_) => {
                fsutil::clear_readonly(path);
                std::fs::remove_dir(path).is_ok() && all_ok
            }
        };
    }
    let len = md.len();
    if fsutil::force_remove_file(path) {
        *files += 1;
        *bytes += len;
        em.add_file(len);
        true
    } else {
        false
    }
}
