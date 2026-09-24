import { Signal } from '../core/signal';
import type { Health } from '../contracts/generated/Health';
import { CONTRACTS_VERSION } from '../contracts/version';

/**
 * Whether the KentOS API is reachable (Faz A: `GET /v1/health` only). The
 * drawing never depends on it: without a server the app works as before and
 * saves to local .kcad files. Checked once the app is idle after start, when
 * the window regains focus or the network comes back, and on request. The
 * full connection state machine with sync and outbox is Faz B (CLAUDE.md §21).
 */

export type ServerState = 'checking' | 'online' | 'offline' | 'incompatible';

const HEALTH_URL = '/v1/health';
const TIMEOUT_MS = 4000;
/** Focus checks closer together than this reuse the last answer. */
const REFOCUS_MS = 15000;

/** A health response read as untrusted data: the contract's fields, or what is wrong with it. */
export function readHealth(value: unknown): { ok: true; health: Health } | { ok: false; error: string } {
  if (!value || typeof value !== 'object') return { ok: false, error: 'yanıt bir nesne değil' };
  const v = value as Record<string, unknown>;
  if (v.status !== 'ok') return { ok: false, error: 'durum “ok” değil' };
  if (typeof v.service !== 'string' || typeof v.version !== 'string') return { ok: false, error: 'hizmet adı ya da sürümü eksik' };
  if (v.commit !== undefined && typeof v.commit !== 'string') return { ok: false, error: 'commit metin değil' };
  if (typeof v.contracts !== 'number' || !Number.isInteger(v.contracts)) return { ok: false, error: 'sözleşme sürümü eksik' };
  return { ok: true, health: { status: 'ok', service: v.service, version: v.version, commit: v.commit as string | undefined, contracts: v.contracts } };
}

export class ServerStatus {
  readonly state = new Signal<ServerState>('checking');
  /** The last good answer (kept while incompatible, to show its version). */
  readonly health = new Signal<Health | null>(null);
  /** Why the server counts as unreachable or incompatible, for the tooltip. */
  readonly detail = new Signal('');
  private readonly fetcher: typeof fetch;
  private checkedAt = -Infinity;
  private pending: Promise<ServerState> | null = null;

  constructor(fetcher: typeof fetch = (...a) => fetch(...a)) {
    this.fetcher = fetcher;
  }

  /** Asks the server now (a check already under way is shared). */
  check(): Promise<ServerState> {
    this.pending ??= this.ask().finally(() => (this.pending = null));
    return this.pending;
  }

  /** Re-checks when the window regains focus (at most every 15 s) or the network returns. */
  watch(win: Window): () => void {
    const onFocus = () => performance.now() - this.checkedAt >= REFOCUS_MS && void this.check();
    const onOnline = () => void this.check();
    win.addEventListener('focus', onFocus);
    win.addEventListener('online', onOnline);
    return () => {
      win.removeEventListener('focus', onFocus);
      win.removeEventListener('online', onOnline);
    };
  }

  private async ask(): Promise<ServerState> {
    if (this.state.value !== 'online') this.state.set('checking');
    const abort = new AbortController();
    const timer = setTimeout(() => abort.abort(), TIMEOUT_MS);
    try {
      const res = await this.fetcher(HEALTH_URL, { cache: 'no-store', headers: { accept: 'application/json' }, signal: abort.signal });
      if (!res.ok) return this.settle('offline', res.status === 503 ? 'API çalışmıyor.' : `Sunucu ${res.status} yanıtı verdi.`);
      let body: unknown;
      try {
        body = await res.json();
      } catch {
        return this.settle('offline', 'Yanıt bir KentOS sunucusundan gelmiyor.');
      }
      const read = readHealth(body);
      if (!read.ok) return this.settle('offline', `Yanıt KentOS sağlık sözleşmesine uymuyor: ${read.error}.`);
      this.health.set(read.health);
      if (read.health.contracts !== CONTRACTS_VERSION)
        return this.settle('incompatible', `Sunucu sözleşme sürümü ${read.health.contracts}, uygulama ${CONTRACTS_VERSION} bekliyor. Uygulamayı ya da sunucuyu güncelleyin.`);
      return this.settle('online', '');
    } catch {
      return this.settle('offline', abort.signal.aborted ? `Sunucu ${TIMEOUT_MS / 1000} saniyede yanıt vermedi.` : 'Sunucuya ulaşılamadı.');
    } finally {
      clearTimeout(timer);
    }
  }

  private settle(state: ServerState, detail: string): ServerState {
    this.checkedAt = performance.now();
    if (state === 'offline') this.health.set(null);
    this.detail.set(detail);
    this.state.set(state);
    return state;
  }
}
