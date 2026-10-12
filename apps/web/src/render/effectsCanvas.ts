import type { RGBA } from './types';

/**
 * Draw effects in a Canvas2D picture (docs/adr/0226 §2), for the symbol previews (the style manager's thumbnails,
 * the symbol designer, legends): the composing shader's passes (shaders/wgsl/compose) done on the CPU in the same
 * order and with the same numbers, each step kept in 8 bits as a GPU's picture keeps it. An effect group's picture
 * K (alpha A) and its effects over the target with source-over: shadow c·o·Bσ(T(A)), outer glow c·o·Bσ(Dₛ(A)), the
 * group (Bσ(K) when blurred), inner shadow c·o·(1 − Bσ(T(A)))·A, inner glow c·o·Bσ(D₋ₛ(1 − A))·A; Bσ a Gaussian of
 * σ = blur/3 (weights over ±⌈3σ⌉ that sum to 1), made at a half or a quarter of the size above 8 and 32 px, Dₛ the
 * largest (or smallest) value within s px along x then y. Only the box the group and its effects reach is worked on.
 */

/** An effect group's effects in device pixels: a shadow's offset x right, y down; colours straight, alpha the opacity. */
export interface EffectPx {
  shadow?: { dx: number; dy: number; blur: number; color: RGBA };
  glow?: { spread: number; blur: number; color: RGBA };
  innerShadow?: { dx: number; dy: number; blur: number; color: RGBA };
  innerGlow?: { spread: number; blur: number; color: RGBA };
  blur: number;
}

const MAX = 64;

const q8 = (v: number): number => Math.round(Math.min(1, Math.max(0, v)) * 255) / 255;

/** A one- or four-channel picture of `w` × `h`. */
interface Pic {
  w: number;
  h: number;
  c: number;
  d: Float32Array;
}

const pic = (w: number, h: number, c: number): Pic => ({ w, h, c, d: new Float32Array(w * h * c) });

/** The level a blur of σ device pixels is made at. */
const levelOf = (sigma: number): number => (sigma <= 8 ? 1 : sigma <= 32 ? 2 : 4);

/** Gaussian weights over ±⌈3σ⌉ that sum to 1. */
function weights(sigma: number): Float32Array {
  const r = Math.ceil(3 * sigma);
  const out = new Float32Array(2 * r + 1);
  let sum = 0;
  for (let k = -r; k <= r; k++) sum += Math.exp(-(k * k) / (2 * sigma * sigma));
  for (let k = -r; k <= r; k++) out[k + r] = Math.exp(-(k * k) / (2 * sigma * sigma)) / sum;
  return out;
}

/** A Gaussian along x (`dx` 1) or y, zero beyond the picture, kept in 8 bits. */
function blurPass(p: Pic, sigma: number, horizontal: boolean): Pic {
  if (sigma <= 0) return p;
  const wt = weights(sigma);
  const r = (wt.length - 1) / 2;
  const out = pic(p.w, p.h, p.c);
  const { w, h, c, d } = p;
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      for (let ch = 0; ch < c; ch++) {
        let v = 0;
        for (let k = -r; k <= r; k++) {
          const xx = horizontal ? x + k : x;
          const yy = horizontal ? y : y + k;
          if (xx < 0 || yy < 0 || xx >= w || yy >= h) continue;
          v += wt[k + r] * d[(yy * w + xx) * c + ch];
        }
        out.d[(y * w + x) * c + ch] = q8(v);
      }
  return out;
}

/** The largest (or smallest) value within `n` px along x then y, zero beyond, kept in 8 bits. */
function squareFilter(p: Pic, n: number, largest: boolean): Pic {
  const one = (src: Pic, horizontal: boolean): Pic => {
    const out = pic(src.w, src.h, 1);
    for (let y = 0; y < src.h; y++)
      for (let x = 0; x < src.w; x++) {
        let v = largest ? 0 : 1;
        for (let k = -n; k <= n; k++) {
          const xx = horizontal ? x + k : x;
          const yy = horizontal ? y : y + k;
          const s = xx < 0 || yy < 0 || xx >= src.w || yy >= src.h ? 0 : src.d[yy * src.w + xx];
          v = largest ? Math.max(v, s) : Math.min(v, s);
        }
        out.d[y * src.w + x] = q8(v);
      }
    return out;
  };
  return one(one(p, true), false);
}

