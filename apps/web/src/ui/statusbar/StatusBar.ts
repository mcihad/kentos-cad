import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { effectiveWorkspace, WORKSPACES, workspaceById } from '../../app/workspaces';
import type { LogEntry } from '../../app/state';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { webgpuSupported } from '../../render/webgpu/support';
import { fitBar } from '../widgets/fit';
import { PopupMenu } from '../widgets/PopupMenu';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import { accountMenu, saveCell } from './cloudCells';

/** Screen metres per CSS pixel → map-like scale at 96 dpi. */
const screenScale = (pxPerMetre: number) => Math.round(1 / pxPerMetre / 0.00026458);
const fmtScale = (n: number) => n.toLocaleString('tr-TR');
const SERVER_TEXT = { checking: 'Sunucu…', online: 'Sunucu: bağlı', offline: 'Sunucu: yok', incompatible: 'Sunucu: uyumsuz' } as const;

export class StatusBar extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private flashTimer = 0;
  /** Fits the bar again after a cell's text changed (set once the bar is built). */
  private refit = () => {};

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const y = h('span', { class: 'status__value num' });
    const x = h('span', { class: 'status__value num' });
    const coords = h(
      'div',
      { class: 'status__coords', 'aria-live': 'off' },
      h('span', { class: 'status__axis' }, 'Y'),
      y,
      h('span', { class: 'status__axis' }, 'X'),
      x,
    );
    const flash = h('div', { class: 'status__flash', 'aria-live': 'polite' });
    const selCount = h('span', { class: 'status__cell status__sel num' });

    const toggles = h(
      'div',
      { class: 'status__toggles', role: 'group', 'aria-label': 'Çizim yardımcıları' },
      this.toggle('draft.snap', 'Kenet'),
      this.toggle('draft.grid', 'Izgara'),
      this.toggle('draft.ortho', 'Orto'),
      this.toggle('draft.polar', 'Kutupsal'),
      this.toggle('draft.tracking', 'İzleme'),
      this.toggle('view.lineWeights', 'Kalınlık'),
    );

    const crs = h('button', { class: 'status__cell status__btn status__crs', type: 'button' }, icon('crs', 14), h('span'));
    crs.addEventListener('click', () => ctx.commands.execute('crs.set'));
    const zoom = h('span', { class: 'status__cell status__zoom num' });
    const rendererName = h('span', { class: 'status__renderer-name' });
    const renderer = h('button', { class: 'status__cell status__btn status__renderer', type: 'button', 'aria-haspopup': 'menu' }, icon('chip', 14), rendererName);
    renderer.addEventListener('click', () =>
      PopupMenu.open(
        [
          { kind: 'header', label: 'Çizim motoru' },
          commandItem(ctx, 'view.renderer.webgl2', { hint: 'varsayılan' }),
          commandItem(ctx, 'view.renderer.webgpu', { hint: webgpuSupported() ? undefined : 'desteklenmiyor' }),
          { kind: 'separator' },
          commandItem(ctx, 'tools.options'),
        ],
        renderer.getBoundingClientRect(),
        { placement: 'below', owner: renderer },
      ),
    );

    // The project's work mode: which menus, tabs and tools show (app/workspaces.ts).
    const modeName = h('span', { class: 'status__mode-name' });
    const modeIcon = h('span', { class: 'status__mode-icon' });
    const mode = h('button', { class: 'status__cell status__btn status__mode', type: 'button', 'aria-haspopup': 'menu' }, modeIcon, modeName);
    mode.addEventListener('click', () =>
      PopupMenu.open(
        [
          { kind: 'header', label: 'Çalışma modu' },
          ...WORKSPACES.filter((w) => w.status === 'ready').map((w) => commandItem(ctx, `workspace.${w.id}`)),
          { kind: 'separator' },
          ...WORKSPACES.filter((w) => w.status === 'soon').map((w) => commandItem(ctx, `workspace.${w.id}`)),
        ],
        mode.getBoundingClientRect(),
        { placement: 'below', owner: mode },
      ),
    );
    this.d.add(
      ctx.doc.settings.workspace.subscribe((id) => {
        const w = effectiveWorkspace(id);
        modeName.textContent = w.label;
        this.refit();
        modeIcon.replaceChildren(icon(w.icon, 14));
        mode.dataset.mode = w.id;
      }, true),
    );
    this.d.add(
      tooltip(mode, () => {
        const set = workspaceById(ctx.doc.settings.workspace.value);
        const w = effectiveWorkspace(set.id);
        const note = set.id !== w.id ? ` Proje “${set.label}” modunda kaydedilmiş; bu mod yakında geliyor, şimdilik Hibrit gösteriliyor.` : '';
        return { title: `Çalışma modu: ${w.label}`, description: `${w.description}${note} Proje ayarıdır; değiştirmek için tıklayın. Gizlenen komutlar komut satırından yine çalışır.` };
      }, 'top'),
    );

    const serverText = h('span', { class: 'status__server-text' });
    const server = h('button', { class: 'status__cell status__btn status__server', type: 'button' }, h('span', { class: 'status__lamp', 'aria-hidden': 'true' }), serverText);
    server.addEventListener('click', () => accountMenu(ctx, server));
    const save = saveCell(ctx, this.d);

    this.el = h('footer', { class: 'status' }, coords, flash, selCount, toggles, zoom, mode, crs, save, server, renderer);

    // Narrower windows (DESIGN.md §7.7): the least needed cell gives way first, until the message has
    // room. Every name stays in the cell's tooltip; the CRS is also in the title bar.
    const STEPS = ['renderer', 'crs', 'zoom', 'cloud', 'mode', 'toggles'] as const;
    const fit = fitBar(
      this.el,
      STEPS.length,
      (level) => STEPS.forEach((s, i) => this.el.toggleAttribute(`data-fit-${s}`, i < level)),
      // The message cell takes what is left: it should hold a short message (15 × its type size).
      () => flash.getBoundingClientRect().width >= 15 * parseFloat(getComputedStyle(flash).fontSize) && this.el.scrollWidth <= this.el.clientWidth,
    );
    this.d.add(fit.dispose);
    this.refit = () => fit.refit();
    // The cloud cells change their text with the connection and the project's sync.
    const cloudText = new MutationObserver(() => fit.refit());
    for (const cell of [save, server]) cloudText.observe(cell, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['hidden'] });
    this.d.add(() => cloudText.disconnect());
    this.d.add(ctx.prefs.uiFont.subscribe(() => fit.refit()));
    this.d.add(listen(document.fonts, 'loadingdone', () => fit.refit()));

    this.d.add(
      ctx.view.cursorWorld.subscribe((p) => {
        y.textContent = p ? ctx.format.coord(p.x) : '—';
        x.textContent = p ? ctx.format.coord(p.y) : '—';
      }, true),
    );
    this.d.add(ctx.view.camera.changed.subscribe(() => (zoom.textContent = `Ekran 1:${fmtScale(screenScale(ctx.view.camera.scale))}`), true));
    this.d.add(tooltip(zoom, () => ({ title: 'Ekran ölçeği', description: 'Görünümün 96 dpi ekrandaki yaklaşık ölçeği. Çizim ölçeği araç çubuğundan seçilir.' }), 'top'));
    this.d.add(
      ctx.doc.crs.subscribe((c) => {
        crs.querySelector('span')!.textContent = c.name;
        this.refit();
      }, true),
    );
    this.d.add(tooltip(crs, () => ({ title: 'Koordinat sistemi', description: `EPSG:${ctx.doc.crs.value.srid}. Y sağa, X yukarı değerdir. Değiştirmek için tıklayın.` }), 'top'));
    const syncRenderer = () => {
      const k = ctx.view.backendKind.value;
      rendererName.textContent = k === 'webgpu' ? 'WebGPU' : k === 'webgl2' ? 'WebGL2' : ctx.view.backendLabel.value;
      renderer.dataset.kind = k ?? '';
      renderer.toggleAttribute('data-error', !k && ctx.view.backendLabel.value !== 'Başlatılıyor…');
      this.refit();
    };
    syncRenderer();
    this.d.add(watchAll([ctx.view.backendKind, ctx.view.backendLabel], syncRenderer));
    this.d.add(
      tooltip(renderer, () => ({ title: 'Çizim motoru', description: `${ctx.view.backendLabel.value}. Değiştirmek için tıklayın; seçim hemen uygulanır ve hatırlanır.` }), 'top'),
    );
    this.d.add(
      ctx.server.state.subscribe((s) => {
        server.dataset.state = s;
        serverText.textContent = SERVER_TEXT[s];
        // An open tooltip was written for the previous answer.
        hideTooltip(server);
      }, true),
    );
    this.d.add(
      tooltip(server, () => {
        const s = ctx.server;
        const hl = s.health.value;
        const about = hl ? `${hl.service} ${hl.version}${hl.commit ? ` (${hl.commit.slice(0, 8)})` : ''}, sözleşme sürümü ${hl.contracts}.` : '';
        const offline = `${s.detail.value} Çizim sunucusuz çalışır; kayıt yerel .kcad dosyasına yapılır.${import.meta.env.DEV ? ' Geliştirmede sunucuyu “pnpm api” ile başlatın.' : ''}`;
        const text = s.state.value === 'online' ? about : s.state.value === 'incompatible' ? `${about} ${s.detail.value}` : s.state.value === 'checking' ? 'Sunucuya soruluyor…' : offline;
        return { title: 'KentOS sunucusu', description: `${text} Hesap, bulut projeleri ve bağlantı denetimi için tıklayın.` };
      }, 'top'),
    );
    this.d.add(
      ctx.selection.ids.subscribe((ids) => {
        selCount.hidden = ids.size === 0;
        selCount.textContent = `${ids.size} seçili`;
        this.refit();
      }, true),
    );
    this.d.add(
      ctx.log.entries.subscribe((list) => {
        const last = list.at(-1);
        if (last && last.level !== 'command' && !last.text.startsWith('  ')) this.flash(flash, last);
      }),
    );
  }

  private toggle(id: string, label: string): HTMLElement {
    const cmd = this.ctx.commands.get(id)!;
    const b = h('button', { class: 'status__toggle', type: 'button', 'aria-pressed': 'false' }, label);
    b.addEventListener('click', () => this.ctx.commands.execute(id));
    const sync = () => b.setAttribute('aria-pressed', String(!!cmd.isChecked?.()));
    sync();
    if (cmd.watch) this.d.add(watchAll(cmd.watch, sync));
    this.d.add(tooltip(b, () => ({ title: cmd.title, shortcut: this.ctx.keymap.chordFor(id), description: cmd.description }), 'top'));
    return b;
  }

  private flash(el: HTMLElement, e: LogEntry): void {
    clearTimeout(this.flashTimer);
    const ic = e.level === 'warn' ? 'warning' : e.level === 'error' ? 'error' : e.level === 'success' ? 'success' : 'info';
    replaceChildren(el, icon(ic, 14), h('span', null, e.text));
    el.dataset.level = e.level;
    el.dataset.show = '';
    this.flashTimer = window.setTimeout(() => delete el.dataset.show, e.level === 'error' || e.level === 'warn' ? 9000 : 5000);
  }
}
