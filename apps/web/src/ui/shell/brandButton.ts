import type { AppContext } from '../../app/context';
import { listen, type DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';

/**
 * The KentOS mark and wordmark at the left of the menu bar and of the
 * ribbon's tab row (DESIGN.md §2). A button: it opens the application menu
 * (ui/appmenu/AppMenu.ts, loaded on first use), the place for files,
 * import and export, the cloud and the settings.
 */

/**
 * The mark: a navy tile with a white K whose arms meet at a survey point.
 * The brand colour is its own (--c-brand), not the accent the user picks.
 */
export function brandMark(size = 20): SVGSVGElement {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('width', String(size));
  svg.setAttribute('height', String(size));
  svg.setAttribute('aria-hidden', 'true');
  svg.classList.add('brand__mark');
  const id = `brand-g-${Math.random().toString(36).slice(2, 8)}`;
  svg.innerHTML =
    `<defs><linearGradient id="${id}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="var(--c-brand-hi)"/><stop offset="1" stop-color="var(--c-brand)"/></linearGradient></defs>` +
    `<rect x="1" y="1" width="22" height="22" rx="6" fill="url(#${id})"/>` +
    '<rect x="1.5" y="1.5" width="21" height="21" rx="5.5" fill="none" stroke="rgba(255,255,255,0.28)" stroke-width="1"/>' +
    '<path d="M8 6.2v11.6" stroke="var(--c-brand-ink)" stroke-width="2.3" stroke-linecap="round"/>' +
    '<path d="M16.6 6.4 10.3 12l6.3 5.6" fill="none" stroke="var(--c-brand-ink)" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"/>' +
    '<circle cx="10.3" cy="12" r="2.1" fill="var(--c-brand-ink)"/>' +
    '<circle cx="10.3" cy="12" r="0.8" fill="var(--c-brand-hi)"/>';
  return svg;
}

/** The mark, the wordmark and a caret: opens the application menu. */
export function brandButton(ctx: AppContext, d: DisposableStore, extraClass = ''): HTMLButtonElement {
  const b = h(
    'button',
    { class: `brand ${extraClass}`.trim(), type: 'button', 'aria-haspopup': 'dialog', 'aria-expanded': 'false', 'aria-label': 'KentOS uygulama menüsü' },
    brandMark(20),
    h('span', { class: 'brand__word' }, 'Kent', h('span', { class: 'brand__os' }, 'OS')),
    h('span', { class: 'brand__caret' }, icon('chevronDown', 12)),
  );
  let opening = false;
  const open = async (keyboard: boolean) => {
    if (opening) return;
    opening = true;
    try {
      const { openAppMenu } = await import('../appmenu/AppMenu');
      openAppMenu(ctx, b, keyboard);
    } catch (e) {
      ctx.log.error(`Uygulama menüsü yüklenemedi: ${(e as Error).message}. Bağlantınızı denetleyip yeniden deneyin.`);
    } finally {
      opening = false;
    }
  };
  // Pressing opens (as the menus do); a second press closes it (the menu listens for presses outside it).
  d.add(
    listen<PointerEvent>(b, 'pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      if (b.getAttribute('aria-expanded') === 'true') return;
      void open(false);
    }),
  );
  d.add(
    listen<KeyboardEvent>(b, 'keydown', (e) => {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown') {
        e.preventDefault();
        void open(true);
      }
    }),
  );
  d.add(tooltip(b, () => (b.getAttribute('aria-expanded') === 'true' ? null : { title: 'KentOS', description: 'Uygulama menüsü: yeni, aç, kaydet, içe ve dışa aktar, bulut ve ayarlar.' }), 'bottom'));
  return b;
}
