import type { CommandWarning } from '../contracts/generated/CommandWarning';
import type { EntitiesTransform } from '../contracts/generated/EntitiesTransform';
import type { EntitiesTransformed } from '../contracts/generated/EntitiesTransformed';
import type { EntitiesTransformPlan } from '../contracts/generated/EntitiesTransformPlan';
import type { Entity as PlannedEntity } from '../contracts/generated/Entity';
import type { Transform } from '../contracts/generated/Transform';
import type { CadDocument } from '../model/document';
import type { Entity, NewEntity } from '../model/entities';
import { withoutLink } from '../model/linkedTexts';
import { geometryIsFinite, transformObjects, withGeometry } from '../model/ops/transform';
import { ARRANGE_LABELS, aligns, arrangeBoxes, arrangeMoves } from '../model/ops/arrange';
import { rubberSheet } from '../model/ops/rubber';
import { rubberShapes, warpShapes, type Warp } from '../model/ops/warp';
import { fixed } from '../core/displayNumber';
import type { Geometry } from '../wasm/pack';
import { checkRevision, checkUids, error, failed, findObjects, notFinite, notFiniteValue, validated, type Stop } from './checks';
import type { ProductCommand } from './command';
import { assignElevations, elevatedPaths } from './elevation';

/**
 * `cad.entities.transform` v1 (docs/adr/0037, 0047): objects named by their
 * persistent ids moved, rotated, scaled, mirrored or aligned as one undo
 * step, in place or as copies. The web's handler over `CadDocument`; the
 * desktop's is `crates/native/application/src/transform.rs`. Both pass the
 * shared cases in fixtures/commands/v1/cad.entities.transform.json.
 *
 * The modify tools (Taşı, Kopyala, Döndür, Ölçekle, Aynala, Hizala) make the
 * selection explicit here (TODOS.md CMD-07). The geometry is the shared
 * core's, as on the desktop: the core builds the matrix from the
 * transform's numbers and moves every kind of object, packed, with no JSON
 * (`transformObjects`); nothing is computed here.
 *
 * The checks, in order (the first that fails answers): at least one id,
 * each lowercase UUID text with hyphens; the transform's numbers finite in
 * their order, a scale factor above zero, a mirror axis with a direction,
 * an alignment's second pair whole and apart from the first; the expected
 * revision (checks.ts); each id names an object; not every object on a
 * locked layer; no coordinate carried past the largest float64. A repeated
 * id counts once.
 *
 * Objects on a locked layer are neither changed nor copied. The tools used
 * to copy them onto their locked layer; ADR 0037 records the change.
 *
 * Oturt's similarity, affine and projective transforms (docs/adr/0156) are
 * the core's warp (`warpShapes`): what it does to each kind, with the paths'
 * elevations; an object may change its kind (a circle becomes an ellipse or
 * a polyline). Their checks add, after the numbers, a transform that
 * squashes the plane (`invalid_transform`) and, after the locked layers, a
 * point beyond a projective transform's horizon (`beyond_horizon`); a
 * transform that is not a similarity warns how many curves became straight
 * vertices and how many texts, notes, blocks, dimensions and hatch patterns
 * kept their shape.
 *
 * Kauçuk levha (docs/adr/0158) is the core's sheet (`rubberSheet`) and its
 * rules (`rubberShapes`): only vertices move, kinds stay. Its checks add,
 * after the links' numbers, links that give no sheet (`invalid_links`); it
 * warns how many shapes bend over 0.1 mm from their true image.
 *
 * Hizala ve dağıt (docs/adr/0194) moves each object on its own: the objects'
 * boxes (`arrangeBoxes`, the drawing's blocks and typeface) give each its
 * displacement (`arrangeMoves`), which moves it as Taşı does. Its checks
 * add, after its number, `at` for an alignment and only for one
 * (`invalid_transform`) and, after the locked layers, a spread of fewer than
 * three objects (`too_few_objects`).
 */

interface Checked {
  /** The objects to transform, with their ids, in the input's order. */
  sources: { entity: Entity; uid: string }[];
  /** Each of them as it would be written, in the same order (slot and persistent id its own). */
  moved: Entity[];
  /** The ids left alone on locked layers. */
  locked: string[];
  warnings: CommandWarning[];
}

const lockedMessage = (n: number) => `${n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın.`;