/** A 2 × 2 box: half the size rounded up, zero beyond; 8 bits. */
function shrink(p: Pic): Pic {
  const w = Math.ceil(p.w / 2);
  const h = Math.ceil(p.h / 2);
  const out = pic(w, h, p.c);
  const at = (x: number, y: number, ch: number) => (x < p.w && y < p.h ? p.d[(y * p.w + x) * p.c + ch] : 0);
  for (let y = 0; y < h; y++)
    for (let x = 0; x < w; x++)
      for (let ch = 0; ch < p.c; ch++) out.d[(y * w + x) * p.c + ch] = q8((at(2 * x, 2 * y, ch) + at(2 * x + 1, 2 * y, ch) + at(2 * x, 2 * y + 1, ch) + at(2 * x + 1, 2 * y + 1, ch)) / 4);
  return out;
}

/** A level's map read at the full picture's pixel centres, bilinearly, the last texel clamped. */
function grow(p: Pic, level: number, w: number, h: number): Pic {
  if (level === 1) return p;
  const out = pic(w, h, p.c);
  for (let y = 0; y < h; y++) {
    const fy = (y + 0.5) / level - 0.5;
    const y0 = Math.floor(fy);
    const ty = fy - y0;
    const ya = Math.min(Math.max(y0, 0), p.h - 1);
    const yb = Math.min(Math.max(y0 + 1, 0), p.h - 1);
    for (let x = 0; x < w; x++) {
      const fx = (x + 0.5) / level - 0.5;
      const x0 = Math.floor(fx);
      const tx = fx - x0;
      const xa = Math.min(Math.max(x0, 0), p.w - 1);
      const xb = Math.min(Math.max(x0 + 1, 0), p.w - 1);
      for (let ch = 0; ch < p.c; ch++) {
        const v = (a: number, b: number) => p.d[(a * p.w + b) * p.c + ch];
        out.d[(y * w + x) * p.c + ch] = (v(ya, xa) * (1 - tx) + v(ya, xb) * tx) * (1 - ty) + (v(yb, xa) * (1 - tx) + v(yb, xb) * tx) * ty;
      }
    }
  }
  return out;
}

/** Blurred at its level and read back at full size. */
function blurred(p: Pic, sigma: number): Pic {
  sigma = Math.min(sigma, MAX);
  const level = levelOf(sigma);
  let m = p;
  for (let at = 1; at < level; at *= 2) m = shrink(m);
  m = blurPass(blurPass(m, sigma / level, true), sigma / level, false);
  return grow(m, level, p.w, p.h);
}

/** A map of the group's alpha: spread (> 0) or shrunk (< 0) by |spread| px, then blurred by σ. */
function alphaMap(alpha: Pic, spread: number, sigma: number): Pic {
  let m = alpha;
  const n = Math.round(Math.min(Math.abs(spread), MAX));
  if (n >= 1) m = squareFilter(m, n, spread > 0);
  return blurred(m, sigma);
}

/** `m` read at (x − dx, y − dy) bilinearly, zero beyond. */
function shifted(m: Pic, x: number, y: number, dx: number, dy: number): number {
  const sx = x - dx;
  const sy = y - dy;
  const x0 = Math.floor(sx);
  const y0 = Math.floor(sy);
  const tx = sx - x0;
  const ty = sy - y0;
  const at = (xx: number, yy: number) => (xx < 0 || yy < 0 || xx >= m.w || yy >= m.h ? 0 : m.d[yy * m.w + xx]);
  return (at(x0, y0) * (1 - tx) + at(x0 + 1, y0) * tx) * (1 - ty) + (at(x0, y0 + 1) * (1 - tx) + at(x0 + 1, y0 + 1) * tx) * ty;
}

/** Laid `m` (an effect's map, times its colour) over `t`, premultiplied; 8 bits. */
function tintOver(t: Pic, color: RGBA, coverage: (i: number, x: number, y: number) => number): void {
  for (let y = 0; y < t.h; y++)
    for (let x = 0; x < t.w; x++) {
      const i = y * t.w + x;
      const a = Math.min(1, Math.max(0, color[3] * coverage(i, x, y)));
      if (a <= 0) continue;
      const k = 1 - a;
      t.d[i * 4] = q8(color[0] * a + t.d[i * 4] * k);
      t.d[i * 4 + 1] = q8(color[1] * a + t.d[i * 4 + 1] * k);
      t.d[i * 4 + 2] = q8(color[2] * a + t.d[i * 4 + 2] * k);
      t.d[i * 4 + 3] = q8(a + t.d[i * 4 + 3] * k);
    }
}

