import type { AppContext } from '../../app/context';
import type { LibraryItem, Symbol } from '../../model/style';
import { drawSymbolPreview, type PreviewGeometry } from '../../render/symbolPreview';
import { h } from '../dom';

/**
 * Symbol pictures for lists and grids: each canvas is drawn only when it
 * scrolls into view (a library of hundreds of symbols stays quick to open)
 * and again when an SVG it needs has loaded.
 */

/** A library item as something drawable: SVG and raster assets become a marker of themselves. */
export function symbolOfItem(item: LibraryItem): Symbol {
  if (item.kind === 'symbol') return item.symbol;
  return { type: 'marker', layers: [item.format === 'svg' ? { id: 'a', type: 'svg', asset: item.id, size: 14, fill: 'ink' } : { id: 'a', type: 'raster', asset: item.id, size: 14 }] };
}

export class Thumbs {
  private readonly ctx: AppContext;
  private readonly io: IntersectionObserver;
  private readonly draws = new WeakMap<Element, () => void>();

  constructor(ctx: AppContext, scrollRoot: Element | null) {
    this.ctx = ctx;
    this.io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (!e.isIntersecting) continue;
          this.io.unobserve(e.target);
          this.draws.get(e.target)?.();
        }
      },
      { root: scrollRoot, rootMargin: '160px' },
    );
  }

  /** A canvas of `w` × `h` CSS px showing `symbol`, drawn when it becomes visible. */
  canvas(symbol: Symbol, w: number, height: number, geometry?: PreviewGeometry): HTMLCanvasElement {
    const c = h('canvas', { class: 'sthumb', width: String(w), height: String(height), style: `width:${w}px;height:${height}px`, 'aria-hidden': 'true' });
    const draw = () => {
      if (!c.isConnected) return;
      drawSymbolPreview(c, symbol, { palette: this.ctx.view.palette, library: this.ctx.styles.library, geometry, background: 'paper', onLoad: draw });
    };
    this.draws.set(c, draw);
    this.io.observe(c);
    return c;
  }

  dispose(): void {
    this.io.disconnect();
  }
}

/** Draws a symbol into a canvas now (and again when its images arrive). */
export function drawNow(ctx: AppContext, c: HTMLCanvasElement, symbol: Symbol, geometry?: PreviewGeometry, pxPerMm?: number): void {
  const draw = () => {
    if (c.isConnected) drawSymbolPreview(c, symbol, { palette: ctx.view.palette, library: ctx.styles.library, geometry, background: 'paper', pxPerMm, onLoad: draw });
  };
  draw();
}
