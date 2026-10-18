// 文件系统工具：目录遍历/体积统计、路径展开、安全删除、格式化
// 安全铁律：junction / 符号链接（reparse point）一律不进入、不统计、不当作普通目录删除。
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use crate::state::TaskCtl;

#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// 是否为 junction / 符号链接 / 挂载点（取不到属性时按普通路径处理）
pub fn is_reparse(p: &Path) -> bool {
    match std::fs::symlink_metadata(p) {
        Ok(md) => {
            if md.file_type().is_symlink() {
                return true;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                return md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
            }
            #[allow(unreachable_code)]
            false
        }
        Err(_) => false,
    }
}

pub struct TreeStat {
    pub bytes: u64,
    pub files: u64,
    /// 树内遇到的第一个联接（用于拦截"目录里还套着已迁移子目录"）
    pub nested_reparse: Option<PathBuf>,
    pub cancelled: bool,
}

/// 扫描一棵目录树，返回体积/文件数。带节流进度回调、可中断、可暂停。
pub fn count_tree<F>(root: &Path, ctl: &TaskCtl, on_tick: &mut F) -> TreeStat
where
    F: FnMut(u64, u64, &Path),
{
    let mut st = TreeStat {
        bytes: 0,
        files: 0,
        nested_reparse: None,
        cancelled: false,
    };
    if is_reparse(root) {
        st.nested_reparse = Some(root.to_path_buf());
        return st;
    }
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut last = Instant::now();
    while let Some(cur) = stack.pop() {
        if ctl.wait_gate() {
            st.cancelled = true;
            break;
        }
        let rd = match std::fs::read_dir(&cur) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            let md = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if md.file_type().is_symlink() || is_reparse(&p) {
                if st.nested_reparse.is_none() {
                    st.nested_reparse = Some(p);
                }
                continue;
            }
            if md.is_dir() {
                stack.push(p);
            } else {
                st.bytes += md.len();
                st.files += 1;
            }
            if last.elapsed() >= Duration::from_millis(100) {
                last = Instant::now();
                on_tick(st.bytes, st.files, &cur);
                if ctl.is_cancelled() {
                    st.cancelled = true;
                    return st;
                }
            }
        }
    }
    on_tick(st.bytes, st.files, root);
    st
}

/// 单个路径的 (体积, 文件数)；联接返回 (0, 0)。
pub fn path_size(p: &Path) -> (u64, u64) {
    if is_reparse(p) {
        return (0, 0);
    }
    let md = match std::fs::symlink_metadata(p) {
        Ok(m) => m,
        Err(_) => return (0, 0),
    };
    if md.is_dir() {
        let ctl = TaskCtl::noop();
        let mut noop = |_: u64, _: u64, _: &Path| {};
        let st = count_tree(p, &ctl, &mut noop);
        (st.bytes, st.files)
    } else {
        (md.len(), 1)
    }
}

// ---------- 路径展开 ----------

pub fn expand_env(input: &str) -> String {
    let mut s = input.to_string();
    if s.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            s = format!("{}{}", home.display(), &s[1..]);
        }
    }
    // %VAR% 风格
    loop {
        let Some(a) = s.find('%') else { break };
        let Some(rel) = s[a + 1..].find('%') else { break };
        let name = s[a + 1..a + 1 + rel].to_string();
        if name.is_empty() || name.contains(['/', '\\', ' ']) {
            break;
        }
        let val = std::env::var(&name).unwrap_or_default();
        s = format!("{}{}{}", &s[..a], val, &s[a + 2 + rel..]);
    }
    // ${VAR} / $VAR 风格
    loop {
        let Some(a) = s.find('$') else { break };
        let rest = &s[a + 1..];
        let (name, len) = if let Some(stripped) = rest.strip_prefix('{') {
            match stripped.find('}') {
                Some(end) => (stripped[..end].to_string(), end + 2),
                None => break,
            }
        } else {
            let n: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            (n.clone(), n.len() + 1)
        };
        if name.is_empty() {
            break;
        }
        let val = std::env::var(&name).unwrap_or_default();
        s = format!("{}{}{}", &s[..a], val, &s[a + len..]);
    }
    s
}

