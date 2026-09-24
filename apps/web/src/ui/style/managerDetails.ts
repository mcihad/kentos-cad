import type { AppContext } from '../../app/context';
import type { LibraryItem, Sourced, Symbol } from '../../model/style';
import type { PreviewGeometry } from '../../render/symbolPreview';
import { importStyles, type ConflictMode, type StyleFile } from '../../style/file';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { askRemove } from '../widgets/confirm';
import { PopupMenu } from '../widgets/PopupMenu';
import { note, segmented } from '../widgets/controls';
import { downloadStyles } from './styleFiles';
import { drawNow, symbolOfItem } from './thumbs';

/**
 * The right column of the style manager: a large preview of the chosen
 * item on sample geometry, what it is and where it comes from, its
 * editable fields (user and project items), and what can be done with it.
 * Also the import panel shown there while a .kstil file is being taken in.
 */

export interface DetailsHost {
  readonly ctx: AppContext;
  readonly pick?: { onPick(id: string): void };
  select(id: string): void;
  say(text: string, kind?: 'ok' | 'warn'): void;
  design(what: { id?: string }): void;
  edit(id: string): void;
  refreshDetails(): void;
}

const KIND_LABEL: Record<Symbol['type'], string> = { fill: 'Alan sembolü', line: 'Çizgi sembolü', marker: 'İşaret sembolü' };
const SOURCE_LABEL = { system: 'Sistem', user: 'Kitaplığım', project: 'Proje' } as const;

const GEOMETRY_OPTIONS: Record<Symbol['type'], { value: PreviewGeometry; label: string }[]> = {
  marker: [{ value: 'point', label: 'Nokta' }],
  line: [
    { value: 'line', label: 'Düz' },
    { value: 'bent', label: 'Kırık' },
    { value: 'area', label: 'Alan kenarı' },
  ],
  fill: [
    { value: 'area', label: 'Alan' },
    { value: 'hole', label: 'Adalı' },
  ],
};

/** Preview geometry chosen per symbol kind, kept while the window is open. */
const chosenGeometry = new Map<Symbol['type'], PreviewGeometry>();

export function renderDetails(host: DetailsHost, item: Sourced | undefined): Child {
  if (!item) {
    return h(
      'div',
      { class: 'smgr__none' },
      icon('styles', 28),
      h('div', { class: 'smgr__none-title' }, 'Bir sembol seçin'),
      h('p', null, 'Soldan bir kaynak ya da kategori, ortadan bir sembol seçin. Sistem sembolleri salt okunurdur; kopyalayıp kendi kitaplığınızda ya da projede düzenleyebilirsiniz.'),
    );
  }
  const ctx = host.ctx;
  const lib = ctx.styles.library;
  const editable = lib.canEdit(item.id);
  const symbol = symbolOfItem(item);
  const type = symbol.type;

  // Preview
  const canvas = h('canvas', { class: 'smgr__preview', width: '300', height: '172', style: 'width:300px;height:172px' });
  const geomOptions = item.kind === 'symbol' ? GEOMETRY_OPTIONS[type] : [];
  let geometry = chosenGeometry.get(type) ?? geomOptions[0]?.value;
  const redraw = () => drawNow(ctx, canvas, symbol, geometry, 3.2);
  queueMicrotask(redraw);
  const geomPicker =
    geomOptions.length > 1
      ? segmented({
          label: 'Önizleme geometrisi',
          options: geomOptions,
          value: geometry!,
          onChange: (v) => {
            chosenGeometry.set(type, v);
            host.refreshDetails();
          },
        })
      : null;

  // Fields
  const field = (label: string, value: Child) => h('div', { class: 'smgr__field' }, h('div', { class: 'smgr__flabel' }, label), h('div', { class: 'smgr__fvalue' }, value));
  const edit = (value: string, commit: (v: string) => void, opts: { multiline?: boolean; placeholder?: string; label: string }) => {
    if (!editable) return value ? h('span', null, value) : h('span', { class: 'smgr__muted' }, '—');
    const el = opts.multiline
      ? h('textarea', { class: 'field smgr__area', rows: '3', 'aria-label': opts.label, placeholder: opts.placeholder ?? null, spellcheck: 'false' }, value)
      : h('input', { class: 'field', value, 'aria-label': opts.label, placeholder: opts.placeholder ?? null, spellcheck: 'false' });
    let done = false;
    const save = () => {
      if (done) return;
      const v = (el as HTMLInputElement).value.trim();
      if (v !== value) {
        done = true;
        commit(v);
      }
    };
    el.addEventListener('blur', save);
    el.addEventListener('keydown', (e) => {
      if ((e as KeyboardEvent).key === 'Enter' && !opts.multiline) save();
    });
    return el;
  };

  const title = editable
    ? edit(item.name, (v) => lib.rename(item.id, v), { label: 'Ad' })
    : h('h3', { class: 'smgr__name' }, item.name);
  const kindLine = h(
    'div',
    { class: 'smgr__kindline' },
    h('span', null, item.kind === 'symbol' ? KIND_LABEL[type] : `Çizim (${item.format.toUpperCase()})`),
    h('span', { class: `scard__src scard__src--${item.source}` }, item.source === 'system' ? 'Sistem · salt okunur' : SOURCE_LABEL[item.source]),
  );
  const fields: Child[] = [
    field('Kategori', edit(item.path.join(' / '), (v) => lib.move(item.id, v.split('/').map((s) => s.trim()).filter(Boolean)), { label: 'Kategori', placeholder: 'Ana / Alt' })),
    field('Kimlik', h('code', { class: 'smgr__id' }, item.id)),
  ];
  if (item.kind === 'symbol') {
    if (item.reference || editable) fields.push(field('Kaynak', edit(item.reference ?? '', (v) => lib.update(item.id, { reference: v || undefined }), { label: 'Kaynak', placeholder: 'Yönetmelik, sayfa' })));
    fields.push(field('Açıklama', edit(item.description ?? '', (v) => lib.update(item.id, { description: v || undefined }), { multiline: true, label: 'Açıklama' })));
  }
  fields.push(field('Etiketler', edit((item.tags ?? []).join(', '), (v) => lib.update(item.id, { tags: v.split(',').map((t) => t.trim()).filter(Boolean) }), { label: 'Etiketler', placeholder: 'virgülle ayırın' })));
  if (item.kind === 'asset') {
    const users = lib.usersOf(item.id);
    fields.push(field('Kullanan', users.length ? `${users.length} sembol` : h('span', { class: 'smgr__muted' }, 'Hiçbir sembol kullanmıyor')));
  }

  return h(
    'div',
    { class: 'smgr__detail' },
    h('div', { class: 'smgr__previewbox' }, canvas, geomPicker),
    h('div', { class: 'smgr__head' }, title, kindLine),
    h('div', { class: 'smgr__fields' }, fields),
    actions(host, item, editable),
  );
}

