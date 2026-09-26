import type { LogEntry } from '../../app/state';

/**
 * The Uyarılar tab's badge: the warnings and errors logged after `seenUpTo`,
 * the id of the newest log entry when the tab was last on screen. The log's
 * ids only grow, so neither Geçmişi temizle nor the log dropping its oldest
 * entries (it keeps 500) can hide a new warning, as a count of the warnings
 * seen did (that count stayed above the log's after a clear from another tab).
 */
export function unseenWarnings(entries: readonly LogEntry[], seenUpTo: number): number {
  let n = 0;
  for (const e of entries) if ((e.level === 'warn' || e.level === 'error') && e.id > seenUpTo) n++;
  return n;
}
