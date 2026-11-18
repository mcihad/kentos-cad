// The holidays the legal deadlines are counted over (docs/adr/9000 §19), on the platform's settings page: a year's
// days, the law's fixed ones and the religious ones the administrators add when their dates are announced.
import { holidays as api } from '../../api/portal';
import type { HolidayView } from '../../api/types';
import { h } from '../../lib/dom';
import { day } from '../../lib/format';
import { checkbox, checked, form, formError, grid, textField, values } from '../../ui/forms';
import { badge, button, panel, table } from '../../ui/kit';
import { toast } from '../../ui/toast';

export function holidaysPanel(): HTMLElement {
  const box = h('div', { class: 'stack' });
  let year = new Date().getFullYear();
  const yearLabel = h('strong', { class: 'num' }, String(year));
  const draw = async () => {
    yearLabel.textContent = String(year);
    const list = await api.list(year);
    const religious = list.filter((d) => !d.fixed).length;
    box.replaceChildren(
      religious
        ? h('div', { class: 'field-hint' }, `${year} yılında ${religious} dinî bayram günü yazılı; arife yarım gündür ve süre için iş günü sayılır.`)
        : h('div', { class: 'notice tone-warning' }, h('div', { class: 'notice-text' }, `${year} yılının dinî bayramları yazılmamış: o yıla denk gelen süreler bayramda uzamaz. Diyanet'in açıkladığı günleri ekleyin.`)),
      table(
        [
          { label: 'Gün', cell: (d: HolidayView) => h('span', { class: 'num' }, day(d.day)) },
          { label: 'Ad', cell: (d: HolidayView) => d.name },
          { label: '', class: 'narrow', cell: (d: HolidayView) => (d.fixed ? badge('neutral', '2429 sayılı Kanun') : d.halfDay ? badge('info', 'Yarım gün') : badge('success', 'Tam gün')) },
          {
            label: '',
            class: 'narrow',
            cell: (d: HolidayView) =>
              d.fixed
                ? null
                : button('Kaldır', {
                    kind: 'ghost',
                    icon: 'trash',
                    onClick: async () => {
                      await api.remove(d.day);
                      toast('Tatil kaldırıldı.');
                      void draw();
                    },
                  }),
          },
        ],
        list,
      ),
    );
  };
  const add = form(
    [
      grid(
        textField({ label: 'Gün', name: 'day', type: 'date', required: true }),
        textField({ label: 'Adı', name: 'name', required: true, placeholder: 'Ramazan Bayramı 1. gün' }),
      ),
      checkbox('Yarım gün (arife): süre için iş günü sayılır', 'halfDay', false),
      formError(),
      h('div', { class: 'form-actions' }, button('Ekle', { type: 'submit', icon: 'plus' })),
    ],
    async (el) => {
      const v = values(el);
      await api.add({ day: v.day, name: v.name, halfDay: checked(el, 'halfDay') });
      toast('Tatil eklendi; süreler yeniden hesaplanır.');
      el.reset();
      year = Number(v.day.slice(0, 4)) || year;
      void draw();
    },
  );
  const step = (by: number) => () => {
    year += by;
    void draw();
  };
  void draw();
  return panel(
    'Tatil takvimi',
    h(
      'div',
      { class: 'stack' },
      h('p', { class: 'panel-text' }, 'Yasal sürenin son günü hafta sonuna ya da tatile denk gelirse süre ilk iş gününe uzar (2577 sayılı İYUK m.8). Sabit tatiller 2429 sayılı Kanun’dandır; dinî bayramlar her yıl buraya yazılır. İdari izin tatil değildir, süreyi uzatmaz.'),
      h('div', { class: 'toolbar' }, button('Önceki yıl', { kind: 'ghost', onClick: step(-1) }), yearLabel, button('Sonraki yıl', { kind: 'ghost', onClick: step(1) })),
      box,
      add,
    ),
  );
}
