import type { KcadDownload, Transfer } from '../../app/cloud/api';
import { sizeText, verifyDownload } from '../../app/cloud/transfer';
import type { AppContext } from '../../app/context';
import { writeFile, writeFailure } from '../../app/fileAccess';
import { DOCUMENT_MIME } from '../../model/snapshot';
import { h } from '../dom';

/**
 * A `.kcad` the server sends, saved where the user chooses (docs/adr/0038):
 * a file project's revision, a database project's snapshot of one moment
 * (docs/adr/0033) or a checkpoint's file (docs/adr/0034). The place is asked
 * first, while the click still counts (the browser's own save dialog); then
 * the bytes come, with how far, are checked against the server's SHA-256,
 * and only then written. Where the browser cannot write files they are
 * handed over as a download. Nothing here changes the drawing on screen.
 */

export interface DownloadRequest {
  /** The name the save dialog suggests (with `.kcad`). */
  name: string;
  /** Gets the bytes (with progress; `signal` stops it). */
  fetch: (progress: Transfer, signal?: AbortSignal) => Promise<KcadDownload>;
  /** The SHA-256 a list gave for it, checked besides the server's own. */
  listed?: string | null;
  /** How far it is, in words (the window that asked shows it). */
  say?: (text: string, fraction: number) => void;
  signal?: AbortSignal;
}

const withKcad = (name: string) => (name.toLowerCase().endsWith('.kcad') ? name : `${name}.kcad`);
/** A file name without what file systems refuse. */
const safe = (name: string) => name.replace(/[\\/:*?"<>|\u0000-\u001f]/g, '_').trim() || 'proje';

/** Downloads and saves; true when the file was written (or handed over as a download). Failures are said in the log. */
export async function downloadKcad(ctx: AppContext, r: DownloadRequest): Promise<boolean> {
  const name = withKcad(safe(r.name));
  let handle;
  try {
    handle = await ctx.files.picker.save(name);
  } catch (e) {
    ctx.log.error(`Kaydetme penceresi açılamadı: ${e instanceof Error ? e.message : String(e)}. Tarayıcının dosya iznini denetleyin.`);
    return false;
  }
  if (handle === null) return false;
  let d: KcadDownload;
  try {
    d = await r.fetch((done, total) => r.say?.(`İndiriliyor: ${sizeText(done)}${total ? ` / ${sizeText(total)}` : ''}`, total ? done / total : 0), r.signal);
    r.say?.('İndirilen dosya denetleniyor (SHA-256)…', 1);
    await verifyDownload(d, r.listed);
  } catch (e) {
    ctx.log.error(`“${name}” indirilemedi: ${e instanceof Error ? e.message : String(e)}`);
    return false;
  }
  if (handle) {
    try {
      await writeFile(handle, d.bytes);
    } catch (e) {
      ctx.log.error(writeFailure(handle.name, e));
      return false;
    }
    ctx.log.success(`“${handle.name}” indirildi: ${sizeText(d.bytes.byteLength)}${d.revision ? `, revizyon ${d.revision}` : ''}.`);
    return true;
  }
  // No file access in this browser: a download (the browser keeps it where it keeps downloads).
  const url = URL.createObjectURL(new Blob([d.bytes], { type: DOCUMENT_MIME }));
  const a = h('a', { href: url, download: name });
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  ctx.log.success(`“${name}” indirme olarak verildi: ${sizeText(d.bytes.byteLength)}.`);
  return true;
}
