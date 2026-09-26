import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import { listen } from '../../core/disposable';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { commandButton } from '../widgets/CommandButton';
import { fitBar } from '../widgets/fit';
import { PopupMenu } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { colorField, layerField, lineTypeField, propertiesField, scaleField, weightField } from './fields';

/** Görünüm commands that fold under ⋯ first: the wheel and the middle button do them too. */
const VIEW_MORE = ['view.zoomIn', 'view.zoomOut', 'tool.pan'];

/** Field widths in CSS px at the standard type scale: full, then narrowed. The scale field keeps its width: “1:1000” must show whole. */
const WIDTHS = { layer: [196, 150], color: [150, 124], lineType: [150, 124], weight: [160, 132] } as const;

/**
 * Toolbar = groups of command buttons + current-property fields.
 *
 * It fits its width in steps (DESIGN.md §7.3), so nothing is cut off at the
 * shell's 1100 px, on any type scale; the first step that fits is kept:
 *   1. the layer, colour, type and weight fields narrow (their values
 *      shorten with an ellipsis);
 *   2. Yakınlaştır, Uzaklaştır and Kaydır also go under the Görünüm
 *      group's ⋯;
 *   3. Renk, Tip and Kalınlık fold into one “Özellikler” field whose menu
 *      holds the three lists; the layer field gets its width back;
 *   4. the layer field narrows again (the largest type scales).
 */
export class Toolbar extends Component {
  readonly el: HTMLElement;

  constructor(ctx: AppContext) {
    super();
    const btn = (id: string) => commandButton(ctx, id, this.d);
    const group = (label: string, ...children: HTMLElement[]) => h('div', { class: 'toolbar__group', role: 'group', 'aria-label': label }, ...children);

    const folded = VIEW_MORE.map(btn);
    const more = h('button', { class: 'tbtn toolbar__more', type: 'button', 'aria-label': 'Diğer görünüm komutları', 'aria-haspopup': 'menu', hidden: true }, icon('more', 18));
    this.d.add(listen(more, 'click', () => PopupMenu.open(VIEW_MORE.map((id) => commandItem(ctx, id)), more.getBoundingClientRect(), { owner: more, minWidth: 220 })));
    this.d.add(tooltip(more, () => ({ title: 'Diğer görünüm komutları', description: 'Yakınlaştır, Uzaklaştır ve Kaydır: pencere daralınca buraya girer.' }), 'bottom'));

    const layer = layerField(ctx, this.d, { width: WIDTHS.layer[0] });
    const props = [colorField(ctx, this.d, { width: WIDTHS.color[0] }), lineTypeField(ctx, this.d, { width: WIDTHS.lineType[0] }), weightField(ctx, this.d, { width: WIDTHS.weight[0] })];
    const fold = propertiesField(ctx, this.d);
    fold.hidden = true;
    const scale = scaleField(ctx, this.d);

    this.el = h(
      'div',
      { class: 'toolbar', role: 'toolbar', 'aria-label': 'Araç çubuğu' },
      group('Dosya', btn('file.new'), btn('file.open'), btn('file.save')),
      group('Geçmiş', btn('edit.undo'), btn('edit.redo')),
      group('Görünüm', btn('view.zoomExtents'), btn('tool.zoomWindow'), btn('view.zoomSelection'), ...folded, more),
      group('Geçerli özellikler', layer, ...props, fold),
      h('div', { class: 'toolbar__spacer' }),
      group('Çizim ölçeği', scale),
      group('Paneller', btn('view.toolbox'), btn('view.bottomPanel'), btn('view.rightPanel')),
    );

    const width = (el: HTMLElement, w: readonly [number, number], narrow: boolean) => (el.style.width = `calc(${narrow ? w[1] : w[0]}px * var(--ui-scale))`);
    const fit = fitBar(
      this.el,
      4,
      (level) => {
        width(layer, WIDTHS.layer, level === 1 || level === 2 || level === 4);
        props.forEach((f, i) => width(f, [WIDTHS.color, WIDTHS.lineType, WIDTHS.weight][i], level >= 1));
        folded.forEach((b) => (b.hidden = level >= 2));
        more.hidden = level < 2;
        props.forEach((f) => (f.hidden = level >= 3));
        fold.hidden = level < 3;
        this.el.dataset.fit = String(level);
      },
      () => this.el.scrollWidth <= this.el.clientWidth,
    );
    this.d.add(fit.dispose);
    // Another typeface changes the fields' and buttons' widths without resizing the bar.
    this.d.add(ctx.prefs.uiFont.subscribe(() => fit.refit()));
    this.d.add(listen(document.fonts, 'loadingdone', () => fit.refit()));
  }
}
