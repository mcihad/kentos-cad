import { describe, expect, it } from 'vitest';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import { fixture, fixtureText, testEngine } from '../../product/sheet/engineTesting';
import { fontsOf, mapPixels, paintList, type PaintSources } from './painter';

/**
 * The display list painter (docs/sheet/design.md §8) against a recording
 * canvas: the engine's plan of every item kind (fixtures/sheet/v1/display)
 * painted as the engine's SVG writer writes it: the paper first, arcs as
 * the ellipse arcs they are (clockwise on the paper), text lines from their
 * baseline at the size they show, clips and groups balanced, a map without
 * a picture as the grey “harita” box, a missing picture as its grey box.
 */

interface Call {
  readonly name: string;
  readonly args: readonly unknown[];
}

/** A 2D context that records what is asked of it (Node has no canvas). */
function recorder(): { g: CanvasRenderingContext2D; calls: Call[]; sets: Record<string, unknown[]> } {
  const calls: Call[] = [];
  const sets: Record<string, unknown[]> = {};
  const target: Record<string, unknown> = { globalAlpha: 1 };
  const g = new Proxy(target, {
    get(t, key: string) {
      if (key in t) return t[key];
      if (key === 'measureText') return (text: string) => ({ width: text.length * 10 });
      if (key === 'getTransform') return () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 });
      return (...args: unknown[]) => calls.push({ name: key, args });
    },
    set(t, key: string, value) {
      t[key] = value;
      (sets[key] ??= []).push(value);
      return true;
    },
  }) as unknown as CanvasRenderingContext2D;
  return { g, calls, sets };
}

const e = testEngine();
const fx = fixture<{ book: string; sheet: string; inputs: never }>('display/every-kind-design.json');
const list: DisplayList = e.displayList(e.readBook(fixtureText(`display/${fx.book}`)), fx.sheet, fx.inputs);
const none: PaintSources = { image: () => null, map: () => null, family: (f) => `${f}-family` };
const target = { scale: 0.004, x: 10, y: 20, minLinePx: 0, minTextPx: 0, placeholderFont: 'sans' };

describe('display list painter', () => {
  it('paints the paper first, in the plan’s colour, over the whole page', () => {
    const r = recorder();
    paintList(r.g, list, target, none);
    expect(r.calls.find((c) => c.name === 'fillRect')!.args).toEqual([0, 0, list.size.width, list.size.height]);
    expect(r.sets.fillStyle?.[0]).toBe(list.paper);
    // Micrometres to device pixels: the paper's corner at (10, 20), 0.004 px per µm.
    expect(r.calls.find((c) => c.name === 'setTransform')!.args).toEqual([0.004, 0, 0, 0.004, 10, 20]);
  });

  it('keeps every save matched by a restore (clips and groups nested)', () => {
    const r = recorder();
    paintList(r.g, list, target, none);
    const saves = r.calls.filter((c) => c.name === 'save').length;
    expect(saves).toBeGreaterThan(5);
    expect(r.calls.filter((c) => c.name === 'restore').length).toBe(saves);
    expect(r.calls.filter((c) => c.name === 'clip').length).toBe(list.prims.filter((p) => p.type === 'pushClip').length);
  });

  it('draws an arc as the engine gives it: centre, radii, turn, start and sweep in radians, clockwise', () => {
    const arcs = list.prims.flatMap((p) => (p.type === 'path' ? p.segments.flatMap((s) => (typeof s === 'object' && 'a' in s ? [s.a] : [])) : []));
    expect(arcs.length).toBeGreaterThan(0);
    const r = recorder();
    paintList(r.g, list, target, none);
    const ellipses = r.calls.filter((c) => c.name === 'ellipse');
    const a = arcs[0];
    const k = Math.PI / 180_000;
    expect(ellipses[0].args).toEqual([a.center[0], a.center[1], a.radius[0], a.radius[1], a.rotation * k, a.start * k, (a.start + a.sweep) * k, a.sweep < 0]);
  });

  it('writes a text line from its baseline in device pixels, at the size it shows, in the drawing’s typeface', () => {
    const r = recorder();
    paintList(r.g, list, target, none);
    const t = list.prims.find((p) => p.type === 'text' && p.text.trim())! as Extract<DisplayList['prims'][number], { type: 'text' }>;
    const fill = r.calls.find((c) => c.name === 'fillText' && c.args[0] === t.text)!;
    expect(fill.args.slice(1)).toEqual([0, 0]);
    expect(r.calls.some((c) => c.name === 'setTransform' && c.args[4] === 10 + t.at[0] * 0.004 && c.args[5] === 20 + t.at[1] * 0.004)).toBe(true);
    expect(r.sets.font).toContain(`${t.italic ? 'italic ' : ''}${t.weight} ${t.size * 0.004}px ${t.font}-family`);
  });

  it('stands a grey box in for a map without a picture, and puts a picture into the content’s rectangle', () => {
    const map = list.prims.find((p) => p.type === 'map')!;
    const r = recorder();
    paintList(r.g, list, target, none);
    expect(r.sets.fillStyle).toContain('#ececec');
    const pic = { width: 10, height: 10 } as unknown as CanvasImageSource;
    const r2 = recorder();
    paintList(r2.g, list, target, { ...none, map: (p) => (p.item === map.item ? pic : null) });
    const draw = r2.calls.find((c) => c.name === 'drawImage' && c.args[0] === pic)!;
    if (map.type === 'map' && map.view.center) expect(draw.args.slice(1)).toEqual([map.clip.left, map.clip.top, map.clip.width, map.clip.height]);
    expect(mapPixels(map as never, 0.004)).toEqual({ width: Math.round((map as { clip: { width: number } }).clip.width * 0.004), height: Math.round((map as { clip: { height: number } }).clip.height * 0.004) });
  });

  it('leaves out text too small to read on the screen, and names the typefaces to load', () => {
    const r = recorder();
    paintList(r.g, list, { ...target, scale: 0.0001, minTextPx: 2 }, none);
    expect(r.calls.some((c) => c.name === 'fillText')).toBe(false);
    const fonts = fontsOf(list, (f) => f);
    expect(fonts.length).toBeGreaterThan(0);
    expect(new Set(fonts).size).toBe(fonts.length);
  });
});
