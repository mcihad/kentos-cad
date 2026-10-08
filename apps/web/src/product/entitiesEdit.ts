import { tableProblem, type TableShape } from '../model/tables';
import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EditOperation } from '../contracts/generated/EditOperation';
import type { EntitiesEdit } from '../contracts/generated/EntitiesEdit';
import type { EntitiesEdited } from '../contracts/generated/EntitiesEdited';
import type { EntitiesEditPlan } from '../contracts/generated/EntitiesEditPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry } from '../contracts/generated/EntityGeometry';
import { isUuid } from '../core/uuid';
import type { CadDocument } from '../model/document';
import { FACE_FIELDS, faceProblem, LOOK_FIELDS, lookProblem } from '../model/annotationStyles';
import { leaderArrowHolds, MAX_LEADER_ARROW, MAX_WIDTH_FACTOR, MIN_LEADER_ARROW, paragraphProblem, textPathProblem, widthFactorOk, type Entity, type NewEntity, type RasterStyle } from '../model/entities';
import { assocProblem, patternProblem } from '../model/hatchRules';
import { imageProblem } from '../model/imageRules';
import { cleanRasterStyle, rasterProblem } from '../model/rasterRules';
import { geometryIsFinite, SHAPE_FIELDS } from '../model/ops/transform';
import { checkLayer, checkLineWeight, checkRevision, error, failed, isBlank, validated, type Stop } from './checks';
import { checkDimension } from './dimension';
import type { ProductCommand } from './command';
import { carryInto, elevatedPaths, hasElevation } from './elevation';

/**
 * `cad.entities.edit` v1 (docs/adr/0047): objects named by their persistent
 * ids given a new geometry, replaced in their place, followed by new objects
 * made from them, or deleted, as one undo step named after the modify tool.
 * The web's handler over `CadDocument`; the desktop's is
 * `crates/native/application/src/edit.rs`. Both pass the shared cases in
 * fixtures/commands/v1/cad.entities.edit.json.
 *
 * The edge, corner and object tools (Ötele, Buda, Uzat, Köşe yuvarla, Pah,
 * Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil) and Esnet compute the
 * geometry with the shared core and write it here (TODOS.md CMD-07);
 * nothing is computed in this module. Öznitelikler's geometry rows and the
 * in-place text editor write the value typed (operation `properties`); the
 * area tools write what the shared region core made (docs/adr/0065).
 *
 * The checks, in order (the first that fails answers): at least one change,
 * each change's id lowercase UUID text with hyphens; every geometry, in
 * order: enough points for its kind, a text that is not blank, every number
 * finite, a positive radius, an insert's positive scale, and an `add`'s own
 * line weight; the expected revision (checks.ts); each id names an object;
 * no object changed twice; no object on a locked layer (an edit is written
 * whole or not at all); an `add`'s own layer; every insert's block is the
 * drawing's (docs/adr/0144).
 */

/** The undo step's name: the tool's (docs/adr/0047); Öznitelikler's is the document's own “Değiştir”. */
export const EDIT_LABEL: Record<EditOperation, string> = {
  offset: 'Ötele',
  trim: 'Buda',
  extend: 'Uzat',
  fillet: 'Köşe yuvarla',
  chamfer: 'Pah',
  break: 'Kır',
  join: 'Birleştir',
  explode: 'Patlat',
  lengthen: 'Uzat-kısalt',
  vertexAdd: 'Köşe ekle',
  vertexRemove: 'Köşe sil',
  stretch: 'Esnet',
  properties: 'Değiştir',
  areaUnion: 'Alan birleştir',
  areaIntersect: 'Alan kesiştir',
  areaSubtract: 'Alan çıkar',
  areaSplit: 'Alan böl',
  toArea: 'Alana çevir',
  toPolyline: 'Çizgiye çevir',
  grip: 'Tutamaçla düzenle',
  straightEdge: 'Düz kenar yap',
  arcEdge: 'Yaya dönüştür',
  split: 'Parçala',
  reverse: 'Yönü çevir',
  simplify: 'Sadeleştir',
  cleanup: 'Çizimi temizle',
  elevation: 'Kot ver',
  partsJoin: 'Parçaları birleştir',
  partsSplit: 'Parçalara ayır',
  // Okunur yap and Bul ve değiştir (docs/adr/0145).
  readable: 'Okunur yap',
  replaceText: 'Bul ve değiştir',
  // Topolojik temizlik (docs/adr/0148).
  topology: 'Topolojik temizlik',
  // Kenar eşleme (docs/adr/0159).
  edgematch: 'Kenar eşle',
  // Biçim değiştir, Sürdür and the holes (docs/adr/0173).
  reshape: 'Biçim değiştir',
  continue: 'Sürdür',
  holeAdd: 'Delik ekle',
  holeRemove: 'Deliği sil',
  holeFill: 'Deliği doldur',
  textStyle: 'Yazı stili',
  dimensionStyle: 'Ölçü stili',
  // Tabloyu düzenle and Tabloyu güncelle (docs/adr/0184 §6).
  table: 'Tablo',
  tableUpdate: 'Tabloyu güncelle',
  // Paralel kaydır (docs/adr/0191).
  edgeShift: 'Paralel kaydır',
  // Resmi kırp (docs/adr/0192 §5).
  imageClip: 'Resmi kırp',
  // Eğri boyunca yazı (docs/adr/0196 §4).
  textPath: 'Eğriye oturt',
  textTurn: 'Doğrultuya döndür',
  textStraighten: 'Düzleştir',
  roadJunctions: 'Kavşak temizle',
  medianClose: 'Refüj kapat',
  // Topoloji sekmesinin düzeltmeleri (docs/adr/0202 §4).
  topologyFix: 'Topoloji düzelt',
  // Ağ dengelemelerinin Çizime yaz'ı (docs/adr/0203 §8).
  networkAdjust: 'Yatay ağ dengelemesi',
  levelAdjust: 'Kot ağı dengelemesi',
  // Raster stili and Raster oturt (docs/adr/0204 §9).
  rasterStyle: 'Raster stili',
  rasterGeoref: 'Raster oturt',
  // Ölçek ya da genel yükseklik değişince izleyenler (docs/adr/0205 §3).
  annotationScale: 'Yazı yüksekliklerini uydur',
};

