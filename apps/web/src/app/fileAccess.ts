import type { DrawingFileHandle } from './fileIO';

/**
 * Reading and writing a chosen file (File System Access, or a stand-in in
 * tests), with what can go wrong on the way said in Turkish (TODOS.md
 * FILE-17, FILE-18, docs/adr/0030):
 *
 * - `writeAccess`: the browser forgets a file's permission between visits
 *   and may take it back; it is asked for again while the user's Ctrl+S or
 *   click still counts, before anything is encoded;
 * - `writeFile`: the browser writes to a copy beside the file and puts it in
 *   place only when the writer closes, so a failed or interrupted write
 *   (permission taken back, disk full, the tab closed) leaves the previous
 *   file as it was; a failed write is aborted so its copy goes too;
 * - `readFile`: a large file is read in chunks, with progress, and can stop
 *   between two.
 */

/** Why a file could not be written, and what the user can do, for a DOMException's name. */
export function writeFailure(name: string, e: unknown): string {
  const what = (e as DOMException | null)?.name;
  const text = e instanceof Error ? e.message : String(e);
  switch (what) {
    case 'NotAllowedError':
    case 'SecurityError':
      return `“${name}” için yazma izni yok ya da geri alındı. Önceki dosya olduğu gibi duruyor; Kaydet'e yeniden basıp izni verin ya da Farklı kaydet ile başka bir yere kaydedin.`;
    case 'QuotaExceededError':
      return `“${name}” yazılamadı: diskte ya da tarayıcının depolama alanında yer kalmadı. Önceki dosya olduğu gibi duruyor; yer açıp yeniden kaydedin ya da Farklı kaydet ile başka bir diske kaydedin.`;
    case 'NotFoundError':
      return `“${name}” yazılamadı: dosya ya da klasörü artık yok (taşınmış ya da silinmiş). Farklı kaydet ile yeni bir yer seçin.`;
    case 'NoModificationAllowedError':
    case 'InvalidStateError':
      return `“${name}” yazılamadı: dosya başka bir program ya da sekme tarafından kullanılıyor ya da değiştirildi (${text}). Önceki dosya olduğu gibi duruyor; biraz sonra yeniden deneyin ya da Farklı kaydet ile başka bir yere kaydedin.`;
    case 'AbortError':
      return `“${name}” yazılması yarıda kesildi. Önceki dosya olduğu gibi duruyor; yeniden kaydedin.`;
    default:
      return `“${name}” yazılamadı: ${text}. Önceki dosya olduğu gibi duruyor; başka bir yere kaydetmeyi deneyin (Farklı kaydet).`;
  }
}

type Permissioned = DrawingFileHandle & {
  queryPermission?(o: { mode: 'readwrite' }): Promise<PermissionState>;
  requestPermission?(o: { mode: 'readwrite' }): Promise<PermissionState>;
};

/**
 * Whether the file may be written: asks the browser again when it forgot or
 * the permission was taken back. Null when it may; else why not (Turkish).
 * A handle without permissions (a stand-in) may be written.
 */
export async function writeAccess(handle: DrawingFileHandle): Promise<string | null> {
  const h = handle as Permissioned;
  try {
    if (!h.queryPermission || (await h.queryPermission({ mode: 'readwrite' })) === 'granted') return null;
    if ((await h.requestPermission?.({ mode: 'readwrite' })) === 'granted') return null;
  } catch {
    // Asking failed (no user activation left): the answer is "not granted".
  }
  return `“${handle.name}” dosyasına yazma izni verilmedi; çizim kaydedilmedi ve önceki dosya olduğu gibi duruyor. Kaydet'e yeniden basıp izni verin ya da Farklı kaydet ile başka bir yere kaydedin.`;
}

/**
 * Writes the bytes: the writer's copy replaces the file only when it closes.
 * On failure the writer is aborted (its copy goes) and the error is thrown;
 * the file was never replaced.
 */
export async function writeFile(handle: DrawingFileHandle, bytes: Uint8Array): Promise<void> {
  const w = await handle.createWritable();
  try {
    await w.write(bytes);
    await w.close();
  } catch (e) {
    await (w as { abort?: () => Promise<void> }).abort?.().catch(() => {});
    throw e;
  }
}

/** Bytes above which a file is read in chunks, with progress. */
const STREAMED = 8 << 20;

/**
 * The largest drawing file the browser opens (docs/adr/0030): a 97 MB file
 * took the page and its worker about 0.85 GB on top of the drawing on screen,
 * so a file near the format's 1 GiB would exhaust a tab. The format allows a
 * platform a smaller limit if it says so (docs/specs/kcad-v2.md §3.4).
 */
export const BROWSER_LIMIT = 256 * 1024 * 1024;

/** Why a file is too large for the browser, or null. */
export function tooLarge(name: string, size: number): string | null {
  if (size <= BROWSER_LIMIT) return null;
  const mb = (n: number) => Math.round(n / (1024 * 1024)).toLocaleString('tr-TR');
  return `“${name}” ${mb(size)} MB; tarayıcı en çok ${mb(BROWSER_LIMIT)} MB'lık bir çizim dosyası açar (daha büyüğü sekmenin belleğini aşar). Dosyayı masaüstü uygulamasıyla açın ya da çizimi birkaç dosyaya bölün.`;
}

/**
 * A file's bytes. A large one is read in chunks into one buffer of its size:
 * `progress` hears how far, and a read whose `stale` turns true stops (null).
 */
export async function readFile(file: Blob, progress?: (done: number, total: number) => void, stale?: () => boolean): Promise<Uint8Array | null> {
  if (file.size < STREAMED || typeof file.stream !== 'function') return new Uint8Array(await file.arrayBuffer());
  const out = new Uint8Array(file.size);
  const reader = file.stream().getReader();
  let at = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    // A file that grew while it was read is read again whole: it is not what was chosen.
    if (at + value.length > out.length) {
      await reader.cancel().catch(() => {});
      return new Uint8Array(await file.arrayBuffer());
    }
    out.set(value, at);
    at += value.length;
    progress?.(at, out.length);
    if (stale?.()) {
      await reader.cancel().catch(() => {});
      return null;
    }
  }
  return at === out.length ? out : out.slice(0, at);
}
