import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import { yieldToPage } from '../../app/drawingFile';
import { holdDrawing } from '../../app/hold';
import type { ProgressiveImport } from '../../io/drawingImport';
import { h, overlayRoot } from '../dom';

/**
 * A large import going into the drawing (docs/adr/0138): a slice of time
 * per frame, so the page keeps drawing and the drawing fills in before the
 * user's eyes, with a panel at the bottom right that counts the objects and
 * stops it. Meanwhile only the view moves (app/hold.ts); Durdur, Esc and the
 * page closing take everything back. The desktop's is the same
 * (apps/desktop/src/exchange/drawing_import.rs `importing_view`).
 */

/** The first slice, and the range the next ones are kept in (ms): a third of the time the page took between two. */
const SLICE_LEAST = 10;
const SLICE_MOST = 50;

/** How an import that went in slices ended. */
export type Written = { kind: 'done'; objects: number } | { kind: 'stopped' } | { kind: 'failed'; error: string };

const count = (n: number) => n.toLocaleString('tr-TR');

/** Writes `work` into the drawing a frame at a time; `then` hears how it ended. */
export function writeImport(ctx: AppContext, name: string, work: ProgressiveImport, then: (w: Written) => void): void {
  const title = `İçe aktarılıyor: ${name}`;
  const bar = h('span');
  const detail = h('p', { class: 'importing__detail', role: 'status' });
  const stop = h('button', { class: 'btn btn--small', type: 'button' }, 'Durdur');
  const panel = h(
    'section',
    { class: 'importing', 'aria-label': title },
    h('div', { class: 'importing__head' }, h('span', { class: 'importing__title', title }, title), stop),
    h('div', { class: 'importing__bar', role: 'progressbar', 'aria-label': title, 'aria-valuemin': '0', 'aria-valuemax': '100' }, bar),
    detail,
  );
  let over = false;
  const show = () => {
    const share = Math.round(work.share * 100);
    bar.style.width = `${share}%`;
    bar.parentElement!.setAttribute('aria-valuenow', String(share));
    detail.textContent = `${count(work.done)} / ${count(work.total)} nesne çizime yazıldı`;
  };
  // In the drawing's bottom right corner, over the drawing and clear of the panels round it.
  const place = () => {
    const r = ctx.view.element?.getBoundingClientRect();
    panel.style.right = `${Math.max(16, innerWidth - (r?.right ?? innerWidth) + 16)}px`;
    panel.style.bottom = `${Math.max(16, innerHeight - (r?.bottom ?? innerHeight) + 16)}px`;
  };
  const end = (w: Written) => {
    if (over) return;
    over = true;
    release();
    removeEventListener('pagehide', halt);
    removeEventListener('resize', place);
    panel.remove();
    then(w);
  };
  const halt = () => {
    if (over) return;
    work.stop();
    end({ kind: 'stopped' });
  };
  const release = holdDrawing(ctx, { why: `“${name}” içe aktarılıyor: bitince ya da sağ alttaki Durdur'a basınca çizim yeniden düzenlenebilir; görünüm bu arada kaydırılıp yakınlaştırılabilir.`, keep: panel, onEscape: halt });
  addEventListener('pagehide', halt);
  addEventListener('resize', place);
  stop.addEventListener('click', halt);
  show();
  place();
  overlayRoot().append(panel);
  stop.focus();

  let last: number | null = null;
  const frame = () => {
    if (over) return;
    const budget = last === null ? SLICE_LEAST : Math.min(SLICE_MOST, Math.max(SLICE_LEAST, (performance.now() - last) / 3));
    const r = work.step(budget);
    last = performance.now();
    show();
    if (r === 'more') return next();
    if (r === 'done') end({ kind: 'done', objects: work.done });
    else end({ kind: 'failed', error: r.error });
  };
  // Right after a frame is painted: the slice runs, then the page draws what it wrote. A hidden
  // page paints nothing and is given a frame a second at most: its slices follow one another.
  const next = () => (document.hidden ? void yieldToPage().then(frame) : requestAnimationFrame(() => setTimeout(frame, 0)));
  next();
}
