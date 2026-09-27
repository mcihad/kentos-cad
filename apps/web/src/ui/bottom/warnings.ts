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

/**
 * The newest id seen: while the Uyarılar tab is on screen (the panel open
 * on it) every line so far is seen; otherwise, and over an empty log, it
 * stays (fixtures/shell/v1/log.json `badge`).
 */
export function seenNow(entries: readonly LogEntry[], seenUpTo: number, onScreen: boolean): number {
  return onScreen ? (entries.at(-1)?.id ?? seenUpTo) : seenUpTo;
}
