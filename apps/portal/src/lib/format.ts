// How the portal writes times, days, sizes and names (DESIGN.md §10): Turkish calendar words, a dot before decimals.

const DAY = new Intl.DateTimeFormat('tr-TR', { day: 'numeric', month: 'long', year: 'numeric' });
const DAY_SHORT = new Intl.DateTimeFormat('tr-TR', { day: 'numeric', month: 'short' });
const TIME = new Intl.DateTimeFormat('tr-TR', { hour: '2-digit', minute: '2-digit' });

/** "17 Kasım 2026". */
export function day(iso: string | null | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso.length === 10 ? `${iso}T12:00:00` : iso);
  return Number.isNaN(d.getTime()) ? '—' : DAY.format(d);
}

/** "17 Kasım 2026 14:32". */
export function moment(iso: string | null | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '—' : `${DAY.format(d)} ${TIME.format(d)}`;
}

/** "bugün 14:32", "dün 09:10", "12 Kas" for older ones. */
export function since(iso: string | null | undefined, now = new Date()): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '—';
  const days = Math.floor((startOf(now) - startOf(d)) / 86_400_000);
  if (days === 0) return `bugün ${TIME.format(d)}`;
  if (days === 1) return `dün ${TIME.format(d)}`;
  return DAY_SHORT.format(d);
}

function startOf(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** A legal or requested deadline in words: "5 gün kaldı", "bugün son gün", "2 gün geçti". */
export function daysLeft(n: number): string {
  if (n > 0) return `${n} gün kaldı`;
  if (n === 0) return 'bugün son gün';
  return `${-n} gün geçti`;
}

/** Days from today to `iso` (YYYY-MM-DD), Turkey's calendar as the browser's. */
export function daysUntil(iso: string, now = new Date()): number {
  const target = new Date(`${iso}T12:00:00`);
  return Math.round((startOf(target) - startOf(now)) / 86_400_000);
}

/** "1.4 MB", "820 KB", "512 B". */
export function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/** Two letters of a name for its avatar: "Ayşe Yılmaz" → "AY". */
export function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  const first = parts[0]?.[0] ?? '?';
  const last = parts.length > 1 ? parts[parts.length - 1][0] : '';
  return `${first}${last}`.toLocaleUpperCase('tr-TR');
}

/** A short name for a URL from an organisation's name: "Örnek Belediyesi" → "ornek-belediyesi". */
export function slugOf(name: string): string {
  const map: Record<string, string> = { ç: 'c', ğ: 'g', ı: 'i', i: 'i', ö: 'o', ş: 's', ü: 'u' };
  return name
    .toLocaleLowerCase('tr-TR')
    .replace(/[çğıiöşü]/g, (c) => map[c] ?? c)
    .normalize('NFKD')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 63);
}

/** "2 dosya", "1 dosya": counts are written before their noun, never pluralised in Turkish. */
export function count(n: number, noun: string): string {
  return `${n} ${noun}`;
}