/** The contract's geometry fields by kind (`EntityGeometry`): what the command writes of a geometry. */
const FIELDS: Record<EntityGeometry['kind'], readonly string[]> = {
  point: ['p', 'z', 'parts'],
  line: ['a', 'b', 'zs'],
  polyline: ['pts', 'bulges', 'zs', 'parts'],
  polygon: ['pts', 'bulges', 'holes', 'zs', 'parts'],
  circle: ['c', 'r'],
  arc: ['c', 'r', 'a0', 'a1'],
  ellipse: ['c', 'major', 'ratio', 't0', 't1'],
  spline: ['pts', 'closed'],
  xline: ['p', 'dir'],
  ray: ['p', 'dir'],
  // Its curve (docs/adr/0196 §1) goes with its geometry.
  text: ['p', 'text', 'height', 'rotation', 'align', 'widthFactor', 'mask', 'boxWidth', 'lineSpacing', 'runs', ...FACE_FIELDS, 'path'],
  dimension: ['a', 'b', 'offset', 'height', 'text', 'style', 'angle', 'c', 'mask', 'za', 'zb', ...LOOK_FIELDS],
  hatch: ['ring', 'holes', 'pattern', 'assoc'],
  insert: ['block', 'p', 'scale', 'rotation', 'mirror'],
  leader: ['pts', 'text', 'height', 'rotation', 'arrow', 'arrowSize', 'mask'],
  table: ['p', 'rotation', 'height', 'rows', 'columns', 'cells', 'merges', 'aligns', 'header', 'grid', 'frame', ...FACE_FIELDS, 'source'],
  image: ['p', 'width', 'height', 'rotation', 'mirror', 'asset', 'file', 'clip', 'opacity'],
  raster: ['affine', 'width', 'height', 'bands', 'sample', 'asset', 'file', 'srid', 'style', 'opacity'],
};

interface Checked {
  /** The objects updated or replaced: slot, id and the object as it will be, in the input's order. */
  changed: { id: number; uid: string; init: NewEntity }[];
  /** The new objects, in the order of their `add`s. */
  created: NewEntity[];
  /** The objects to delete: slot and id, in the input's order. */
  removed: { id: number; uid: string }[];
  /** That elevations were lost (docs/adr/0142). */
  warnings: CommandWarning[];
}

/** The id a change names and the field that holds it: `uid`, or `from` for an `add`. */
const named = (c: EntityEdit): [string, 'uid' | 'from'] => (c.kind === 'add' ? [c.from, 'from'] : [c.uid, 'uid']);

