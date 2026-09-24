import { describe, expect, it } from 'vitest';
import type { LibraryAsset, LibrarySymbol, MarkerSymbol, Symbol } from '../../model/style';
import { validateSymbol } from '../file';
import { SYSTEM_LIBRARY } from './index';

/** Every shipped symbol must load, draw and be found: the library is data, so it is checked as data. */

const symbols = SYSTEM_LIBRARY.items.filter((i): i is LibrarySymbol => i.kind === 'symbol');
const assets = new Set(SYSTEM_LIBRARY.items.filter((i): i is LibraryAsset => i.kind === 'asset').map((a) => a.id));

/** Asset ids a symbol draws with (SVG and raster markers, image fills), nested markers included. */
function assetsOf(sym: Symbol | MarkerSymbol): string[] {
  const out: string[] = [];
  for (const l of sym.layers as unknown as readonly Record<string, unknown>[]) {
    if ((l.type === 'svg' || l.type === 'raster' || l.type === 'imageFill') && typeof l.asset === 'string') out.push(l.asset);
    if (l.marker) out.push(...assetsOf(l.marker as MarkerSymbol));
  }
  return out;
}

describe('system library', () => {
  it('has unique ids', () => {
    const ids = SYSTEM_LIBRARY.items.map((i) => i.id);
    const dup = ids.filter((id, n) => ids.indexOf(id) !== n);
    expect(dup).toEqual([]);
  });

  it('has only valid symbols', () => {
    const issues = symbols.flatMap((s) => validateSymbol(s.symbol, s.id));
    expect(issues).toEqual([]);
  });

  it('draws only with assets it ships', () => {
    const missing = symbols.flatMap((s) => assetsOf(s.symbol).filter((a) => !assets.has(a)).map((a) => `${s.id} → ${a}`));
    expect(missing).toEqual([]);
  });

  it('puts every item in a declared category', () => {
    const declared = new Set(SYSTEM_LIBRARY.categories.map((c) => c.path.join('/')));
    const loose = SYSTEM_LIBRARY.items.filter((i) => !declared.has(i.path.join('/'))).map((i) => i.id);
    expect(loose).toEqual([]);
  });
});
