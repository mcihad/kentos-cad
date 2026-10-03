/**
 * The drawing's typefaces for the PDF (docs/sheet/design.md §9a; ADR 0055):
 * the TrueType files the desktop has (`apps/desktop/assets/fonts/drawing`,
 * made from the web's own faces by `scripts/fonts/drawing_fonts.py`), so
 * the core's writer embeds the same bytes whichever platform asks. They are
 * fetched when a PDF is made, never before: each is its own asset in the
 * build, and the glob below only names them (nothing is loaded until a
 * file is asked for).
 */

const FILES = import.meta.glob<string>('../../../../desktop/assets/fonts/drawing/*.ttf', { query: '?url', import: 'default' });

/** The file names there (`Arimo-400.ttf` …): what the core may ask for. */
export const FONT_FILES: readonly string[] = Object.keys(FILES)
  .map((p) => p.slice(p.lastIndexOf('/') + 1))
  .sort();

const loaded = new Map<string, Promise<Uint8Array>>();

/** A face's bytes by its file name; refused with the reason when there is no such file or it could not be fetched. */
export function fontFile(name: string): Promise<Uint8Array> {
  const hit = loaded.get(name);
  if (hit) return hit;
  const entry = Object.entries(FILES).find(([p]) => p.endsWith(`/${name}`));
  if (!entry) return Promise.reject(new Error(`“${name}” yazı tipi dosyası uygulamada yok.`));
  const job = entry[1]()
    .then((url) =>
      fetch(url).catch(() => {
        throw new Error(`“${name}” yazı tipi indirilemedi: sunucuya ulaşılamadı.`);
      }),
    )
    .then(async (res) => {
      if (!res.ok) throw new Error(`“${name}” yazı tipi indirilemedi (${res.status}).`);
      return new Uint8Array(await res.arrayBuffer());
    });
  // A failed fetch is tried again next time.
  loaded.set(name, job);
  job.catch(() => loaded.delete(name));
  return job;
}

/** Several faces' bytes, by file name. */
export async function fontFiles(names: readonly string[]): Promise<Map<string, Uint8Array>> {
  const out = new Map<string, Uint8Array>();
  await Promise.all([...new Set(names)].map(async (n) => out.set(n, await fontFile(n))));
  return out;
}
