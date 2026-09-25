import type { CrosshairSize } from '../app/state';
import type { CadDocument } from '../model/document';
import { dist, type Vec2 } from '../model/geometry';
import type { DimensionLayout } from '../model/geom/dimension';
import { resolveColor, type CanvasPalette } from '../render/color';
import type { ToolCursor } from '../tools/Tool';
import type { Camera } from './Camera';
import type { TrackHit } from './objectTracking';
import { SNAP_LABEL, type SnapHit } from './picking';
import { DEFAULT_LABELS, DIMENSION_PREFIX, LABEL, LABEL_STRIDE, type GripSet } from './storeRecords';

/**
 * Text that is part of the drawing (text objects, dimension values, labels)
 * keeps one face whatever the interface typeface is: it is data everyone sees
 * alike. The overlay's own marks (snap names, scale bar, north arrow) use the
 * interface typeface (CanvasPalette.font).
 */
const DRAWING_FONT = 'Barlow, system-ui, sans-serif';

/** Screen-space annotation layer drawn with Canvas2D above the GPU canvas. */


function haloText(g: CanvasRenderingContext2D, text: string, x: number, y: number, fill: string, halo: string): void {
  g.lineJoin = 'round';
  g.lineWidth = 3;
  g.strokeStyle = halo;
  g.strokeText(text, x, y);
  g.fillStyle = fill;
  g.fillText(text, x, y);
}

/**
 * Entity labels and text. Which ones a frame draws and where comes from the
 * geometry store (`labels`: visible layer, box in view, text size on
 * screen, the LabelStyle's scale range and smallest feature; see
 * ./storeRecords); size, template and colour from the layer's LabelStyle,
 * so this function knows nothing about specific layers.
 */
export function drawLabels(
  g: CanvasRenderingContext2D,
  doc: CadDocument,
  cam: Camera,
  pal: CanvasPalette,
  spots: Float64Array,
  dimensionText: (l: Pick<DimensionLayout, 'prefix' | 'unit' | 'value'>) => string,
): void {
  const layers = doc.layers;
  const ink = { fg: pal.fg, 'fg-dim': pal.fgDim, label: pal.label } as const;
  g.save();
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  for (let i = 0; i < spots.length; i += LABEL_STRIDE) {
    const e = doc.get(spots[i]);
    if (!e) continue;
    const what = spots[i + 1];
    const x = spots[i + 2];
    const y = spots[i + 3];
    if (what === LABEL.dimension && e.kind === 'dimension') {
      const px = e.height * cam.scale;
      const s = cam.worldToScreen({ x, y });
      g.save();
      g.translate(s.x, s.y);
      g.rotate((-spots[i + 4] * Math.PI) / 180);
      g.font = `500 ${px.toFixed(1)}px ${DRAWING_FONT}`;
      g.textAlign = 'center';
      g.textBaseline = 'alphabetic';
      const color = e.color ?? layers.get(e.layerId)?.style.color;
      const measured = { value: spots[i + 5], unit: spots[i + 6] ? ('angle' as const) : ('length' as const), prefix: DIMENSION_PREFIX[spots[i + 7]] ?? '' };
      haloText(g, e.text || dimensionText(measured), 0, 0, !color || color === 'fg' || color === 'fg-dim' ? pal.label : resolveColor(color, pal), pal.labelHalo);
      g.restore();
      continue;
    }
    if (what === LABEL.text && e.kind === 'text') {
      const px = e.height * cam.scale;
      const s = cam.worldToScreen({ x, y });
      g.save();
      g.translate(s.x, s.y);
      g.rotate((-spots[i + 4] * Math.PI) / 180);
      g.font = `italic 400 ${px.toFixed(1)}px ${DRAWING_FONT}`;
      g.textAlign = 'left';
      g.textBaseline = 'alphabetic';
      haloText(g, e.text, 0, 0, pal.label, pal.labelHalo);
      g.restore();
      continue;
    }
    const st = layers.get(e.layerId)?.style.label ?? DEFAULT_LABELS[e.kind];
    if (!st || !e.label) continue;
    const size = Math.min(st.maxSize ?? st.size, st.size + (st.grow ?? 0) * cam.scale);
    const text = st.template ? st.template.replace('{label}', e.label) : e.label;
    const color = ink[st.ink ?? 'label'];
    g.font = `${st.weight ?? 500} ${size.toFixed(1)}px ${DRAWING_FONT}`;

    switch (what) {
      case LABEL.center: {
        const s = cam.worldToScreen({ x, y });
        haloText(g, text, s.x, s.y, color, pal.labelHalo);
        break;
      }
      case LABEL.corner: {
        const tl = cam.worldToScreen({ x, y });
        g.textAlign = 'left';
        haloText(g, text, tl.x + 8, tl.y + 14, color, pal.labelHalo);
        g.textAlign = 'center';
        break;
      }
      case LABEL.beside: {
        const s = cam.worldToScreen({ x, y });
        g.textAlign = 'left';
        haloText(g, text, s.x + 7, s.y - 7, color, pal.labelHalo);
        g.textAlign = 'center';
        break;
      }
      case LABEL.along: {
        // Placed a third of the way along, kept upright.
        const a = cam.worldToScreen({ x, y });
        const c = cam.worldToScreen({ x: spots[i + 4], y: spots[i + 5] });
        let ang = Math.atan2(c.y - a.y, c.x - a.x);
        if (ang > Math.PI / 2 || ang < -Math.PI / 2) ang += Math.PI;
        g.save();
        g.translate((a.x + c.x) / 2, (a.y + c.y) / 2);
        g.rotate(ang);
        haloText(g, text, 0, 0, color, pal.labelHalo);
        g.restore();
        break;
      }
    }
  }
  g.restore();
}

