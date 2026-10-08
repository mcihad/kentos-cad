import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import { rasterService } from '../../render/rasterService';
import { h, overlayRoot, replaceChildren } from '../dom';

/**
 * The rasters' long work (docs/adr/0204 §3, §6; the desktop's `rasters/jobs.rs`): a large raster's pyramid pass and
 * Raster oturt's resampling, each a line with its share and Durdur, in a panel at the drawing's lower right (the large
 * import's, ui/io/importing.ts). The drawing stays in use meanwhile.
 */
export function mountRasterJobs(ctx: AppContext): () => void {
  const service = rasterService();
  const panel = h('section', { class: 'importing raster-jobs', 'aria-label': 'Raster işleri' });
  const place = () => {
    const r = ctx.view.element?.getBoundingClientRect();
    panel.style.right = `${Math.max(16, innerWidth - (r?.right ?? innerWidth) + 16)}px`;
    panel.style.bottom = `${Math.max(16, innerHeight - (r?.bottom ?? innerHeight) + 16)}px`;
  };
  const line = (title: string, share: number, detail: string, stop: () => void) => {
    const button = h('button', { class: 'btn btn--small', type: 'button' }, 'Durdur');
    button.addEventListener('click', stop);
    const pct = Math.round(share * 100);
    return h(
      'div',
      { class: 'raster-jobs__line' },
      h('div', { class: 'importing__head' }, h('span', { class: 'importing__title', title }, title), button),
      h('div', { class: 'importing__bar', role: 'progressbar', 'aria-label': title, 'aria-valuemin': '0', 'aria-valuemax': '100', 'aria-valuenow': String(pct) }, h('span', { style: { width: `${pct}%` } })),
      h('p', { class: 'importing__detail', role: 'status' }, detail),
    );
  };
  const render = () => {
    const pyramids = service.pyramids.value;
    const warp = service.warping.value;
    if (!pyramids.length && !warp) {
      panel.remove();
      return;
    }
    replaceChildren(
      panel,
      pyramids.map((p) => line(`Önizleme piramidi: ${p.name}`, p.done, `%${Math.round(p.done * 100)} hazır; bitince raster her ölçekte hızla çizilir`, () => service.stopPyramid(p.key))),
      warp ? line(`Raster oturtuluyor: ${warp.name}`, warp.done, `%${Math.round(warp.done * 100)} yeniden örneklendi`, warp.stop) : null,
    );
    place();
    if (!panel.isConnected) overlayRoot().append(panel);
  };
  const off = [service.pyramids.subscribe(render), service.warping.subscribe(render)];
  addEventListener('resize', place);
  return () => {
    off.forEach((d) => d());
    removeEventListener('resize', place);
    panel.remove();
  };
}
