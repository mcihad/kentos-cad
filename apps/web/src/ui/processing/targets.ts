import type { ExecutionTarget } from '../../processing/types';

/** Where a run happened, lower-case, for "şimdi: …" and history rows (the history panel needs it without the dialog). */
export const TARGET_SHORT: Record<ExecutionTarget, string> = {
  client: 'bu tarayıcıda',
  worker: 'arka planda',
  server: 'sunucuda',
  postgis: 'PostGIS’te',
};