/**
 * Grip squares on selected entities (their grips from the geometry store).
 * Grips closer than 9 px on screen are thinned so dense polylines
 * (contours) stay readable.
 */
export function drawGrips(g: CanvasRenderingContext2D, sets: readonly GripSet[], cam: Camera, pal: CanvasPalette, hot: { id: number; index: number } | null = null): void {
  if (sets.length > 150) return;
  g.save();
  g.fillStyle = pal.accent;
  g.strokeStyle = pal.labelHalo;
  g.lineWidth = 1;
  for (const set of sets) {
    let last: Vec2 | null = null;
    for (let i = 0; i < set.points.length; i++) {
      if (hot && hot.id === set.id && hot.index === i) continue;
      const s = cam.worldToScreen(set.points[i]);
      const x = Math.round(s.x);
      const y = Math.round(s.y);
      if (set.segments[i] >= 0) {
        // Mid grips (add a vertex / bend an arc): small hollow diamonds, hidden on short segments.
        if (!midGripVisible(set, i, cam)) continue;
        g.save();
        g.fillStyle = pal.labelHalo;
        g.strokeStyle = pal.accent;
        g.beginPath();
        g.moveTo(x, y - 4);
        g.lineTo(x + 4, y);
        g.lineTo(x, y + 4);
        g.lineTo(x - 4, y);
        g.closePath();
        g.fill();
        g.stroke();
        g.restore();
        continue;
      }
      if (last && Math.abs(s.x - last.x) < 9 && Math.abs(s.y - last.y) < 9) continue;
      last = s;
      g.fillRect(x - 3, y - 3, 6, 6);
      g.strokeRect(x - 3.5, y - 3.5, 7, 7);
    }
  }
  // The grip being edited ("sıcak tutamaç") is drawn larger in ink colour.
  const hp = hot ? sets.find((set) => set.id === hot.id)?.points[hot.index] : undefined;
  if (hp) {
    const s = cam.worldToScreen(hp);
    g.fillStyle = pal.fg;
    g.strokeStyle = pal.accent;
    g.lineWidth = 1.5;
    g.fillRect(Math.round(s.x) - 4, Math.round(s.y) - 4, 8, 8);
    g.strokeRect(Math.round(s.x) - 4.5, Math.round(s.y) - 4.5, 9, 9);
  }
  g.restore();
}

