import '../../styles/time.css';
import type { AppContext } from '../../app/context';
import { TIME_SPEEDS } from '../../app/timeSlider';
import { watchAll } from '../../core/signal';
import { TIME_UNITS, UNIT_WORD, type TimeUnit } from '../../model/time';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';

/**
 * Zaman sürgüsü's bar under the drawing (docs/adr/0210 §5, §10): Başa, Geri, Oynat/Durdur, İleri, Sona; what the
 * position shows; the slider; Adım (a number and a unit), Anlık | Aralık, Hız, Döngü; Kapat. On the slider ← and →
 * take a step, Home and End go to the ends, Boşluk plays and stops. The bar only shows and sets the session's slider
 * (app/timeSlider.ts); the drawing follows its window.
 */
export class TimeBar extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly when: HTMLElement;
  private readonly range: HTMLInputElement;
  private readonly ends: [HTMLElement, HTMLElement];
  private readonly play: HTMLButtonElement;
  private readonly count: HTMLInputElement;
  private readonly unit: Dropdown;
  private readonly kinds: [HTMLButtonElement, HTMLButtonElement];
  private readonly speed: Dropdown;
  private readonly loop: HTMLButtonElement;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const t = ctx.time;
    const button = (name: string, label: string, run: () => void) => {
      const b = h('button', { class: 'timebar__btn', type: 'button', title: label, 'aria-label': label }, icon(name, 16));
      b.addEventListener('click', run);
      return b;
    };
    this.play = button('play', 'Oynat (Boşluk)', () => (t.playing.value ? t.stop() : t.play()));
    this.when = h('span', { class: 'timebar__when', 'aria-live': 'polite' });
    this.range = h('input', { class: 'timebar__range', type: 'range', min: '0', step: '1', 'aria-label': 'Zaman' });
    this.range.addEventListener('input', () => {
      t.stop();
      t.go(Number(this.range.value));
    });
    this.range.addEventListener('keydown', (e) => {
      if (e.key !== ' ') return;
      e.preventDefault();
      if (t.playing.value) t.stop();
      else t.play();
    });
    this.ends = [h('span', { class: 'timebar__end' }), h('span', { class: 'timebar__end' })];
    this.count = h('input', { class: 'timebar__count', type: 'number', min: '1', max: '999', step: '1', 'aria-label': 'Adım sayısı' });
    this.count.addEventListener('change', () => {
      const n = Math.round(Number(this.count.value));
      if (!(n >= 1 && n <= 999)) {
        this.count.value = String(t.step.value.n);
        return ctx.log.warn('Adım 1 ile 999 birim arasında olmalı.');
      }
      this.setStep(n, t.step.value.unit);
    });
    this.unit = new Dropdown({
      ariaLabel: 'Adımın birimi',
      className: 'timebar__unit',
      items: () => TIME_UNITS.map((u) => ({ label: UNIT_WORD[u], radio: true, checked: t.step.value.unit === u, run: () => this.setStep(t.step.value.n, u) })),
    });
    const kind = (ranged: boolean, label: string, hint: string) => {
      const b = h('button', { class: 'seg__opt', type: 'button', role: 'radio', title: hint }, label);
      b.addEventListener('click', () => t.setRanged(ranged));
      return b;
    };
    this.kinds = [kind(false, 'Anlık', 'Konumun anında görünenler'), kind(true, 'Aralık', 'Konumdan bir sonrakine kadar görünenler')];
    this.speed = new Dropdown({
      ariaLabel: 'Oynatma hızı',
      className: 'timebar__speed',
      items: () => TIME_SPEEDS.map((v) => ({ label: `${String(v).replace('.', ',')} adım/sn`, radio: true, checked: t.speed.value === v, run: () => t.speed.set(v) })),
    });
    this.loop = button('loop', 'Döngü: sonda başa dön', () => t.loop.set(!t.loop.value));
    this.el = h(
      'div',
      { class: 'timebar', role: 'toolbar', 'aria-label': 'Zaman sürgüsü', hidden: true },
      h(
        'div',
        { class: 'timebar__nav' },
        button('timeFirst', 'Başa (Home)', () => this.jump(0)),
        button('timePrev', 'Geri (←)', () => this.jump(t.position.value - 1)),
        this.play,
        button('timeNext', 'İleri (→)', () => this.jump(t.position.value + 1)),
        button('timeLast', 'Sona (End)', () => this.jump(t.last.value)),
      ),
      this.when,
      h('div', { class: 'timebar__track' }, this.ends[0], this.range, this.ends[1]),
      h('label', { class: 'timebar__label' }, h('span', { class: 'timebar__word' }, 'Adım'), this.count),
      this.unit.el,
      h('div', { class: 'seg timebar__kind', role: 'radiogroup', 'aria-label': 'Pencere' }, ...this.kinds),
      h('span', { class: 'timebar__word' }, 'Hız'),
      this.speed.el,
      this.loop,
      button('close', 'Zaman sürgüsünü kapat', () => t.close()),
    );
    this.d.add(watchAll([t.open, t.window, t.step, t.last, t.position, t.playing, t.speed, t.loop, t.ranged], () => this.render()));
    this.render();
  }

  /** A position chosen by hand: playback stops there. */
  private jump(k: number): void {
    this.ctx.time.stop();
    this.ctx.time.go(k);
  }

  private setStep(n: number, unit: TimeUnit): void {
    const why = this.ctx.time.setStep({ n, unit });
    if (why) this.ctx.log.warn(why);
    this.render();
  }

  private render(): void {
    const t = this.ctx.time;
    this.el.hidden = !t.open.value;
    if (this.el.hidden) return;
    this.when.textContent = t.label();
    this.range.max = String(t.last.value);
    this.range.value = String(t.position.value);
    this.range.setAttribute('aria-valuetext', t.label());
    const unit = t.step.value.unit;
    this.ends[0].textContent = t.positionText(0);
    this.ends[1].textContent = t.positionText(t.last.value);
    replaceChildren(this.play, icon(t.playing.value ? 'pause' : 'play', 16));
    const playing = t.playing.value ? 'Durdur (Boşluk)' : 'Oynat (Boşluk)';
    this.play.title = playing;
    this.play.setAttribute('aria-label', playing);
    if (document.activeElement !== this.count) this.count.value = String(t.step.value.n);
    this.unit.set(UNIT_WORD[unit]);
    this.kinds.forEach((k, i) => {
      const on = (i === 1) === t.ranged.value;
      k.setAttribute('aria-checked', String(on));
      k.tabIndex = on ? 0 : -1;
    });
    this.speed.set(`${String(t.speed.value).replace('.', ',')}×`);
    this.loop.setAttribute('aria-pressed', String(t.loop.value));
  }
}
