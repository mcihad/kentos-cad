import type { AppContext } from '../../app/context';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { TemplateMeta } from '../../contracts/generated/sheet/TemplateMeta';
import type { Workspace } from '../../contracts/generated/Workspace';
import { DisposableStore } from '../../core/disposable';
import { categoryLabel, type TemplateCard } from '../../product/sheet/templates';
import { h, replaceChildren } from '../dom';
import { note, segmented, toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import type { SheetHost } from './host';
import { choice, field, longText, textInput } from './widgets/form';

/**
 * Şablon olarak kaydet (docs/sheet/design.md §12): the open sheet as a
 * template on this device. The engine takes the maps' place out (their
 * scale stays), takes the pictures along and turns the sheet's values into
 * the questions a use asks. What the window asks: the name, a description,
 * the kind (the gallery's filter), the work modes and project types it is
 * for (design §11a; both modes: common), and the papers it suits (this
 * one first). A sheet made from one of the user's templates may update it
 * (its revision goes up) instead of making a new one.
 */

export type TemplateWords = Pick<TemplateMeta, 'name' | 'description' | 'category' | 'tags' | 'papers' | 'workspaces' | 'projectTypes'>;

const PROJECT_TYPES: readonly { value: ProjectType; label: string }[] = [
  { value: 'cad', label: 'CAD' },
  { value: 'gis', label: 'CBS' },
  { value: 'subdivision', label: 'İfraz / tevhit' },
  { value: 'landReadjustment', label: 'Arazi düzenlemesi' },
  { value: 'zoningPlan', label: 'İmar planı' },
  { value: 'road', label: 'Yol' },
  { value: 'architecture', label: 'Mimari' },
];

export function openSaveTemplate(_ctx: AppContext, host: SheetHost, sheetId: string, save: (words: TemplateWords, replace: TemplateCard | null) => Promise<boolean>): void {
  const engine = host.engine();
  const sheet = host.book()?.book.sheets.find((s) => s.id === sheetId);
  if (!engine || !sheet) return;
  // The sheet's template, when the user may write it: their own (on this device or in the cloud), or one shared with them to edit.
  // Written over: one's own, a shared one one may edit, an organisation's one published or administers.
  const writable = host.providers.flatMap((p) => p.cards.value).filter((c) => c.source === 'device' || c.source === 'cloud' || (c.source === 'shared' && c.role === 'editor') || (c.source === 'org' && (c.role === 'owner' || c.role === 'admin')));
  const at = sheet.origin?.templateId;
  const origin = at ? (writable.find((c) => c.id === at) ?? writable.find((c) => c.formerIds?.includes(at))) : undefined;
  const ws = host.workspace();
  const here: PaperChoice = { paper: sheet.page.paper, orientation: sheet.page.orientation };
  const categories = [...new Set(engine.systemTemplates().map((t) => t.meta.category))];
  const s = {
    replace: !!origin,
    name: origin?.name ?? sheet.name,
    description: origin?.description ?? '',
    category: origin?.category ?? categories[0] ?? 'genel',
    tags: (origin?.tags ?? []).join(', '),
    workspaces: new Set<Workspace>(origin?.workspaces ?? (ws === 'hybrid' ? ['cad', 'gis'] : [ws])),
    projectTypes: new Set<ProjectType>(origin?.projectTypes ?? []),
  };
  const d = new DisposableStore();
  const body = h('div', { class: 'sheet-form' });
  const toggles = <T extends string>(values: readonly { value: T; label: string }[], set: Set<T>) =>
    h(
      'div',
      { class: 'sheet-toggles' },
      values.map((v) =>
        h(
          'label',
          { class: 'sheet-toggle' },
          toggleSwitch({ label: v.label, checked: set.has(v.value), onChange: (on) => (on ? set.add(v.value) : set.delete(v.value), render()) }),
          h('span', null, v.label),
        ),
      ),
    );
  const render = () => {
    d.dispose();
    replaceChildren(
      body,
      origin
        ? field(
            'Kaydet',
            segmented<'new' | 'update'>({
              label: 'Kaydet',
              value: s.replace ? 'update' : 'new',
              options: [
                { value: 'update', label: `“${origin.name}” güncellensin` },
                { value: 'new', label: 'Yeni şablon' },
              ],
              onChange: (v) => ((s.replace = v === 'update'), render()),
            }),
            s.replace
              ? origin.source === 'device'
                ? `Revizyonu ${origin.revision + 1} olur; ondan yapılmış paftalar “Yeni sürüm var” der.`
                : `Bulutta yeni revizyon olur (şimdi ${origin.revision}); eşitlenince ondan yapılmış paftalar “Yeni sürüm var” der${origin.source === 'shared' ? ', sahibi ve paylaşılanlar da görür' : ''}.`
              : null,
          )
        : null,
      textInput({ label: 'Ad', key: 'name', value: s.name, onCommit: (v) => ((s.name = v), render()) }, d),
      longText({ label: 'Açıklama', key: 'description', value: s.description, rows: 2, onCommit: (v) => ((s.description = v), render()) }, d),
      choice({ label: 'Tür', key: 'category', value: s.category, options: categories.map((c) => ({ value: c, label: categoryLabel(c) })), onChange: (v) => ((s.category = v), render()) }, d),
      textInput({ label: 'Etiketler (virgülle)', key: 'tags', value: s.tags, placeholder: 'kadastro, ifraz', onCommit: (v) => ((s.tags = v), render()) }, d),
      field('Çalışma modları', toggles<Workspace>([{ value: 'cad', label: 'CAD' }, { value: 'gis', label: 'CBS' }], s.workspaces), s.workspaces.size === 2 ? 'İki kip de seçili: ortak şablon, her kipte gösterilir.' : null),
      field('Proje türleri', toggles(PROJECT_TYPES, s.projectTypes), s.projectTypes.size ? null : 'Hiçbiri: her proje türüne.'),
      field('Kâğıt', h('span', null, `${sheet.page.paper.toUpperCase()} ${sheet.page.orientation === 'landscape' ? 'yatay' : 'dikey'}`), 'Şablon her kâğıtta kullanılır; öğeler kısıtlarıyla yerleşir.'),
      s.workspaces.size ? null : note('warn', 'En az bir çalışma modu seçin.'),
    );
  };
  render();
  const ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({ title: 'Şablon olarak kaydet', width: 520, className: 'sheet-dialog', content: [body], footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, ok], onClose: () => d.dispose() });
  cancel.addEventListener('click', () => dialog.close());
  ok.addEventListener('click', () => {
    (document.activeElement as HTMLElement | null)?.blur?.();
    if (!s.name.trim() || !s.workspaces.size) return render();
    const words: TemplateWords = {
      name: s.name.trim(),
      description: s.description.trim(),
      category: s.category,
      tags: s.tags.split(',').map((t) => t.trim()).filter(Boolean),
      papers: [here],
      workspaces: [...s.workspaces],
      projectTypes: [...s.projectTypes],
    };
    ok.disabled = true;
    void save(words, s.replace ? (origin ?? null) : null).then((done) => {
      ok.disabled = false;
      if (done) dialog.close();
    });
  });
}