/** The undo step's name: the tool's (docs/adr/0037). */
export function transformLabel(t: Transform, copy: boolean): string {
  switch (t.kind) {
    case 'move':
      return copy ? 'Kopyala' : 'Taşı';
    case 'rotate':
      return 'Döndür';
    case 'scale':
      return 'Ölçekle';
    case 'mirror':
      return 'Aynala';
    case 'align':
      return 'Hizala';
    case 'similarity':
    case 'affine':
    case 'projective':
      return 'Oturt';
    case 'rubbersheet':
      return 'Kauçuk levha';
    case 'arrange':
      return ARRANGE_LABELS[t.mode];
  }
}

/** An arrangement's own checks after its number: `at` for an alignment and only for one (`invalid_transform`). */
function checkArrange(t: Extract<Transform, { kind: 'arrange' }>): Stop | null {
  if (t.at !== undefined) {
    const stop = notFiniteValue(t.at, 'Hizalamanın doğusu ya da kuzeyi', 'Değeri sonlu bir sayıyla verin.', 'transform.at');
    if (stop) return stop;
  }
  if (aligns(t.mode) && t.at === undefined)
    return failed(error('invalid_transform', 'Hizalamanın varacağı doğu ya da kuzey (at) verilmedi. Başvurunun kenarının ya da ortasının değerini verin.', 'transform.at'));
  if (!aligns(t.mode) && t.at !== undefined) return failed(error('invalid_transform', 'Dağıtma bir değere hizalamaz; at verilmez. Değeri çıkarın ya da bir hizalama kipi seçin.', 'transform.at'));
  return null;
}

/** The objects moved each by its own displacement (Hizala ve dağıt), or why not: a spread needs three. */
function arranged(doc: CadDocument, sources: readonly Entity[], t: Extract<Transform, { kind: 'arrange' }>): Stop | Entity[] {
  if (!aligns(t.mode) && sources.length < 3)
    return failed(error('too_few_objects', `${ARRANGE_LABELS[t.mode]} için en az üç nesne gerekir (kilitli katmandakiler sayılmaz). Dağıtılacak nesneleri seçin.`, 'uids'));
  const boxes = arrangeBoxes(sources, doc.blocks.value, doc.settings.drawingFont.value);
  const moves = arrangeMoves(boxes, t.mode, t.at ?? null);
  return sources.map((e, i) => transformObjects([e], { kind: 'move', dx: moves[i].x, dy: moves[i].y })[0]);
}

const NUMBER_FIX = 'Dönüşümün sayılarını sonlu verin.';

/** Whether a linear part [a, b, c, d] squashes the plane: its determinant within 1e-12 of its columns' lengths' product. */
const singular = ([a, b, c, d]: readonly number[]) => Math.abs(a * d - b * c) <= 1e-12 * Math.hypot(a, b) * Math.hypot(c, d);

const SINGULAR = failed(
  error('invalid_transform', 'Dönüşüm tekil: doğrusal kısmı nesneleri bir doğruya ya da noktaya ezer. Dönüşümün sayılarını denetleyin ya da başka bir dönüşüm türü seçin.', 'transform'),
);

/** Oturt's centres and numbers, in their order: finite (named by their place), then not singular. */
function checkWarp(t: Extract<Transform, { kind: 'similarity' | 'affine' | 'projective' }>): Stop | null {
  const centres = notFinite(t.from, 'Kaynak merkezinin', 'transform.from') ?? notFinite(t.to, 'Hedef merkezinin', 'transform.to');
  if (centres) return centres;
  const numbers = (values: readonly number[], field: string) => {
    for (let i = 0; i < values.length; i++) {
      const stop = notFiniteValue(values[i], `Dönüşümün ${i + 1}. sayısı`, NUMBER_FIX, `transform.${field}[${i}]`);
      if (stop) return stop;
    }
    return null;
  };
  switch (t.kind) {
    case 'similarity':
      return notFiniteValue(t.a, 'Dönüşümün a sayısı', NUMBER_FIX, 'transform.a') ?? notFiniteValue(t.b, 'Dönüşümün b sayısı', NUMBER_FIX, 'transform.b') ?? (singular([t.a, t.b, -t.b, t.a]) ? SINGULAR : null);
    case 'affine':
      return numbers(t.m, 'm') ?? (singular(t.m) ? SINGULAR : null);
    case 'projective': {
      const stop = numbers(t.h, 'h');
      if (stop) return stop;
      // The derivative at the source centre (w = 1 there).
      const [a1, a2, a3, b1, b2, b3, c1, c2] = t.h;
      return singular([a1 - a3 * c1, b1 - b3 * c1, a2 - a3 * c2, b2 - b3 * c2]) ? SINGULAR : null;
    }
  }
}

