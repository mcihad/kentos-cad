import { commandItem, resolveMenu } from '../../app/menus';
import type { AppContext } from '../../app/context';
import type { Disposable } from '../../core/disposable';
import type { Entity, EntityGeometry, PolylineEntity } from '../../model/entities';
import { bulgeAt, bulgeRingArea, isArcBulge, segmentMid } from '../../model/geom/bulge';
import { holeGrip, midGripSegment } from '../../model/ops/grips';
import { insertVertex, removeVertex } from '../../model/ops/vertex';
import { SNAP_LABEL, type SnapKind } from '../../viewport/picking';
import type { Vec2 } from '../../model/geometry';
import { canCalcPoint } from '../../tools/pointCalc';
import { calcMenuItems } from './calcMenu';
import { parsePrompt, runPromptOption } from '../promptOptions';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';

/** Snap kinds offered as one-shot overrides, in the order surveyors reach for them. */
const SNAP_ORDER: SnapKind[] = ['endpoint', 'midpoint', 'intersection', 'center', 'perpendicular', 'tangent', 'quadrant', 'node', 'nearest'];

/** Menu icons drawn like the snap markers on the canvas. */
const SNAP_ICON: Record<SnapKind, string> = {
  endpoint: 'snapEndpoint',
  midpoint: 'snapMidpoint',
  center: 'snapCenter',
  node: 'snapNode',
  quadrant: 'snapQuadrant',
  intersection: 'snapIntersection',
  perpendicular: 'snapPerpendicular',
  tangent: 'snapTangent',
  nearest: 'snapNearest',
};

/**
 * Right-button menus over the drawing: the idle menu (with grip actions
 * when a grip is under the cursor), the command menu (hold the right
 * button while a command runs) and the one-shot snap menu (Shift + right).
 */
export function bindViewportMenus(ctx: AppContext): Disposable {
  return ctx.view.events.on('contextmenu', ({ clientX, clientY, screen, kind }) => {
    const at = { x: clientX, y: clientY };
    if (kind === 'snap') return PopupMenu.open(snapItems(ctx, true), at, { minWidth: 220 });
    if (kind === 'command') return PopupMenu.open(commandItems(ctx), at, { minWidth: 240 });
    PopupMenu.open([...gripItems(ctx, screen), ...idleItems(ctx)], at, { minWidth: 220 });
  });
}

function snapItems(ctx: AppContext, withHeader: boolean): MenuItem[] {
  const current = ctx.view.snapOverride.value;
  return [
    ...(withHeader ? [{ kind: 'header' as const, label: 'Tek seferlik kenet (sonraki tık)' }] : []),
    ...SNAP_ORDER.map((k): MenuItem => ({ label: SNAP_LABEL[k], icon: SNAP_ICON[k], checked: current === k, run: () => ctx.view.snapOverride.set(k) })),
    { kind: 'separator' },
    { label: 'Kenet ayarları…', icon: 'settings', run: () => ctx.commands.execute('tools.options') },
  ];
}

function commandItems(ctx: AppContext): MenuItem[] {
  const p = parsePrompt(ctx.tools.prompt.value);
  const options = p.options.filter((o) => o.key !== 'Enter' && o.key !== 'Esc');
  return [
    { kind: 'header', label: p.tool || 'Komut' },
    { label: 'Onayla / bitir', icon: 'check', shortcut: 'Enter', run: () => ctx.commands.execute('tool.confirm') },
    { label: 'İptal', icon: 'close', shortcut: 'Esc', run: () => ctx.commands.execute('tool.cancel') },
    ...(options.length
      ? [
          { kind: 'separator' as const },
          ...options.map((o): MenuItem => ({ label: o.value ? `${o.label}: ${o.value}` : o.label, hint: o.key, run: () => runPromptOption(ctx, o.key) })),
        ]
      : []),
    { kind: 'separator' },
    ...(canCalcPoint(ctx)
      ? [{ label: 'Nokta hesapla', icon: 'calc', items: () => calcMenuItems(ctx) }]
      : []),
    { label: 'Tek seferlik kenet', icon: 'snap', items: () => snapItems(ctx, false) },
    commandItem(ctx, 'draft.snap'),
    commandItem(ctx, 'draft.ortho'),
    commandItem(ctx, 'draft.polar'),
    commandItem(ctx, 'draft.tracking'),
    { kind: 'separator' },
    commandItem(ctx, 'view.zoomExtents'),
  ];
}

