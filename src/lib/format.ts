// Shared formatting helpers extracted from components. Semantics match the
// original inline implementations exactly.

function scale1024(n: number, units: readonly string[]): { v: number; u: string } {
  let v = n / 1024;
  let u = 0;
  while (v >= 1024 && u < units.length - 1) { v /= 1024; u++ }
  return { v, u: units[u] };
}

function fixed(v: number): string {
  return v.toFixed(v < 10 ? 2 : v < 100 ? 1 : 0);
}

export function fmtBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const { v, u } = scale1024(n, ["KB", "MB", "GB", "TB"]);
  return `${fixed(v)} ${u}`;
}

export function fmtRate(bytesPerSec: number): { v: string; u: string } {
  if (bytesPerSec < 1024) return { v: bytesPerSec.toFixed(0), u: "B/s" };
  const { v, u } = scale1024(bytesPerSec, ["KB/s", "MB/s", "GB/s"]);
  return { v: fixed(v), u };
}

export function fmtTs(ts: number | null | undefined): string {
  if (ts == null) return "never";
  const d = new Date(ts * 1000);
  return d.toLocaleString();
}

export function daysFrom(now: number, ts: number): number {
  return Math.ceil((ts - now) / 86_400_000);
}
