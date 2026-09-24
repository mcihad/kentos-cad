import type { LayerInit } from './layers';

/**
 * The standard layer tree of a cadastral sheet: what the sample project
 * draws on and what a new project starts with. The map tools write to
 * layers of this tree by id (`parsel`, `kot`, see tools/catalog.ts), so a
 * new project must carry them. A fresh copy each call: layer styles are
 * project data and must not be shared between drawings.
 */

/** The layer new objects go to at first. */
export const STANDARD_ACTIVE_LAYER = 'taslak';

/** `plotScale` is written into the sheet frame's label ("Pafta P-12   1:1000"). */
export function standardLayers(plotScale: number): LayerInit[] {
  return [
    { id: 'taslak', name: 'Taslak', style: { color: 'ink' } },
    {
      id: 'g-kadastro',
      name: 'Kadastro',
      children: [
        {
          id: 'ada',
          name: 'Ada sınırı',
          style: {
            color: 'fg',
            lineWeight: 0.35,
            label: { placement: 'center', size: 12, grow: 3, maxSize: 22, weight: 600, template: '{label} ada', minFeaturePx: 90, maxScale: 5, ink: 'fg' },
          },
        },
        { id: 'parsel', name: 'Parsel sınırı', style: { color: 'fg-dim', lineWeight: 0.18, label: { placement: 'center', size: 9, grow: 1.2, maxSize: 14, minFeaturePx: 26 } } },
        { id: 'yapi', name: 'Yapı', style: { color: '#7FB2E5', lineWeight: 0.25, fill: '#7FB2E52E' } },
      ],
    },
    {
      id: 'g-ulasim',
      name: 'Ulaşım',
      children: [
        { id: 'yol-ekseni', name: 'Yol ekseni', style: { color: '#E06C75', lineType: 'dashdot', lineWeight: 0.13 } },
        { id: 'kaldirim', name: 'Kaldırım', style: { color: '#8C9AAA', lineWeight: 0.18 } },
      ],
    },
    {
      id: 'g-topo',
      name: 'Topografya',
      children: [
        { id: 'esyukselti', name: 'Eşyükselti', style: { color: '#A87C54', lineWeight: 0.13 } },
        { id: 'ana-esyukselti', name: 'Ana eşyükselti', style: { color: '#C9955F', lineWeight: 0.25, label: { placement: 'along', size: 10, minScale: 1.6 } } },
        { id: 'kot', name: 'Kot noktaları', style: { color: '#9CCB7E', point: { symbol: 'cross', size: 7 }, label: { placement: 'beside', size: 10.5, minScale: 2.2 } } },
      ],
    },
    {
      id: 'g-jeodezi',
      name: 'Jeodezi',
      children: [{ id: 'poligon', name: 'Poligon noktaları', style: { color: '#56B6C2', point: { symbol: 'triangle', size: 11 }, label: { placement: 'beside', size: 10.5, minScale: 0.9 } } }],
    },
    {
      id: 'g-pafta',
      name: 'Pafta',
      expanded: false,
      children: [
        {
          id: 'pafta',
          name: 'Pafta çerçevesi',
          style: { color: '#6B7785', lineWeight: 0.5, pickInterior: false, label: { placement: 'corner', size: 12, template: `Pafta {label}   1:${plotScale}`, minFeaturePx: 200, ink: 'fg-dim' } },
        },
        { id: 'karelaj', name: 'Karelaj', style: { color: '#6B7785', lineWeight: 0.13 } },
        { id: 'yazi', name: 'Yazılar', style: { color: 'fg-dim' } },
      ],
    },
  ];
}
