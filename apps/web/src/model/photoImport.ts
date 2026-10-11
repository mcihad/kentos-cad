import type { Attachment } from '../contracts/generated/Attachment';
import type { Entity as ContractEntity } from '../contracts/generated/Entity';
import type { LineError } from '../contracts/generated/LineError';
import type { PhotoExif } from '../contracts/generated/PhotoExif';
import { fixed } from '../core/displayNumber';
import { crsTransformIn, type DatumChoice, type System, type Unreached } from './geom/crsTransform';
import { accuracyText } from './secondCrs';

/**
 * Konumlu fotoğraflar (docs/adr/0225 §3; the desktop's `kentos_interaction::photos`): photos whose EXIF gives a place,
 * moved from WGS 84 into the project's system the way Koordinat dönüştür moves them (docs/adr/0167, the project's datum
 * choices taken), each a point named after its file, with when it was taken, the direction it looked, its height and the
 * camera as attributes, and the photo itself as its attachment. A photo without a place, or one the system does not
 * reach, is said and left out. The height is an attribute, never an elevation: EXIF does not say its datum. The shared
 * cases are fixtures/photos/v1/import.json (scripts/fixtures/photo_import_cases.py, with PROJ).
 */

/** A photo as the window read it: its file's name, what its EXIF says and where its bytes are kept. */
export interface Photo {
  readonly name: string;
  readonly exif: PhotoExif;
  readonly asset?: string;
  readonly file?: string;
}

/** A photo as the import writes it, and its place in the list (1 for the first). */
export interface PlacedPhoto {
  readonly index: number;
  readonly name: string;
  readonly p: { readonly x: number; readonly y: number };
  readonly attrs: Readonly<Record<string, string>>;
  readonly attachment: Attachment;
}

export interface PhotoPlan {
  readonly placed: PlacedPhoto[];
  readonly skipped: LineError[];
}

/** What is said of a photo without a place, and of one the project's system does not reach. */
export const NO_PLACE = '{name}: {why} Eklenmedi.';
export const PHOTO_SKIPPED = '{name}: projenin sistemine çevrilemedi ({why}); eklenmedi.';

const WHY: Record<Unreached, string> = {
  outside: 'sistemin ulaştığı yerin dışında',
  noLink: "datumlardan birinin WGS 84'e dönüşümü yok",
  noGrid: 'datum dönüşümünün ızgarası bu cihazda yok',
  outsideGrid: 'datum dönüşümünün ızgarasının dışında',
};

const WGS84: System = { kind: 'geographic', datum: 'WGS84' };

/** A file's name without its last extension (`IMG_0012.JPG` → `IMG_0012`). */
export function stem(name: string): string {
  const k = name.lastIndexOf('.');
  return k > 0 ? name.slice(0, k) : name;
}

/** The photos in `to` (named `system`), in their order. */
export function placePhotos(photos: readonly Photo[], to: System, system: string, choices: readonly DatumChoice[] = []): PhotoPlan {
  const plan: PhotoPlan = { placed: [], skipped: [] };
  photos.forEach((photo, i) => {
    const index = i + 1;
    const say = (text: string, why: string): LineError => ({ line: index, message: text.replace('{name}', photo.name).replace('{why}', why) });
    const e = photo.exif;
    if (e.latitude == null || e.longitude == null) {
      plan.skipped.push(say(NO_PLACE, e.problem ?? 'Fotoğrafta GPS konumu yok.'));
      return;
    }
    const moved = crsTransformIn(WGS84, to, { x: e.longitude, y: e.latitude }, choices);
    if ('error' in moved) {
      plan.skipped.push(say(PHOTO_SKIPPED, WHY[moved.error]));
      return;
    }
    const name = stem(photo.name);
    const attrs: Record<string, string> = { Ad: name };
    if (e.taken != null) attrs.Tarih = e.taken;
    if (e.direction != null) attrs.Yön = `${fixed(e.direction, 2)}${e.magnetic ? ' M' : ''}`;
    if (e.altitude != null) attrs.Yükseklik = fixed(e.altitude, 2);
    if (e.camera != null) attrs.Kamera = e.camera;
    attrs.Dönüşüm = `WGS 84 → ${system}: ${accuracyText(moved)}`;
    const attachment: Attachment = { name: photo.name, ...(photo.asset != null && { asset: photo.asset }), ...(photo.file != null && { file: photo.file }) };
    plan.placed.push({ index, name, p: moved.point, attrs, attachment });
  });
  return plan;
}

/** The placed photos as the import writes them (`io/apply.ts`): points on the source layer `''`, their names as labels, the photo their attachment. */
export function photoEntities(placed: readonly PlacedPhoto[]): ContractEntity[] {
  return placed.map((pt) => ({ kind: 'point', id: 0, layerId: '', attrs: { ...pt.attrs }, label: pt.name, attachments: [{ ...pt.attachment }], p: { x: pt.p.x, y: pt.p.y } }));
}
