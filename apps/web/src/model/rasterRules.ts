/**
 * A raster's rules (docs/adr/0204 §2, §4, §9), twin of the contract's
 * `RasterFields::problem` and `RasterStyle::problem`
 * (crates/shared/contracts/src/raster.rs): its affine, size, bands, one
 * source, a linked file's path, its look and its opacity, in the commands'
 * words. Whether an embedded raster's asset is in the project's library is
 * the command's to check (`unknown_asset`).
 */

import type { RasterStyle } from './entities';

export const MIN_RASTER_OPACITY = 0.1;
export const MAX_RASTER_OPACITY = 1;
/** The most pixels a raster may have on a side, and bands. */
export const MAX_RASTER_SIDE = 4_000_000;
export const MAX_RASTER_BANDS = 255;
/** The longest a linked file's path may be, in letters. */
export const MAX_RASTER_PATH = 4096;
/** The ramps a single band may be coloured with (docs/adr/0204 §4), in the menu's order. */
export const RASTER_RAMPS = ['Gri', 'Arazi', 'Spektral', 'Viridis', 'Mavi-kırmızı', 'Sıcaklık'] as const;
/** Gölgeli kabartma's light when the look names none. */
export const DEFAULT_AZIMUTH = 315;
export const DEFAULT_ALTITUDE = 45;
export const DEFAULT_Z_FACTOR = 1;

/** What places and shows a raster (the contract's `RasterFields`). */
export interface RasterShape {
  readonly affine: readonly number[];
  readonly width: number;
  readonly height: number;
  readonly bands: number;
  readonly asset?: string | null;
  readonly file?: string | null;
  readonly style: RasterStyle;
  readonly opacity?: number | null;
}

// Unicode's control letters (Rust's `char::is_control`).
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

/** Why a look does not suit a raster of `bands` bands, in the commands' words; null when it does. */
export function rasterStyleProblem(st: RasterStyle, bands: number): string | null {
  const [need, alpha] = st.render === 'rgb' ? [3, true] : [1, false];
  const n = st.bands.length;
  if (!(n === need || (alpha && n === need + 1))) return st.render === 'rgb' ? 'RGB görünüş üç bant ister (dördüncüsü alfa olabilir).' : 'Bu görünüş tek bant ister.';
  const wrong = st.bands.find((b) => b === 0 || b > bands);
  if (wrong !== undefined) return `Rasterin ${bands} bandı var; ${wrong}. bant gösterilemez.`;
  if (st.stretch === 'manual') {
    const { min, max } = st;
    if (!(min != null && max != null && Number.isFinite(min) && Number.isFinite(max) && min < max))
      return 'Elle gerdirmenin en küçüğü ve en büyüğü sonlu sayılar olmalı, en küçük en büyükten küçük.';
  }
  if (st.ramp != null && !(RASTER_RAMPS as readonly string[]).includes(st.ramp))
    return `“${st.ramp}” diye bir renk rampası yok; ${RASTER_RAMPS.join(', ')} rampalarından biri seçilmeli.`;
  const az = st.azimuth ?? DEFAULT_AZIMUTH;
  const alt = st.altitude ?? DEFAULT_ALTITUDE;
  const z = st.zFactor ?? DEFAULT_Z_FACTOR;
  const lit = Number.isFinite(az) && az >= 0 && az <= 360 && Number.isFinite(alt) && alt >= 0 && alt <= 90 && Number.isFinite(z) && z > 0;
  if (!lit) return 'Gölgeli kabartmanın ışığı 0–360° doğrultudan, 0–90° yükseklikten gelmeli; yükseklik çarpanı sıfırdan büyük olmalı.';
  if (st.nodata != null && !Number.isFinite(st.nodata)) return 'Nodata değeri sonlu bir sayı olmalı.';
  return null;
}

/** Why the fields do not make a raster, in the commands' words; null when they do. */
export function rasterProblem(g: RasterShape): string | null {
  if (g.affine.length !== 6 || !g.affine.every(Number.isFinite)) return 'Rasterin dönüşümü sonlu altı sayı olmalı.';
  const [, a, b, , c, d] = g.affine;
  const det = a * d - b * c;
  if (!(Number.isFinite(det) && det !== 0)) return 'Rasterin dönüşümü tersinmiyor: pikselin iki kenarı aynı doğrultuda ya da sıfır.';
  const side = (v: number) => Number.isInteger(v) && v >= 1 && v <= MAX_RASTER_SIDE;
  if (!(side(g.width) && side(g.height))) return `Rasterin genişliği ve yüksekliği 1 ile ${MAX_RASTER_SIDE} piksel arasında olmalı.`;
  if (!(Number.isInteger(g.bands) && g.bands >= 1 && g.bands <= MAX_RASTER_BANDS)) return `Rasterin 1 ile ${MAX_RASTER_BANDS} arasında bandı olmalı.`;
  const asset = g.asset?.trim() || null;
  const file = g.file?.trim() || null;
  if (asset && file) return 'Rasterin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.';
  // One given, the other absent (not only blank); a null is no value, as serde reads it.
  if (!((asset && g.file == null) || (file && g.asset == null)))
    return 'Rasterin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.';
  if (file && ([...file].length > MAX_RASTER_PATH || CONTROL.test(file)))
    return `Bağlı dosyanın yolu en çok ${MAX_RASTER_PATH} harf olmalı ve denetim karakteri içermemeli.`;
  const style = rasterStyleProblem(g.style, g.bands);
  if (style) return style;
  const o = g.opacity;
  if (o != null && !(Number.isFinite(o) && o >= MIN_RASTER_OPACITY && o <= MAX_RASTER_OPACITY))
    return `Rasterin donukluğu ${MIN_RASTER_OPACITY} ile ${MAX_RASTER_OPACITY} arasında olmalı; ${o} verildi.`;
  return null;
}

/** Whether every number of a raster is finite: its affine, its opacity and its look's. */
export function rasterFinite(g: RasterShape): boolean {
  const st = g.style;
  return [...g.affine, g.opacity, st.min, st.max, st.azimuth, st.altitude, st.zFactor, st.nodata].every((v) => v == null || Number.isFinite(v));
}

/** A look as the contract writes it: its fields' defaults left out (no stretch, not inverted, bilinear). */
export function cleanRasterStyle(st: RasterStyle): RasterStyle {
  const out: RasterStyle = { ...st };
  if (out.stretch === 'none') delete out.stretch;
  if (out.invert !== true) delete out.invert;
  if (out.resampling === 'bilinear') delete out.resampling;
  for (const k of ['min', 'max', 'ramp', 'azimuth', 'altitude', 'zFactor', 'nodata'] as const) if (out[k] == null) delete out[k];
  return out;
}
