import { describe, expect, it } from 'vitest';
import type { Entity } from '../model/entities';
import type { LayerStyle } from '../model/layers';
import type { Symbol } from '../model/style';
import { legendOf, type LegendLayer } from './legend';

const base: LayerStyle = { color: '#336699', lineType: 'continuous', lineWeight: 0.25 } as LayerStyle;
const area = (attrs: Record<string, string>, symbol?: string): Entity => ({ id: 1, kind: 'polygon', layerId: 'p', attrs, symbol, pts: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }] });
const lib: Record<string, Symbol> = { hatch: { type: 'fill', layers: [{ id: '0', type: 'hatchFill', angle: 45, spacing: 2, width: 0.2, color: 'ink' }] } };
const src = (entities: Entity[]) => ({ entities: () => entities, symbol: (r: string) => lib[r], itemName: (id: string) => (id === 'hatch' ? 'Tarama' : undefined) });

describe('legend', () => {
  it('shows a plain layer by its own look and skips layers without drawable objects', () => {
    const layers: LegendLayer[] = [
      { id: 'p', name: 'Parseller', style: base },
      { id: 't', name: 'Yazılar', style: base },
    ];
    const g = legendOf(layers, { ...src([area({})]), entities: (id) => (id === 'p' ? [area({})] : []) });
    expect(g.map((x) => [x.layerName, x.entries.map((e) => e.label)])).toEqual([['Parseller', ['Parseller']]]);
    expect(g[0].entries[0].symbol?.type).toBe('fill');
  });

  it('lists categories, the other values, rules with their parents, and objects\' own symbols', () => {
    const fill = (color: string): Symbol => ({ type: 'fill', layers: [{ id: '0', type: 'simpleFill', color }] });
    const cat: LegendLayer = {
      id: 'p',
      name: 'Parseller',
      style: {
        ...base,
        renderer: {
          type: 'categorized',
          expr: 'Nitelik',
          categories: [
            { value: 'Arsa', label: 'Arsa', symbols: { fill: fill('#FF0000') } },
            { value: 'Tarla', label: '', symbols: { fill: { ref: 'hatch' } } },
            { value: 'Bağ', label: 'Bağ', symbols: { fill: fill('#00FF00') }, enabled: false },
          ],
          other: { fill: fill('#999999') },
        },
      },
    };
    const g = legendOf([cat], src([area({ Nitelik: 'Arsa' }, 'hatch')]));
    expect(g[0].entries.map((e) => e.label)).toEqual(['Arsa', 'Tarla', 'Diğer değerler', 'Tarama']);
    const rules: LegendLayer = {
      id: 'p',
      name: 'Parseller',
      style: { ...base, renderer: { type: 'rules', rules: [{ id: 'a', label: 'Büyük', filter: '$alan > 500', symbols: { fill: fill('#FF0000') }, children: [{ id: 'b', label: 'Arsa', symbols: { fill: fill('#00FF00') } }] }, { id: 'c', label: 'Kapalı', enabled: false, symbols: { fill: fill('#0000FF') } }] } },
    };
    expect(legendOf([rules], src([area({})]))[0].entries.map((e) => e.label)).toEqual(['Büyük', 'Büyük › Arsa']);
  });
});