export function drawSnap(g: CanvasRenderingContext2D, hit: SnapHit, cam: Camera, pal: CanvasPalette): void {
  const s = cam.worldToScreen(hit.point);
  const x = Math.round(s.x) + 0.5;
  const y = Math.round(s.y) + 0.5;
  g.save();
  g.strokeStyle = pal.snap;
  g.lineWidth = 1.5;
  g.beginPath();
  switch (hit.kind) {
    case 'midpoint':
      g.moveTo(x, y - 6);
      g.lineTo(x + 6, y + 5);
      g.lineTo(x - 6, y + 5);
      g.closePath();
      break;
    case 'center':
    case 'node':
      g.arc(x, y, 5.5, 0, Math.PI * 2);
      break;
    case 'quadrant':
      g.moveTo(x, y - 6);
      g.lineTo(x + 6, y);
      g.lineTo(x, y + 6);
      g.lineTo(x - 6, y);
      g.closePath();
      break;
    case 'intersection':
      g.moveTo(x - 5, y - 5);
      g.lineTo(x + 5, y + 5);
      g.moveTo(x + 5, y - 5);
      g.lineTo(x - 5, y + 5);
      break;
    case 'perpendicular':
      g.moveTo(x - 6, y - 6);
      g.lineTo(x - 6, y + 5);
      g.lineTo(x + 6, y + 5);
      g.moveTo(x - 6, y);
      g.lineTo(x, y);
      g.lineTo(x, y + 5);
      break;
    case 'tangent':
      g.arc(x, y + 1, 4.5, 0, Math.PI * 2);
      g.moveTo(x - 6.5, y - 4.5);
      g.lineTo(x + 6.5, y - 4.5);
      break;
    case 'nearest':
      g.moveTo(x - 5, y - 5);
      g.lineTo(x + 5, y - 5);
      g.lineTo(x - 5, y + 5);
      g.lineTo(x + 5, y + 5);
      g.closePath();
      break;
    default:
      g.rect(x - 5, y - 5, 10, 10);
  }
  g.stroke();
  g.font = `500 10.5px ${pal.font}`;
  g.textBaseline = 'bottom';
  // Above-right, so it never collides with the tool's measurement tag (below-right).
  haloText(g, SNAP_LABEL[hit.kind], x + 9, y - 7, pal.snap, pal.labelHalo);
  g.restore();
}

const CROSSHAIR_ARM: Record<CrosshairSize, number> = { small: 16, medium: 40, full: 1e5 };

export function drawCrosshair(g: CanvasRenderingContext2D, at: Vec2, cursor: ToolCursor, pal: CanvasPalette, size: CrosshairSize): void {
  if (cursor === 'grab') return;
  const x = Math.round(at.x) + 0.5;
  const y = Math.round(at.y) + 0.5;
  const arm = size === 'full' ? CROSSHAIR_ARM.full : cursor === 'pick' ? Math.round(CROSSHAIR_ARM[size] * 0.55) : CROSSHAIR_ARM[size];
  const box = cursor === 'pick' ? 5 : 0;
  g.save();
  g.strokeStyle = pal.fg;
  g.globalAlpha = 0.85;
  g.lineWidth = 1;
  g.beginPath();
  g.moveTo(x - arm, y);
  g.lineTo(x - box, y);
  g.moveTo(x + box, y);
  g.lineTo(x + arm, y);
  g.moveTo(x, y - arm);
  g.lineTo(x, y - box);
  g.moveTo(x, y + box);
  g.lineTo(x, y + arm);
  if (box) g.rect(x - box, y - box, box * 2, box * 2);
  g.stroke();
  g.restore();
}

