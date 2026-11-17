// A short message after a step (DESIGN.md §10.1: the same verb as the button): it appears at the bottom, stays a few
// seconds and goes; a refusal stays until it is closed.
import { h } from '../lib/dom';
import type { Tone } from '../app/state';
import { icon } from './icons';

function host(): HTMLElement {
  let el = document.getElementById('toasts');
  if (!el) {
    el = h('div', { id: 'toasts', class: 'toasts', 'aria-live': 'polite' });
    document.body.append(el);
  }
  return el;
}

export function toast(text: string, tone: Tone = 'success'): void {
  const close = h('button', { class: 'icon-btn', type: 'button', 'aria-label': 'Kapat' }, icon('close', 14));
  const el = h('div', { class: `toast tone-${tone}`, role: tone === 'danger' ? 'alert' : 'status' }, icon(tone === 'danger' || tone === 'warning' ? 'alert' : 'check', 16), h('span', null, text), close);
  close.addEventListener('click', () => el.remove());
  host().append(el);
  if (tone !== 'danger') setTimeout(() => el.remove(), 4200);
}