/** A geometry's own fields, copied: nothing else a caller put beside them reaches the drawing. */
export function geometryOf(g: EntityGeometry): Record<string, unknown> {
  const src = g as unknown as Record<string, unknown>;
  const out: Record<string, unknown> = { kind: g.kind };
  for (const key of FIELDS[g.kind]) if (src[key] !== undefined) out[key] = structuredClone(src[key]);
  // An insert is mirrored or has no `mirror` (docs/adr/0144): false is not written. So is a picture (docs/adr/0192 §1),
  // whose other optional fields are no value when null, as serde reads them.
  if ((g.kind === 'insert' || g.kind === 'image') && out.mirror !== true) delete out.mirror;
  if (g.kind === 'image') for (const key of ['asset', 'file', 'clip', 'opacity']) if (out[key] === null) delete out[key];
  // A raster's (docs/adr/0204 §2): no value when null; its look's defaults are no fields.
  if (g.kind === 'raster') {
    for (const key of ['asset', 'file', 'opacity']) if (out[key] === null) delete out[key];
    out.style = cleanRasterStyle(out.style as RasterStyle);
  }
  // A text's defaults are no fields (docs/adr/0145): no mask, a width factor of 1.
  if (g.kind === 'text') {
    if (out.mask !== true) delete out.mask;
    if (out.widthFactor === 1) delete out.widthFactor;
    // A multi-line text's (docs/adr/0182 §1): no box, a spacing of 1, no runs; a null is no value.
    if (out.boxWidth === null) delete out.boxWidth;
    if (out.lineSpacing === null || out.lineSpacing === 1) delete out.lineSpacing;
    if (Array.isArray(out.runs) && !out.runs.length) delete out.runs;
    // Its face's (docs/adr/0183 §2): upright, not bold, not italic, the project's typeface, no style.
    for (const key of FACE_FIELDS) if (out[key] === null || out[key] === false) delete out[key];
    // A straight text has no curve (docs/adr/0196 §1); a null is no value.
    if (out.path === null) delete out.path;
  }
  // So are a leader's (docs/adr/0146 §1): no mask, a filled arrow, no note; a null is no value, as serde reads it.
  if (g.kind === 'leader') {
    if (out.mask !== true) delete out.mask;
    if (out.arrow === null) delete out.arrow;
    if (out.text === null) delete out.text;
  }
  // A table's (docs/adr/0184 §1): no ranges, no heading, all lines, a line for a frame, left aligned, no source;
  // its face's as a text's; a null is no value.
  if (g.kind === 'table') {
    if (Array.isArray(out.merges) && !out.merges.length) delete out.merges;
    if (out.header !== true) delete out.header;
    for (const key of ['grid', 'frame', 'aligns', 'source']) if (out[key] === null) delete out[key];
    for (const key of FACE_FIELDS) if (out[key] === null || out[key] === false) delete out[key];
  }
  // And a dimension's mask (docs/adr/0147 §1); a null elevation is no value.
  if (g.kind === 'dimension') {
    if (out.mask !== true) delete out.mask;
    if (out.za === null) delete out.za;
    if (out.zb === null) delete out.zb;
    // Its look's (docs/adr/0183 §3): a null is no value.
    for (const key of LOOK_FIELDS) if (out[key] === null) delete out[key];
  }
  return out;
}

/** The elevations an object holds beside its geometry fields (docs/adr/0142): a line's two ends, a path's vertices. */
const ELEVATION_FIELDS: Partial<Record<Entity['kind'], readonly string[]>> = { line: ['za', 'zb'], polyline: ['zs'], polygon: ['zs'] };

/**
 * `e` with another geometry: every field but its geometry kept (`update`),
 * in the object's own order, so an update that changes nothing is no edit
 * (`CadDocument.replace` compares as JSON writes, key order included). An
 * update that changes the kind (Kenar eşle's Parça ekle makes a line a
 * polyline, docs/adr/0159) leaves the old kind's elevations too: they were
 * its vertices'; the desktop builds the object from the geometry alike.
 */
function reshaped(e: Entity, g: EntityGeometry): NewEntity {
  const src = e as unknown as Record<string, unknown>;
  const old = new Set([...(SHAPE_FIELDS[e.kind] ?? []), ...(e.kind !== g.kind ? (ELEVATION_FIELDS[e.kind] ?? []) : [])]);
  const geometry = geometryOf(g);
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(src)) {
    if (key === 'kind') out.kind = geometry.kind;
    else if (key in geometry) out[key] = geometry[key];
    else if (!old.has(key)) out[key] = key === 'attrs' ? { ...e.attrs } : src[key];
  }
  for (const key of Object.keys(geometry)) if (!(key in out)) out[key] = geometry[key];
  // A text keeps its link (docs/adr/0175 §4); what is no longer a text writes no object's label.
  if (geometry.kind !== 'text') {
    delete out.labelOf;
    delete out.labelScale;
  }
  return out as unknown as NewEntity;
}

/**
 * What a replacement or a new piece takes from its object: the layer, the
 * colour and its own line weight (a trimmed 0.70 mm line stays 0.70 mm,
 * docs/adr/0139), the attributes and the label with `keepData`; not the
 * symbol (the tools' `inherit`).
 */
function inherited(e: Entity, g: EntityGeometry, keepData: boolean): NewEntity {
  return {
    ...geometryOf(g),
    layerId: e.layerId,
    color: e.color,
    lineWeight: e.lineWeight,
    attrs: keepData ? { ...e.attrs } : {},
    label: keepData ? e.label : undefined,
  } as unknown as NewEntity;
}

