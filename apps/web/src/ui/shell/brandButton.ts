import type { AppContext } from '../../app/context';
import { listen, type DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';

/**
 * The KentOS mark and wordmark at the left of the ribbon's tab row
 * (DESIGN.md §2). A button: it opens the application menu
 * (ui/appmenu/AppMenu.ts, loaded on first use), the place for files,
 * import and export, the cloud and the settings.
 */

/** The supplied icon and light/dark wordmarks, bundled from the shared assets. */
const appIcon = new URL('../../../../../assets/images/kentos_app_icon.png', import.meta.url).href;
const darkLogo = new URL('../../../../../assets/images/kentos_dark_logo.png', import.meta.url).href;
const lightLogo = new URL('../../../../../assets/images/kentos_light_logo.png', import.meta.url).href;
export function brandMark(size = 20): HTMLImageElement {
  return h('img', { class: 'brand__mark', src: appIcon, width: size, height: size, alt: '', 'aria-hidden': 'true' });
}
export function brandLogo(width = 100): HTMLSpanElement {
  return h('span', { class: 'brand__logo', style: `width:${width}px`, role: 'img', 'aria-label': 'KentOS' },
    h('img', { class: 'brand__logo-dark', src: darkLogo, alt: '' }),
    h('img', { class: 'brand__logo-light', src: lightLogo, alt: '' }));
}

/** The mark, the wordmark and a caret: opens the application menu. */
export function brandButton(ctx: AppContext, d: DisposableStore, extraClass = ''): HTMLButtonElement {
  const compact = brandMark(20);
  compact.classList.add('brand__compact');
  const b = h(
    'button',
    { class: `brand ${extraClass}`.trim(), type: 'button', 'aria-haspopup': 'dialog', 'aria-expanded': 'false', 'aria-label': 'KentOS uygulama menüsü' },
    brandLogo(),
    compact,
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
