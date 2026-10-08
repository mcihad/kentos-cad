/**
 * A point cloud's rules (docs/adr/0207 §3, §5), twin of the contract's
 * `PointCloudFields::problem` and `PointCloudStyle::problem`
 * (crates/shared/contracts/src/pointcloud.rs): its files, each with one
 * source, their bounds and points, its look and opacity, in the commands'
 * words. The web keeps a cloud as it is (the desktop draws its points) and
 * checks it as the desktop does when a drawing is read or a command writes
 * one. Whether an embedded file's asset is in the project's library is the
 * command's to check (`unknown_asset`).
 */

import type { PointCloudFields } from '../contracts/generated/PointCloudFields';
import type { PointCloudStyle } from '../contracts/generated/PointCloudStyle';
import { RASTER_RAMPS, urlProblem } from './rasterRules';

/** The most members a virtual cloud may have. */
export const MAX_CLOUD_SOURCES = 4096;
/** The longest a path or address may be, in letters. */
export const MAX_CLOUD_PATH = 4096;
/** A point's least and most size (pixels or metres). */
export const MIN_POINT_SIZE = 0.5;
export const MAX_POINT_SIZE = 32;
/** The least and most opacity a cloud is drawn with. */
export const MIN_CLOUD_OPACITY = 0.1;
export const MAX_CLOUD_OPACITY = 1;
/** The looks that colour by a ramp over `min` to `max` (`CloudRender::ramped`). */
const RAMPED: ReadonlySet<string> = new Set(['elevation', 'intensity']);
/** Rust's `char::is_control`: the C0 and C1 controls. */
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

/** A value given: absent and blank are none, as the contract trims them. */
const given = (v: string | null | undefined): string | null => (v == null ? null : v.trim() || null);

const badBounds = (b: readonly number[]): boolean => !(b.length === 6 && b.every(Number.isFinite) && b[0] <= b[3] && b[1] <= b[4] && b[2] <= b[5]);

/** Why the look does not make one, in the commands' words; null when it does. */
export function pointCloudStyleProblem(st: PointCloudStyle): string | null {
  if (st.ramp != null && !(RASTER_RAMPS as readonly string[]).includes(st.ramp))
    return `“${st.ramp}” diye bir renk rampası yok; ${RASTER_RAMPS.join(', ')} rampalarından biri seçilmeli.`;
  const lo = st.min ?? null;
  const hi = st.max ?? null;
  if ((lo !== null || hi !== null) && !(lo !== null && hi !== null && Number.isFinite(lo) && Number.isFinite(hi) && lo < hi))
    return 'Görünüşün aralığı iki sonlu sayı olmalı, en küçük en büyükten küçük.';
  if (RAMPED.has(st.render) && (lo === null || hi === null)) return 'Rampalı görünüşün aralığı (en küçük ve en büyük) verilmeli.';
  if (!(Number.isFinite(st.size) && st.size >= MIN_POINT_SIZE && st.size <= MAX_POINT_SIZE)) return `Noktanın boyu ${MIN_POINT_SIZE} ile ${MAX_POINT_SIZE} arasında olmalı.`;
  const hidden = st.hidden ?? [];
  for (let i = 1; i < hidden.length; i++) if (hidden[i - 1] >= hidden[i]) return 'Gizlenen sınıflar küçükten büyüğe ve birer kez yazılmalı.';
  return null;
}

/** Why the fields do not make a cloud, in the commands' words; null when they do. */
export function pointCloudProblem(g: PointCloudFields): string | null {
  if (g.sources.length === 0) return 'Nokta bulutunun en az bir dosyası olmalı.';
  if (g.sources.length > MAX_CLOUD_SOURCES) return `Sanal bulutun en çok ${MAX_CLOUD_SOURCES} dosyası olabilir.`;
  for (const [i, s] of g.sources.entries()) {
    const file = given(s.file);
    const url = given(s.url);
    const n = Number(given(s.asset) !== null) + Number(file !== null) + Number(url !== null);
    const raw = Number(s.asset != null) + Number(s.file != null) + Number(s.url != null);
    if (n !== 1 || raw !== 1) return `${i + 1}. dosyanın kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.`;
    if (file !== null && ([...file].length > MAX_CLOUD_PATH || CONTROL.test(file)))
      return `Bağlı dosyanın yolu en çok ${MAX_CLOUD_PATH} harf olmalı ve denetim karakteri içermemeli.`;
    const address = url === null ? null : urlProblem(url);
    if (address) return address;
    if (badBounds(s.bounds)) return `${i + 1}. dosyanın kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.`;
  }
  if (badBounds(g.bounds)) return 'Nokta bulutunun kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.';
  // The files' points add up exactly (the contract's u64 sum; a sum past 2^53 is not exact here and not equal).
  let total = 0;
  for (const s of g.sources) total += s.count;
  if (!Number.isSafeInteger(total) || total !== g.count) return 'Nokta bulutunun nokta sayısı dosyalarınkinin toplamı olmalı.';
  const style = pointCloudStyleProblem(g.style);
  if (style) return style;
  if (g.opacity != null && !(Number.isFinite(g.opacity) && g.opacity >= MIN_CLOUD_OPACITY && g.opacity <= MAX_CLOUD_OPACITY))
    return `Nokta bulutunun donukluğu ${MIN_CLOUD_OPACITY} ile ${MAX_CLOUD_OPACITY} arasında olmalı; ${g.opacity} verildi.`;
  return null;
}
