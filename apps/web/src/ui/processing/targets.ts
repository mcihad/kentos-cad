import type { ExecutionTarget } from '../../processing/types';

/** Where a tool can run, as the dialog's "Nerede çalışır" list names it. */
export const TARGET_LABEL: Record<ExecutionTarget, string> = {
  client: 'Bu tarayıcıda',
  worker: 'Arka planda (worker)',
  server: 'KentOS sunucusunda',
  postgis: 'PostGIS veritabanında',
};

/** Where a run happened, lower-case, for "şimdi: …" and history rows (the history panel needs it without the dialog). */
export const TARGET_SHORT: Record<ExecutionTarget, string> = {
  client: 'bu tarayıcıda',
  worker: 'arka planda',
  server: 'sunucuda',
  postgis: 'PostGIS’te',
};
