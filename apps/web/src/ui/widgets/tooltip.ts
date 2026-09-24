import { listen, type Disposable } from '../../core/disposable';
import { formatChord } from '../../core/keymap';
import { h, overlayRoot, replaceChildren } from '../dom';

export interface TooltipContent {
  title: string;
  shortcut?: string;
  description?: string;
  note?: string;
  /** Numbered "how to use" steps (tools). */
  steps?: readonly string[];
}

const DELAY = 450;
let tip: HTMLElement | null = null;
let timer = 0;
let owner: HTMLElement | null = null;
/** Target waiting for the show delay. */
let pending: HTMLElement | null = null;
/** Once a tooltip was shown, neighbours appear instantly (toolbar scanning). */
let warmUntil = 0;

function ensure(): HTMLElement {
  if (!tip) {
    tip = h('div', { class: 'tooltip', role: 'tooltip' });
    overlayRoot().append(tip);
  }
  return tip;
}

function show(target: HTMLElement, content: TooltipContent, placement: 'right' | 'bottom' | 'top'): void {
  const el = ensure();
  replaceChildren(
    el,
    h(
      'div',
      { class: 'tooltip__head' },
      h('span', { class: 'tooltip__title' }, content.title),
      content.shortcut ? h('kbd', { class: 'kbd' }, formatChord(content.shortcut)) : null,
    ),
    content.description ? h('div', { class: 'tooltip__desc' }, content.description) : null,
    content.note ? h('div', { class: 'tooltip__note' }, content.note) : null,
    content.steps?.length ? h('ol', { class: 'tooltip__steps' }, content.steps.map((t) => h('li', null, t))) : null,
  );
  el.dataset.open = '';
  const r = target.getBoundingClientRect();
  const t = el.getBoundingClientRect();
  let x = placement === 'right' ? r.right + 8 : r.left + r.width / 2 - t.width / 2;
  let y = placement === 'right' ? r.top + r.height / 2 - t.height / 2 : placement === 'top' ? r.top - t.height - 8 : r.bottom + 8;
  x = Math.max(8, Math.min(x, innerWidth - t.width - 8));
  y = Math.max(8, Math.min(y, innerHeight - t.height - 8));
  el.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
}

/** Hides the tooltip; with `scope`, only when it belongs to an element inside it (a list about to be redrawn). */
export function hideTooltip(scope?: HTMLElement): void {
  if (scope && !(owner && scope.contains(owner)) && !(pending && scope.contains(pending))) return;
  pending = null;
  clearTimeout(timer);
  if (tip && owner) {
    delete tip.dataset.open;
    warmUntil = performance.now() + 600;
  }
  owner = null;
}

/**
 * Attach a rich tooltip; content is resolved lazily so shortcuts stay
 * current. Content can be null to show nothing this time (a name that fits).
 */
export function tooltip(
  target: HTMLElement,
  content: () => TooltipContent | null,
  placement: 'right' | 'bottom' | 'top' = 'bottom',
): Disposable {
  const enter = () => {
    clearTimeout(timer);
    pending = target;
    const delay = performance.now() < warmUntil ? 0 : DELAY;
    timer = window.setTimeout(() => {
      pending = null;
      if (!target.isConnected) return;
      const c = content();
      if (!c) return;
      owner = target;
      show(target, c, placement);
    }, delay);
  };
  const subs = [
    listen(target, 'pointerenter', enter),
    listen(target, 'pointerleave', () => hideTooltip()),
    listen(target, 'pointerdown', () => hideTooltip()),
    listen(target, 'focus', enter),
    listen(target, 'blur', () => hideTooltip()),
  ];
  return () => subs.forEach((d) => d());
}
