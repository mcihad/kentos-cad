import type { BottomTab, LogLevel } from '../../app/state';

/**
 * How the web writes its log (CLAUDE.md §4.10, DESIGN.md §7.7): a line's
 * time and level in the bottom panel's Komut geçmişi and Uyarılar
 * (BottomPanel.ts), which lines each tab lists, what the user typed as the
 * history echoes it, and which lines the status bar shows and for how long
 * (StatusBar.ts). Apart from the DOM; fixtures/shell/v1/log.json holds it for
 * the desktop (format in fixtures/shell/README.md), with the rows' look as
 * panels.css draws it. How many lines are kept is MessageLog's (app/state.ts),
 * the Uyarılar badge's count warnings.ts's.
 */

/** The panel's tabs, in order: their names and icons. */
export const BOTTOM_TABS: readonly { id: BottomTab; label: string; icon: string }[] = [
  { id: 'history', label: 'Komut geçmişi', icon: 'history' },
  { id: 'coords', label: 'Koordinat listesi', icon: 'table' },
  { id: 'points', label: 'Noktalar', icon: 'pointEditor' },
  { id: 'table', label: 'Tablo', icon: 'featureTable' },
  { id: 'search', label: 'Arama', icon: 'dataSearch' },
  // Topoloji kuralları' findings (docs/adr/0202 §5).
  { id: 'topology', label: 'Topoloji', icon: 'topologyCheck' },
  { id: 'messages', label: 'Uyarılar', icon: 'warning' },
];

/** The panel's words: its tab row's name, its two buttons, the button on the command line, its edge, the empty lists. */
export const BOTTOM_TEXTS = {
  tabs: 'Alt panel',
  clear: 'Geçmişi temizle',
  close: 'Paneli kapat',
  open: 'Komut geçmişini aç',
  edge: 'Alt panel yüksekliği',
  empty: {
    history: 'Henüz komut çalıştırılmadı. Bir araç seçin ya da komut satırına yazın.',
    messages: 'Uyarı yok.',
  },
} as const;

const two = (n: number) => String(n).padStart(2, '0');

/**
 * A line's time: the local wall clock when it was written, as hours,
 * minutes and seconds, 24-hour, two digits each (“09:05:07”); the second
 * is the one the clock shows, never rounded up.
 */
export function logTime(at: Date): string {
  return `${two(at.getHours())}:${two(at.getMinutes())}:${two(at.getSeconds())}`;
}

/** The icons' size in the lists and in the status bar's message (CSS px). */
export const ICON_SIZE = 14;

/** A level's icon in the lists: none for a command or a plain line. */
export const LEVEL_ICON: Record<LogLevel, string | null> = { command: null, info: null, success: 'success', warn: 'warning', error: 'error' };

/** Whether a tab lists a line of this level: Komut geçmişi every line, Uyarılar the warnings and errors. */
export function listedIn(tab: 'history' | 'messages', level: LogLevel): boolean {
  return tab === 'history' || level === 'warn' || level === 'error';
}

/** New lines scroll the list to its end only when it was this close to its end (CSS px): one being read stays put. */
export const FOLLOW_WITHIN = 24;

/** What the user typed or chose (a value, an option's letter), as the history writes it before it is handled. */
export const echo = (text: string): string => `› ${text}`;

/**
 * Whether the status bar shows a line, with its icon and for how long (ms):
 * a command's line never, nor a line that goes on the one before it
 * (written indented, two spaces first: a clicked point, a distance); a
 * warning or an error for 9 s, anything else for 5 s.
 */
export function flashOf(level: LogLevel, text: string): { icon: string; ms: number } | null {
  if (level === 'command' || text.startsWith('  ')) return null;
  const warns = level === 'warn' || level === 'error';
  return { icon: level === 'warn' ? 'warning' : level === 'error' ? 'error' : level === 'success' ? 'success' : 'info', ms: warns ? 9000 : 5000 };
}