/**
 * An alignment's own checks after its numbers (`invalid_align`): the second
 * pair whole, its points apart from the first pair's by the core's own
 * measure (`align`: within a nanometre the direction is lost; `Math.hypot`
 * is the core's `js_hypot`).
 */
function checkAlign(t: Extract<Transform, { kind: 'align' }>): Stop | null {
  const refuse = (message: string, path: string) => failed(error('invalid_align', message, path));
  const { source2, target2 } = t;
  if (source2 && !target2)
    return refuse(
      'Hizalamanın ikinci hedef noktası verilmedi; ikinci çift iki noktayla verilir. İkinci hedef noktasını verin ya da ikinci kaynak noktasını çıkarın.',
      'transform.target2',
    );
  if (!source2 && target2)
    return refuse(
      'Hizalamanın ikinci kaynak noktası verilmedi; ikinci çift iki noktayla verilir. İkinci kaynak noktasını verin ya da ikinci hedef noktasını çıkarın.',
      'transform.source2',
    );
  if (!source2 || !target2) return null;
  if (!(Math.hypot(source2.x - t.source.x, source2.y - t.source.y) >= 1e-9))
    return refuse('Kaynak noktaları çakışıyor; kaynak doğrultusunun yönü yok. Birbirinden ayrı iki kaynak noktası verin.', 'transform.source2');
  if (!(Math.hypot(target2.x - t.target.x, target2.y - t.target.y) >= 1e-9))
    return refuse('Hedef noktaları çakışıyor; hedef doğrultusunun yönü yok. Birbirinden ayrı iki hedef noktası verin.', 'transform.target2');
  return null;
}

/** Why a rubber sheet's links give no sheet, in the user's words (the desktop's `links_message`). */
const LINKS: Record<string, string> = {
  too_few: 'Kauçuk levha için en az 3 bağ gerekir. Bağ ekleyin.',
  duplicate: "İki bağın kaynağı aynı nokta. Birini çıkarın ya da Kullan'dan bırakın.",
  collinear: 'Bağların kaynakları bir doğru üstünde; levha kurulamaz. Doğrunun dışında bir bağ ekleyin.',
  singular: 'Bağların denklem takımının tek çözümü yok. Birbirine çok yakın kaynakları birleştirin.',
  too_many: 'En çok 1000 bağ alınır. Bağları azaltın.',
};

/** Kauçuk levha's links: finite (named by their place), then one sheet (the core's own solution). */
function checkLinks(t: Extract<Transform, { kind: 'rubbersheet' }>): Stop | null {
  for (let i = 0; i < t.links.length; i++) {
    const l = t.links[i];
    const stop = notFinite(l.from, `${i + 1}. bağın kaynağının`, `transform.links[${i}].from`) ?? notFinite(l.to, `${i + 1}. bağın hedefinin`, `transform.links[${i}].to`);
    if (stop) return stop;
  }
  const answer = rubberSheet(t.links, []);
  return 'error' in answer ? failed(error('invalid_links', LINKS[answer.error] ?? LINKS.singular, 'transform.links')) : null;
}

/** The rubber sheet's warning: how many shapes bend over 0.1 mm and the largest, mm with one decimal (the display rule), a decimal comma. */
export const bendsMessage = (bent: number, bend: number) =>
  `${bent} nesne gerçek görüntüsünden 0,1 mm'den çok sapıyor (en çok ${fixed(bend * 1000, 1).replace('.', ',')} mm): kauçuk levha yalnız köşeleri taşır, kenarlar doğru, yaylar şişkinliğiyle kalır.`;

