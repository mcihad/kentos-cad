import type { ItemKind } from '../../../contracts/generated/sheet/ItemKind';
import type { Stroke } from '../../../contracts/generated/sheet/Stroke';
import type { TextStyle } from '../../../contracts/generated/sheet/TextStyle';
import { h, type Child } from '../../dom';
import { segmented } from '../../widgets/controls';
import { northSection } from './northSection';
import { area, color, flag, kinds, labelled, mapOptions, mmField, numField, pair, patchKind, pick, shared, text, type SectionCtx } from './parts';

/**
 * The inspector's sections of the kinds (docs/sheet/design.md §3.1): a text's
 * content and letters, a legend's map and layout, a scale bar's parts, a
 * north arrow's north (northSection.ts), a picture's fit, a shape's and a line's look, a sheet
 * frame's lines. The map's section is mapSection.ts; the table, the
 * coordinate list and the title block are dataSections.ts. Each field's
 * change is one `SetItemProps` of every chosen item.
 */

/** A typeface's id as the interface writes it (`architects-daughter` → Architects Daughter). */
export const fontLabel = (id: string): string =>
  id
    .split('-')
    .map((w) => (w === 'plex' ? 'IBM Plex' : w.charAt(0).toLocaleUpperCase('tr-TR') + w.slice(1)))
    .join(' ');

/** Letters: typeface, size (mm), weight, italic, colour. `get` reads the style of a kind, `put` makes the patch. */
export function letters(c: SectionCtx, t: ItemKind['type'], key: string, get: (k: ItemKind) => TextStyle, put: (s: Partial<TextStyle>) => Record<string, unknown>): Child[] {
  const fonts = c.host.engine()?.info.fonts ?? [];
  const v = <V>(of: (s: TextStyle) => V) => shared(c, t, (k) => of(get(k as ItemKind)));
  const label = 'Yazı';
  return [
    pair(
      pick(c, 'Yazı tipi', `${key}.font`, v((s) => s.font), fonts.map((f) => ({ value: f, label: fontLabel(f) })), (font) => patchKind(c, label, put({ font }))),
      mmField(c, 'Boy', `${key}.size`, v((s) => s.size), (size) => patchKind(c, label, put({ size })), { min: 0.5, max: 200, decimals: 2 }),
    ),
    pair(
      pick(c, 'Kalınlık', `${key}.weight`, v((s) => s.weight), [400, 500, 600, 700].map((w) => ({ value: w, label: w === 400 ? 'Normal' : w === 500 ? 'Orta' : w === 600 ? 'Yarı kalın' : 'Kalın' })), (weight) => patchKind(c, label, put({ weight }))),
      color(c, 'Renk', `${key}.color`, v((s) => s.color), (col) => patchKind(c, label, put({ color: col }))),
    ),
    flag(c, 'Eğik', v((s) => s.italic), (italic) => patchKind(c, label, put({ italic }))),
  ];
}

/** A stroke: colour and width (and its dash shown as solid or dashed). */
function strokeFields(c: SectionCtx, label: string, key: string, s: Stroke | null, put: (s: Partial<Stroke>) => void): Child[] {
  return [
    pair(
      color(c, `${label}: renk`, `${key}.color`, s ? s.color : null, (v) => put({ color: v })),
      mmField(c, `${label}: kalınlık`, `${key}.width`, s ? s.width : null, (v) => put({ width: Math.max(1, v) }), { min: 0.01, max: 20, decimals: 2 }),
    ),
  ];
}

function textSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'text' }>) => V) => shared(c, 'text', of);
  return [
    area(c, 'İçerik', 'text.content', v((k) => k.content), (content) => patchKind(c, 'Metin', { content }), 4),
    h('p', { class: 'sheet-insp__hint' }, '[% @pafta_adi %], [% @olcek %], [% @tarih %] gibi parçalar paftada değerleriyle yazılır; Ctrl+Enter alır.'),
    ...letters(c, 'text', 'text.style', (k) => (k as Extract<ItemKind, { type: 'text' }>).style, (style) => ({ style })),
    labelled(
      'Hizalama',
      h(
        'div',
        { class: 'sheet-segs' },
        segmented({ label: 'Yatay hizalama', value: v((k) => k.align) ?? 'left', options: [{ value: 'left', label: 'Sol' }, { value: 'center', label: 'Orta' }, { value: 'right', label: 'Sağ' }], onChange: (align) => patchKind(c, 'Hizalama', { align }) }),
        segmented({ label: 'Düşey hizalama', value: v((k) => k.valign) ?? 'top', options: [{ value: 'top', label: 'Üst' }, { value: 'middle', label: 'Orta' }, { value: 'bottom', label: 'Alt' }], onChange: (valign) => patchKind(c, 'Hizalama', { valign }) }),
      ),
    ),
    pair(
      numField(c, 'Satır aralığı', 'text.lineHeight', v((k) => k.lineHeight), (lineHeight) => patchKind(c, 'Satır aralığı', { lineHeight }), { min: 50, max: 400, unit: '%' }),
      pick(c, 'Sığdırma', 'text.fit', v((k) => k.fit), [{ value: 'none', label: 'Yok' }, { value: 'shrinkToFit', label: 'Küçülterek sığdır' }], (fit) => patchKind(c, 'Sığdırma', { fit })),
    ),
    flag(c, 'Satır kır', v((k) => k.wrap), (wrap) => patchKind(c, 'Satır kır', { wrap }), 'Uzun satırlar çerçevenin genişliğinde kırılır; kelime bölünmez.'),
    pick(c, '@olcek hangi haritadan', 'text.map', v((k) => k.map ?? ''), mapOptions(c, 'Paftanın ilk haritası'), (map) => patchKind(c, 'Bağlı harita', { map: map || null })),
  ];
}

function legendSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'legend' }>) => V) => shared(c, 'legend', of);
  return [
    text(c, 'Başlık', 'legend.title', v((k) => k.title), (title) => patchKind(c, 'Lejant başlığı', { title })),
    pick(c, 'Harita', 'legend.map', v((k) => k.map ?? ''), mapOptions(c, 'Bütün katmanlar (harita yok)'), (map) => patchKind(c, 'Bağlı harita', { map: map || null })),
    pick(c, 'Gösterilen', 'legend.filter', v((k) => k.filter), [
      { value: 'all', label: 'Haritanın bütün katmanları' },
      { value: 'visibleInMap', label: 'Yalnız haritada görünenler' },
      { value: 'atlasFeature', label: 'Yalnız atlas nesnesindekiler' },
    ], (filter) => patchKind(c, 'Lejant süzgeci', { filter })),
    pair(
      numField(c, 'Sütun', 'legend.columns', v((k) => k.columns), (columns) => patchKind(c, 'Lejant sütunları', { columns: Math.max(1, Math.round(columns)) }), { min: 1, max: 8 }),
      mmField(c, 'Kaydırma genişliği', 'legend.wrap', v((k) => k.wrapWidth), (wrapWidth) => patchKind(c, 'Lejant', { wrapWidth }), { min: 0 }),
    ),
    pair(
      mmField(c, 'Simge genişliği', 'legend.symbolW', v((k) => k.symbol.width), (width) => patchKind(c, 'Lejant simgesi', { symbol: { width } }), { min: 1 }),
      mmField(c, 'Simge yüksekliği', 'legend.symbolH', v((k) => k.symbol.height), (height) => patchKind(c, 'Lejant simgesi', { symbol: { height } }), { min: 1 }),
    ),
    h('p', { class: 'sheet-insp__hint' }, 'Satırlar haritanın katmanlarından ve simgelerinden gelir (lejant penceresinin gösterdiği gibi).'),
  ];
}

function scaleBarSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'scaleBar' }>) => V) => shared(c, 'scaleBar', of);
  const length = v((k) => k.length);
  return [
    pick(c, 'Harita', 'scaleBar.map', v((k) => k.map ?? ''), mapOptions(c, 'Paftanın ilk haritası'), (map) => patchKind(c, 'Bağlı harita', { map: map || null })),
    pick(c, 'Biçim', 'scaleBar.style', v((k) => k.style), [
      { value: 'singleBox', label: 'Tek kutu' },
      { value: 'doubleBox', label: 'Çift kutu' },
      { value: 'ticks', label: 'Çentikli' },
      { value: 'stepped', label: 'Basamaklı' },
      { value: 'hollow', label: 'Oyuk' },
      { value: 'numeric', label: 'Yalnız sayısal (1/1000)' },
    ], (style) => patchKind(c, 'Ölçek çubuğu biçimi', { style })),
    pair(
      numField(c, 'Sağ parça', 'scaleBar.segments', v((k) => k.segments), (segments) => patchKind(c, 'Ölçek çubuğu', { segments: Math.max(1, Math.round(segments)) }), { min: 1, max: 20 }),
      numField(c, 'Sol parça', 'scaleBar.left', v((k) => k.leftSegments), (leftSegments) => patchKind(c, 'Ölçek çubuğu', { leftSegments: Math.max(0, Math.round(leftSegments)) }), { min: 0, max: 4 }),
    ),
    pair(
      pick(c, 'Uzunluk', 'scaleBar.length', length ? length.type : null, [{ value: 'auto', label: 'Kendiliğinden (1-2-5)' }, { value: 'fixed', label: 'Sabit' }], (t) => patchKind(c, 'Ölçek çubuğu', { length: t === 'auto' ? { type: 'auto' } : { type: 'fixed', metres: 100 } })),
      pick(c, 'Birim', 'scaleBar.unit', v((k) => k.unit), [{ value: 'auto', label: 'Kendiliğinden' }, { value: 'm', label: 'm' }, { value: 'km', label: 'km' }], (unit) => patchKind(c, 'Ölçek çubuğu birimi', { unit })),
    ),
    flag(c, '“Ölçek 1/1000” yazısı', v((k) => k.showScale), (showScale) => patchKind(c, 'Ölçek çubuğu', { showScale })),
    color(c, 'Renk', 'scaleBar.color', v((k) => k.color), (col) => patchKind(c, 'Ölçek çubuğu rengi', { color: col })),
  ];
}

function pictureSection(c: SectionCtx, choosePicture: () => void): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'picture' }>) => V) => shared(c, 'picture', of);
  const sha = v((k) => k.asset ?? null);
  const meta = sha ? c.host.book()?.book.assets.find((a) => a.sha256 === sha) : undefined;
  const choose = h('button', { class: 'btn btn--small', type: 'button', disabled: c.readOnly !== null }, meta ? 'Başka resim seç…' : 'Resim seç…');
  choose.addEventListener('click', choosePicture);
  return [
    labelled('Resim', h('div', { class: 'sheet-insp__row' }, h('span', null, meta ? `${meta.name} · ${meta.width} × ${meta.height} px` : 'Seçilmedi'), choose), 'PNG, JPEG ya da SVG; en çok 4 MB. Şablon ve .kpafta resmi kendisiyle taşır.'),
    pick(c, 'Sığdırma', 'picture.fit', v((k) => k.fit), [
      { value: 'contain', label: 'İçine sığdır' },
      { value: 'cover', label: 'Kapla' },
      { value: 'stretch', label: 'Ger' },
      { value: 'original', label: 'Özgün boy' },
    ], (fit) => patchKind(c, 'Resim sığdırma', { fit })),
    flag(c, 'Çerçevede kırp', v((k) => k.clip), (clip) => patchKind(c, 'Resim kırpma', { clip })),
  ];
}

function shapeSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'shape' }>) => V) => shared(c, 'shape', of);
  const kind = v((k) => k.shape.type);
  const fill = v((k) => k.fill ?? null);
  const stroke = v((k) => k.stroke ?? null);
  return [
    kind === 'rect' ? mmField(c, 'Köşe yarıçapı', 'shape.radius', v((k) => (k.shape.type === 'rect' ? k.shape.radius : 0)), (radius) => patchKind(c, 'Köşe yarıçapı', { shape: { type: 'rect', radius } }), { min: 0 }) : null,
    flag(c, 'Dolgu', v((k) => !!k.fill), (on) => patchKind(c, 'Dolgu', { fill: on ? (fill ?? '#eeeeee') : null })),
    fill ? color(c, 'Dolgu rengi', 'shape.fill', fill, (col) => patchKind(c, 'Dolgu rengi', { fill: col })) : null,
    flag(c, 'Çizgi', v((k) => !!k.stroke), (on) => patchKind(c, 'Çizgi', { stroke: on ? (stroke ?? { color: '#000000', width: 250, dash: [], cap: 'butt', join: 'miter' }) : null })),
    ...(stroke ? strokeFields(c, 'Çizgi', 'shape.stroke', stroke, (s) => patchKind(c, 'Çizgi', { stroke: s })) : []),
  ];
}

function lineSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'line' }>) => V) => shared(c, 'line', of);
  const ends = [
    { value: 'none' as const, label: 'Yok' },
    { value: 'arrow' as const, label: 'Ok' },
    { value: 'dot' as const, label: 'Nokta' },
    { value: 'bar' as const, label: 'Çubuk' },
  ];
  return [
    ...strokeFields(c, 'Çizgi', 'line.stroke', v((k) => k.stroke), (s) => patchKind(c, 'Çizgi', { stroke: s })),
    pick(c, 'Çizgi türü', 'line.dash', v((k) => (k.stroke.dash.length ? 'dash' : 'solid')), [{ value: 'solid', label: 'Düz' }, { value: 'dash', label: 'Kesikli' }], (t) => patchKind(c, 'Çizgi türü', { stroke: { dash: t === 'dash' ? [2000, 1000] : [] } })),
    pair(pick(c, 'Baş', 'line.start', v((k) => k.start), ends, (start) => patchKind(c, 'Çizgi ucu', { start })), pick(c, 'Son', 'line.end', v((k) => k.end), ends, (end) => patchKind(c, 'Çizgi ucu', { end }))),
  ];
}

function borderSection(c: SectionCtx): Child[] {
  const v = <V>(of: (k: Extract<ItemKind, { type: 'border' }>) => V) => shared(c, 'border', of);
  const double = v((k) => k.style) === 'double';
  return [
    pick(c, 'Biçim', 'border.style', v((k) => k.style), [{ value: 'single', label: 'Tek çizgi' }, { value: 'double', label: 'Çift çizgi' }], (style) => patchKind(c, 'Çerçeve biçimi', { style })),
    mmField(c, 'İç pay', 'border.inset', v((k) => k.inset), (inset) => patchKind(c, 'Çerçeve', { inset }), { min: 0 }),
    ...strokeFields(c, 'Dış çizgi', 'border.stroke', v((k) => k.stroke), (s) => patchKind(c, 'Çerçeve çizgisi', { stroke: s })),
    ...(double
      ? [
          ...strokeFields(c, 'İç çizgi', 'border.inner', v((k) => k.innerStroke), (s) => patchKind(c, 'Çerçeve çizgisi', { innerStroke: s })),
          mmField(c, 'Bant', 'border.gap', v((k) => k.gap), (gap) => patchKind(c, 'Çerçeve bandı', { gap }), { min: 0 }),
          flag(c, 'Bölge işaretleri (1, 2, 3 … / A, B, C …)', v((k) => !!k.zones), (on) => patchKind(c, 'Bölge işaretleri', { zones: on ? { size: 50000, text: { font: 'barlow', size: 2500, weight: 500, italic: false, color: '#000000' } } : null })),
        ]
      : []),
    flag(c, 'Ortalama işaretleri', v((k) => k.centringMarks), (centringMarks) => patchKind(c, 'Ortalama işaretleri', { centringMarks })),
  ];
}

/** The section of a kind (null for a kind whose own fields are elsewhere: map, table, list, title block, group). */
export function kindSection(c: SectionCtx, kind: ItemKind['type'], choosePicture: () => void): Child[] | null {
  void kinds;
  switch (kind) {
    case 'text':
      return textSection(c);
    case 'legend':
      return legendSection(c);
    case 'scaleBar':
      return scaleBarSection(c);
    case 'northArrow':
      return northSection(c);
    case 'picture':
      return pictureSection(c, choosePicture);
    case 'shape':
      return shapeSection(c);
    case 'line':
      return lineSection(c);
    case 'border':
      return borderSection(c);
    default:
      return null;
  }
}
