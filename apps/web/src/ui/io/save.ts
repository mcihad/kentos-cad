import type { AppContext } from '../../app/context';
import type { DrawingFileHandle, FileKind } from '../../app/fileIO';
import { h } from '../dom';

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/**
 * Asks where to write an exported file (through `ctx.files.picker`, which
 * the smoke test replaces) and writes it: the name it was written as, or
 * null when the user cancelled or it failed (said in the log). Where the
 * browser cannot write files the bytes are offered as a download. The
 * drawing itself is not affected either way: an export is not a save.
 */
export async function saveExport(ctx: AppContext, bytes: Uint8Array, suggestedName: string, kind: FileKind): Promise<string | null> {
  let handle: DrawingFileHandle | null | undefined;
  try {
    handle = await ctx.files.picker.save(suggestedName, kind);
  } catch (e) {
    ctx.log.error(`Kaydetme penceresi açılamadı: ${message(e)}. Tarayıcının dosya iznini denetleyin.`);
    return null;
  }
  if (handle === null) return null;
  if (handle === undefined) {
    const url = URL.createObjectURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: Object.keys(kind.accept)[0] ?? 'application/octet-stream' }));
    const a = h('a', { href: url, download: suggestedName });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return suggestedName;
  }
  try {
    const w = await handle.createWritable();
    await w.write(bytes);
    await w.close();
  } catch (e) {
    ctx.log.error(`“${handle.name}” yazılamadı: ${message(e)}. Başka bir yere kaydetmeyi deneyin.`);
    return null;
  }
  return handle.name;
}