/** A new object with what its `add` gives of its own instead of `from`'s (docs/adr/0144). */
function own(init: NewEntity, c: Extract<EntityEdit, { kind: 'add' }>): NewEntity {
  const out = init as unknown as Record<string, unknown>;
  if (c.layerId !== undefined) out.layerId = c.layerId;
  if (c.color !== undefined) out.color = c.color;
  if (c.lineWeight !== undefined) out.lineWeight = c.lineWeight;
  if (c.attrs !== undefined) out.attrs = { ...c.attrs };
  if (c.label !== undefined) out.label = c.label;
  return init;
}

/**
 * A closed area's ring (its outline or a hole) encloses something: 3 corners
 * or more, or 2 when one of its two edges is an arc (a bulge not 0; absent
 * counts as 0). A circle made an area, a lens and a circular segment are
 * such 2-corner rings (the region core writes them so), and the document
 * holds them.
 */
const ringCloses = (pts: readonly unknown[], bulges: readonly number[] | undefined): boolean =>
  pts.length >= 3 || (pts.length === 2 && ((bulges?.[0] ?? 0) !== 0 || (bulges?.[1] ?? 0) !== 0));

/**
 * The `i`-th geometry of the input's `list` (`changes`; `objects` of
 * `cad.entities.create`): enough points for its kind (a closed area's ring by
 * `ringCloses`), a text that is not empty or only white space, every number
 * finite, a positive radius. `whose` names it in a message: “değişikliğin”.
 */
