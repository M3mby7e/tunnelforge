const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Human-readable byte count, e.g. 1536 → "1.5 KB". */
export function formatBytes(bytes: number): string {
  if (bytes < 1) return "0 B";
  const exp = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), UNITS.length - 1);
  const value = bytes / Math.pow(1024, exp);
  const rounded = exp === 0 ? value : Math.round(value * 10) / 10;
  return `${rounded} ${UNITS[exp]}`;
}

/** Compact uptime, e.g. 45 → "45s", 3732 → "1h 2m". */
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ${seconds % 60}s`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}
