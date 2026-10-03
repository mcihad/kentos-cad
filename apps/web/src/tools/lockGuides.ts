import type { AppContext } from '../app/context';
import type { Vec2 } from '../model/geometry';
import type { CanvasPalette } from '../render/color';
import type { Camera } from '../viewport/Camera';
import { hasLocks, lockedDirection, lockReference, lockTravel, lockWords } from './locks';

/**
 * The digitizing locks over the drawing (docs/adr/0166 §6), in the snap colour as the other guides: the locked
 * direction a dashed line from the reference (a ray when it runs one way), the edge a Paralel or Dik lock was picked on
 * solid, the locked length a dashed circle round it, and above-left of the cursor “Kilit: …” (the value card sits above-right, the tool's tag below-right) while the value
 * card is closed: open, its chips say the locks.
 */
export function drawLocks(ctx: AppContext, g: CanvasRenderingContext2D, cam: Camera, pal: CanvasPalette, cursor: Vec2 | null): void {
  const s = ctx.settings.locks.value;
  if (!hasLocks(s)) return;
  const ref = lockReference(ctx);
  if (!ref) return;
  const o = cam.worldToScreen(ref);
  g.save();
  g.strokeStyle = pal.snap;
  g.lineWidth = 1;
  g.globalAlpha = 0.85;
  g.setLineDash([3, 4]);
  // The edge a Paralel or Dik lock was picked on, solid (docs/adr/0166 §3).
  const edge = s.edge;
  if (edge) {
    g.save();
    g.setLineDash([]);
    g.globalAlpha = 1;
    g.lineWidth = 2;
    g.beginPath();
    if (edge.kind === 'seg') {
      const a = cam.worldToScreen(edge.a);
      const b = cam.worldToScreen(edge.b);
      g.moveTo(a.x, a.y);
      g.lineTo(b.x, b.y);
    } else {
      const n = Math.max(8, Math.ceil(Math.abs(edge.sweep) / (Math.PI / 48)));
      for (let i = 0; i <= n; i++) {
        const t = edge.a0 + (edge.sweep * i) / n;
        const q = cam.worldToScreen({ x: edge.c.x + edge.r * Math.cos(t), y: edge.c.y + edge.r * Math.sin(t) });
        if (i) g.lineTo(q.x, q.y);
        else g.moveTo(q.x, q.y);
      }
    }
    g.stroke();
    g.restore();
  }
  const dir = lockedDirection(s, lockTravel(ctx), ctx.format.angles);
  if (dir) {
    const back = dir.both ? 1e4 : 0;
    g.beginPath();
    g.moveTo(o.x - dir.u.x * back, o.y + dir.u.y * back);
    g.lineTo(o.x + dir.u.x * 1e4, o.y - dir.u.y * 1e4);
    g.stroke();
  }
  if (s.length !== null) {
    const r = s.length * cam.scale;
    // A circle that is a dot, or wider than any screen, says nothing.
    if (r > 1 && r < 1e5) {
      g.beginPath();
      g.arc(o.x, o.y, r, 0, Math.PI * 2);
      g.stroke();
    }
  }
  g.restore();
  if (cursor && !ctx.settings.valueCard.value) drawLockTag(g, cursor, `Kilit: ${lockWords(ctx).join(' · ')}`, pal);
}

/** The tag above-left of the cursor; on its other side where the drawing's edge would cut it off. */
function drawLockTag(g: CanvasRenderingContext2D, at: Vec2, text: string, pal: CanvasPalette): void {
  g.save();
  g.font = `500 11px ${pal.font}`;
  const w = Math.ceil(g.measureText(text).width) + 12;
  const h = 20;
  let x = Math.round(at.x - 16 - w);
  let y = Math.round(at.y - 16 - h);
  if (x < 2) x = Math.round(at.x + 16);
  if (y < 2) y = Math.round(at.y + 16);
  g.fillStyle = pal.labelHalo;
  g.globalAlpha = 0.92;
  g.fillRect(x, y, w, h);
  g.globalAlpha = 1;
  g.strokeStyle = pal.snap;
  g.lineWidth = 1;
  g.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
  g.fillStyle = pal.snap;
  g.textBaseline = 'middle';
  g.fillText(text, x + 6, y + h / 2 + 0.5);
  g.restore();
}
