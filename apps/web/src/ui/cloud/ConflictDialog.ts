import type { AppContext } from '../../app/context';
import type { SyncConflict } from '../../app/cloud/sync';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';

/**
 * Save conflicts (CLAUDE.md §15, §21.3): someone else saved the same
 * objects first. Nothing was overwritten and nothing more is sent until the
 * user chooses: take the server's copies (safe, the default) or save their
 * own over them. The drawing shows the user's own copies until then.
 */

const REASON: Record<SyncConflict['reason'], string> = {
  changed: 'başkası değiştirdi',
  remote: 'başkası değiştirdi',
  deleted: 'başkası sildi',
  exists: 'kimliği başkasında',
  project: 'proje bilgileri değişti',
};

const KIND: Record<string, string> = {
  point: 'Nokta', line: 'Çizgi', polyline: 'Çoklu çizgi', polygon: 'Alan', circle: 'Daire', arc: 'Yay', ellipse: 'Elips',
  spline: 'Eğri', xline: 'Yardımcı çizgi', ray: 'Işın', text: 'Yazı', dimension: 'Ölçü', hatch: 'Tarama',
};

export function openConflictDialog(ctx: AppContext): void {
  const sync = ctx.cloud.sync.value;
  const list = sync?.conflicts.value ?? [];
  if (!sync || !list.length) {
    ctx.log.info('Çözülecek bir çakışma yok.');
    return;
  }
  const describe = (c: SyncConflict) => {
    if (c.reason === 'project') return 'Proje bilgileri (katman ağacı, ayarlar, ad, stiller)';
    const e = c.localId !== null ? ctx.doc.get(c.localId) : undefined;
    const layer = e ? (ctx.doc.layers.get(e.layerId)?.name ?? e.layerId) : '';
    return `${e ? KIND[e.kind] ?? e.kind : 'Nesne'}${layer ? ` · ${layer}` : ''}${e?.label ? ` · ${e.label}` : ''}`;
  };
  const shown = list.slice(0, 12);
  const take = h('button', { class: 'btn btn--primary', type: 'button' }, 'Sunucudakini al');
  const mine = h('button', { class: 'btn', type: 'button' }, 'Benimkini kaydet');
  const later = h('button', { class: 'btn', type: 'button' }, 'Sonra');
  const dialog = new Dialog({
    title: 'Kayıt çakışması',
    width: 520,
    className: 'dialog--cloud',
    content: [
      h('p', null, `${list.length} değişikliğiniz kaydedilmedi: aynı nesneleri başka biri daha önce kaydetti. Hiçbir şeyin üzerine yazılmadı; seçene kadar değişiklikleriniz yalnız bu cihazda.`),
      h(
        'ul',
        { class: 'cloud-conflicts' },
        shown.map((c) => h('li', null, h('span', null, describe(c)), h('span', { class: 'cloud-row__meta' }, REASON[c.reason]))),
        list.length > shown.length ? h('li', { class: 'cloud-row__meta' }, `… ve ${list.length - shown.length} nesne daha`) : null,
      ),
      h('p', { class: 'cloud-hint' }, '“Sunucudakini al” sizin değişikliklerinizi bırakır. “Benimkini kaydet” başkasının değişikliğinin üzerine sizinkini yazar.'),
    ],
    footer: [later, h('div', { class: 'dialog__foot-spacer' }), mine, take],
  });
  const choose = async (choice: 'server' | 'mine') => {
    take.disabled = mine.disabled = true;
    await sync.resolve(choice);
    dialog.close();
    ctx.log.success(choice === 'server' ? 'Sunucudaki hâller alındı.' : 'Sizin değişiklikleriniz kaydediliyor.');
  };
  take.addEventListener('click', () => void choose('server'));
  mine.addEventListener('click', () => void choose('mine'));
  later.addEventListener('click', () => dialog.close());
}
