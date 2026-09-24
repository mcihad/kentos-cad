import type { AppContext } from '../../app/context';
import { Component } from '../Component';
import { h } from '../dom';
import { commandButton } from '../widgets/CommandButton';
import { colorField, layerField, lineTypeField, scaleField, weightField } from './fields';

/** Toolbar = groups of command buttons + current-property fields. */
export class Toolbar extends Component {
  readonly el: HTMLElement;

  constructor(ctx: AppContext) {
    super();
    const btn = (id: string) => commandButton(ctx, id, this.d);
    const group = (label: string, ...children: HTMLElement[]) => h('div', { class: 'toolbar__group', role: 'group', 'aria-label': label }, ...children);

    this.el = h(
      'div',
      { class: 'toolbar', role: 'toolbar', 'aria-label': 'Araç çubuğu' },
      group('Dosya', btn('file.new'), btn('file.open'), btn('file.save')),
      group('Geçmiş', btn('edit.undo'), btn('edit.redo')),
      group('Görünüm', btn('view.zoomExtents'), btn('tool.zoomWindow'), btn('view.zoomSelection'), btn('view.zoomIn'), btn('view.zoomOut'), btn('tool.pan')),
      group('Geçerli özellikler', layerField(ctx, this.d), colorField(ctx, this.d), lineTypeField(ctx, this.d), weightField(ctx, this.d)),
      h('div', { class: 'toolbar__spacer' }),
      group('Çizim ölçeği', scaleField(ctx, this.d)),
      group('Paneller', btn('view.toolbox'), btn('view.bottomPanel'), btn('view.rightPanel')),
    );
  }
}
