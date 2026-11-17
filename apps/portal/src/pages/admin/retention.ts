// Kişisel verilerin saklanması (docs/adr/9000 §21), on the platform's settings page: the periods, the runs of the
// periodic destruction and a run by hand. What is destroyed and what is only archived is the server's rule.
import { platform } from '../../api/portal';
import type { RetentionPage, RetentionRunView } from '../../api/types';
import { h } from '../../lib/dom';
import { moment } from '../../lib/format';
import { form, formError, grid, textField, values } from '../../ui/forms';
import { badge, button, panel, table } from '../../ui/kit';
import { toast } from '../../ui/toast';

export function retentionPanel(): HTMLElement {
  const box = h('div', { class: 'stack' });
  const draw = (p: RetentionPage) => {
    const s = p.settings;
    const f = form(
      [
        grid(
          textField({ label: 'Kapanan dosya arşive', name: 'archiveAfterDays', type: 'number', value: String(s.archiveAfterDays), hint: 'gün sonra geçer (silinmez; idari kayıttır)' }),
          textField({ label: 'Gönderilmemiş taslak', name: 'draftKeepDays', type: 'number', value: String(s.draftKeepDays), hint: 'gün dokunulmazsa belgeleriyle imha edilir' }),
          textField({ label: 'Bildirimler', name: 'notificationKeepDays', type: 'number', value: String(s.notificationKeepDays), hint: 'günden eskisi imha edilir' }),
          textField({ label: 'Teknik kayıtlar', name: 'technicalKeepDays', type: 'number', value: String(s.technicalKeepDays), hint: 'gün: giden e-posta / SMS kayıtları, imza bırakmamış imza istekleri' }),
        ),
        formError(),
        h(
          'div',
          { class: 'form-actions' },
          button('Şimdi çalıştır', {
            kind: 'ghost',
            onClick: async () => {
              const run = await platform.runRetention();
              toast(`Periyodik imha çalıştı: ${run.archived} arşiv, ${run.draftsDeleted} taslak, ${run.notificationsDeleted} bildirim.`);
              draw(await platform.retention());
            },
          }),
          button('Kaydet', { type: 'submit' }),
        ),
      ],
      async (el) => {
        const v = values(el);
        const n = (k: string) => Number(v[k]);
        draw(await platform.setRetention({ archiveAfterDays: n('archiveAfterDays'), draftKeepDays: n('draftKeepDays'), notificationKeepDays: n('notificationKeepDays'), technicalKeepDays: n('technicalKeepDays') }));
        toast('Saklama süreleri kaydedildi.');
      },
    );
    box.replaceChildren(
      h('p', { class: 'panel-text' }, 'Sunucu periyodik imhayı her gün çalıştırır (KVKK Yönetmeliği m.11: iki periyodik imha arası en çok 6 ay). Her çalışma aşağıdaki kayda geçer; kayıtlar saklanır.'),
      f,
      table(
        [
          { label: 'Zaman', cell: (r: RetentionRunView) => h('span', { class: 'nowrap' }, moment(r.at)) },
          { label: 'Çalıştıran', cell: (r: RetentionRunView) => (r.trigger === 'schedule' ? badge('neutral', 'Günlük') : badge('info', r.actorName ?? 'Elle')) },
          {
            label: 'Yapılan',
            cell: (r: RetentionRunView) =>
              `${r.archived} arşiv · ${r.draftsDeleted} taslak (${r.filesDeleted} belge) · ${r.notificationsDeleted} bildirim · ${r.deliveriesDeleted + r.signingRequestsDeleted} teknik`,
          },
        ],
        p.runs,
        { empty: h('div', { class: 'muted' }, 'Henüz çalışmadı; sunucu açıldıktan bir saat sonra ilk kez çalışır.') },
      ),
    );
  };
  void platform.retention().then(draw).catch((e) => box.replaceChildren(h('div', { class: 'muted' }, e instanceof Error ? e.message : String(e))));
  return panel('Kişisel verilerin saklanması', box);
}
