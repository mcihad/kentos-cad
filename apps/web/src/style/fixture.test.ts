import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import type { LayerStyle } from '../model/layers';
import type { Symbol } from '../model/style';
import { buildStyledLayer } from '../render/styledLayer';
import { PickIndex } from '../viewport/picking';
import { ASSETS, layerDocument, PALETTE } from './cases';
import { compileSymbol } from './compile';
import { captureStyled, decodeNumber, type StyledCall } from './fixture';
import type { Primitives } from './primitives';

/**
 * The frozen answers of the style compiler (fixtures/style/v1/cases.json,
 * scripts/fixtures/record-style.test.ts) through the app's path: one symbol
 * on one object (previews), and whole layers, where the page's half of the
 * call (the program, the objects' numbers, the expression table) is held to
 * the file as well as the core's answer. The Rust core checks the same file
 * natively (crates/shared/style-core/tests/style.rs).
 */

interface SymbolCase {
  symbol: Symbol;
  entity: Entity;
  plotScale: number;
  layerName: string;
  primitives: Primitives;
}

interface LayerCase extends StyledCall {
  layer: { name: string; style: LayerStyle };
  library: Record<string, Symbol>;
  entities: Entity[];
  origin: Vec2;
  plotScale: number;
  clip: Bounds | null;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/style/v1/cases.json', import.meta.url), 'utf8')) as { assets: Record<string, [number, number]>; symbols: SymbolCase[]; layers: LayerCase[] };

describe('style fixture', () => {
  it(`draws ${file.symbols.length} symbols on objects as frozen`, () => {
    const wrong: string[] = [];
    file.symbols.forEach((c, i) => {
      const got = compileSymbol(c.symbol, c.entity, { plotScale: c.plotScale, layerName: c.layerName, assets: file.assets });
      if (JSON.stringify(got) !== JSON.stringify(c.primitives)) wrong.push(`sembol ${i}`);
    });
    expect(wrong).toEqual([]);
  });

  it(`builds ${file.layers.length} layers as frozen, the page's half of the call included`, () => {
    const wrong: string[] = [];
    file.layers.forEach((c, i) => {
      const doc = layerDocument(c.layer.name, c.layer.style, c.entities);
      let call: StyledCall | null = null;
      buildStyledLayer('k', [...doc.all()], doc.layers.get('k')!.style, {
        origin: c.origin,
        palette: PALETTE,
        plotScale: c.plotScale,
        library: { symbol: (id) => c.library[id], asset: (id) => ASSETS.find((a) => a.id === id) },
        layerName: (id) => doc.layers.get(id)?.name ?? id,
        geometry: captureStyled(new PickIndex(doc), (x) => (call = x)),
        clip: c.clip ?? undefined,
      });
      const got = call as StyledCall | null;
      if (!got) return void wrong.push(`katman ${i}: çekirdeğe gidilmedi`);
      if (got.program !== c.program) wrong.push(`katman ${i}: program`);
      if (JSON.stringify(got.objects) !== JSON.stringify(c.objects)) wrong.push(`katman ${i}: nesneler`);
      if (JSON.stringify(got.table) !== JSON.stringify(c.table)) wrong.push(`katman ${i}: ifade tablosu`);
      if (JSON.stringify(got.batches) !== JSON.stringify(c.batches)) wrong.push(`katman ${i}: topluluklar`);
      const data = got.data.map(decodeNumber);
      const want = c.data.map(decodeNumber);
      if (data.length !== want.length || data.some((x, k) => !Object.is(x, want[k]))) wrong.push(`katman ${i}: sayılar`);
    });
    expect(wrong).toEqual([]);
  });
});
