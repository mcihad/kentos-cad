// Records the expression language's answers into fixtures/expression/v1/cases.json
// (docs/adr/0008 “İfade dili”). Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-expression.test.ts
// The committed answers were recorded after the core agreed with the
// TypeScript it replaced on 20 000 random sources (src/model/expression/parity.test.ts,
// since removed); rewriting them is a deliberate change of the language, to
// be read in the diff. The Rust core (crates/shared/style-core/tests/cases.rs)
// and the app's path (src/model/expression/fixture.test.ts) keep checking them.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { layerName, randomMeasures, randomObject, randomSource } from '../../src/model/expression/cases';
import { compileExpression, type ExprAs, type ExprValue } from '../../src/model/expression/expression';
import { ENTITY_KIND_LABEL } from '../../src/model/entities';
import { Gen } from '../../src/wasm/calls/harness';

const OUT = new URL('../../../../fixtures/expression/v1/cases.json', import.meta.url);
const CASES = 400;
const MODES: readonly ExprAs[] = ['value', 'number', 'text', 'bool', 'textNumber'];

/** A number as JSON cannot keep it: "-0", "Infinity", "-Infinity" as text (a number of a result mode may be infinite: "1e400" reads as ∞). */
const num = (x: number) => (Object.is(x, -0) ? '-0' : Number.isFinite(x) ? x : String(x));
/** A value in JSON: null, or its kind and value. */
const encode = (v: ExprValue) => (v === null ? null : typeof v === 'number' ? ['n', num(v)] : typeof v === 'string' ? ['t', v] : ['b', v]);

it.runIf(!!process.env.GOLDEN_WRITE)('records the expression language’s answers', () => {
  const g = new Gen(7070);
  const cases: unknown[] = [];
  for (let c = 0; c < CASES; c++) {
    const source = randomSource(g);
    const r = compileExpression(source);
    if (!r.ok) {
      cases.push({ source, error: { message: r.error, at: r.at } });
      continue;
    }
    const n = g.int(1, 6);
    const entities = Array.from({ length: n }, (_, i) => randomObject(g, i + 1));
    const measures = randomMeasures(g, n);
    const plotScale = g.pick([undefined, 500, 1000]);
    // Each object as an expression reads it: attributes, label, layer and kind names, id, corners, geometry values.
    const objects = entities.map((e, i) => {
      const vertices = e.kind === 'polygon' ? e.pts.length + (e.holes ?? []).reduce((s, h) => s + h.pts.length, 0) : e.kind === 'polyline' || e.kind === 'spline' ? e.pts.length : e.kind === 'line' ? 2 : e.kind === 'point' ? 1 : null;
      return { attrs: e.attrs, label: e.label ?? null, layer: layerName(e.layerId), kind: ENTITY_KIND_LABEL[e.kind], id: e.id, vertices, measures: [...measures.slice(i * 6, i * 6 + 6)].map(num) };
    });
    const results = Object.fromEntries(
      MODES.map((as) => {
        const col = r.expr.evaluateAll({ entities, layerName, plotScale, measures: () => measures }, as);
        return [as, entities.map((_, i) => encode(col.value(i)))];
      }),
    );
    cases.push({ source, fields: r.expr.fields, plotScale: plotScale ?? null, objects, results });
  }
  const file = {
    format: 'kentos.expression-cases',
    version: 1,
    note: 'İfade dilinin yanıtları: kaynak, nesneler (öznitelikler, etiket, katman ve tür adı, numara, köşe sayısı, ölçüler: bayraklar, uzunluk, alan, yer noktası x ve y, boş), beklenen hata ya da her sonuç türünde değerler. Değer: null, ya da [tür, değer]: n sayı, t yazı, b doğru/yanlış. JSON’un tutamadığı sayılar yazıdır: "-0", "Infinity", "-Infinity" (sonuçta da ölçülerde de).',
    cases,
  };
  writeFileSync(OUT, `${JSON.stringify(file, null, 1)}\n`);
});
