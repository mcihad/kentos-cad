/**
 * A picture's rules (docs/adr/0192 §1, §6), twin of the contract's
 * `ImageFields::problem` (crates/shared/contracts/src/image.rs): its size,
 * its one source, a linked file's path, its clip and its opacity, in the
 * commands' words. Whether an embedded picture's asset is in the project's
 * library is the command's to check (`unknown_asset`).
 */

/** The least and most opacity a picture is drawn with. */
export const MIN_IMAGE_OPACITY = 0.1;
export const MAX_IMAGE_OPACITY = 1;
/** The most corners a clip boundary may have. */
export const MAX_CLIP_CORNERS = 10_000;
/** The longest a picture may be on a side, metres. */
export const MAX_IMAGE_SIZE = 1e7;
/** The longest a linked file's path may be, in letters. */
export const MAX_IMAGE_PATH = 4096;

/** What places and shows a picture (the contract's `ImageFields`). */
export interface ImageShape {
  readonly p: { readonly x: number; readonly y: number };
  readonly width: number;
  readonly height: number;
  readonly rotation: number;
  readonly mirror?: boolean;
  readonly asset?: string;
  readonly file?: string;
  readonly clip?: readonly { readonly x: number; readonly y: number }[];
  readonly opacity?: number;
}

// Unicode's control letters (Rust's `char::is_control`).
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

/** Why the fields do not make a picture, in the commands' words; null when they do. */
export function imageProblem(g: ImageShape): string | null {
  if (!(Number.isFinite(g.p.x) && Number.isFinite(g.p.y) && Number.isFinite(g.rotation))) return 'Resmin köşesi ve dönüşü sonlu sayılar olmalı.';
  if (!(g.width > 0 && g.width <= MAX_IMAGE_SIZE && g.height > 0 && g.height <= MAX_IMAGE_SIZE))
    return `Resmin genişliği ve yüksekliği sıfırdan büyük ve en çok ${MAX_IMAGE_SIZE} m olmalı.`;
  const asset = g.asset?.trim() || null;
  const file = g.file?.trim() || null;
  if (asset && file) return 'Resmin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.';
  // One given, the other absent (not only blank); a null is no value, as serde reads it.
  if (!((asset && g.file == null) || (file && g.asset == null)))
    return 'Resmin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.';
  if (file && ([...file].length > MAX_IMAGE_PATH || CONTROL.test(file)))
    return `Bağlı dosyanın yolu en çok ${MAX_IMAGE_PATH} harf olmalı ve denetim karakteri içermemeli.`;
  if (g.clip != null) {
    const inside = (v: number) => Number.isFinite(v) && v >= 0 && v <= 1;
    if (g.clip.length < 3 || g.clip.length > MAX_CLIP_CORNERS || !g.clip.every((q) => inside(q.x) && inside(q.y)))
      return `Resmin kırpma sınırı en az 3, en çok ${MAX_CLIP_CORNERS} köşe olmalı, köşeleri resmin kesirleriyle 0 ile 1 arasında.`;
  }
  const o = g.opacity;
  if (o != null && !(Number.isFinite(o) && o >= MIN_IMAGE_OPACITY && o <= MAX_IMAGE_OPACITY))
    return `Resmin donukluğu ${MIN_IMAGE_OPACITY} ile ${MAX_IMAGE_OPACITY} arasında olmalı; ${o} verildi.`;
  return null;
}
