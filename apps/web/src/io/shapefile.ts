import type { PickedFile } from '../app/fileIO';
import type { ShapefileFiles } from './client';

/**
 * A Shapefile layer among the files a user chose together (docs/adr/0046):
 * the one .shp and the .shx, .dbf, .prj and .cpg of its name (compared
 * without case). A file of another name or kind is not used, and said; two
 * .shp files are two layers, which are imported one at a time.
 */
export type ShapefileSet = { name: string; files: ShapefileFiles; parts: string[]; unused: string[] } | { error: string };

const PARTS = ['shx', 'dbf', 'prj', 'cpg'] as const;

const extension = (name: string) => /\.([^.]+)$/.exec(name)?.[1]?.toLowerCase() ?? '';
const stem = (name: string) => name.replace(/\.[^.]+$/, '') || name;

export function shapefileSet(files: readonly PickedFile[]): ShapefileSet {
  const shps = files.filter((f) => extension(f.name) === 'shp');
  if (!shps.length) return { error: 'Seçilen dosyalarda .shp yok. Shapefile katmanının .shp dosyasını, yanındaki .shx, .dbf, .prj ve .cpg ile birlikte seçin.' };
  if (shps.length > 1)
    return {
      error: `Seçilen dosyalarda ${shps.length} .shp var (${shps.map((f) => f.name).join(', ')}). Bir seferde bir katman alınır: “Başka dosya…” ile bir katmanın dosyalarını seçin.`,
    };
  const shp = shps[0]!;
  const name = stem(shp.name);
  const key = name.toLowerCase();
  const set: ShapefileFiles = { shp: shp.bytes };
  const parts = ['.shp'];
  const unused: string[] = [];
  for (const f of files) {
    if (f === shp) continue;
    const ext = extension(f.name);
    const part = PARTS.find((p) => p === ext);
    if (part && stem(f.name).toLowerCase() === key && !set[part]) {
      set[part] = f.bytes;
      parts.push(`.${part}`);
    } else unused.push(f.name);
  }
  return { name, files: set, parts, unused };
}