export function checkGeometry(g: EntityGeometry, i: number, list = 'changes', whose = 'değişikliğin'): Stop | null {
  const at = (field: string) => `${list}[${i}].geometry${field}`;
  if (g.kind === 'polyline' && g.pts.length < 2)
    return failed(error('too_few_points', `Çoklu çizginin en az 2 noktası olmalı; ${g.pts.length} nokta verildi. Eksik noktaları ekleyin.`, at('.pts')));
  // A multi-part polyline's other parts are paths as its own is, without holes (docs/adr/0174).
  if (g.kind === 'polyline')
    for (const [k, part] of (g.parts ?? []).entries()) {
      if (part.pts.length < 2)
        return failed(
          error('too_few_points', `${k + 2}. parçanın en az 2 noktası olmalı; ${part.pts.length} nokta verildi. Eksik noktaları ekleyin ya da parçayı çıkarın.`, at(`.parts[${k}].pts`)),
        );
      if (part.holes !== undefined)
        return failed(
          error('part_holes', `Çoklu çizginin ${k + 2}. parçasının deliği olamaz; delik yalnız kapalı alanda olur. Deliği çıkarın.`, at(`.parts[${k}].holes`)),
        );
    }
  if (g.kind === 'polygon') {
    if (!ringCloses(g.pts, g.bulges))
      return failed(error('too_few_corners', `Kapalı alanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); ${g.pts.length} köşe verildi. Eksik köşeleri ekleyin.`, at('.pts')));
    for (const [h, ring] of (g.holes ?? []).entries())
      if (!ringCloses(ring.pts, ring.bulges))
        return failed(
          error(
            'too_few_corners',
            `${h + 1}. deliğin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); ${ring.pts.length} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`,
            at(`.holes[${h}].pts`),
          ),
        );
    // A multi-part area's other parts close as its own ring does (docs/adr/0143); the first is the area's own.
    for (const [k, part] of (g.parts ?? []).entries()) {
      if (!ringCloses(part.pts, part.bulges))
        return failed(
          error(
            'too_few_corners',
            `${k + 2}. parçanın en az 3 köşesi olmalı (kenarlarından biri yaysa 2); ${part.pts.length} köşe verildi. Eksik köşeleri ekleyin ya da parçayı çıkarın.`,
            at(`.parts[${k}].pts`),
          ),
        );
      for (const [h, ring] of (part.holes ?? []).entries())
        if (!ringCloses(ring.pts, ring.bulges))
          return failed(
            error(
              'too_few_corners',
              `${k + 2}. parçanın ${h + 1}. deliğinin en az 3 köşesi olmalı (kenarlarından biri yaysa 2); ${ring.pts.length} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`,
              at(`.parts[${k}].holes[${h}].pts`),
            ),
          );
    }
  }
  if (g.kind === 'hatch') {
    // Its pattern's own fields within their bounds, its tie's too (docs/adr/0186 §1, §6); a number that is not
    // finite is said as such below (`not_finite`).
    const problem = geometryIsFinite(g as unknown as Entity) ? (patternProblem(g.pattern) ?? (g.assoc ? assocProblem(g.assoc) : null)) : null;
    if (problem) return failed(error('invalid_hatch', problem[1], at(`.${problem[0]}`)));
    if (g.ring.length < 3) return failed(error('too_few_corners', `Taramanın en az 3 köşesi olmalı; ${g.ring.length} köşe verildi. Eksik köşeleri ekleyin.`, at('.ring')));
    for (const [h, hole] of (g.holes ?? []).entries())
      if (hole.length < 3)
        return failed(error('too_few_corners', `${h + 1}. deliğin en az 3 köşesi olmalı; ${hole.length} köşe verildi. Eksik köşeleri ekleyin ya da deliği çıkarın.`, at(`.holes[${h}]`)));
  }
  if (g.kind === 'text' && isBlank(g.text))
    return failed(error('empty_text', 'Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin.', at('.text')));
  // A leader (docs/adr/0146 §1): its tip and one vertex more; no note is the field's absence.
  if (g.kind === 'leader' && g.pts.length < 2)
    return failed(error('too_few_points', `Kılavuzun en az 2 köşesi olmalı; ${g.pts.length} köşe verildi. Okun ucunu ve en az bir köşe daha verin.`, at('.pts')));
  if (g.kind === 'leader' && typeof g.text === 'string' && isBlank(g.text))
    return failed(error('empty_text', 'Kılavuzun notu boş olamaz; yalnız boşluktan oluşan not da boştur. Notu yazın ya da notsuz kılavuz için alanı kaldırın.', at('.text')));
  // Elevations as written (docs/adr/0142): one for each vertex.
  const elevations = writtenElevations(g);
  for (const [zs, n, path] of elevations)
    if (zs.length !== n)
      return failed(error('invalid_elevations', `Kotların sayısı köşelerin sayısıyla aynı olmalı; ${n} köşeye ${zs.length} kot verildi. Her köşeye bir kot verin; kotsuz köşeye null.`, at(path)));
  if (!geometryIsFinite(g as unknown as Entity) || elevations.some(([zs]) => zs.some((z) => z !== null && !Number.isFinite(z))))
    return failed(error('not_finite', `${i + 1}. ${whose} geometrisinde sonlu olmayan bir değer var (NaN ya da sonsuz). Geometriyi sonlu sayılarla verin.`, at('')));
  // A dimension its kind's rules and the core can draw (docs/adr/0147 §6).
  const dimension = checkDimension(g, at);
  if (dimension) return dimension;
  if ((g.kind === 'circle' || g.kind === 'arc') && !(g.r > 0)) return failed(error('invalid_radius', 'Yarıçap sıfırdan büyük olmalı. Pozitif bir yarıçap verin.', at('.r')));
  // An insert's scale (docs/adr/0144).
  if (g.kind === 'insert' && !(g.scale > 0)) return failed(error('invalid_scale', 'Blok ölçeği sıfırdan büyük olmalı. Pozitif bir ölçek verin.', at('.scale')));
  // A leader's height measures its note, arrowhead and landing (docs/adr/0146 §1).
  if (g.kind === 'leader' && !(g.height > 0))
    return failed(error('invalid_height', `Kılavuzun yüksekliği sıfırdan büyük olmalı; ${g.height} verildi. Notun yüksekliğini metre olarak, pozitif verin.`, at('.height')));
  // A leader's arrowhead size, times its note's height (docs/adr/0205 §7).
  if (g.kind === 'leader' && g.arrowSize !== undefined && !leaderArrowHolds(g.arrowSize))
    return failed(
      error(
        'invalid_arrow_size',
        `Kılavuzun ok boyu notun yüksekliğinin ${MIN_LEADER_ARROW} ile ${MAX_LEADER_ARROW} katı olmalı; ${g.arrowSize} verildi. Bu aralıkta verin ya da alanı kaldırın (notun yüksekliği kadar).`,
        at('.arrowSize'),
      ),
    );
  // A text's width factor (docs/adr/0145).
  if (g.kind === 'text' && g.widthFactor != null && !widthFactorOk(g.widthFactor))
    return failed(
      error(
        'invalid_width_factor',
        `Yazının genişlik çarpanı 0'dan büyük, en çok ${MAX_WIDTH_FACTOR} olmalı; ${g.widthFactor} verildi. Çarpanı bu aralıkta verin ya da alanı kaldırın (1).`,
        at('.widthFactor'),
      ),
    );
  // A multi-line text's box, spacing and letter formats (docs/adr/0182 §6).
  if (g.kind === 'text') {
    const problem = paragraphProblem(g.text, g);
    if (problem) return failed(error('invalid_paragraph', problem[1], at(`.${problem[0]}`)));
    // Its curve (docs/adr/0196 §1): a text of one line; a link is the create command's to refuse.
    const curve = g.path ? textPathProblem(g.path, g.text, g, false) : null;
    if (curve) return failed(error('invalid_path', curve, at('.path')));
  }
  // A table's rows, columns, cells, merged ranges and source (docs/adr/0184 §6).
  if (g.kind === 'table') {
    const problem = tableProblem(g as unknown as TableShape);
    if (problem) return failed(error('invalid_table', problem[1], at(`.${problem[0]}`)));
  }
  // A picture's size, its one source, its clip and opacity (docs/adr/0192 §6); a number that is not finite is
  // `not_finite`'s, said above.
  if (g.kind === 'image') {
    const problem = imageProblem(g);
    if (problem) return failed(error('invalid_image', problem, at('')));
  }
  // A raster's affine, size, bands, samples, source, look and opacity (docs/adr/0204 §9).
  if (g.kind === 'raster') {
    const problem = rasterProblem(g);
    if (problem) return failed(error('invalid_raster', problem, at('')));
  }
  // A text's face and a dimension's look (docs/adr/0183 §9); a table's face as a text's.
  const style = g.kind === 'text' || g.kind === 'table' ? faceProblem(g) : g.kind === 'dimension' ? lookProblem(g) : null;
  if (style) return failed(error('invalid_style', style[1], at(`.${style[0]}`)));
  return null;
}

