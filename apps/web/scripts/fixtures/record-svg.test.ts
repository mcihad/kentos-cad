// Records the SVG core's answers into fixtures/svg/v1/cases.json
// (docs/adr/0008 “SVG düzenleyicisi”). Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-svg.test.ts
// The committed answers were recorded after the core agreed with the
// TypeScript it replaced, bit for bit and key for key, on 20 000 random
// calls per operation (src/style/svg/parity.test.ts, since removed);
// rewriting them is a deliberate change of the editor's geometry, to be
// read in the diff. The Rust core (crates/shared/svg-core/tests/cases.rs)
// and the app's path (src/style/svg/fixture.test.ts) keep checking them.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { TRACE_DEFAULTS } from '../../src/style/svg/trace';
import { CALLS, bitmap, snapCase, traceOptions } from '../../src/style/svg/cases/calls';
import { pngBytes } from '../../src/style/svg/cases/files';
import { SnapIndex, callOp, crc32, inkMask, inkMaskNumbers, opId, traceBitmap, traceBitmapNumbers, traceContours, withPngDpi } from '../../src/style/svg/pkg/kentos_svg_wasm.js';
import { writeArgs } from '../../src/wasm/core';
import { Gen, toJson } from '../../src/wasm/calls/harness';

const OUT = new URL('../../../../fixtures/svg/v1/cases.json', import.meta.url);
/** Random calls kept per operation… */
const KEEP = 16;
/** …within this many bytes per operation, keeping at least MIN_KEPT (a traced picture alone is 15 KB). */
const BUDGET = 16_000;
const MIN_KEPT = 3;

/** A case: the operation, its arguments as the page sends them, and the core's answer (JSON as the core wrote it) or its message. */
type Case = Record<string, unknown> & { fn: string };

/** The core's answer to a table operation, as it wrote it; a refused call keeps its message. */
function answer(fn: string, args: unknown[]): Record<string, unknown> {
  const id = opId(fn);
  if (id < 0) throw new Error(`SVG çekirdeğinde “${fn}” yok`);
  try {
    return { expect: JSON.parse(callOp(id, writeArgs(args))) as unknown };
  } catch (e) {
    return { throws: e instanceof Error ? e.message : String(e) };
  }
}

/** The typed entries: the snap index, the picture's bytes and the PNG's. */
const TYPED: Record<string, (g: Gen) => Case> = {
  SnapIndex: (g) => {
    const [source, queries] = snapCase(g);
    const index = SnapIndex.of(writeArgs(source));
    const expect = queries.map(([p, r, from]) => JSON.parse(index.query(p[0], p[1], r, !!from, from ? from[0] : 0, from ? from[1] : 0)) as unknown);
    index.free();
    return { fn: 'SnapIndex', source: toJson(source), queries: queries.map(([p, r, from]) => [p[0], p[1], r, !!from, from ? from[0] : 0, from ? from[1] : 0]), expect };
  },
  inkMask: (g) => {
    const img = bitmap(g);
    const threshold = g.pick([64, 128, 250]);
    const invert = g.chance(0.3);
    const numbers = !(img.data instanceof Uint8ClampedArray);
    const mask = numbers ? inkMaskNumbers(img.width, img.height, Float64Array.from(img.data), threshold, invert) : inkMask(img.width, img.height, new Uint8Array(img.data.buffer), threshold, invert);
    return { fn: 'inkMask', width: img.width, height: img.height, data: [...img.data], numbers, threshold, invert, expect: [...mask] };
  },
  traceContours: (g) => {
    const img = bitmap(g);
    const mask = [...inkMask(img.width, img.height, Uint8Array.from(img.data), 128, false)];
    return { fn: 'traceContours', mask, width: img.width, height: img.height, expect: JSON.parse(traceContours(Uint8Array.from(mask), img.width, img.height)) as unknown };
  },
  traceBitmap: (g) => {
    const img = bitmap(g);
    const options = { ...TRACE_DEFAULTS, ...traceOptions(g) };
    const numbers = !(img.data instanceof Uint8ClampedArray);
    const text = numbers ? traceBitmapNumbers(img.width, img.height, Float64Array.from(img.data), writeArgs(options)) : traceBitmap(img.width, img.height, new Uint8Array(img.data.buffer), writeArgs(options));
    return { fn: 'traceBitmap', width: img.width, height: img.height, data: [...img.data], numbers, options, expect: JSON.parse(text) as unknown };
  },
  crc32: (g) => {
    const bytes = pngBytes(g);
    return { fn: 'crc32', bytes: [...bytes], expect: crc32(bytes) };
  },
  withPngDpi: (g) => {
    const bytes = pngBytes(g);
    const dpi = g.pick([72, 96, 300, 1200, g.num(1, 5000), -10, 1e12]);
    const out = withPngDpi(bytes, dpi);
    return { fn: 'withPngDpi', bytes: [...bytes], dpi, expect: out ? [...out] : null };
  },
};

it.runIf(!!process.env.GOLDEN_WRITE)('records the SVG core’s answers', () => {
  const g = new Gen(2409);
  const cases: Case[] = [];
  const record = (fn: string, make: () => Case) => {
    let bytes = 0;
    for (let kept = 0, tries = 0; kept < KEEP && tries < 4 * KEEP; tries++) {
      const c = make();
      const size = JSON.stringify(c).length;
      if (kept >= MIN_KEPT && bytes + size > BUDGET) continue;
      cases.push(c);
      bytes += size;
      kept++;
    }
    if (!cases.some((c) => c.fn === fn)) throw new Error(`${fn}: kayıt yok`);
  };
  for (const [fn, make] of Object.entries(CALLS))
    record(fn, () => {
      const args = toJson(make(g)) as unknown[];
      return { fn, args, ...answer(fn, args) };
    });
  for (const [fn, make] of Object.entries(TYPED)) record(fn, () => make(g));
  const head = {
    format: 'kentos.svg-cases',
    version: 1,
    note: 'SVG düzenleyicisinin çekirdeğinin (crates/shared/svg-core) yanıtları. Tablo işlemlerinde fn işlemin adı, args sayfanın gönderdiği bağımsız değişkenler, expect çekirdeğin yazdığı yanıt (JSON’un tutamadığı sayılar "#NaN", "#Inf", "#-Inf"), throws reddedilen çağrının iletisidir; yeni şekil ve grup kimlikleri "\\u0001<k>" ve "\\u0002<k>" adlarıyla gelir (sayfa doldurur). Türlü girişler: SnapIndex (kaynak, sorgular: x, y, yarıçap, başlangıç noktası var mı, onun x ve y’si), inkMask, traceContours ve traceBitmap (resmin baytları ya da numbers ile sayıları), crc32 ve withPngDpi (PNG baytları; PNG değilse null).',
  };
  // One case a line: a changed answer shows as its own line in the diff.
  writeFileSync(OUT, `${JSON.stringify(head, null, 1).slice(0, -2)},\n "cases": [\n${cases.map((c) => `  ${JSON.stringify(c)}`).join(',\n')}\n ]\n}\n`);
}, 600_000);
