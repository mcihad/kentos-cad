import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { overlapMenu } from './overlapMenu';
import { selectFilterMenu } from './selectFilterMenu';
import { snapMenu } from './snapMenu';
import { screenScale, snapInRange } from '../../viewport/snapRange';
import { effectiveWorkspace, WORKSPACES, workspaceById } from '../../app/workspaces';
import type { LogEntry } from '../../app/state';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { webgpuSupported } from '../../render/webgpu/support';
import { fitBar } from '../widgets/fit';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import { ICON_SIZE, flashOf } from '../bottom/logPlan';
import { SERVER_TEXT, serverTip } from './cellsPlan';
import { accountMenu, saveCell } from './cloudCells';
import { scaleText } from '../../model/newProjectWizard';
import { offeredScales, typedScale } from './scaleSelector';
import { projectCrsName, projectCrsTitle } from '../../model/projectCrs';
import { cursorUnreached, SecondCrs } from '../../model/secondCrs';
import { crsMenu, secondMenu } from './secondMenu';

const fmtScale = (n: number) => n.toLocaleString('tr-TR');

export class StatusBar extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private flashTimer = 0;
  /** Fits the bar again after a cell's text changed (set once the bar is built). */
  private refit = () => {};
  /** Shows the second system's values, or leaves their room to a message (set once the bar is built). */
  private fitSecond = () => {};

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const y = h('span', { class: 'status__value num' });
    const x = h('span', { class: 'status__value num' });
    // East and north as the project's type names them (docs/adr/0165 §4).
    const east = h('span', { class: 'status__axis' }, 'Y');
    const north = h('span', { class: 'status__axis' }, 'X');
    const coords = h('div', { class: 'status__coords', 'aria-live': 'off' }, east, y, north, x);
    // The second coordinate system's values (docs/adr/0167 §2): its name, then its two values with their names.
    const secondName = h('span', { class: 'status__second-name' });
    const secondA = h('span', { class: 'status__axis' });
    const secondAValue = h('span', { class: 'status__value num' });
    const secondB = h('span', { class: 'status__axis' });
    const secondBValue = h('span', { class: 'status__value num' });
    const second = h(
      'button',
      { class: 'status__cell status__btn status__second', type: 'button', 'aria-haspopup': 'menu', hidden: true },
      secondName,
      secondA,
      secondAValue,
      secondB,
      secondBValue,
    );
    second.addEventListener('click', () => PopupMenu.open(secondMenu(ctx), second.getBoundingClientRect(), { placement: 'below', owner: second }));
    let secondCrs: SecondCrs | null = null;
    const flash = h('div', { class: 'status__flash', 'aria-live': 'polite' });
    const selCount = h('span', { class: 'status__cell status__sel num' });

    const toggles = h(
      'div',
      { class: 'status__toggles', role: 'group', 'aria-label': 'Çizim yardımcıları' },
      // The snap kinds, Çizilmekte olan nesneye and the Karelaj spacing are on the cell's right-click menu (docs/adr/0163 §6).
      this.snapCell(this.toggle('draft.snap', 'Kenet', () => snapMenu(ctx), () => this.snapNote())),
      this.toggle('draft.grid', 'Izgara'),
      this.toggle('draft.ortho', 'Orto'),
      this.toggle('draft.rightAngle', 'Dik açı'),
      this.toggle('draft.polar', 'Kutupsal'),
      this.toggle('draft.tracking', 'İzleme'),
      // Topolojik düzenleme (docs/adr/0160 §1): its option, Noktalar da, is on the cell's right-click menu.
      this.toggle('draft.topology', 'Topoloji', () => [commandItem(ctx, 'draft.topologyPoints', { label: 'Noktalar da' })]),
      // Çakışma denetimi (docs/adr/0162 §1): its modes and Seçili katmanlarda önle's layers are on the cell's right-click menu.
      this.toggle('draft.overlap', 'Çakışma', () => overlapMenu(ctx)),
      // Seçim süzgeci (docs/adr/0187 §5): its kinds are on the cell's right-click menu.
      this.toggle('edit.selectFilter', 'Süzgeç', () => selectFilterMenu(ctx)),
      this.toggle('view.lineWeights', 'Kalınlık'),
    );

    const crs = h('button', { class: 'status__cell status__btn status__crs', type: 'button' }, icon('crs', 14), h('span'));
    crs.addEventListener('click', () => ctx.commands.execute('crs.set'));
    // The second system on the cell's right-click menu (docs/adr/0167 §1).
    crs.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      PopupMenu.open(crsMenu(ctx), { x: e.clientX, y: e.clientY }, { placement: 'point' });
    });
    // The scale selector (docs/adr/0165 §5): the view's screen scale, chosen from the type's scales or typed.
    const zoomText = h('span', { class: 'status__zoom-text num' });
    const zoom = h('button', { class: 'status__cell status__btn status__zoom', type: 'button', 'aria-haspopup': 'menu' }, zoomText, icon('chevronUp', 12));
    const scaleField = h('input', { class: 'field field--inline status__scale-field num', type: 'text', inputmode: 'numeric', 'aria-label': 'Ekran ölçeği (1:N)' });
    // “Ekran 1:” and the number, as the cell reads.
    const scaleEdit = h('span', { class: 'status__cell status__scale-edit', hidden: true }, h('span', null, 'Ekran 1:'), scaleField);
    const closeField = () => {
      scaleEdit.hidden = true;
      zoom.hidden = false;
    };
    const typeScale = () => {
      zoom.hidden = true;
      scaleEdit.hidden = false;
      scaleField.value = String(screenScale(ctx.view.camera.scale));
      scaleField.focus();
      scaleField.select();
    };
    scaleField.addEventListener('keydown', (e) => {
      e.stopPropagation();
      if (e.key === 'Escape') return void (e.preventDefault(), closeField(), ctx.view.focus());
      if (e.key !== 'Enter') return;
      e.preventDefault();
      const n = typedScale(scaleField.value);
      if (n === null) ctx.log.warn(`“${scaleField.value}” bir ölçek değil. 1:N biçiminde bir tam sayı yazın, ör. 1:500.`);
      else ctx.view.zoomToScale(n);
      closeField();
      ctx.view.focus();
    });
    scaleField.addEventListener('blur', closeField);
    zoom.addEventListener('click', () =>
      PopupMenu.open(
        [
          { kind: 'header', label: 'Ekran ölçeği' },
          ...offeredScales(ctx.format.axes).map((n): MenuItem => ({ label: scaleText(n), run: () => ctx.view.zoomToScale(n) })),
          { kind: 'separator' },
          { label: 'Ölçek yaz…', run: typeScale },
        ],
        zoom.getBoundingClientRect(),
        { placement: 'below', owner: zoom },
      ),
    );
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

    // The project's type: its scene, axes and ribbon (app/workspaces.ts, docs/adr/0165).
    const modeName = h('span', { class: 'status__mode-name' });
    const modeIcon = h('span', { class: 'status__mode-icon' });
    const mode = h('button', { class: 'status__cell status__btn status__mode', type: 'button', 'aria-haspopup': 'menu' }, modeIcon, modeName);
    mode.addEventListener('click', () =>
      PopupMenu.open(
        [
          { kind: 'header', label: 'Proje türü' },
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
        const id = ctx.doc.settings.workspace.value;
        const w = effectiveWorkspace(id);
        const note =
          id === null
            ? ' Projenin türü henüz seçilmedi; CBS olarak gösteriliyor.'
            : id !== w.id
              ? ` Proje “${workspaceById(id).label}” türünde kaydedilmiş; bu tür yakında geliyor, şimdilik CBS olarak gösteriliyor.`
              : '';
        return { title: `Proje türü: ${w.label}`, description: `${w.description}${note} Proje ayarıdır; değiştirmek için tıklayın. Şeridinde olmayan komutlar komut satırından yine çalışır.` };
      }, 'top'),
    );

    const serverText = h('span', { class: 'status__server-text' });
    const server = h('button', { class: 'status__cell status__btn status__server', type: 'button' }, h('span', { class: 'status__lamp', 'aria-hidden': 'true' }), serverText);
    server.addEventListener('click', () => accountMenu(ctx, server));
    const save = saveCell(ctx, this.d);

    // The second system's values and the message share the room after the coordinates (docs/adr/0167 §2): the values
    // show while there is no message, or there is room for both; a message too long for both takes it for its seconds.
    const slot = h('div', { class: 'status__slot' }, second, flash);
    this.el = h('footer', { class: 'status' }, coords, slot, selCount, toggles, zoom, scaleEdit, mode, crs, save, server, renderer);

    // A short message's room: 15 times its type size.
    const least = () => 15 * parseFloat(getComputedStyle(flash).fontSize);
    // The second system's cell as wide as it is with the drawing's origin's values: it does not change as the cursor
    // comes and goes. The cells give way until the room after the coordinates holds it; it shows while no message
    // does, or there is room for both (the steps do not follow the message: the bar stays still).
    let secondWidth = 0;
    this.fitSecond = () => {
      second.hidden = !secondCrs || slot.getBoundingClientRect().width < secondWidth + (flash.hasAttribute('data-show') ? least() : 0);
    };
    const room = new ResizeObserver(() => this.fitSecond());
    room.observe(slot);
    this.d.add(() => room.disconnect());
    // A message comes and goes: the values give it their room, then take it back.
    const shown = new MutationObserver(() => this.fitSecond());
    shown.observe(flash, { attributes: true, attributeFilter: ['data-show'] });
    this.d.add(() => shown.disconnect());

    // Narrower windows (DESIGN.md §7.7): the least needed cell gives way first, until the message has
    // room. Every name stays in the cell's tooltip; the CRS is also in the title bar.
    const STEPS = ['renderer', 'crs', 'zoom', 'cloud', 'mode', 'toggles'] as const;
    const fit = fitBar(
      this.el,
      STEPS.length,
      (level) => STEPS.forEach((s, i) => this.el.toggleAttribute(`data-fit-${s}`, i < level)),
      // The message cell takes what is left: it should hold a short message (15 × its type size).
      () => slot.getBoundingClientRect().width >= Math.max(least(), secondWidth) && this.el.scrollWidth <= this.el.clientWidth,
    );
    this.d.add(fit.dispose);
    this.refit = () => fit.refit();
    // The cloud cells change their text with the connection and the project's sync.
    const cloudText = new MutationObserver(() => fit.refit());
    for (const cell of [save, server]) cloudText.observe(cell, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['hidden'] });
    this.d.add(() => cloudText.disconnect());
    this.d.add(ctx.prefs.uiFont.subscribe(() => fit.refit()));
    this.d.add(listen(document.fonts, 'loadingdone', () => fit.refit()));

    // The second system, taken again when the project's settings change; its values follow the cursor.
    const showSecond = (p = ctx.view.cursorWorld.value) => {
      const moved = p && secondCrs ? secondCrs.point(p) : null;
      const t = moved && !('error' in moved) ? moved : null;
      if (!secondCrs) return;
      const values = t ? secondCrs.values(t.point, ctx.format, ctx.prefs.geographic.value) : null;
      const named = !secondCrs.geographic;
      secondA.textContent = named ? (values?.[0][0] ?? ctx.format.eastLabel) : '';
      secondB.textContent = named ? (values?.[1][0] ?? ctx.format.northLabel) : '';
      secondAValue.textContent = values?.[0][1] ?? '—';
      secondBValue.textContent = values?.[1][1] ?? '—';
    };
    const takeSecond = () => {
      secondCrs = SecondCrs.of(ctx.doc.settings);
      secondName.textContent = secondCrs?.short ?? '';
      secondWidth = 0;
      if (secondCrs) {
        showSecond(ctx.doc.origin);
        second.hidden = false;
        secondWidth = second.getBoundingClientRect().width;
      }
      showSecond();
      fit.refit();
      this.fitSecond();
    };
    // The project's grids read into the core change its values too (docs/adr/0168 §4).
    this.d.add(watchAll([ctx.doc.settings.changed, ctx.prefs.geographic, ctx.prefs.uiFont, ctx.prefs.textSize, ctx.grids.revision], takeSecond));
    this.d.add(listen(document.fonts, 'loadingdone', takeSecond));
    takeSecond();
    this.d.add(
      tooltip(
        second,
        () => {
          if (!secondCrs) return { title: 'İkinci koordinat sistemi' };
          const p = ctx.view.cursorWorld.value;
          const t = p ? secondCrs.point(p) : null;
          const sure = !t ? '' : 'error' in t ? cursorUnreached(t.error) : `${secondCrs.accuracy(t)}.`;
          return {
            title: `İkinci koordinat sistemi: ${secondCrs.name}`,
            description: `${secondCrs.title} değerleri, projeninkilerden dönüştürülerek. ${sure} Sistemi değiştirmek ya da kaldırmak için tıklayın.`,
          };
        },
        'top',
      ),
    );

    this.d.add(
      ctx.view.cursorWorld.subscribe((p) => {
        y.textContent = p ? ctx.format.coord(p.x) : '—';
        x.textContent = p ? ctx.format.coord(p.y) : '—';
        showSecond();
      }, true),
    );
    this.d.add(
      ctx.doc.settings.workspace.subscribe(() => {
        east.textContent = ctx.format.eastLabel;
        north.textContent = ctx.format.northLabel;
      }, true),
    );
    this.d.add(ctx.view.camera.changed.subscribe(() => (zoomText.textContent = `Ekran 1:${fmtScale(screenScale(ctx.view.camera.scale))}`), true));
    this.d.add(tooltip(zoom, () => ({ title: 'Ekran ölçeği', description: 'Görünümün 96 dpi ekrandaki yaklaşık ölçeği; tıklayın, listeden seçin ya da yazın. Çizim ölçeği şeritten seçilir.' }), 'top'));
    // The registry's system or the project's own definition (docs/adr/0168 §1).
    const showCrs = () => {
      crs.querySelector('span')!.textContent = projectCrsName(ctx.doc.settings);
      this.refit();
    };
    this.d.add(watchAll([ctx.doc.crs, ctx.doc.settings.customCrs], showCrs));
    showCrs();
    this.d.add(
      tooltip(
        crs,
        () => ({
          title: 'Koordinat sistemi',
          description: `${projectCrsTitle(ctx.doc.settings)}. ${ctx.format.eastLabel} sağa, ${ctx.format.northLabel} yukarı değerdir. Değiştirmek için tıklayın; ikinci sistem sağ tık menüsünde.`,
        }),
        'top',
      ),
    );
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
    this.d.add(tooltip(server, () => serverTip({ state: ctx.server.state.value, health: ctx.server.health.value, detail: ctx.server.detail.value, dev: import.meta.env.DEV }), 'top'));
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
        if (last) this.flash(flash, last);
      }),
    );
  }

  /** The view's screen scale when it lies out of the snap's range (docs/adr/0163 §5), else null. */
  private snapOut(): number | null {
    const { ctx } = this;
    const n = screenScale(ctx.view.camera.scale);
    return snapInRange(n, ctx.prefs.snapScaleMin.value, ctx.prefs.snapScaleMax.value) ? null : n;
  }

  /** What Kenet's tooltip says first while the view is out of the snap's scale range. */
  private snapNote(): string | null {
    const n = this.snapOut();
    return n === null ? null : `Ölçek aralığının dışında (1:${fmtScale(n)}): kenet bu ölçekte çalışmaz.`;
  }

  /** Kenet out of its scale range (docs/adr/0163 §5): still on, its lamp only outlined and its name dim. */
  private snapCell(b: HTMLElement): HTMLElement {
    const { ctx } = this;
    const sync = () => b.toggleAttribute('data-out', this.snapOut() !== null);
    this.d.add(ctx.view.camera.changed.subscribe(sync, true));
    this.d.add(watchAll([ctx.prefs.snapScaleMin, ctx.prefs.snapScaleMax], sync));
    return b;
  }

  /** A drafting aid's cell; `options`, its right-click menu; `note`, what its tooltip says first when there is something. */
  private toggle(id: string, label: string, options?: () => MenuItem[], note?: () => string | null): HTMLElement {
    const cmd = this.ctx.commands.get(id)!;
    const b = h('button', { class: 'status__toggle', type: 'button', 'aria-pressed': 'false', 'data-command': id }, label);
    b.addEventListener('click', () => this.ctx.commands.execute(id));
    if (options)
      b.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        // Given as a function: a row that stays reads the rows again (the selection filter's and the snap kinds).
        PopupMenu.open(options, { x: e.clientX, y: e.clientY }, { placement: 'point' });
      });
    const sync = () => b.setAttribute('aria-pressed', String(!!cmd.isChecked?.()));
    sync();
    if (cmd.watch) this.d.add(watchAll(cmd.watch, sync));
    this.d.add(
      tooltip(
        b,
        () => {
          const first = note?.();
          return { title: cmd.title, shortcut: this.ctx.keymap.chordFor(id), description: first ? [first, cmd.description].filter(Boolean).join(' ') : cmd.description };
        },
        'top',
      ),
    );
    return b;
  }

  /** The newest line in the message cell, if the status bar shows it (logPlan.ts `flashOf`), for as long as it says. */
  private flash(el: HTMLElement, e: LogEntry): void {
    const shown = flashOf(e.level, e.text);
    if (!shown) return;
    clearTimeout(this.flashTimer);
    replaceChildren(el, icon(shown.icon, ICON_SIZE), h('span', null, e.text));
    el.dataset.level = e.level;
    el.dataset.show = '';
    this.flashTimer = window.setTimeout(() => delete el.dataset.show, shown.ms);
  }
}
