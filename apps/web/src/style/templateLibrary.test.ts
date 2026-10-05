import { describe, expect, it } from 'vitest';
import type { LibraryItem } from '../model/style';
import { StyleLibrary } from './library';
import { templateSymbol } from './templateSymbol';

/**
 * Object templates in the style library (docs/adr/0176 §2): found by id and by a search of their description, copied
 * into a project with the user symbol they draw with and that symbol's drawings, and pictured by their own symbol or
 * their look. The desktop's library holds the same in crates/native/style/tests/all/object_templates.rs.
 */

const drawing: LibraryItem = { kind: 'asset', id: 'a-agac', name: 'Ağaç', path: ['Çizimler'], format: 'svg', data: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4"/>', width: 4, height: 4 };
const tree: LibraryItem = { kind: 'symbol', id: 's-agac', name: 'Ağaç', path: ['Semboller'], symbol: { type: 'marker', layers: [{ id: 'v', type: 'svg', asset: 'a-agac', size: 4 }] } };
const treeTemplate: LibraryItem = {
  kind: 'template',
  id: 't-agac',
  name: 'Ağaç',
  path: ['Şablonlar'],
  description: 'Yeşil alanın ağaçları',
  template: { tool: 'point', layer: { path: ['Peyzaj'], name: 'Ağaç' }, symbol: 's-agac', attrs: { Tür: 'Çınar' } },
};

describe('templates in the style library', () => {
  it('are found by id and by their description, and copied into a project with what they draw with', () => {
    const lib = new StyleLibrary();
    lib.load('user', [drawing, tree, treeTemplate]);
    expect(lib.template('t-agac')?.template.tool).toBe('point');
    expect(lib.template('s-agac')).toBeUndefined();
    const found = lib.tree({ query: 'yeşil alan' }).flatMap((c) => c.items.map((i) => i.id));
    expect(found).toEqual(['t-agac']);
    const copy = lib.copy('t-agac', 'project');
    expect(copy.kind).toBe('template');
    expect(lib.items('project').map((i) => i.id).sort()).toEqual(['a-agac', copy.id, 's-agac'].sort());
  });

  it('are pictured by their own symbol, else by their look in their tool’s shape', () => {
    const lib = new StyleLibrary();
    lib.load('user', [drawing, tree]);
    expect(templateSymbol({ tool: 'point', layer: { path: [], name: 'A' }, symbol: 's-agac' }, lib)).toEqual((tree as { symbol: unknown }).symbol);
    expect(templateSymbol({ tool: 'polyline', layer: { path: [], name: 'Yol', lineWeight: 0.5 }, color: '#F5A524' }, lib)).toEqual({
      type: 'line',
      layers: [{ id: 'l', type: 'simpleLine', color: '#F5A524', width: 0.5 }],
    });
    // A symbol the library lacks: the look again; an area's outline in the layer's colour.
    expect(templateSymbol({ tool: 'polygon', layer: { path: [], name: 'Parsel', color: '#E5484D' }, symbol: 'yok' }, lib)).toEqual({
      type: 'fill',
      layers: [{ id: 'l', type: 'simpleLine', color: '#E5484D', width: 0.35 }],
    });
    expect(templateSymbol({ tool: 'text', layer: { path: [], name: 'Yazılar' } }).layers[0]).toMatchObject({ type: 'text', text: 'Abc', color: 'ink' });
  });
});
