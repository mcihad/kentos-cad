import { naturalOrder } from './ops/pointEditor';

/**
 * Kaynaklar's folder listing (docs/adr/0199 §7; the desktop's `kentos_interaction::sources`), both platforms held to
 * the shared cases fixtures/sources/v1/cases.json written by scripts/fixtures/source_list_cases.py: a folder shows its
 * folders and the files the panel adds as layers, but those whose name begins with a dot, in the natural order; a
 * file's kind by its extension, case aside; a Shapefile as its .shp with the files of the same name and the extensions
 * .shx, .dbf, .prj and .cpg as its parts, which are not shown on their own.
 */

export type SourceKind = 'geojson' | 'shapefile' | 'dxf' | 'ncz' | 'gnss' | 'coords';

const KINDS: Readonly<Record<string, SourceKind>> = {
  geojson: 'geojson',
  json: 'geojson',
  shp: 'shapefile',
  dxf: 'dxf',
  ncz: 'ncz',
  gpx: 'gnss',
  nmea: 'gnss',
  nma: 'gnss',
  ncn: 'coords',
  txt: 'coords',
  csv: 'coords',
  xyz: 'coords',
  dat: 'coords',
  asc: 'coords',
};

/** What each kind is called in the panel. */
export const SOURCE_LABELS: Readonly<Record<SourceKind, string>> = {
  geojson: 'GeoJSON',
  shapefile: 'Shapefile',
  dxf: 'DXF',
  ncz: 'Netcad NCZ',
  gnss: 'GNSS (GPX, NMEA)',
  coords: 'Koordinat listesi',
};

/** The icon of each kind: its import command's. */
export const SOURCE_ICONS: Readonly<Record<SourceKind, string>> = {
  geojson: 'importGeojson',
  shapefile: 'importShp',
  dxf: 'importDxf',
  ncz: 'importNcz',
  gnss: 'importGnss',
  coords: 'importNcn',
};

const PARTS = ['shx', 'dbf', 'prj', 'cpg'];

export interface SourceEntry {
  name: string;
  dir: boolean;
}

/** A file the panel shows: its name, its kind, the files read with it (a Shapefile's parts; the file itself first). */
export interface SourceFile {
  name: string;
  kind: SourceKind;
  parts: string[];
}

/** The stem and the extension (lower case); no extension without a dot past the first letter. */
function split(name: string): [string, string] {
  const at = name.lastIndexOf('.');
  return at <= 0 ? [name, ''] : [name.slice(0, at), name.slice(at + 1).toLowerCase()];
}

/** A file's kind by its extension; null for one the panel does not add. */
export function sourceKind(name: string): SourceKind | null {
  return KINDS[split(name)[1]] ?? null;
}

const natural = (names: readonly string[]): string[] => naturalOrder(names).map((i) => names[i]);

/** What a folder with these entries shows. */
export function sourceListing(entries: readonly SourceEntry[]): { folders: string[]; files: SourceFile[] } {
  const shown = entries.filter((e) => !e.name.startsWith('.'));
  const folders = natural(shown.filter((e) => e.dir).map((e) => e.name));
  const names = shown.filter((e) => !e.dir).map((e) => e.name);
  const files: SourceFile[] = [];
  for (const name of natural(names)) {
    const kind = sourceKind(name);
    if (!kind) continue;
    const parts = [name];
    if (kind === 'shapefile') {
      const stem = split(name)[0];
      for (const p of PARTS) {
        const part = names.find((f) => {
          const [s, x] = split(f);
          return s === stem && x === p;
        });
        if (part !== undefined) parts.push(part);
      }
    }
    files.push({ name, kind, parts });
  }
  return { folders, files };
}