function idleItems(ctx: AppContext): MenuItem[] {
  const last = ctx.tools.lastToolLabel;
  const items = resolveMenu(ctx, [
    ...(last ? ['tool.repeat'] : []),
    '-',
    'view.zoomExtents',
    'view.zoomSelection',
    'tool.pan',
    '-',
    'edit.selectAll',
    'edit.deselect',
    '-',
    'tool.move',
    'tool.copy',
    'edit.copy',
    'edit.paste',
    'tool.erase',
    '-',
    'view.coords',
  ]).filter((it, i, arr) => !(it.kind === 'separator' && (i === 0 || arr[i - 1].kind === 'separator')));
  if (last && items[0]) items[0].label = `Yinele: ${last}`;
  return items;
}

/** Actions for the grip under the cursor of a selected polyline or polygon. */
function gripItems(ctx: AppContext, screen: Vec2): MenuItem[] {
  const hit = ctx.view.gripAt(screen);
  const e = hit && ctx.doc.get(hit.id);
  // Hole vertices only move by dragging; editing a hole's shape needs Patlat.
  if (!hit || !e || (e.kind !== 'polyline' && e.kind !== 'polygon') || holeGrip(e, hit.index)) return [];
  const seg = midGripSegment(e, hit.index);
  const apply = (label: string, r: { geometry: EntityGeometry } | { error: string }) => {
    if ('error' in r) return ctx.log.warn(r.error);
    ctx.doc.transact(label, () => ctx.doc.update(e.id, { bulges: undefined, ...r.geometry } as Partial<Entity>));
    ctx.log.success(`${label}: tamam.`);
  };
  if (seg === null) {
    return [
      { kind: 'header', label: `Köşe ${hit.index + 1}` },
      { label: 'Köşeyi sil', icon: 'erase', run: () => apply('Köşeyi sil', removeVertex(e, hit.index)) },
      { kind: 'separator' },
    ];
  }
  const n = e.pts.length;
  const a = e.pts[seg];
  const b = e.pts[(seg + 1) % n];
  const arc = isArcBulge(bulgeAt(e.bulges, seg));
  return [
    { kind: 'header', label: `Kenar ${seg + 1}` },
    { label: 'Ortasına köşe ekle', icon: 'vertex', run: () => apply('Köşe ekle', insertVertex(e, seg, segmentMid(a, b, bulgeAt(e.bulges, seg)))) },
    arc
      ? { label: 'Düz kenar yap', icon: 'line', run: () => apply('Düz kenar yap', withBulge(e, seg, 0)) }
      : { label: 'Yaya dönüştür', icon: 'arc', run: () => apply('Yaya dönüştür', withBulge(e, seg, outwardBulge(e))) },
    { kind: 'separator' },
  ];
}

/**
 * A gentle arc (sagitta = a quarter of the chord) bowing out of a ring, or
 * to the right of an open path; its mid grip then shapes it further.
 */
function outwardBulge(e: PolylineEntity): number {
  if (e.kind !== 'polygon') return 0.5;
  // Outside of a counter-clockwise ring is right of travel, where a positive bulge bows.
  return bulgeRingArea(e.pts, e.bulges) > 0 ? 0.5 : -0.5;
}

function withBulge(e: PolylineEntity, seg: number, bulge: number): { geometry: EntityGeometry } {
  const bulges = e.pts.map((_, i) => (i === seg ? bulge : bulgeAt(e.bulges, i)));
  return { geometry: { kind: e.kind, pts: e.pts, ...(bulges.some(isArcBulge) && { bulges }) } };
}
