import type { AppContext } from '../app/context';
import { formatChord } from '../core/keymap';
import { h } from './dom';
import { icon } from './icons';
import { Dialog } from './widgets/Dialog';

/** Every bound shortcut grouped by command category, generated from the keymap. */
/** What each mouse action does (left: meaning, right: the action). */
const MOUSE_ACTIONS: [string, string][] = [
  ['Seç, nokta koy, adımı onayla', 'Sol tık'],
  ['Komutu onayla ya da bitir (Enter gibi)', 'Sağ tık'],
  ['Komut menüsü: seçenekler, tek seferlik kenet, orto', 'Sağ tuşu basılı tut'],
  ['Tek seferlik kenet (uç, orta, kesişim, dik…)', 'Shift + sağ tık'],
  ['Boştayken bağlam menüsü', 'Sağ tık'],
  ['Köşeyi sil, kenara köşe ekle, yaya dönüştür', 'Tutamaca sağ tık'],
  ['Nesnenin türü, katmanı, alanı', 'Üzerinde bekle'],
  ['İzleme noktası al ya da bırak (nesne izleme)', 'Kenet noktasında bekle'],
  ['Mesafe ya da koordinat (imleç yanında açılır)', 'Komut sırasında sayı yaz'],
  ['Pencere seçimi: tamamen içeride kalanlar', 'Soldan sağa sürükle'],
  ['Kesişim seçimi: dokunan her şey', 'Sağdan sola sürükle'],
  ['Seçime ekle ya da çıkar', 'Shift + tık'],
  ['Görünümü kaydır', 'Orta tuşla sürükle'],
  ['Yakınlaştır, uzaklaştır', 'Tekerlek'],
  ['Tümünü göster', 'Orta tuşa çift tık'],
  ['Yazıyı ya da ölçüyü yerinde düzenle', 'Çift tık'],
  ['Köşeyi taşı; kenar ortasındaki baklava köşe ekler', 'Tutamacı sürükle'],
];

export function openShortcutsDialog(ctx: AppContext): void {
  const byCategory = new Map<string, { title: string; icon?: string; chords: string[]; aliases: string[] }[]>();
  const seen = new Map<string, { chords: string[] }>();
  for (const b of ctx.keymap.all()) {
    const cmd = ctx.commands.get(b.command);
    if (!cmd) continue;
    const existing = seen.get(cmd.id);
    if (existing) {
      if (!existing.chords.includes(b.chord)) existing.chords.push(b.chord);
      continue;
    }
    const row = { title: cmd.title, icon: cmd.icon, chords: [b.chord], aliases: [...(cmd.aliases ?? [])].slice(0, 2) };
    seen.set(cmd.id, row);
    const cat = cmd.category ?? 'Diğer';
    byCategory.set(cat, [...(byCategory.get(cat) ?? []), row]);
  }
  const search = h('input', { class: 'field field--search', type: 'search', placeholder: 'Komut ya da tuş ara', 'aria-label': 'Kısayol ara' });
  const grid = h('div', { class: 'shortcuts' });
  const render = () => {
    const q = search.value.trim().toLocaleLowerCase('tr-TR');
    grid.textContent = '';
    const mouse = MOUSE_ACTIONS.filter(([what, how]) => !q || what.toLocaleLowerCase('tr-TR').includes(q) || how.toLocaleLowerCase('tr-TR').includes(q));
    if (mouse.length) {
      grid.append(
        h(
          'section',
          { class: 'shortcuts__group' },
          h('h3', { class: 'shortcuts__title' }, 'Fare'),
          h(
            'dl',
            { class: 'shortcuts__list' },
            mouse.map(([what, how]) => [h('dt', null, what), h('dd', null, h('kbd', { class: 'kbd' }, how))]),
          ),
        ),
      );
    }
    for (const [cat, rows] of byCategory) {
      const match = rows.filter((r) => !q || r.title.toLocaleLowerCase('tr-TR').includes(q) || r.chords.some((c) => c.toLowerCase().includes(q)) || r.aliases.some((a) => a.toLowerCase().includes(q)));
      if (!match.length) continue;
      grid.append(
        h(
          'section',
          { class: 'shortcuts__group' },
          h('h3', { class: 'shortcuts__title' }, cat),
          h(
            'dl',
            { class: 'shortcuts__list' },
            match.map((r) => [
              h('dt', null, h('span', { class: 'shortcuts__icon' }, r.icon ? icon(r.icon, 15) : null), r.title, r.aliases.length ? h('span', { class: 'shortcuts__alias' }, r.aliases.join(', ')) : null),
              h('dd', null, r.chords.map((c) => h('kbd', { class: 'kbd' }, formatChord(c)))),
            ]),
          ),
        ),
      );
    }
    if (!grid.children.length) grid.append(h('p', { class: 'empty__text' }, 'Aramayla eşleşen kısayol yok.'));
  };
  search.addEventListener('input', render);
  render();
  new Dialog({
    title: 'Fare ve klavye kısayolları',
    width: 880,
    content: [
      h('p', { class: 'dialog__lead' }, 'Harf kısayolları klavyenizdeki harfe göre çalışır; Türkçe Q ve F düzeninde de aynı harfe basın. Komutları takma adıyla komut satırına yazıp Enter’a da basabilirsiniz.'),
      search,
      grid,
    ],
  });
  search.focus();
}

export function openAboutDialog(ctx: AppContext): void {
  new Dialog({
    title: 'KentOS CAD',
    width: 440,
    content: [
      h('p', { class: 'dialog__lead' }, 'Web tabanlı harita ve kadastro çizim ortamı.'),
      h(
        'dl',
        { class: 'about' },
        h('dt', null, 'Sürüm'),
        h('dd', { class: 'num' }, '0.1.0 (arayüz önizlemesi)'),
        h('dt', null, 'Çizim motoru'),
        h('dd', null, ctx.view.backendLabel.value),
        h('dt', null, 'Koordinat sistemi'),
        h('dd', null, `${ctx.doc.crs.value.name} (EPSG:${ctx.doc.crs.value.srid})`),
      ),
    ],
  });
}
