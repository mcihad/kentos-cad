import type { Entity } from '../../../model/entities';
import { apportion, geoOverlay, type OverlayMode, type OverlayPiece } from '../../../model/ops/geoprocess';
import { fieldNames } from '../../parameters';
import { defineTool, type Feedback } from '../../types';
import { AREA_KINDS, GEO_KINDS, GEO_SCOPES, emptyNote, inputNotes, newObject, outputStyle } from './shared';

/**
 * Kesişim, Fark, Simetrik fark and Birleşim (docs/adr/0201 §5; ArcGIS Intersect, Erase, Symmetrical Difference, Union):
 * two sets of objects overlaid in the core's overlay (`ops::geoprocess::clip`), each piece written with the
 * attributes of the objects it is of: the first side's as they are (Alan oranıyla paylaştır's fields times the piece's
 * share), the second side's with Önek before their names, “ad (2)” where the first side has the name already.
 */

/** The second side's attributes added to the first's: each name with the prefix, “ad (2)”, “ad (3)” … where taken. */
export function withSecond(first: Record<string, string>, second: Record<string, string>, prefix: string): Record<string, string> {
  const out = { ...first };
  for (const [k, v] of Object.entries(second)) {
    let name = `${prefix}${k}`;
    if (Object.hasOwn(out, name)) {
      let n = 2;
      while (Object.hasOwn(out, `${name} (${n})`)) n++;
      name = `${name} (${n})`;
    }
    out[name] = v;
  }
  return out;
}

/** A piece's attributes; `skipped` counts the values Alan oranıyla paylaştır could not read. */
function pieceAttrs(p: OverlayPiece, a: readonly Entity[], b: readonly Entity[], prefix: string, fields: readonly string[], skipped: { n: number }): Record<string, string> {
  let attrs: Record<string, string> = {};
  if (p.a !== undefined) {
    attrs = { ...a[p.a].attrs };
    const names = fields.filter((f) => Object.hasOwn(attrs, f));
    if (names.length && p.share !== undefined) {
      const shared = apportion(
        names.map((f) => attrs[f]),
        p.share,
      );
      names.forEach((f, k) => {
        const s = shared[k];
        if (s === null) skipped.n++;
        else attrs[f] = s;
      });
    }
  }
  if (p.b !== undefined) attrs = withSecond(attrs, b[p.b].attrs, prefix);
  return attrs;
}

/** An overlay's run: its pieces on the output layer, the notes, and how many pieces each side gave. */
function overlayRun(mode: OverlayMode, a: readonly Entity[], b: readonly Entity[], layerId: string, prefix: string, fields: readonly string[], feedback: Feedback) {
  inputNotes(feedback, a, b);
  feedback.progress(0, 'Alanlar bindiriliyor');
  const pieces = geoOverlay(a, b, mode);
  const skipped = { n: 0 };
  const add = pieces.map((p) => newObject(p.shape, layerId, pieceAttrs(p, a, b, prefix, fields, skipped)));
  if (skipped.n) feedback.warn(`${skipped.n} değer sayı olarak okunamadığı için paylaştırılmadı.`);
  // Objects that gave no piece: the first side's, and in Simetrik fark and Birleşim the second's.
  const second = mode === 'symDifference' || mode === 'union';
  const none = a.filter((_, i) => !pieces.some((p) => p.a === i)).length + (second ? b.filter((_, j) => !pieces.some((p) => p.b === j)).length : 0);
  emptyNote(none, feedback);
  const count = (f: (p: OverlayPiece) => boolean) => pieces.filter(f).length;
  return { add, both: count((p) => p.a !== undefined && p.b !== undefined), first: count((p) => p.a !== undefined && p.b === undefined), second: count((p) => p.a === undefined) };
}

const PREFIX = { name: 'prefix', label: 'Önek', type: 'string', default: '', allowEmpty: true, maxLength: 24, description: 'İkinci tarafın öznitelik adlarının önüne yazılır; ad birinci tarafta da varsa “ad (2)” olur.' } as const;