/** A box of a canvas's pixels, premultiplied. */
function readBox(g: CanvasRenderingContext2D, x0: number, y0: number, w: number, h: number): Pic {
  const img = g.getImageData(x0, y0, w, h).data;
  const p = pic(w, h, 4);
  for (let i = 0; i < w * h; i++) {
    const a = img[i * 4 + 3] / 255;
    p.d[i * 4] = q8((img[i * 4] / 255) * a);
    p.d[i * 4 + 1] = q8((img[i * 4 + 1] / 255) * a);
    p.d[i * 4 + 2] = q8((img[i * 4 + 2] / 255) * a);
    p.d[i * 4 + 3] = a;
  }
  return p;
}

/**
 * Lays an effect group's picture (`group`, the target's size) over `target` with its effects. Both in device pixels;
 * the target's transform is not used.
 */
export function composeEffects(target: CanvasRenderingContext2D, group: CanvasRenderingContext2D, fx: EffectPx): void {
  const W = target.canvas.width;
  const H = target.canvas.height;
  if (!W || !H) return;
  // The group's box, grown by what its effects reach.
  const all = group.getImageData(0, 0, W, H).data;
  let [minX, minY, maxX, maxY] = [W, H, -1, -1];
  for (let y = 0; y < H; y++)
    for (let x = 0; x < W; x++)
      if (all[(y * W + x) * 4 + 3] > 0) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
  if (maxX < 0) return;
  const reachOf = (blur: number, spread = 0, dx = 0, dy = 0) => Math.ceil(3 * Math.min(blur / 3, MAX) + 2 * levelOf(blur / 3) + Math.min(Math.abs(spread), MAX) + Math.hypot(dx, dy)) + 2;
  let reach = reachOf(fx.blur);
  if (fx.shadow) reach = Math.max(reach, reachOf(fx.shadow.blur, 0, fx.shadow.dx, fx.shadow.dy));
  if (fx.glow) reach = Math.max(reach, reachOf(fx.glow.blur, fx.glow.spread));
  if (fx.innerShadow) reach = Math.max(reach, reachOf(fx.innerShadow.blur, 0, fx.innerShadow.dx, fx.innerShadow.dy));
  if (fx.innerGlow) reach = Math.max(reach, reachOf(fx.innerGlow.blur, fx.innerGlow.spread));
  const x0 = Math.max(0, minX - reach);
  const y0 = Math.max(0, minY - reach);
  const w = Math.min(W, maxX + 1 + reach) - x0;
  const h = Math.min(H, maxY + 1 + reach) - y0;
  const k = readBox(group, x0, y0, w, h);
  const t = readBox(target, x0, y0, w, h);
  const alpha = pic(w, h, 1);
  for (let i = 0; i < w * h; i++) alpha.d[i] = q8(k.d[i * 4 + 3]);
  if (fx.shadow) {
    const s = fx.shadow;
    const m = alphaMap(alpha, 0, s.blur / 3);
    tintOver(t, s.color, (_i, x, y) => shifted(m, x, y, s.dx, s.dy));
  }
  if (fx.glow) {
    const g = fx.glow;
    const m = alphaMap(alpha, g.spread, g.blur / 3);
    tintOver(t, g.color, (i) => m.d[i]);
  }
  const layer = fx.blur >= 0.5 ? blurred(k, fx.blur / 3) : k;
  for (let i = 0; i < w * h; i++) {
    const a = layer.d[i * 4 + 3];
    const rest = 1 - a;
    for (let ch = 0; ch < 4; ch++) t.d[i * 4 + ch] = q8(layer.d[i * 4 + ch] + t.d[i * 4 + ch] * rest);
  }
  if (fx.innerShadow) {
    const s = fx.innerShadow;
    const m = alphaMap(alpha, 0, s.blur / 3);
    tintOver(t, s.color, (i, x, y) => (1 - shifted(m, x, y, s.dx, s.dy)) * alpha.d[i]);
  }
  if (fx.innerGlow) {
    const g = fx.innerGlow;
    const m = alphaMap(alpha, -g.spread, g.blur / 3);
    tintOver(t, g.color, (i) => (1 - m.d[i]) * alpha.d[i]);
  }
  // Back to the canvas's straight colours.
  const out = target.createImageData(w, h);
  for (let i = 0; i < w * h; i++) {
    const a = t.d[i * 4 + 3];
    out.data[i * 4 + 3] = Math.round(a * 255);
    if (a > 0) for (let ch = 0; ch < 3; ch++) out.data[i * 4 + ch] = Math.round(Math.min(1, t.d[i * 4 + ch] / a) * 255);
  }
  target.putImageData(out, x0, y0);
}
