import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { FileUpload } from '../../contracts/generated/FileUpload';
import { ApiFailure, type CloudApi, type KcadDownload, type Transfer } from './api';

/**
 * Moving a `.kcad` file between this browser and a cloud project
 * (docs/adr/0031, 0033, 0036, 0038): its SHA-256, an upload that survives a
 * cut-off connection, a download checked against the server's hash, and the
 * commands that make an upload a file project's revision or a new database
 * project's content. Nothing here touches the DOM or the drawing.
 */

/** The file project's revision key in `expectedVersions` (ADR 0031). */
export const FILE_KEY = '@file';

/** The bytes' SHA-256, lowercase hex, as the server declares and checks it. */
export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes as Uint8Array<ArrayBuffer>));
  let out = '';
  for (const b of digest) out += b.toString(16).padStart(2, '0');
  return out;
}

/** A size as the interface writes it: 812 bayt, 12,4 KB, 48,3 MB. */
export function sizeText(bytes: number): string {
  const n = (v: number) => v.toLocaleString('tr-TR', { maximumFractionDigits: 1 });
  if (bytes < 1024) return `${bytes.toLocaleString('tr-TR')} bayt`;
  if (bytes < 1024 * 1024) return `${n(bytes / 1024)} KB`;
  return `${n(bytes / (1024 * 1024))} MB`;
}

/**
 * The server no longer has this upload (expired, committed, not this
 * account's): a 404 of its own words, told apart from the project's own 404
 * (“Proje bulunamadı.”, access taken away, docs/adr/0015).
 */
export const uploadGone = (e: unknown): boolean => e instanceof ApiFailure && e.notFound && e.message.startsWith('Yükleme bulunamadı');

/** Where an upload goes. */
export interface UploadTarget {
  tenantId: string;
  projectId: string;
}

export interface UploadOptions {
  /** The bytes sent so far, of all. */
  progress?: Transfer;
  /** The last byte went: the server is reading and checking them. */
  verifying?: () => void;
  signal?: AbortSignal;
  /** Waits before each next try of a cut-off `PUT` (then it fails and says why). */
  waits?: readonly number[];
  /** A file larger than this goes in parts of this size (tests use a small one; docs/adr/0045). */
  part?: number;
}

/** How long to wait before each next try of an upload whose connection was cut. */
export const UPLOAD_WAITS_MS = [1000, 2000, 4000, 8000, 16000];

/**
 * A file larger than this goes to the server in parts of this size
 * (docs/adr/0045): the desktop's part, a quarter of the most one request
 * takes (32 MiB). A cut then costs only the part on its way.
 */
export const UPLOAD_PART = 8 * 1024 * 1024;

/**
 * The upload, when its bytes already arrived and only the answer to them
 * was lost (docs/adr/0040): the server refuses them a second time. Null
 * when they did not, or when it cannot be told (a server without the
 * route, the connection still down): the bytes are then sent again, and
 * that send says whether the upload is still there. With `any`, the upload
 * as it stands also before all its bytes arrived (parts, docs/adr/0045).
 */
async function arrived(api: CloudApi, target: UploadTarget, upload: string, signal?: AbortSignal, any = false): Promise<FileUpload | null> {
  try {
    const state = await api.uploadState(target.tenantId, target.projectId, upload, signal);
    return state.received || any ? state : null;
  } catch {
    return null;
  }
}

/**
 * Uploads `bytes` (with their SHA-256) into the project: opens an upload,
 * sends the bytes, and answers the upload once the server has verified
 * them (size, hash, a KCAD v2 it can read). A connection cut while the
 * bytes go is tried again with the same upload (the server kept nothing
 * of a body cut short); before that the upload is asked whether its bytes
 * arrived after all, so bytes whose answer was lost are not sent twice.
 * An upload the server no longer has (expired) is opened again once. A
 * refusal (the file is not what was declared, no right) is thrown with
 * the server's words.
 */
export async function uploadBytes(api: CloudApi, target: UploadTarget, bytes: Uint8Array, sha256: string, o: UploadOptions = {}): Promise<FileUpload> {
  const part = o.part ?? UPLOAD_PART;
  if (bytes.byteLength > part) return uploadInParts(api, target, bytes, sha256, part, o);
  const waits = o.waits ?? UPLOAD_WAITS_MS;
  const begin = () => api.beginUpload(target.tenantId, target.projectId, { size: bytes.byteLength, sha256 });
  let upload = await begin();
  let reopened = false;
  for (let tries = 0; ; tries++) {
    try {
      o.progress?.(0, bytes.byteLength);
      return await api.sendUpload(target.tenantId, target.projectId, upload.id, bytes, (done, total) => {
        o.progress?.(done, total);
        if (done >= bytes.byteLength) o.verifying?.();
      }, o.signal);
    } catch (e) {
      if (!(e instanceof ApiFailure) || o.signal?.aborted) throw e;
      // Gone meanwhile (it expired): a new upload, once.
      if (uploadGone(e) && !reopened) {
        reopened = true;
        upload = await begin();
        continue;
      }
      if (!e.transient || tries >= waits.length) throw e;
      await new Promise((resolve) => setTimeout(resolve, Math.max(waits[tries], (e.retryAfter ?? 0) * 1000)));
      const done = await arrived(api, target, upload.id, o.signal);
      if (done) {
        o.progress?.(bytes.byteLength, bytes.byteLength);
        return done;
      }
    }
  }
}

