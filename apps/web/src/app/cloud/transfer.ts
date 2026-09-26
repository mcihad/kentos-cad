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
}

/** How long to wait before each next try of an upload whose connection was cut. */
export const UPLOAD_WAITS_MS = [1000, 2000, 4000, 8000, 16000];

/**
 * Uploads `bytes` (with their SHA-256) into the project: opens an upload,
 * sends the bytes, and answers the upload once the server has verified
 * them (size, hash, a KCAD v2 it can read). A connection cut while the
 * bytes go is tried again with the same upload (the server kept nothing
 * of it); an upload the server no longer has (expired) is opened again
 * once. A refusal (the file is not what was declared, no right) is thrown
 * with the server's words.
 */
export async function uploadBytes(api: CloudApi, target: UploadTarget, bytes: Uint8Array, sha256: string, o: UploadOptions = {}): Promise<FileUpload> {
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
