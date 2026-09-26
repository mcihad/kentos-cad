import { ApiFailure, dispositionName, failureOf, type KcadDownload, type Transfer } from './api';

/**
 * A file's bytes from the server (docs/adr/0031, 0033, 0045): read part by
 * part as they come (`progress` hears them), given up only when no byte
 * arrived for a while, not after a fixed time (a large file on a slow line
 * is still coming). A download cut short is tried again: a file that does
 * not change (a revision, a checkpoint) goes on from the bytes that arrived
 * with `Range: bytes=N-` — the server answers 206 with the same entity tag,
 * or the whole file again (200), and a continuation that is not of the same
 * file (another tag, another start) starts over; a file made anew for every
 * request (a database project's snapshot) starts over. At most five tries
 * in a row bring nothing. The caller checks the bytes against the tag.
 */

export interface DownloadOptions {
  progress?: Transfer;
  signal?: AbortSignal;
  /** The file does not change between requests: a cut goes on from the bytes that arrived. */
  resumable?: boolean;
  /** Waits before each next try after a cut (the last one repeats). */
  waits?: readonly number[];
  /** No byte for this long counts as a cut. */
  quietMs?: number;
}

/** The waits between the tries of a download cut short. */
export const DOWNLOAD_WAITS_MS: readonly number[] = [500, 1000, 2000, 4000, 8000];
/** Tries in a row that bring no byte before a download gives up. */
const TRIES = 5;

/** The quoted value of a header, unquoted (`"abc"` → `abc`). */
export const unquoted = (v: string | null) => (v ? v.replace(/^W\//, '').replace(/^"|"$/g, '') : null);

/** Where a 206 answer's bytes start, and the size of the whole file (`Content-Range: bytes N-M/S`). */
function range(h: Headers): { start: number; size: number } | null {
  const m = /^bytes (\d+)-\d+\/(\d+)$/.exec(h.get('content-range') ?? '');
  return m ? { start: Number(m[1]), size: Number(m[2]) } : null;
}

const sleep = (ms: number, signal?: AbortSignal) =>
  new Promise<void>((resolve) => {
    const t = setTimeout(resolve, ms);
    signal?.addEventListener('abort', () => (clearTimeout(t), resolve()), { once: true });
  });

export async function downloadFile(fetcher: typeof fetch, path: string, o: DownloadOptions): Promise<KcadDownload> {
  const progress = o.progress ?? (() => {});
  const waits = o.waits ?? DOWNLOAD_WAITS_MS;
  const quietMs = o.quietMs ?? 30_000;
  const lost = () =>
    new ApiFailure(o.signal?.aborted ? 499 : 0, { error: o.signal?.aborted ? 'aborted' : 'network' }, o.signal?.aborted ? 'İndirme durduruldu.' : 'Sunucuya ulaşılamadı; dosya indirilemedi.');
  /** The bytes held so far, and the answer they began with (its tag, revision, cursor, name). */
  let parts: Uint8Array[] = [];
  let done = 0;
  let total = 0;
  let head: Headers | null = null;
  let tries = 0;
  const restart = () => {
    parts = [];
    done = 0;
    head = null;
  };
  for (;;) {
    if (o.signal?.aborted) throw lost();
    const from = o.resumable && head ? done : 0;
    if (from === 0 && done > 0) restart();
    const before = done;
    const abort = new AbortController();
    let quiet = setTimeout(() => abort.abort(), quietMs);
    const alive = () => {
      clearTimeout(quiet);
      quiet = setTimeout(() => abort.abort(), quietMs);
    };
    const onAbort = () => abort.abort();
    o.signal?.addEventListener('abort', onAbort);
    let cut = false;
    try {
      let res: Response;
      try {
        const headers: Record<string, string> = { accept: 'application/octet-stream, application/json', 'x-kentos-client': 'web' };
        if (from > 0) headers.range = `bytes=${from}-`;
        res = await fetcher(path, { method: 'GET', credentials: 'same-origin', cache: 'no-store', headers, signal: abort.signal });
      } catch {
        if (o.signal?.aborted) throw lost();
        cut = true;
        res = null as unknown as Response;
      }
      if (!cut) {
        if (res.status === 416) {
          // The bytes held are not a start of the file there now: from the beginning.
          restart();
          cut = true;
        } else if (!res.ok) throw failureOf(res.status, await res.text().catch(() => ''));
        else if (res.status === 206) {
          const r = range(res.headers);
          // Not the rest of the same file (another tag or start): from the beginning.
          if (!r || r.start !== from || !head || unquoted(res.headers.get('etag')) !== unquoted(head.get('etag'))) {
            await res.body?.cancel().catch(() => {});
            restart();
            cut = true;
          } else total = r.size;
        } else {
          // The whole file (a first answer, or a server that does not continue): read from its start.
          restart();
          head = res.headers;
          total = Number(res.headers.get('content-length') ?? 0) || 0;
        }
        if (!cut) {
          try {
            if (res.body) {
              const reader = res.body.getReader();
              for (;;) {
                const { done: end, value } = await reader.read();
                if (end) break;
                alive();
                parts.push(value);
                done += value.byteLength;
                progress(done, total);
              }
            } else {
              const all = new Uint8Array(await res.arrayBuffer());
              parts.push(all);
              done += all.byteLength;
              progress(done, total);
            }
          } catch {
            if (o.signal?.aborted) throw lost();
            cut = true;
          }
          // Ended before the size it announced: a cut, not the file.
          if (!cut && total && done < total) cut = true;
        }
      }
    } finally {
      clearTimeout(quiet);
      o.signal?.removeEventListener('abort', onAbort);
    }
    if (!cut) break;
    tries = done > before ? 0 : tries + 1;
    if (tries >= TRIES) throw lost();
    await sleep(waits[Math.min(tries, waits.length - 1)] ?? 0, o.signal);
  }
  const bytes = new Uint8Array(done);
  let at = 0;
  for (const p of parts) (bytes.set(p, at), (at += p.byteLength));
  const h = head as Headers | null;
  return {
    bytes,
    sha256: unquoted(h?.get('etag') ?? null),
    revision: h?.get('x-kentos-revision') ?? null,
    cursor: h?.get('x-kentos-event-cursor') ?? null,
    fileName: dispositionName(h?.get('content-disposition') ?? null),
  };
}
