import '../../styles/start.css';
import type { AppContext } from '../../app/context';
import { watchAll } from '../../core/signal';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { brandMark } from '../shell/brandButton';
import { Dialog } from '../widgets/Dialog';
import { toggleSwitch } from '../widgets/controls';
import { recentFileRow } from './recentList';

/**
 * The start screen (Başlangıç): what to do first, as AutoCAD's and
 * Netcad's start pages offer it: a new project, a drawing file, a cloud
 * project, the recent files, or on with the drawing already on screen (the
 * sample project at first). Opens with the app (Uygulama ayarları →
 * Görünüm → Açılış) and from Dosya → Başlangıç ekranı. Loaded on first use.
 */
export function openStartScreen(ctx: AppContext): void {
  let dialog: Dialog | null = null;
  const go = (id: string) => {
    dialog?.close();
    ctx.commands.execute(id);
  };
  const action = (id: string, ic: string, title: string, sub: string) => {
    const b = h(
      'button',
      { class: 'start__action', type: 'button', dataset: { command: id } },
      h('span', { class: 'start__action-icon' }, icon(ic, 24)),
      h('span', { class: 'start__action-text' }, h('span', { class: 'start__action-title' }, title), h('span', { class: 'start__action-sub' }, sub)),
      h('kbd', { class: 'kbd start__kbd' }, ctx.keymap.chordFor(id) ?? ''),
    );
    b.addEventListener('click', () => go(id));
    return b;
  };
  const cloud = action('cloud.open', 'cloud', 'Bulut projesi aç', '');
  const cloudSub = cloud.querySelector<HTMLElement>('.start__action-sub')!;
  const keep = h(
    'button',
    { class: 'start__action start__action--quiet', type: 'button', dataset: { start: 'keep' } },
    h('span', { class: 'start__action-icon' }, icon('play', 24)),
    h('span', { class: 'start__action-text' }, h('span', { class: 'start__action-title' }, 'Çizime devam et'), h('span', { class: 'start__action-sub' })),
  );
  const keepSub = keep.querySelector<HTMLElement>('.start__action-sub')!;
  keep.addEventListener('click', () => dialog?.close());

  const list = h('div', { class: 'start__recent', role: 'list', 'aria-label': 'Son dosyalar' });
  const showRecent = () =>
    replaceChildren(
      list,
      ctx.files.recent.list.value.length
        ? ctx.files.recent.list.value.map((f) => recentFileRow(ctx, f, () => dialog?.close()))
        : h('p', { class: 'start__empty' }, 'Henüz dosya yok. Açtığınız ya da kaydettiğiniz çizimler burada görünür; tek tıkla yeniden açılır.'),
    );
  const sync = () => {
    const online = ctx.server.state.value === 'online';
    cloud.toggleAttribute('disabled', !ctx.commands.isEnabled('cloud.open'));
    cloudSub.textContent = online ? (ctx.cloud.me.value ? `${ctx.cloud.me.value.user.displayName} olarak giriş yapıldı` : 'Kurumunuzun projeleri; önce giriş yapılır') : 'Sunucuya ulaşılamıyor';
    keepSub.textContent = `${ctx.doc.name.value} · ${ctx.doc.size.toLocaleString('tr-TR')} nesne`;
  };

  let stop: (() => void) | null = null;
  const shown = toggleSwitch({ label: 'Açılışta göster', checked: ctx.prefs.startScreen.value, onChange: (v) => ctx.prefs.startScreen.set(v) });
  dialog = new Dialog({
    title: 'Başlangıç',
    className: 'start',
    width: 880,
    onClose: () => stop?.(),
    content: [
      h(
        'div',
        { class: 'start__grid' },
        h(
          'section',
          { class: 'start__main' },
          h(
            'div',
            { class: 'start__brand' },
            brandMark(44),
            h('div', null, h('div', { class: 'start__product' }, 'KentOS ', h('span', null, 'CAD')), h('div', { class: 'start__tagline' }, 'Harita, kadastro ve kent bilgi sistemi çizimi')),
          ),
          h(
            'div',
            { class: 'start__actions' },
            action('file.new', 'fileNew', 'Yeni proje', 'Çalışma modu, koordinat sistemi ve ölçekle boş çizim'),
            action('file.open', 'fileOpen', 'Dosya aç', 'Bu bilgisayardaki bir .kcad çizimi'),
            cloud,
            action('file.import.dxf', 'import', 'DXF içe aktar', 'AutoCAD ve Netcad çizimleri, katmanlarıyla'),
            keep,
          ),
        ),
        h('section', { class: 'start__side' }, h('h3', { class: 'start__heading' }, 'Son dosyalar'), list),
      ),
    ],
    footer: [h('label', { class: 'start__show' }, shown, h('span', null, 'Açılışta göster')), h('span', { class: 'start__hint' }, 'Dosya → Başlangıç ekranı ile yeniden açılır.')],
  });
  stop = watchAll([ctx.files.recent.list, ctx.server.state, ctx.cloud.me, ctx.doc.name], () => {
    showRecent();
    sync();
  });
  showRecent();
  sync();
  // Stored files arrive a moment after the app opens.
  void ctx.files.recent.ready.then(() => dialog?.el.isConnected && showRecent());
  keep.focus();
}