/** Alternating map scale bar, bottom-right (the toolbox usually sits left). */
export function drawScaleBar(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette): void {
  const targetPx = 120;
  const raw = targetPx / cam.scale;
  const p = Math.pow(10, Math.floor(Math.log10(raw)));
  const len = [1, 2, 5, 10].map((m) => m * p).reduce((best, v) => (Math.abs(v * cam.scale - targetPx) < Math.abs(best * cam.scale - targetPx) ? v : best));
  const px = len * cam.scale;
  const x0 = cam.width - 20 - px;
  const y0 = cam.height - 22;
  g.save();
  for (let i = 0; i < 4; i++) {
    g.fillStyle = i % 2 ? pal.labelHalo : pal.fg;
    g.fillRect(x0 + (px / 4) * i, y0, px / 4, 4);
  }
  g.strokeStyle = pal.fg;
  g.lineWidth = 1;
  g.strokeRect(x0 + 0.5, y0 + 0.5, px, 4);
  g.font = `500 10.5px ${pal.font}`;
  g.textBaseline = 'bottom';
  g.textAlign = 'left';
  haloText(g, '0', x0, y0 - 3, pal.label, pal.labelHalo);
  g.textAlign = 'right';
  const unit = len >= 1000 ? `${len / 1000} km` : len >= 1 ? `${len} m` : `${(len * 100).toPrecision(2)} cm`;
  haloText(g, unit, x0 + px, y0 - 3, pal.label, pal.labelHalo);
  g.restore();
}

/** Grid-north arrow with Turkish "K" (Kuzey), top-right. */
export function drawNorthArrow(g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette): void {
  const x = cam.width - 30;
  const y = 22;
  g.save();
  g.fillStyle = pal.fg;
  g.strokeStyle = pal.fg;
  g.lineWidth = 1;
  g.beginPath();
  g.moveTo(x, y + 6);
  g.lineTo(x + 6, y + 28);
  g.lineTo(x, y + 23);
  g.closePath();
  g.fill();
  g.beginPath();
  g.moveTo(x, y + 6);
  g.lineTo(x - 6, y + 28);
  g.lineTo(x, y + 23);
  g.closePath();
  g.stroke();
  g.font = `600 11px ${pal.font}`;
  g.textAlign = 'center';
  g.textBaseline = 'bottom';
  haloText(g, 'K', x, y + 3, pal.fg, pal.labelHalo);
  g.restore();
}

/** A mid grip is offered only when its segment is long enough on screen to tell it from the vertices. */
export function midGripVisible(set: GripSet, index: number, cam: Camera): boolean {
  const seg = set.segments[index];
  if (!(seg >= 0)) return true;
  const a = cam.worldToScreen(set.points[seg]);
  const b = cam.worldToScreen(set.points[(seg + 1) % set.vertices]);
  return Math.hypot(b.x - a.x, b.y - a.y) >= 28;
}

/**
 * Object tracking: acquired points as small crosses, the alignment line(s)
 * the cursor is locked to (dashed, through the whole view) and a tag with
 * the distance and angle from the tracked point.
 */
export function drawObjectTracking(g: CanvasRenderingContext2D, acquired: readonly Vec2[], track: TrackHit | null, cam: Camera, pal: CanvasPalette, formatLength: (m: number) => string): void {
  if (!acquired.length) return;
  g.save();
  g.strokeStyle = pal.snap;
  g.lineWidth = 1.5;
  for (const p of acquired) {
    const s = cam.worldToScreen(p);
    const x = Math.round(s.x) + 0.5;
    const y = Math.round(s.y) + 0.5;
    g.beginPath();
    g.moveTo(x - 5, y);
    g.lineTo(x + 5, y);
    g.moveTo(x, y - 5);
    g.lineTo(x, y + 5);
    g.stroke();
  }
  if (track) {
    g.lineWidth = 1;
    g.globalAlpha = 0.85;
    g.setLineDash([3, 4]);
    for (const l of track.lines) {
      const o = cam.worldToScreen(l.origin);
      const r = (l.angle * Math.PI) / 180;
      g.beginPath();
      g.moveTo(o.x, o.y);
      g.lineTo(o.x + Math.cos(r) * 1e4, o.y - Math.sin(r) * 1e4);
      g.stroke();
    }
    g.setLineDash([]);
    g.globalAlpha = 1;
    const at = cam.worldToScreen(track.point);
    const l = track.lines[0];
    const text = track.lines.length > 1 ? 'İzleme: kesişim' : `İzleme ${formatLength(dist(l.origin, track.point))} < ${l.angle}°`;
    g.font = `500 10.5px ${pal.font}`;
    g.textBaseline = 'bottom';
    haloText(g, text, Math.round(at.x) + 9, Math.round(at.y) - 7, pal.snap, pal.labelHalo);
  }
  g.restore();
}