const APPORTION = { name: 'apportion', label: 'Alan oranıyla paylaştır', type: 'field', of: 'input', multiple: true, optional: true, advanced: true, description: 'Bu alanların sayı değerleri parçanın payıyla çarpılır: alanda alan, çizgide uzunluk oranı.' } as const;

const APPORTION_HELP = 'Alan oranıyla paylaştır: seçilen alanların sayı değerleri, parçanın birinci taraftaki nesnenin alanına (çizgide uzunluğuna, noktada sayısına) oranıyla çarpılır ve değerin basamağından iki fazla basamağa yarım çifte yuvarlanır; sayı olarak okunamayan değer olduğu gibi kalır ve söylenir.';

export const geometryIntersection = defineTool({
  id: 'geometry.intersection',
  label: 'Kesişim',
  category: 'geometry',
  icon: 'geoIntersection',
  description: 'Her nesnenin kesen alanlarla ortak parçalarını, iki tarafın öznitelikleriyle yeni katmana yazar.',
  help: [
    'Her nesne, kesiştiği her alanla ayrı bir parça verir: alanda ortak alan, çizgide içerideki parçalar, noktada içerideki noktalar. Parça nesnenin ve kesen alanın özniteliklerini taşır; kesen alanınkiler Önek ile yazılır.',
    APPORTION_HELP,
    'Örnek: imar adalarıyla kesişen parsellerin her adadaki parçası, parselin ve adanın bilgileriyle; parselin değeri parçanın alan oranıyla.',
  ].join('\n\n'),
  keywords: ['kesişim', 'kesiştir', 'intersect', 'bindirme', 'overlay', 'ortak alan', 'paylaştır'],
  aliases: ['KESISIMAL', 'GEOINTERSECT'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Kesilecek alanlar, çizgiler ve noktalar.' },
    { name: 'overlay', label: 'Kesen alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description: 'Her nesne kesiştiği her alanla ayrı parça verir.' },
    PREFIX,
    APPORTION,
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Kesişim' }, newLayerStyle: outputStyle('#F76B15'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const r = overlayRun('intersection', v.input.entities, v.overlay.entities, v.layer.id, v.prefix, fieldNames({ multiple: true }, v.apportion ?? ''), feedback);
    return { changes: { add: r.add }, outputs: { count: r.add.length }, summary: `${r.add.length} kesişim parçası yazıldı.` };
  },
});

export const geometryDifference = defineTool({
  id: 'geometry.difference',
  label: 'Fark',
  category: 'geometry',
  icon: 'geoDifference',
  description: 'Her nesneden çıkarılacak alanların kapladığı kısmı atar, kalanı öznitelikleriyle yeni katmana yazar.',
  help: [
    'Çıkarılacak alanlar önce birleşir; her nesnenin onların dışında kalan kısmı yazılır: alanın kalan parçaları, çizginin dışarıdaki parçaları, dışarıdaki noktalar. Sınır üstündeki çizgi parçası atılır.',
    'Bütünüyle örtülen nesne yazılmaz ve söylenir. Öznitelikler nesnenin kendisinindir.',
  ].join('\n\n'),
  keywords: ['fark', 'çıkar', 'erase', 'difference', 'sil', 'bindirme', 'overlay'],
  aliases: ['FARKAL', 'GEOERASE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Nesneler', type: 'features', kinds: GEO_KINDS, scopes: GEO_SCOPES, description: 'Kısımları çıkarılacak alanlar, çizgiler ve noktalar.' },
    { name: 'overlay', label: 'Çıkarılacak alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description: 'Kapladıkları kısım nesnelerden atılır.' },
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Fark' }, newLayerStyle: outputStyle('#E5484D'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Kalan nesneler', type: 'features' },
    { name: 'count', label: 'Yazılan nesne sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const r = overlayRun('difference', v.input.entities, v.overlay.entities, v.layer.id, '', [], feedback);
    return { changes: { add: r.add }, outputs: { count: r.add.length }, summary: `${r.add.length} nesnenin farkı yazıldı.` };
  },
});

export const geometrySymDifference = defineTool({
  id: 'geometry.symDifference',
  label: 'Simetrik fark',
  category: 'geometry',
  icon: 'geoSymDifference',
  description: 'İki alan kümesinin yalnız birinin kapladığı parçaları, kendi taraflarının öznitelikleriyle yeni katmana yazar.',
  help: [
    'Önce birinci alanların ikincilerin dışında kalan parçaları, sonra ikincilerin birincilerin dışında kalanları yazılır; ortak kısımlar atılır.',
    'Birinci taraftan gelen parça kendi özniteliklerini, ikinciden gelen kendininkileri Önek ile taşır.',
  ].join('\n\n'),
  keywords: ['simetrik fark', 'symmetrical difference', 'xor', 'bindirme', 'overlay', 'değişen alanlar'],
  aliases: ['SIMFARK', 'SYMDIFF'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Birinci alanlar', type: 'features', kinds: AREA_KINDS, scopes: GEO_SCOPES, description: 'Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler.' },
    { name: 'overlay', label: 'İkinci alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description: 'Birincilerle karşılaştırılan alanlar.' },
    PREFIX,
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Simetrik fark' }, newLayerStyle: outputStyle('#E93D82'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const r = overlayRun('symDifference', v.input.entities, v.overlay.entities, v.layer.id, v.prefix, [], feedback);
    return { changes: { add: r.add }, outputs: { count: r.add.length }, summary: `${r.add.length} parça yazıldı: ${r.first} birinci, ${r.second} ikinci alanlardan.` };
  },
});

export const geometryUnion = defineTool({
  id: 'geometry.union',
  label: 'Birleşim',
  category: 'geometry',
  icon: 'geoUnion',
  description: 'İki alan kümesini bindirir: ortak parçalar iki tarafın, kalan parçalar kendi taraflarının öznitelikleriyle yeni katmana yazılır.',
  help: [
    'Önce her birinci alanın her ikinciyle ortak parçası (iki tarafın öznitelikleriyle), sonra birincilerin ikincilerin dışında kalan, en son ikincilerin birincilerin dışında kalan parçaları yazılır. İkinci tarafın öznitelikleri Önek ile yazılır.',
    APPORTION_HELP,
  ].join('\n\n'),
  keywords: ['birleşim', 'union', 'bindirme', 'overlay', 'paylaştır'],
  aliases: ['BIRLESIM', 'GEOUNION'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'input', label: 'Birinci alanlar', type: 'features', kinds: AREA_KINDS, scopes: GEO_SCOPES, description: 'Kapalı alanlar, daireler, tam elipsler ve kapalı eğriler.' },
    { name: 'overlay', label: 'İkinci alanlar', type: 'features', kinds: AREA_KINDS, scopes: ['layer', 'selection', 'visible', 'all'], description: 'Birincilerle bindirilen alanlar.' },
    PREFIX,
    APPORTION,
    { name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: 'Birleşim' }, newLayerStyle: outputStyle('#3E63DD'), description: 'Bu adda katman yoksa oluşturulur.' },
  ] as const,
  outputs: [
    { name: 'pieces', label: 'Parçalar', type: 'features' },
    { name: 'count', label: 'Parça sayısı', type: 'number' },
  ],
  run: (v, _ctx, feedback) => {
    const r = overlayRun('union', v.input.entities, v.overlay.entities, v.layer.id, v.prefix, fieldNames({ multiple: true }, v.apportion ?? ''), feedback);
    return {
      changes: { add: r.add },
      outputs: { count: r.add.length },
      summary: `${r.add.length} parça yazıldı: ${r.both} ortak, ${r.first} yalnız birinci, ${r.second} yalnız ikinci alanlarda.`,
    };
  },
});
