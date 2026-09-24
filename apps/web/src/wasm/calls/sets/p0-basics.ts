import { repeat, type CallSet } from '../harness';

/** The first operations, which set up the call path itself (Dilim 0). */
export const P0: CallSet = {
  file: 'calls-p0-basics.json',
  named: [
    { name: 'aynı nokta', fn: 'dist', args: [{ x: 1, y: 2 }, { x: 1, y: 2 }] },
    { name: '3-4-5', fn: 'dist', args: [{ x: 0, y: 0 }, { x: 3, y: 4 }] },
    { name: 'TM koordinatı', fn: 'dist', args: [{ x: 486512.34, y: 4420123.45 }, { x: 486598.12, y: 4420001.07 }] },
    { name: 'iki köşe', fn: 'signedArea', args: [[{ x: 0, y: 0 }, { x: 1, y: 1 }]] },
    { name: 'saat yönünde kare', fn: 'signedArea', args: [[{ x: 0, y: 0 }, { x: 0, y: 1 }, { x: 1, y: 1 }, { x: 1, y: 0 }]] },
    { name: 'TM parseli', fn: 'signedArea', args: [[{ x: 486500.1, y: 4420100.2 }, { x: 486540.35, y: 4420101.9 }, { x: 486538.8, y: 4420141.15 }, { x: 486501.05, y: 4420139.6 }]] },
    { name: 'kenarda nokta', fn: 'pointInPolygon', args: [{ x: 0, y: 0.5 }, [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }, { x: 0, y: 1 }]] },
    { name: 'boş halka', fn: 'pointInPolygon', args: [{ x: 0, y: 0 }, []] },
  ],
  random: (g, n) => [
    ...repeat(g, 'dist', n, () => [g.pt(), g.pt()]),
    ...repeat(g, 'signedArea', n, () => [g.ring(g.int(3, 12), 80, g.chance(0.5))]),
    ...repeat(g, 'pointInPolygon', n, () => [g.chance(0.5) ? g.gridPt(10) : g.pt(), g.chance(0.5) ? g.ring(g.int(3, 9)) : Array.from({ length: g.int(3, 7) }, () => g.gridPt(10))]),
  ],
};
