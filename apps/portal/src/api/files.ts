// Files sent to the cloud (docs/api/README.md §6.1): the file's SHA-256 first, then its parts from where the server
// says the upload stands, so a connection that breaks costs one part and the upload goes on by itself. The last part
// is checked and scanned on the server; a refused file comes back as the server's sentence.
import { ApiError, get, post, putBytes } from './client';
import type { UploadView } from './types';

/** A part's size: small enough that a mobile connection resends little, large enough to be few requests. */
const PART = 8 * 1024 * 1024;

export type Phase = 'hash' | 'send' | 'scan';

export async function sha256Hex(file: Blob): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', await file.arrayBuffer());
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0')).join('');
}

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Sends `file` through `/v1/uploads`; the received upload, ready for a dossier or a project to take. */
export async function sendFile(file: File, progress?: (done: number, total: number, phase: Phase) => void): Promise<UploadView> {
  progress?.(0, file.size, 'hash');
  const sha256 = await sha256Hex(file);
  let up = await post<UploadView>('/v1/uploads', { name: file.name, size: file.size, sha256 });
  let failures = 0;
  while (!up.received) {
    const from = up.receivedBytes;
    const to = Math.min(from + Math.min(PART, up.partMax), file.size);
    progress?.(from, file.size, to === file.size ? 'scan' : 'send');
    try {
      up = await putBytes<UploadView>(`/v1/uploads/${up.id}?offset=${from}`, file.slice(from, to));
      failures = 0;
    } catch (e) {
      // A refusal (the file, the offset) is the server's answer; a broken connection or a busy server is tried again.
      const again = e instanceof ApiError && (e.status === 0 || e.status === 429 || e.status >= 500);
      if (!again || ++failures > 5) throw e;
      await wait(1000 * failures);
      up = await get<UploadView>(`/v1/uploads/${up.id}`);
    }
  }
  progress?.(file.size, file.size, 'scan');
  return up;
}

/** How far a sending file is, in words. */
export function phaseText(done: number, total: number, phase: Phase): string {
  if (phase === 'hash') return 'hazırlanıyor';
  if (phase === 'scan') return 'denetleniyor';
  return `%${total ? Math.floor((done / total) * 100) : 0} gönderildi`;
}