function actions(host: DetailsHost, item: Sourced, editable: boolean): HTMLElement {
  const ctx = host.ctx;
  const lib = ctx.styles.library;
  const btn = (label: string, iconName: string, run: () => void, opts: { disabled?: boolean; danger?: boolean; title?: string } = {}) => {
    const b = h('button', { class: `btn btn--small${opts.danger ? ' btn--danger' : ''}`, type: 'button', disabled: opts.disabled, title: opts.title ?? null }, icon(iconName, 14), label);
    b.addEventListener('click', run);
    return b;
  };
  const list: HTMLElement[] = [];
  if (item.kind === 'asset' && item.format === 'svg') list.push(btn(editable ? 'Düzenle' : 'Kopyasını düzenle', 'edit', () => host.edit(item.id), { title: 'SVG çizim düzenleyicisinde açar' }));
  if (item.kind === 'symbol') {
    list.push(btn(editable ? 'Düzenle' : 'Kopyasını düzenle', 'edit', () => host.edit(item.id), { title: editable ? 'Sembol tasarımcısında açar' : 'Kitaplığım\'a bir kopya alır ve onu açar' }));
    const selected = [...ctx.selection.ids.value];
    list.push(btn(`Seçili nesnelere uygula${selected.length ? ` (${selected.length})` : ''}`, 'check', () => applyToSelection(host, item), { disabled: !selected.length, title: selected.length ? 'Nesnelerin sembolü olur; katman stilinin önüne geçer' : 'Önce çizimde nesne seçin' }));
  }
  const copyBtn = btn('Kopyala', 'copy', () => {
    const r = copyBtn.getBoundingClientRect();
    const copyTo = (to: 'user' | 'project') => {
      const c = lib.copy(item.id, to);
      host.say(`“${item.name}” ${to === 'user' ? 'Kitaplığım' : 'Proje'} kitaplığına kopyalandı.`);
      host.select(c.id);
    };
    PopupMenu.open(
      [
        { label: 'Kitaplığıma', hint: 'bu tarayıcıda', run: () => copyTo('user') },
        { label: 'Projeye', hint: 'proje dosyasında', run: () => copyTo('project') },
      ],
      { x: r.left, y: r.bottom + 4 },
    );
  });
  list.push(copyBtn);
  list.push(btn('Dışa aktar', 'export', () => host.say(`${downloadStyles(ctx, [item.id], item.name)} öğe dışa aktarıldı.`)));
  if (editable) {
    // Asked in a window (DESIGN.md §7.9.1): the library keeps no undo.
    const del = btn('Sil', 'trash', () => {
      const users = item.kind === 'asset' ? lib.usersOf(item.id).length : 0;
      void askRemove({
        title: 'Kitaplıktan sil',
        message: `“${item.name}” ${item.source === 'project' ? 'projenin kitaplığından' : 'Kitaplığım’dan'} silinsin mi? Bu geri alınamaz.`,
        details: users ? [`${users} sembol bu çizimi kullanıyor; silinirse onlarda boş kalır.`] : undefined,
        action: 'Sil',
      }).then((yes) => {
        if (!yes || !lib.get(item.id)) return;
        lib.remove(item.id);
        host.say(`“${item.name}” silindi.`);
      });
    }, { danger: true });
    list.push(del);
  }
  return h('div', { class: 'smgr__actions' }, list);
}