/**
 * Every text's and dimension's style among the geometries is the project's, in order (docs/adr/0183 §9):
 * `unknown_style` at `{list}[i].geometry.textStyle` (`.dimStyle`).
 */
export function checkStyles(doc: CadDocument, geometries: readonly (EntityGeometry | null)[], list: string): Stop | null {
  const settings = doc.settings;
  for (const [i, g] of geometries.entries()) {
    const [id, known, what, field] =
      (g?.kind === 'text' || g?.kind === 'table') && g.textStyle !== undefined
        ? [g.textStyle, settings.textStyle(g.textStyle) !== null, 'yazı stili', 'textStyle']
        : g?.kind === 'dimension' && g.dimStyle !== undefined
          ? [g.dimStyle, settings.dimensionStyle(g.dimStyle) !== null, 'ölçü stili', 'dimStyle']
          : [null, true, '', ''];
    if (id !== null && !known)
      return failed(
        error(
          'unknown_style',
          `“${id}” kimlikli ${what} projede yok: silinmiş ya da başka bir projenin olabilir. Projenin bir ${what}nin kimliğini verin ya da alanı kaldırın (Standart).`,
          `${list}[${i}].geometry.${field}`,
        ),
      );
  }
  return null;
}

/** Whether the project's library has the GeoTIFF, PNG or JPEG raster `id` (docs/adr/0204 §2). */
export function hasRaster(doc: CadDocument, id: string): boolean {
  return doc.styles.value.items.some((it) => it.kind === 'asset' && it.id === id && (it.format === 'tiff' || it.format === 'png' || it.format === 'jpeg'));
}

/** Whether the project's library has the PNG or JPEG image `id` (docs/adr/0192 §2). */
export function hasPicture(doc: CadDocument, id: string): boolean {
  return doc.styles.value.items.some((it) => it.kind === 'asset' && it.id === id && (it.format === 'png' || it.format === 'jpeg'));
}

/**
 * Every insert among the geometries names a block of the drawing, in order (docs/adr/0144): `unknown_block` at
 * `{list}[i].geometry.block`; every embedded picture an image of the project's library (docs/adr/0192 §6):
 * `unknown_asset` at `{list}[i].geometry.asset`.
 */
export function checkBlocks(doc: CadDocument, geometries: readonly (EntityGeometry | null)[], list: string): Stop | null {
  const known = new Set(doc.blocks.value.map((b) => b.id));
  for (const [i, g] of geometries.entries()) {
    if (g?.kind === 'raster' && g.asset != null && !hasRaster(doc, g.asset))
      return failed(
        error(
          'unknown_asset',
          `“${g.asset}” kimlikli raster projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. Projenin kitaplığındaki bir GeoTIFF, PNG ya da JPEG'in kimliğini verin.`,
          `${list}[${i}].geometry.asset`,
        ),
      );
    if (g?.kind === 'image' && g.asset !== undefined && !hasPicture(doc, g.asset))
      return failed(
        error(
          'unknown_asset',
          `“${g.asset}” kimlikli görüntü projenin kitaplığında yok: silinmiş ya da başka bir çizimin olabilir. Projenin kitaplığındaki bir PNG ya da JPEG görüntünün kimliğini verin.`,
          `${list}[${i}].geometry.asset`,
        ),
      );
    if (g?.kind === 'insert' && !known.has(g.block))
      return failed(
        error(
          'unknown_block',
          `“${g.block}” kimlikli blok çizimde tanımlı değil: silinmiş ya da başka bir çizimin olabilir. Çizimde tanımlı bir bloğun kimliğini verin.`,
          `${list}[${i}].geometry.block`,
        ),
      );
  }
  return null;
}

