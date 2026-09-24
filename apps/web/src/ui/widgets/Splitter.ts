import { listen, type Disposable } from '../../core/disposable';
import { h } from '../dom';

/**
 * Drag handle between two regions. `onDrag` receives the pointer delta in
 * px since drag start; the owner decides what it resizes. Keyboard arrows
 * nudge by 16 px.
 */
export function splitter(opts: {
  orientation: 'vertical' | 'horizontal';
  label: string;
  onStart?: () => void;
  onDrag: (delta: number) => void;
  onEnd?: () => void;
  onReset?: () => void;
}): { el: HTMLElement; dispose: Disposable } {
  const el = h('div', {
    class: `splitter splitter--${opts.orientation}`,
    role: 'separator',
    'aria-orientation': opts.orientation,
    'aria-label': opts.label,
    tabindex: '0',
  });
  let start: number | null = null;
  const pos = (e: PointerEvent) => (opts.orientation === 'vertical' ? e.clientX : e.clientY);
  const subs = [
    listen<PointerEvent>(el, 'pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      el.setPointerCapture(e.pointerId);
      start = pos(e);
      el.dataset.active = '';
      document.body.dataset.resizing = opts.orientation;
      opts.onStart?.();
    }),
    listen<PointerEvent>(el, 'pointermove', (e) => {
      if (start !== null) opts.onDrag(pos(e) - start);
    }),
    listen<PointerEvent>(el, 'pointerup', () => {
      if (start === null) return;
      start = null;
      delete el.dataset.active;
      delete document.body.dataset.resizing;
      opts.onEnd?.();
    }),
    listen(el, 'dblclick', () => opts.onReset?.()),
    listen<KeyboardEvent>(el, 'keydown', (e) => {
      const back = opts.orientation === 'vertical' ? 'ArrowLeft' : 'ArrowUp';
      const fwd = opts.orientation === 'vertical' ? 'ArrowRight' : 'ArrowDown';
      if (e.key !== back && e.key !== fwd) return;
      e.preventDefault();
      opts.onStart?.();
      opts.onDrag(e.key === fwd ? 16 : -16);
      opts.onEnd?.();
    }),
  ];
  return { el, dispose: () => subs.forEach((d) => d()) };
}
