// 展示用格式化工具

export function fmtSize(n: number): string {
  if (!n || n <= 0) return "0 B";
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;
  const TB = GB * 1024;
  if (n >= TB) return `${(n / TB).toFixed(2)} TB`;
  if (n >= GB) return `${(n / GB).toFixed(2)} GB`;
  if (n >= MB) return `${(n / MB).toFixed(1)} MB`;
  if (n >= KB) return `${(n / KB).toFixed(1)} KB`;
  return `${n} B`;
}

export function fmtDuration(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
  return `${m}:${String(sec).padStart(2, "0")}`;
}

export function fmtSpeed(bps: number): string {
  if (bps <= 0) return "—";
  return `${fmtSize(bps)}/s`;
}

export function fmtEta(bytesLeft: number, bps: number): string {
  if (bps <= 0 || bytesLeft <= 0) return "—";
  return fmtDuration((bytesLeft / bps) * 1000);
}

export function fmtDate(ms: number): string {
  if (!ms) return "—";
  const d = new Date(ms);
  const p = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

export function fmtCount(n: number): string {
  return n.toLocaleString("zh-CN");
}