/** A geometry's written elevations (docs/adr/0142): each list with its vertex count and its path. */
function writtenElevations(g: EntityGeometry): [readonly (number | null)[], number, string][] {
  const out: [readonly (number | null)[], number, string][] = [];
  if (g.kind === 'line' && g.zs) out.push([g.zs, 2, '.zs']);
  if ((g.kind === 'polyline' || g.kind === 'polygon') && g.zs) out.push([g.zs, g.pts.length, '.zs']);
  // A multi-part polyline's other parts (docs/adr/0174).
  if (g.kind === 'polyline') for (const [k, part] of (g.parts ?? []).entries()) if (part.zs) out.push([part.zs, part.pts.length, `.parts[${k}].zs`]);
  if (g.kind === 'polygon') {
    for (const [h, ring] of (g.holes ?? []).entries()) if (ring.zs) out.push([ring.zs, ring.pts.length, `.holes[${h}].zs`]);
    // A multi-part area's other parts (docs/adr/0143).
    for (const [k, part] of (g.parts ?? []).entries()) {
      if (part.zs) out.push([part.zs, part.pts.length, `.parts[${k}].zs`]);
      for (const [h, ring] of (part.holes ?? []).entries()) if (ring.zs) out.push([ring.zs, ring.pts.length, `.parts[${k}].holes[${h}].zs`]);
    }
  }
  return out;
}

/** Whether the geometry carries its elevations (docs/adr/0142): a multi-part area's or polyline's when its own or a part's list is given (docs/adr/0143, 0174). */
const written = (g: EntityGeometry): boolean =>
  ('zs' in g && g.zs !== undefined) || ((g.kind === 'polygon' || g.kind === 'polyline') && !!g.parts?.some((part) => part.zs !== undefined));

/**
 * Written elevations as the object holds them (docs/adr/0142): a line's two as `za` and `zb`, a path's
 * or a hole's all null as none. In their places, so an edit that changes nothing stays no edit.
 */
function held(init: NewEntity): NewEntity {
  const out = init as unknown as Record<string, unknown>;
  const none = (zs: unknown) => Array.isArray(zs) && zs.every((z) => z === null);
  if (init.kind === 'line' && Array.isArray(out.zs)) {
    const [za, zb] = out.zs as (number | null)[];
    delete out.zs;
    if (za != null) out.za = za;
    else delete out.za;
    if (zb != null) out.zb = zb;
    else delete out.zb;
  }
  if ((init.kind === 'polyline' || init.kind === 'polygon') && none(out.zs)) delete out.zs;
  const bare = <T extends { zs?: (number | null)[] }>(ring: T): T => (none(ring.zs) ? (({ zs: _zs, ...rest }) => rest as T)(ring) : ring);
  if (init.kind === 'polygon' && init.holes) init.holes = init.holes.map(bare);
  // A part's elevations, and its holes', are held as the area's own are (docs/adr/0143); a polyline's part's as the polyline's (docs/adr/0174).
  if (init.kind === 'polygon' && init.parts) init.parts = init.parts.map((part) => ({ ...bare(part), ...(part.holes && { holes: part.holes.map(bare) }) }));
  if (init.kind === 'polyline' && init.parts) init.parts = init.parts.map(bare);
  return init;
}

