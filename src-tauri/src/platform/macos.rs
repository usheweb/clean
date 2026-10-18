// macOS 专属能力：/Applications 与 ~/Applications 下的 .app 盘点（只读）
use std::path::PathBuf;

pub struct AppDir {
    pub name: String,
    pub version: String,
    pub loc: String,
}

fn read_plist_version(app: &std::path::Path) -> String {
    let plist = app.join("Contents/Info.plist");
    let Ok(text) = std::fs::read_to_string(&plist) else {
        return String::new();
    };
    // 直接读取 XML 中的 CFBundleShortVersionString，避免引入 plist 依赖
    let key = "<key>CFBundleShortVersionString</key>";
    if let Some(pos) = text.find(key) {
        let rest = &text[pos + key.len()..];
        if let Some(s) = rest.find("<string>") {
            let after = &rest[s + 8..];
            if let Some(e) = after.find("</string>") {
                return after[..e].trim().to_string();
            }
        }
    }
    String::new()
}

/// 列出 /Applications、~/Applications 里的 .app（只返回存在的）
pub fn list_app_dirs() -> Vec<AppDir> {
    let mut roots: Vec<PathBuf> = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Applications"));
    }
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(&root) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().map(|x| x != "app").unwrap_or(true) {
                continue;
            }
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            out.push(AppDir {
                version: read_plist_version(&p),
                name,
                loc: p.display().to_string(),
            });
        }
    }
    out
}