/** Gives the selected objects this symbol (one undo step); locked layers are skipped and counted. */
function applyToSelection(host: DetailsHost, item: Sourced): void {
  const { doc, selection } = host.ctx;
  const ids = [...selection.ids.value];
  let done = 0;
  let locked = 0;
  doc.transact(`Sembol: ${item.name}`, () => {
    for (const id of ids) {
      const e = doc.get(id);
      if (!e) continue;
      if (doc.layers.isLocked(e.layerId)) {
        locked++;
        continue;
      }
      doc.update(id, { symbol: item.id });
      done++;
    }
  });
  host.say(`${done} nesneye “${item.name}” verildi${locked ? `; kilitli katmandaki ${locked} nesne atlandı` : ''}.`, locked && !done ? 'warn' : 'ok');
}

// ── Import ─────────────────────────────────────────────────────────────

export function renderImport(host: DetailsHost, name: string, file: StyleFile, issues: readonly string[]): Child {
  const lib = host.ctx.styles.library;
  const symbols = file.items.filter((i) => i.kind === 'symbol').length;
  const assets = file.items.length - symbols;
  const clashes = file.items.filter((i: LibraryItem) => lib.get(i.id)).length;
  let to: 'user' | 'project' = 'user';
  let mode: ConflictMode = 'copy';
  const body = h('div', { class: 'smgr__import' });
  const render = () => {
    replaceChildren(
      body,
      h('h3', { class: 'smgr__name' }, 'İçe aktar'),
      h('p', { class: 'smgr__muted' }, `“${name}”: ${symbols} sembol${assets ? `, ${assets} çizim` : ''}.`),
      issues.length ? note('warn', h('b', null, `${issues.length} öğe alınamayacak. `), issues.slice(0, 3).join('; ')) : null,
      h('div', { class: 'smgr__flabel' }, 'Nereye'),
      segmented({ label: 'Hedef', options: [{ value: 'user', label: 'Kitaplığım' }, { value: 'project', label: 'Proje' }], value: to, onChange: (v) => ((to = v), render()) }),
      clashes
        ? [
            h('div', { class: 'smgr__flabel' }, `${clashes} öğe kitaplıkta zaten var`),
            segmented({
              label: 'Çakışma',
              options: [
                { value: 'copy', label: 'Kopya olarak al' },
                { value: 'replace', label: 'Üzerine yaz' },
                { value: 'skip', label: 'Atla' },
              ],
              value: mode,
              onChange: (v) => ((mode = v), render()),
            }),
            mode === 'replace' ? h('p', { class: 'smgr__muted' }, 'Sistem öğelerinin üzerine yazılmaz; onlar kopya olarak alınır.') : null,
          ]
        : null,
      h(
        'div',
        { class: 'smgr__actions' },
        (() => {
          const go = h('button', { class: 'btn btn--primary btn--small', type: 'button' }, icon('import', 14), 'İçe aktar');
          go.addEventListener('click', () => {
            const r = importStyles(lib, file, to, mode);
            host.say(`${r.added} öğe eklendi${r.replaced ? `, ${r.replaced} güncellendi` : ''}${r.skipped ? `, ${r.skipped} atlandı` : ''}.`);
          });
          return go;
        })(),
        (() => {
          const no = h('button', { class: 'btn btn--small', type: 'button' }, 'Vazgeç');
          no.addEventListener('click', () => host.refreshDetails());
          return no;
        })(),
      ),
    );
  };
  render();
  return body;
}
