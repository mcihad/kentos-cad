import type { DateSource } from '../../../contracts/generated/sheet/DateSource';
import type { ItemKind } from '../../../contracts/generated/sheet/ItemKind';
import type { NorthInfo } from '../../../contracts/generated/sheet/NorthInfo';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import { color, flag, labelled, mapOptions, numField, pair, patchKind, pick, shared, type SectionCtx } from './parts';

/**
 * A north arrow's section (docs/sheet/design.md §8, §8a): the map it reads,
 * which north (grid, true, magnetic) and its look (the arrows, the compass,
 * the north diagram of the topographic sheets). Where it shows magnetic north:
 * the declination the paper writes and from what, as the engine says it
 * (`northInfo`: the World Magnetic Model at the map's centre on the sheet's
 * date, “6°19' D · WMM2025 · 2026-10”); the date it is for and where that
 * comes from (the sheet's or the project's “tarih”, else today), with the way
 * to change it; or the declination typed by hand, with what the model gives
 * there and then (`magneticField` at the engine's place and year).
 */

type North = Extract<ItemKind, { type: 'northArrow' }>;

const NORTHS = [
  { value: 'grid' as const, label: 'Grid kuzeyi', detail: 'Haritanın koordinat ağının kuzeyi' },
  { value: 'true' as const, label: 'Coğrafi kuzey', detail: 'Meridyen yakınsaması kadar döner' },
  { value: 'magnetic' as const, label: 'Manyetik kuzey', detail: 'Yakınsama ve manyetik sapma kadar (WMM2025 ya da elle)' },
];
const STYLES = [
  { value: 'kentosK' as const, label: 'KentOS “K” oku' },
  { value: 'simple' as const, label: 'Basit ok' },
  { value: 'compass' as const, label: 'Pusula' },
  { value: 'diagram' as const, label: 'Kuzey çizelgesi', detail: 'Grid, coğrafi ve manyetik kuzey birlikte, açılarıyla' },
];

/** Whether the arrow's declination is typed by hand (a book of before the model: when one was typed). */
export const byHand = (k: North): boolean => k.declinationHand ?? k.declination !== 0;

/** Whether the arrow shows magnetic north (the diagram always does). */
export const showsMagnetic = (k: North): boolean => k.north === 'magnetic' || k.style === 'diagram';

/** An angle in degrees as the paper writes it: “6°19' D”, “2°10' B” (east positive; the minute an ASCII mark, as the core writes it). */
export function degreesText(deg: number): string {
  const total = Math.round(Math.abs(deg) * 60);
  return `${Math.floor(total / 60)}°${String(total % 60).padStart(2, '0')}' ${deg < 0 ? 'B' : 'D'}`;
}

/** The declination's value as the inspector writes it: “6°19' D · WMM2025 · 2026-10”, “2°10' B · elle · 2024”, or why it is not known. */
export function declinationText(info: NorthInfo): string {
  if (info.missing === 'noPlace') return 'Hesaplanamıyor: haritanın konumu yok (koordinat sistemi TM ya da UTM değil)';
  if (info.missing === 'noDate' || info.declination === undefined) return 'Hesaplanamıyor: paftanın tarihi yok';
  if (info.source === 'hand') return `${degreesText(info.declination)} · elle${info.handYear ? ` · ${info.handYear}` : ''}`;
  return `${degreesText(info.declination)} · ${info.model} · ${(info.date ?? '').slice(0, 7)}`;
}

/** The model's declination at the arrow's place and date, when the engine gave both (for a value typed by hand, its hint). */
function modelValue(c: SectionCtx, info: NorthInfo): number | null {
  if (info.lat === undefined || info.lon === undefined || info.year === undefined) return null;
  try {
    return c.host.engine()?.magneticField(info.lat, info.lon, 0, info.year).declination ?? null;
  } catch {
    return null;
  }
}

const FROM: Record<DateSource, string> = {
  sheet: 'paftanın “tarih” değişkeni',
  project: 'projenin “tarih” değişkeni',
  today: 'bugün: paftada ve projede “tarih” değişkeni yok',
};