/** How many of an upload's bytes the server holds, when it says (servers before parts do not). */
const held = (u: FileUpload): number | null => (u.receivedBytes === undefined ? null : Number(u.receivedBytes));

/**
 * The file in parts (docs/adr/0045): each goes on from the bytes the server
 * holds (`?offset=N`). After a cut, or a part the server refused as out of
 * step, the upload is asked how far it came and the next part goes on from
 * there; so a part whose answer was lost is not sent twice, and the last
 * one's lost answer finds the file verified. The tries without progress
 * are counted per part; an expired upload is opened again once.
 */
async function uploadInParts(api: CloudApi, target: UploadTarget, bytes: Uint8Array, sha256: string, part: number, o: UploadOptions): Promise<FileUpload> {
  const waits = o.waits ?? UPLOAD_WAITS_MS;
  const size = bytes.byteLength;
  const begin = () => api.beginUpload(target.tenantId, target.projectId, { size, sha256 });
  let upload = await begin();
  let at = 0;
  let reopened = false;
  let tries = 0;
  o.progress?.(0, size);
  for (;;) {
    const from = at;
    const end = Math.min(from + part, size);
    try {
      const answer = await api.sendUpload(
        target.tenantId,
        target.projectId,
        upload.id,
        bytes.subarray(from, end),
        (done) => {
          o.progress?.(from + done, size);
          if (from + done >= size) o.verifying?.();
        },
        o.signal,
        from,
      );
      if (answer.received) return answer;
      at = held(answer) ?? end;
      if (at > from) tries = 0;
      continue;
    } catch (e) {
      if (!(e instanceof ApiFailure) || o.signal?.aborted) throw e;
      // Gone meanwhile (it expired): a new upload from the start, once.
      if (uploadGone(e) && !reopened) {
        reopened = true;
        upload = await begin();
        at = 0;
        tries = 0;
        continue;
      }
      const outOfStep = e.code === 'invalid' && e.path === 'offset';
      if ((!e.transient && !outOfStep) || tries >= waits.length) throw e;
      if (!outOfStep) await new Promise((resolve) => setTimeout(resolve, Math.max(waits[tries], (e.retryAfter ?? 0) * 1000)));
      tries++;
    }
    // Where the server is now: a part whose answer was lost may be there; the last one completes the file.
    const state = await arrived(api, target, upload.id, o.signal, true);
    if (state?.received) {
      o.progress?.(size, size);
      return state;
    }
    const got = state ? held(state) : null;
    if (got !== null && got !== at) {
      if (got > at) tries = 0;
      at = got;
    }
  }
}

/**
 * A downloaded file checked against the server's SHA-256 (its `ETag`) and,
 * when the caller knows it from a list, the one listed: bytes that changed
 * on the way never become a drawing. Throws with what to do.
 */
export async function verifyDownload(d: KcadDownload, listed?: string | null): Promise<void> {
  const want = listed ?? d.sha256;
  if (!want) return;
  const got = await sha256Hex(d.bytes);
  if (got !== want || (d.sha256 && d.sha256 !== want))
    throw new Error(`İndirilen dosya sunucudakiyle aynı değil (SHA-256 tutmuyor); dosya kullanılmadı. Bağlantınızı denetleyip yeniden deneyin.`);
}

const envelope = (commandName: string, target: UploadTarget, input: unknown, expectedVersions: Record<string, string> = {}): CommandEnvelope => ({
  commandName,
  version: 1,
  tenantId: target.tenantId,
  projectId: target.projectId,
  requestId: `web-${crypto.randomUUID()}`,
  idempotencyKey: crypto.randomUUID(),
  expectedVersions,
  input: input as CommandEnvelope['input'],
});

/**
 * `project.file.commit` v1: the verified upload becomes the revision after
 * `base` ("0" for the first). Built once and sent again unchanged after a
 * lost answer: the server answers the same key from its log.
 */
export function commitEnvelope(target: UploadTarget, uploadId: string, base: string): CommandEnvelope {
  return envelope('project.file.commit', target, { uploadId }, { [FILE_KEY]: base });
}

/** `project.import` v1: the verified upload into a new, empty database project (docs/adr/0036). */
export function importEnvelope(target: UploadTarget, uploadId: string): CommandEnvelope {
  return envelope('project.import', target, { uploadId });
}

/** The revision a refused commit found on the server (`@file` conflict), or null for any other failure. */
export function fileConflict(e: unknown): { expected: string; actual: string } | null {
  if (!(e instanceof ApiFailure) || e.code !== 'conflict') return null;
  const c = e.conflicts.find((x) => x.id === FILE_KEY);
  return c ? { expected: c.expected ?? '', actual: c.actual ?? '' } : null;
}