/// 展开环境变量与通配符，只返回真实存在的路径（目录与文件都可）。
pub fn expand_paths(patterns: &[String]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        let p = expand_env(pat);
        let normalized = p.replace('\\', "/");
        let has_glob = normalized.contains('*') || normalized.contains('?') || normalized.contains('[');
        if has_glob {
            if let Ok(entries) = glob::glob(&normalized) {
                for e in entries.flatten() {
                    if e.exists() && !out.contains(&e) {
                        out.push(e);
                    }
                }
            }
        } else {
            let pb = PathBuf::from(&p);
            if pb.exists() && !out.contains(&pb) {
                out.push(pb);
            }
        }
    }
    // 目录优先、体积大的不排序（排序交给前端）
    out
}

// ---------- 删除 ----------

pub fn clear_readonly(p: &Path) {
    if let Ok(md) = std::fs::symlink_metadata(p) {
        let mut perm = md.permissions();
        if perm.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            let _ = std::fs::set_permissions(p, perm);
        }
    }
}

pub fn force_remove_file(p: &Path) -> bool {
    if std::fs::remove_file(p).is_ok() {
        return true;
    }
    clear_readonly(p);
    std::fs::remove_file(p).is_ok()
}

/// 只删除联接本身，绝不触碰它指向的数据。
pub fn remove_link(p: &Path) -> bool {
    if std::fs::remove_file(p).is_ok() {
        return true;
    }
    std::fs::remove_dir(p).is_ok()
}

// ---------- 进程检测 ----------

/// 检测给定进程名是否在运行；检测失败按"在运行"处理（fail-safe，绝不错杀）。
/// 目前仅 Windows 的高级迁移用得到，macOS 编译时标为允许未使用。
#[cfg_attr(not(windows), allow(dead_code))]
pub fn running_processes(names: &[String]) -> Vec<String> {
    if names.is_empty() {
        return Vec::new();
    }
    #[cfg(windows)]
    {
        match Command::new("tasklist").args(["/FO", "CSV", "/NH"]).output() {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                names
                    .iter()
                    .filter(|n| text.contains(&format!("\"{}\"", n.to_lowercase())))
                    .cloned()
                    .collect()
            }
            Err(_) => names.to_vec(),
        }
    }
    #[cfg(not(windows))]
    {
        match Command::new("ps").args(["-A", "-o", "comm="]).output() {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
                names
                    .iter()
                    .filter(|n| {
                        let n = n.to_lowercase();
                        text.lines().any(|l| {
                            let base = l.rsplit('/').next().unwrap_or(l);
                            base == n || l.contains(&n)
                        })
                    })
                    .cloned()
                    .collect()
            }
            Err(_) => names.to_vec(),
        }
    }
}

// ---------- 格式化 ----------

pub fn fmt_size(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let f = n as f64;
    if f >= GB {
        format!("{:.2} GB", f / GB)
    } else if f >= MB {
        format!("{:.1} MB", f / MB)
    } else if f >= KB {
        format!("{:.1} KB", f / KB)
    } else {
        format!("{} B", n)
    }
}

/// 供进度条显示的短路径（过长则中间省略）
pub fn short_path(p: &Path) -> String {
    let s = p.display().to_string();
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 110 {
        return s;
    }
    let head: String = chars[..52].iter().collect();
    let tail: String = chars[chars.len() - 48..].iter().collect();
    format!("{}…{}", head, tail)
}

/// 写一份清理/迁移报告到 数据目录/reports/
pub fn write_report(data_dir: &Path, prefix: &str, lines: &[String]) -> Option<String> {
    let dir = data_dir.join("reports");
    std::fs::create_dir_all(&dir).ok()?;
    let name = format!(
        "{}-{}.txt",
        prefix,
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    );
    let path = dir.join(name);
    std::fs::write(&path, lines.join("\n")).ok()?;
    Some(path.display().to_string())
}
