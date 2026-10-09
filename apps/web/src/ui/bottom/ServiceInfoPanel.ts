import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { serviceInfo } from './serviceInfoRun';

/**
 * Servis bilgisi, the bottom panel's tab of what the services said about a point (docs/adr/0208 §11; the desktop's
 * `services/info.rs`): the point asked about, then each WMS and ArcGIS layer shown with its records, field and value,
 * or why it gave none. The asking is serviceInfoRun.ts's.
 */
export class ServiceInfoPanel extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.el = h('div', { class: 'svc-info' });
    this.d.add(watchAll([serviceInfo.at, serviceInfo.busy, serviceInfo.answers, ctx.format.changed], () => this.render()));
    this.render();
  }

  private render(): void {
    const at = serviceInfo.at.value;
    if (!at) {
      replaceChildren(
        this.el,
        h(
          'div',
          { class: 'empty empty--inline' },
          'Harita › Altlık › Servis bilgisi ile çizimde bir noktaya tıklayın: görünen WMS ve ArcGIS katmanlarının o noktadaki kayıtları burada listelenir.',
        ),
      );
      return;
    }
    const parts: HTMLElement[] = [h('div', { class: 'svc-info__at' }, `Nokta: ${this.ctx.format.point(at)}`)];
    if (serviceInfo.busy.value) parts.push(h('div', { class: 'svc-info__muted' }, 'Servislere soruluyor…'));
    for (const a of serviceInfo.answers.value) {
      const block: HTMLElement[] = [h('div', { class: 'svc-info__layer' }, a.layer)];
      if ('error' in a.rows) block.push(h('div', { class: 'svc-info__muted' }, a.rows.error));
      else if (!a.rows.length) block.push(h('div', { class: 'svc-info__muted' }, 'Bu noktada kayıt yok.'));
      else
        for (const r of a.rows) {
          if (r.layer) block.push(h('div', { class: 'svc-info__record' }, r.layer));
          block.push(
            h(
              'dl',
              { class: 'svc-info__fields' },
              r.fields.flatMap(([k, v]) => [h('dt', { title: k }, k), h('dd', null, v)]),
            ),
          );
        }
      parts.push(h('section', { class: 'svc-info__block' }, block));
    }
    replaceChildren(this.el, ...parts);
  }
}