/** The declination's part: what the paper writes and from what (the engine's `northInfo`), the date it is for, or the value typed by hand. */
function declinationPart(c: SectionCtx, k: North, id: string): Child[] {
  const hand = byHand(k);
  const info = c.host.northInfo(c.sheet.id, id);
  if (!info) return [h('p', { class: 'sheet-insp__hint' }, 'Manyetik sapma şimdi okunamadı.')];
  const known = info.declination !== undefined && !info.missing;
  // For a value typed by hand, what the model would give there and then (its place and date from the engine).
  const model = hand ? modelValue(c, info) : known ? (info.declination ?? null) : null;
  const variables = h('button', { class: 'btn btn--small', type: 'button', disabled: !c.ctx.commands.isEnabled('sheet.variables') }, icon('sheetVariables', 14), 'Değişkenler…');
  variables.addEventListener('click', () => c.ctx.commands.execute('sheet.variables'));
  c.d.add(tooltip(variables, () => ({ title: 'Tarihi değiştir', description: 'Paftaya ya da projeye ISO biçiminde bir “tarih” değişkeni girin (2026-10-03); sapma o günün değeriyle yazılır.' })));
  const place = info.lat !== undefined && info.lon !== undefined ? ` (${info.lat.toFixed(4)}° K, ${info.lon.toFixed(4)}° D)` : '';
  const out: Child[] = [
    labelled(
      'Manyetik sapma',
      h('div', { class: `sheet-north__value${!known ? ' sheet-north__value--unknown' : ''}`, dataset: { key: 'north.value' } }, declinationText(info)),
      hand ? (model !== null ? `Modelin değeri: ${degreesText(model)} · ${info.model}${info.date ? ` · ${info.date.slice(0, 7)}` : ''}.` : 'Elle girilen değer yazılır.') : `Haritanın merkezinde${place}, paftanın tarihinde; NOAA'nın Dünya Manyetik Modeli (${info.model}: ${info.validFrom}–${info.validUntil}).`,
    ),
  ];
  if (!hand)
    out.push(
      labelled(
        'Hesap tarihi',
        h('div', { class: 'sheet-insp__row sheet-north__date' }, h('span', { class: 'num' }, info.date || '—'), variables),
        `${info.dateSource ? FROM[info.dateSource] : 'tarih yok'}.${info.inModel === false ? ' Tarih modelin geçerlilik döneminin dışında: değer yaklaşıktır.' : ''}`,
      ),
    );
  out.push(
    flag(c, 'Sapmayı elle gir', hand, (on) => {
      // Turned on with nothing typed yet: it starts from the model's value, to the minute as the paper writes it.
      const start = on && k.declination === 0 && model !== null ? Math.round(model * 60) / 60 : null;
      patchKind(c, 'Manyetik sapma', on ? { declinationHand: true, ...(start !== null ? { declination: Math.round(start * 1000) } : {}) } : { declinationHand: false });
    }, 'Kapalıyken sapma modelden hesaplanır; açıkken buraya girilen değer kullanılır ve kâğıtta “elle” yazılır.'),
  );
  if (hand)
    out.push(
      pair(
        numField(c, 'Elle sapma', 'north.declination', k.declination / 1000, (deg) => patchKind(c, 'Manyetik sapma', { declination: Math.round(deg * 1000) }), { min: -180, max: 180, decimals: 2, unit: '°' }),
        numField(c, 'Sapmanın yılı', 'north.year', k.declinationYear ?? null, (y) => patchKind(c, 'Manyetik sapma', { declinationYear: Math.round(y) }), { min: 1900, max: 2100 }),
      ),
      h('p', { class: 'sheet-insp__hint' }, 'Doğu artı, batı eksi; yıl kâğıtta “elle, 2024” diye yazılır.'),
    );
  return out;
}

export function northSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: North) => V) => shared(c, 'northArrow', of);
  const out: Child[] = [
    pick(c, 'Harita', 'north.map', v((k) => k.map ?? ''), mapOptions(c, 'Paftanın ilk haritası'), (map) => patchKind(c, 'Bağlı harita', { map: map || null })),
    pick(c, 'Kuzey', 'north.kind', v((k) => k.north), NORTHS, (north) => patchKind(c, 'Kuzey', { north })),
    pick(c, 'Biçim', 'north.style', v((k) => k.style), STYLES, (style) => patchKind(c, 'Kuzey oku biçimi', { style })),
  ];
  const one = c.items.length === 1 ? (c.items[0].source.kind as North) : null;
  if (one && showsMagnetic(one)) out.push(...declinationPart(c, one, c.items[0].id));
  else if (!one && v((k) => showsMagnetic(k))) out.push(h('p', { class: 'sheet-insp__hint' }, 'Manyetik sapma tek kuzey oku seçiliyken gösterilir.'));
  out.push(
    flag(c, 'Yakınsama notu', v((k) => k.note), (note) => patchKind(c, 'Kuzey oku notu', { note }), 'Okun altında meridyen yakınsaması ve, manyetik kuzeyde, sapma yazılır; çekirdek koordinat sisteminden hesaplar.'),
    color(c, 'Renk', 'north.color', v((k) => k.color), (col) => patchKind(c, 'Kuzey oku rengi', { color: col })),
  );
  return out;
}