/** The transform's own checks, in its fields' order: finite numbers, then what they mean. */
function checkTransform(t: Transform): Stop | null {
  switch (t.kind) {
    case 'move': {
      const fix = 'Kaydırmayı sonlu bir sayıyla verin.';
      return notFiniteValue(t.dx, 'Doğu (Y) yönündeki kaydırma', fix, 'transform.dx') ?? notFiniteValue(t.dy, 'Kuzey (X) yönündeki kaydırma', fix, 'transform.dy');
    }
    case 'rotate':
      return notFinite(t.center, 'Merkezin', 'transform.center') ?? notFiniteValue(t.angle, 'Dönme açısı', 'Açıyı sonlu bir sayıyla verin.', 'transform.angle');
    case 'scale':
      return (
        notFinite(t.center, 'Merkezin', 'transform.center') ??
        notFiniteValue(t.factor, 'Ölçek faktörü', 'Faktörü sonlu bir sayıyla verin.', 'transform.factor') ??
        (t.factor > 0 ? null : failed(error('invalid_factor', 'Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin.', 'transform.factor')))
      );
    case 'mirror': {
      const stop = notFinite(t.a, 'Eksenin ilk noktasının', 'transform.a') ?? notFinite(t.b, 'Eksenin ikinci noktasının', 'transform.b');
      if (stop) return stop;
      // The core's own measure of the axis (`mirror`): zero leaves it no direction.
      const dx = t.b.x - t.a.x;
      const dy = t.b.y - t.a.y;
      return dx * dx + dy * dy === 0 ? failed(error('invalid_axis', 'Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin.', 'transform.b')) : null;
    }
    case 'align':
      return (
        notFinite(t.source, 'Birinci kaynak noktasının', 'transform.source') ??
        notFinite(t.target, 'Birinci hedef noktasının', 'transform.target') ??
        (t.source2 ? notFinite(t.source2, 'İkinci kaynak noktasının', 'transform.source2') : null) ??
        (t.target2 ? notFinite(t.target2, 'İkinci hedef noktasının', 'transform.target2') : null) ??
        checkAlign(t)
      );
    case 'similarity':
    case 'affine':
    case 'projective':
      return checkWarp(t);
    case 'rubbersheet':
      return checkLinks(t);
    case 'arrange':
      return checkArrange(t);
  }
}

const HORIZON =
  'Projektif dönüşümün ufku nesnelerin arasından geçiyor: bir nesnenin noktası ufkun ötesinde kalıyor, dönüştürülemez. O nesneleri dışarıda bırakın ya da kontrol noktalarını denetleyin.';

/**
 * The objects under Oturt's warp (the core's `warpShapes`, with their paths' elevations), each keeping its own
 * fields; a curve may come back of another kind. The counts of the warnings, or why nothing may be written.
 */
function warped(sources: readonly Entity[], warp: Warp): Stop | { moved: Entity[]; curves: number; kept: number } {
  const answer = warpShapes(
    sources,
    sources.map((e) => elevatedPaths(e).map((p) => p.zs)),
    warp,
  );
  if ('error' in answer) return failed(error('beyond_horizon', HORIZON, 'transform'));
  const moved = answer.shapes.map((shape, i) => {
    const e = withGeometry(sources[i], shape as unknown as Geometry);
    assignElevations(e, answer.zs[i]);
    return e;
  });
  return { moved, curves: answer.curves, kept: answer.kept };
}

/** The objects on a rubber sheet (the core's `rubberShapes`), each keeping its kind and fields; the warnings' counts. */
function onSheet(sources: readonly Entity[], links: Extract<Transform, { kind: 'rubbersheet' }>['links']): Stop | { moved: Entity[]; kept: number; bent: number; bend: number } {
  const answer = rubberShapes(
    sources,
    sources.map((e) => elevatedPaths(e).map((p) => p.zs)),
    links,
  );
  if ('error' in answer) return failed(error('invalid_links', LINKS[answer.error] ?? LINKS.singular, 'transform.links'));
  const moved = answer.shapes.map((shape, i) => {
    const e = withGeometry(sources[i], shape as unknown as Geometry);
    assignElevations(e, answer.zs[i]);
    return e;
  });
  return { moved, kept: answer.kept, bent: answer.bent, bend: answer.bend };
}

