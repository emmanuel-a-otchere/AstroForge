import { format, parseISO } from "date-fns";

export const isMac =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.userAgent);
export const isWin =
  typeof navigator !== "undefined" && /Windows/.test(navigator.userAgent);
export const isLinux = !isMac && !isWin;

export function slugFromName(name) {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

export function formatDate(iso) {
  if (!iso) return "—";
  const d = parseISO(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return format(d, "yyyy-MM-dd");
}

export function formatDateLong(iso) {
  if (!iso) return "—";
  const d = parseISO(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return format(d, "MMM d, yyyy");
}

// WCS RA strings arrive as "HHhMMmSS.s" / "HH:MM:SS" / plain degrees.
export function parseRaDeg(ra) {
  if (typeof ra !== "string" || !ra.trim()) return null;
  if (!/[h:]/.test(ra)) {
    const v = Number(ra);
    return Number.isFinite(v) ? v : null;
  }
  const parts = ra.split(/[h:]/).map((p) => Number(p));
  if (parts.some((p) => !Number.isFinite(p))) return null;
  const [h, m = 0, s = 0] = parts;
  return ((h + m / 60 + s / 3600) * 360) / 24;
}

export function parseDecDeg(dec) {
  if (typeof dec !== "string" || !dec.trim()) return null;
  const s = dec.trim();
  const neg = s.startsWith("-");
  if (!/[d°:m]/i.test(s)) {
    const v = Number(s);
    return Number.isFinite(v) ? v : null;
  }
  const parts = s
    .replace(/^[+-]/, "")
    .split(/[d°m:']/i)
    .filter((p) => p !== "")
    .map(Number);
  if (parts.length === 0 || parts.some((p) => !Number.isFinite(p))) return null;
  const [d, m = 0, s2 = 0] = parts;
  const v = d + m / 60 + s2 / 3600;
  return neg ? -v : v;
}

export function countByStatus(items) {
  const counts = { Active: 0, Archived: 0, Exported: 0 };
  for (const p of items) counts[p.status] = (counts[p.status] ?? 0) + 1;
  return counts;
}

export function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  return `${v.toFixed(1)} ${units[i]}`;
}