/** The checks in the contract's order: why nothing may be written, or what may. */
function check(doc: CadDocument, input: EntitiesEdit): Stop | Checked {
  if (!input.changes.length) return failed(error('no_changes', 'Yapılacak değişiklik verilmedi. En az bir değişiklik verin.', 'changes'));
  for (const [i, c] of input.changes.entries()) {
    const [uid, field] = named(c);
    if (!isUuid(uid))
      return failed(
        error(
          'invalid_uid',
          `“${uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.`,
          `changes[${i}].${field}`,
        ),
      );
  }
  for (const [i, c] of input.changes.entries()) {
    const stop = (c.kind === 'remove' ? null : checkGeometry(c.geometry, i)) ?? (c.kind === 'add' ? checkLineWeight(c.lineWeight, `changes[${i}].lineWeight`) : null);
    if (stop) return stop;
  }
  const stop = checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found: Entity[] = [];
  for (const [i, c] of input.changes.entries()) {
    const [uid, field] = named(c);
    const e = doc.byUid(uid);
    if (!e)
      return failed(error('entity_not_found', `“${uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.`, `changes[${i}].${field}`));
    found.push(e);
  }
  // An object changes once: an `add` may come from one that changes.
  const targets = new Set<number>();
  for (const [i, c] of input.changes.entries()) {
    if (c.kind === 'add') continue;
    if (targets.has(found[i].id))
      return failed(error('repeated_entity', `“${c.uid}” kimlikli nesne birden çok değişiklikte değişiyor. Bir nesneye tek değişiklik verin.`, `changes[${i}].uid`));
    targets.add(found[i].id);
  }
  // Nothing on a locked layer is changed or copied from.
  for (const [i, c] of input.changes.entries()) {
    const layerId = found[i].layerId;
    if (!doc.layers.isLocked(layerId)) continue;
    const name = doc.layers.get(layerId)?.name ?? layerId;
    return failed(error('layer_locked', `“${name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın.`, `changes[${i}].${named(c)[1]}`));
  }
  // Tabloyu düzenle and Tabloyu güncelle write tables over tables (docs/adr/0184 §6).
  if (input.operation === 'table' || input.operation === 'tableUpdate')
    for (const [i, c] of input.changes.entries())
      if (!(c.kind === 'update' && c.geometry.kind === 'table' && found[i].kind === 'table'))
        return failed(
          error('not_a_table', 'Tablonun düzenlemesi yalnız tabloları değiştirir: her değişiklik bir tablonun yeni hâli olmalı (update, tablo geometrisi).', `changes[${i}]`),
        );
  // A new object's own layer: known, a layer, not locked (a block's object exploded, docs/adr/0144).
  for (const [i, c] of input.changes.entries()) {
    if (c.kind !== 'add' || c.layerId === undefined) continue;
    const layer = checkLayer(doc, c.layerId, `changes[${i}].layerId`);
    if (!Array.isArray(layer)) return layer;
  }
  const blocks = checkBlocks(
    doc,
    input.changes.map((c) => (c.kind === 'remove' ? null : c.geometry)),
    'changes',
  );
  if (blocks) return blocks;
  // A text's and a dimension's style is the project's (docs/adr/0183 §9).
  const styles = checkStyles(
    doc,
    input.changes.map((c) => (c.kind === 'remove' ? null : c.geometry)),
    'changes',
  );
  if (styles) return styles;
  const checked: Checked = { changed: [], created: [], removed: [], warnings: [] };
  // The elevations of the objects the edit names, for what it writes (docs/adr/0142); nothing to carry
  // when none has one.
  const sources = found.flatMap((e) => elevatedPaths(e)).filter((p) => hasElevation([p]));
  const offset = input.operation === 'offset';
  // The object keeps its own vertices, moved (a grip, Esnet, a typed coordinate), or Ötele's copy has one
  // for each of its source's: a vertex takes the elevation of the one in its place. Any other edit cuts or
  // reshapes, and its vertices take theirs by where they lie.
  const byPlace = offset || input.operation === 'grip' || input.operation === 'stretch' || input.operation === 'properties' || input.operation === 'edgeShift';
  let lost = 0;
  // Deliği doldur's area takes its hole's elevations, if it has any: its area keeps its own (docs/adr/0173 §5).
  const keeps = input.operation === 'holeFill';
  // Elevations written with the geometry are written as they are (Kot ver, Öznitelikler, a script).
  const elevate = (init: NewEntity, from: Entity, g: EntityGeometry): NewEntity => {
    if (written(g)) return held(init);
    if (!sources.length) return init;
    const before = elevatedPaths(from);
    if (!carryInto(init, sources, byPlace ? before : [], offset) && hasElevation(before) && !keeps) lost++;
    return init;
  };
  for (const [i, c] of input.changes.entries()) {
    const e = found[i];
    if (c.kind === 'update') checked.changed.push({ id: e.id, uid: c.uid, init: elevate(reshaped(e, c.geometry), e, c.geometry) });
    else if (c.kind === 'replace') checked.changed.push({ id: e.id, uid: c.uid, init: elevate(inherited(e, c.geometry, c.keepData === true), e, c.geometry) });
    else if (c.kind === 'add') checked.created.push(elevate(own(inherited(e, c.geometry, c.keepData === true), c), e, c.geometry));
    else checked.removed.push({ id: e.id, uid: c.uid });
  }
  if (lost) checked.warnings.push({ code: 'elevation_lost', message: `${lost} nesnenin kotu bu işlemde korunmadı.`, path: 'changes' });
  return checked;
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/**
 * An object as the plan shows it, without undefined fields or its persistent
 * id (an updated object's carries one; the contract's `Entity` has none): its
 * slot, or 0 for a new one.
 */
function planned(init: NewEntity, id: number): PlannedEntity {
  const { uid: _uid, ...rest } = JSON.parse(JSON.stringify({ ...init, id })) as NewEntity & { uid?: string };
  return rest as unknown as PlannedEntity;
}

export const entitiesEdit: ProductCommand<EntitiesEdit, EntitiesEdited, EntitiesEditPlan> = {
  id: 'cad.entities.edit',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : []);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    return {
      status: 'completed',
      output: {
        changed: checked.changed.map((c) => planned(c.init, c.id)),
        created: checked.created.map((init) => planned(init, 0)),
        removed: checked.removed.map((r) => r.uid),
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },

  /**
   * Writes one undo step named after the tool, through the document's own
   * edits: the deletions, the objects changed in their places (`replace`:
   * slot and persistent id kept), then the new objects (`addMany`). Into the
   * open transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const { doc } = cx;
    const label = EDIT_LABEL[input.operation];
    const created = doc.transact(label, () => {
      doc.remove(checked.removed.map((r) => r.id));
      for (const c of checked.changed) doc.replace(c.id, c.init, label);
      return doc.addMany(checked.created, label).map((e) => e.uid);
    });
    return {
      status: 'completed',
      output: { changed: checked.changed.map((c) => c.uid), created, removed: checked.removed.map((r) => r.uid), revision: String(doc.revision) },
      warnings: checked.warnings,
    };
  },
};
