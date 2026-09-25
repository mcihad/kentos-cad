// Records the style compiler's answers into fixtures/style/v1/cases.json
// (docs/adr/0008 “Stil derleyicisi”). Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-style.test.ts
// The committed answers were recorded after the core agreed with the
// TypeScript it replaced on every system symbol, 20 000 random symbols and
// 20 000 random layers (src/style/parity.test.ts, since removed); rewriting
// them is a deliberate change of the style engine, to be read in the diff.
// The Rust core (crates/shared/style-core/tests/style.rs) and the app's
// path (src/style/fixture.test.ts) keep checking them.
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { ENTITY_KIND_LABEL } from '../../src/model/entities';
import { vertexCount } from '../../src/model/expression/expression';
import type { LibrarySymbol } from '../../src/model/style';
import { buildStyledLayer } from '../../src/render/styledLayer';
import { ASSET_SIZES, ASSETS, layerScene, PALETTE, randomLibrary, randomObject, randomSymbol } from '../../src/style/cases';
import { compileSymbol } from '../../src/style/compile';
import { captureStyled, type StyledCall } from '../../src/style/fixture';
import { SYSTEM_LIBRARY } from '../../src/style/system';
import { PickIndex } from '../../src/viewport/picking';
import { Gen } from '../../src/wasm/calls/harness';

const OUT = new URL('../../../../fixtures/style/v1/cases.json', import.meta.url);
const SYMBOLS = 160;
const LAYERS = 48;
/** Cases stay small enough to read in a diff: a symbol's primitives in JSON, a layer's batch numbers. */
const MAX_TEXT = 12_000;
const MAX_NUMBERS = 3_000;

it.runIf(!!process.env.GOLDEN_WRITE)('records the style compiler’s answers', () => {
  const g = new Gen(2409);
  const system = SYSTEM_LIBRARY.items.filter((i): i is LibrarySymbol => i.kind === 'symbol');
  const symbols: unknown[] = [];
  while (symbols.length < SYMBOLS) {
    g.frame();
    // Every fourth case a system symbol, the others random.
    const symbol = symbols.length % 4 === 3 ? g.pick(system).symbol : randomSymbol(g);
    const entity = randomObject(g, g.int(1, 9999), 'a', []);
    const plotScale = g.pick([500, 1000, 2000, 5000]);
    const layerName = g.pick(['Parsel sınırı', 'Yol', '']);
    const primitives = compileSymbol(symbol, entity, { plotScale, layerName, assets: ASSET_SIZES });
    const n = primitives.strokes.length + primitives.fills.length + primitives.markers.length;
    // Most empty results are symbols of another class than the object; a few are kept.
    if ((n === 0 && !g.chance(0.1)) || JSON.stringify(primitives).length > MAX_TEXT) continue;
    symbols.push({ symbol, entity, kindLabel: ENTITY_KIND_LABEL[entity.kind], vertices: vertexCount(entity), plotScale, layerName, primitives });
  }
  const layers: unknown[] = [];
  while (layers.length < LAYERS) {
    const library = randomLibrary(g, system);
    const scene = layerScene(g, [...library.keys()], 6);
    const index = new PickIndex(scene.doc);
    let call: StyledCall | null = null;
    const entities = [...scene.doc.all()];
    buildStyledLayer('k', entities, scene.style, {
      origin: scene.origin,
      palette: PALETTE,
      plotScale: scene.plotScale,
      library: { symbol: (id) => library.get(id), asset: (id) => ASSETS.find((a) => a.id === id) },
      layerName: (id) => scene.doc.layers.get(id)?.name ?? id,
      geometry: captureStyled(index, (c) => (call = c)),
      clip: scene.clip,
    });
    const c = call as StyledCall | null;
    if (!c || c.data.length > MAX_NUMBERS || (!c.batches.length && !g.chance(0.1))) continue;
    layers.push({ layer: { name: scene.name, style: scene.style }, library: Object.fromEntries(library), entities, origin: scene.origin, plotScale: scene.plotScale, clip: scene.clip ?? null, ...c });
  }
  const head = {
    format: 'kentos.style-cases',
    version: 1,
    note: 'Stil derleyicisinin yanıtları. symbols: bir sembol bir nesnede (önizleme yolu): sembol, nesne, tür adı ve köşe sayısı (ifadeler için), çizim ölçeği, katman adı ve çizim ilkelleri; görüntülerin genişlik ve yüksekliği assets’tedir. layers: bir katmanın tek çağrıda kurulması: katmanın adı ve stili, kitaplık sembolleri, nesneler, orijin, çizim ölçeği, kırpma kutusu; sayfanın çekirdeğe verdikleri (program JSON’u, nesne başına dört sayı: kip, küme ya da sembol, basit görünüş kümesi, renk; ifade tablosu) ve çekirdeğin yanıtı (toplulukların tanımı ve float32 sayıları). JSON’un tutamadığı sayılar yazıdır: "NaN", "Infinity", "-Infinity", "-0".',
    assets: ASSET_SIZES,
  };
  // One case a line: a changed answer shows as its own line in the diff.
  const list = (cases: unknown[]) => cases.map((c) => `  ${JSON.stringify(c)}`).join(',\n');
  writeFileSync(OUT, `${JSON.stringify(head, null, 1).slice(0, -2)},\n "symbols": [\n${list(symbols)}\n ],\n "layers": [\n${list(layers)}\n ]\n}\n`);
});
