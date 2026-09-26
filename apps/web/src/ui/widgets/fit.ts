import type { Disposable } from '../../core/disposable';

export interface BarFit {
  /** Fits again at the next frame (the bar's content changed width). */
  refit(): void;
  dispose: Disposable;
}

/**
 * Fits a bar to its width in steps, the way the ribbon does (DESIGN.md
 * §7.3, §7.7): level 0 is the whole bar, and each further level narrows or
 * folds one more thing. `apply(level)` sets a level (attributes the CSS
 * reads, widths); `fits()` measures after it. The first level that fits is
 * kept; past the last one the bar is as small as it gets (below the shell's
 * 1100 px the page scrolls instead).
 *
 * It runs when the bar changes size (the window, the type scale), in the
 * same frame, before paint. `refit` asks again when the content changed.
 * The bar's own size is all that is observed: its cells change size as it
 * fits, and watching them would make the observer loop.
 */
export function fitBar(el: HTMLElement, levels: number, apply: (level: number) => void, fits: () => boolean): BarFit {
  let frame = 0;
  const run = () => {
    frame = 0;
    if (!el.isConnected) return;
    for (let level = 0; level <= levels; level++) {
      apply(level);
      if (level === levels || fits()) return;
    }
  };
  const ro = new ResizeObserver(() => run());
  ro.observe(el);
  return {
    refit: () => {
      if (!frame) frame = requestAnimationFrame(run);
    },
    dispose: () => {
      ro.disconnect();
      cancelAnimationFrame(frame);
    },
  };
}