/** The checks in the contract's order: why nothing may be written, or what may. */
function check(doc: CadDocument, input: EntitiesTransform): Stop | Checked {
  const stop = checkUids(input.uids, 'Dönüştürülecek nesne verilmedi.') ?? checkTransform(input.transform) ?? checkRevision(doc, input.expectedRevision);
  if (stop) return stop;
  const found = findObjects(doc, input.uids);
  if ('status' in found) return found;
  const sources: Checked['sources'] = [];
  const locked: string[] = [];
  for (const f of found) {
    if (doc.layers.isLocked(f.entity.layerId)) locked.push(f.uid);
    else sources.push(f);
  }
  if (!sources.length) return failed(error('layer_locked', lockedMessage(locked.length), 'uids'));
  const t = input.transform;
  const warp = t.kind === 'similarity' || t.kind === 'affine' || t.kind === 'projective' ? warped(sources.map((s) => s.entity), t) : null;
  if (warp && 'status' in warp) return warp;
  const sheet = t.kind === 'rubbersheet' ? onSheet(sources.map((s) => s.entity), t.links) : null;
  if (sheet && 'status' in sheet) return sheet;
  const arrange = t.kind === 'arrange' ? arranged(doc, sources.map((s) => s.entity), t) : null;
  if (arrange && 'status' in arrange) return arrange;
  const moved = warp
    ? warp.moved
    : sheet
      ? sheet.moved
      : arrange
        ? arrange
        : transformObjects(
            sources.map((s) => s.entity),
            t,
          );
  if (sources.some((s, i) => geometryIsFinite(s.entity) && !geometryIsFinite(moved[i])))
    return failed(error('not_finite', 'Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.', 'transform'));
  const warnings: CommandWarning[] = locked.length ? [{ code: 'layer_locked', message: lockedMessage(locked.length), path: 'uids' }] : [];
  if (warp?.curves)
    warnings.push({ code: 'warp_curves', message: `${warp.curves} nesnenin eğrileri 0,1 mm'lik köşelere açıldı; dönüşüm benzerlik değil, eğri olarak kalamazlar.`, path: 'transform' });
  const kept = warp?.kept ?? sheet?.kept ?? 0;
  if (kept)
    warnings.push({
      code: 'warp_shapes',
      message: `${kept} yazı, not, blok, ölçü ya da tarama deseni yerinde döndürülüp ölçeklendi; dönüşüm benzerlik değil, biçimleri eğilmez.`,
      path: 'transform',
    });
  if (sheet?.bent) warnings.push({ code: 'rubber_bends', message: bendsMessage(sheet.bent, sheet.bend), path: 'transform' });
  return { sources, moved, locked, warnings };
}

const isStop = (c: Stop | Checked): c is Stop => 'status' in c;

/** An object as the plan shows it: without its persistent id; a copy's slot is 0, given when it is written. */
function planned(e: Entity, copy: boolean): PlannedEntity {
  const { uid: _uid, ...rest } = e as Entity & { uid?: string };
  return (copy ? { ...withoutLink(rest), id: 0 } : rest) as unknown as PlannedEntity;
}

export const entitiesTransform: ProductCommand<EntitiesTransform, EntitiesTransformed, EntitiesTransformPlan> = {
  id: 'cad.entities.transform',
  version: 1,

  validate(cx, input) {
    const checked = check(cx.doc, input);
    return validated(isStop(checked) ? checked : checked.warnings);
  },

  plan(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const copy = input.copy === true;
    return {
      status: 'completed',
      output: {
        sources: checked.sources.map((s) => s.uid),
        entities: checked.moved.map((e) => planned(e, copy)),
        locked: checked.locked,
        revision: String(cx.doc.revision),
      },
      warnings: checked.warnings,
    };
  },

  /**
   * Writes one undo step named after the tool, through the document's own
   * edits: the objects changed in place (`updateMany`), or their copies
   * added (`addMany`, new persistent ids). Into the open transaction or group, if one is.
   */
  execute(cx, input) {
    const checked = check(cx.doc, input);
    if (isStop(checked)) return checked;
    const copy = input.copy === true;
    const label = transformLabel(input.transform, copy);
    let changed: string[] = [];
    let created: string[] = [];
    if (copy) {
      // A linked text's copy is a text of its own (docs/adr/0175 §4).
      const copies = checked.moved.map((e) => {
        const { id: _id, uid: _uid, ...rest } = withoutLink(e) as Entity & { uid?: string };
        return rest as NewEntity;
      });
      created = cx.doc.addMany(copies, label).map((e) => e.uid);
    } else {
      cx.doc.updateMany(checked.moved, label);
      changed = checked.sources.map((s) => s.uid);
    }
    return {
      status: 'completed',
      output: { changed, created, locked: checked.locked, revision: String(cx.doc.revision) },
      warnings: checked.warnings,
    };
  },
};
