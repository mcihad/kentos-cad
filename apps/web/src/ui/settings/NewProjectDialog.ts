import type { AppContext } from '../../app/context';
import { crsBySrid, DEFAULT_SRID } from '../../geo/crs';
import { NEW_PROJECT_NAME, newProjectContent } from '../../model/newProject';
import { PROJECT_SETTINGS_DEFAULTS } from '../../model/projectSettings';
import { standardLayers } from '../../model/standardLayers';
import { h, replaceChildren } from '../dom';
import { PLOT_SCALES } from '../toolbar/fields';
import { note, segmented, settingRow } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { crsPicker } from './crsPicker';
import { workspacePicker } from './workspacePicker';
import { effectiveWorkspace } from '../../app/workspaces';
import { group } from './SettingsShell';

/**
 * Dosya → Yeni proje: an empty drawing with the standard layer tree, a
 * work mode (app/workspaces.ts; the app's default first), a coordinate system (the app's default for new projects) and a plot scale.
 * Nothing changes until Oluştur. Unsaved changes of the open drawing are
 * asked about then, over this dialog, so Vazgeç there comes back here; an
 * open cloud project is closed first (its changes are sent or kept on this
 * device).
 */

const AREA_UNIT = { m2: 'm²', donum: 'dönüm', ha: 'hektar' } as const;
const ANGLE_UNIT = { grad: 'grad', deg: 'derece' } as const;

export function openNewProjectDialog(ctx: AppContext): void {
  const defaults = PROJECT_SETTINGS_DEFAULTS;
  const fallback = crsBySrid(ctx.prefs.defaultSrid.value) ? ctx.prefs.defaultSrid.value : DEFAULT_SRID;
  // An announced mode is never offered, even if the preference names one.
  const draft = { srid: fallback, plotScale: defaults.plotScale, workspace: effectiveWorkspace(ctx.prefs.defaultWorkspace.value).id };
  const crsState = { query: '' };

  const name = h('input', { class: 'field field--setting', value: NEW_PROJECT_NAME, 'aria-label': 'Proje adı', spellcheck: 'false' });
  const scaleSlot = h('div');
  const crsSlot = h('div');
  const status = h('p', { class: 'newproj__status', role: 'alert' });
  const create = h('button', { class: 'btn btn--primary', type: 'button' }, 'Oluştur');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');

  // A re-rendered control keeps the keyboard where it was.
  const keepFocus = (slot: HTMLElement, render: () => void, target: string) => {
    const had = slot.contains(document.activeElement);
    render();
    if (had) slot.querySelector<HTMLElement>(target)?.focus();
  };
  const renderScale = () =>
    replaceChildren(
      scaleSlot,
      segmented({
        label: 'Çizim ölçeği',
        value: String(draft.plotScale),
        options: PLOT_SCALES.map((s) => ({ value: String(s), label: `1:${s.toLocaleString('tr-TR')}` })),
        onChange: (v) => {
          draft.plotScale = Number(v);
          keepFocus(scaleSlot, renderScale, '[aria-checked="true"]');
        },
      }),
    );
  const renderCrs = () =>
    replaceChildren(
      crsSlot,
      crsPicker({
        value: draft.srid,
        initial: fallback,
        defaultSrid: ctx.prefs.defaultSrid.value,
        mode: 'new',
        state: crsState,
        onChange: (srid, rerender) => {
          draft.srid = srid;
          if (rerender) keepFocus(crsSlot, renderCrs, '.crs-row[aria-selected="true"]');
        },
      }),
    );
  renderScale();
  renderCrs();

  const cloud = ctx.cloud.project.value;
  const groups = standardLayers(draft.plotScale)
    .map((n) => n.name)
    .join(', ');
  // What happens to the drawing on screen, said before anything is done.
  const current = cloud
    ? ctx.cloud.autosaves()
      ? note('info', `“${cloud.name}” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir.`)
      : ctx.doc.dirty.value
        ? note('warn', `“${cloud.name}” projesindeki değişiklikleriniz buluta kaydedilmiyor (salt okunur); Oluştur’a basınca ne yapılacağı sorulur.`)
        : null
    : ctx.doc.dirty.value
      ? note('warn', `“${ctx.doc.name.value}” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur.`)
      : null;

  const dialog = new Dialog({
    title: 'Yeni proje',
    width: 760,
    className: 'dialog--newproj',
    content: [
      group(
        'Proje',
        settingRow('Proje adı', 'İlk kayıtta dosya adı olarak önerilir.', name),
        settingRow('Çizim ölçeği', 'Yazı yükseklikleri ve pafta çıktıları bu ölçeğe göre hesaplanır.', scaleSlot),
      ),
      group('Çalışma modu', workspacePicker({ value: draft.workspace, onChange: (id) => (draft.workspace = id) })),
      group('Koordinat sistemi', crsSlot),
      note(
        'info',
        `Boş bir çizim açılır. Katmanlar: ${groups}. Birimler varsayılanla başlar (uzunluk ${defaults.lengthDecimals}, alan ${defaults.areaDecimals} ondalık, ${AREA_UNIT[defaults.areaUnit]}, ${ANGLE_UNIT[defaults.angleUnit]}); Dosya → Proje ayarları’ndan değiştirilir.`,
      ),
      current,
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, create],
  });

  const refresh = () => {
    create.disabled = !name.value.trim();
    status.textContent = name.value.trim() ? '' : 'Proje adı boş olamaz.';
  };
  const run = async () => {
    if (create.disabled) return;
    if (ctx.files.busy.value) {
      status.textContent = 'Bir dosya işlemi sürüyor; bitince yeniden deneyin.';
      return;
    }
    let content: ReturnType<typeof newProjectContent>;
    try {
      content = newProjectContent({ name: name.value, srid: draft.srid, plotScale: draft.plotScale, workspace: draft.workspace, drawingFont: ctx.prefs.defaultDrawingFont.value });
    } catch (e) {
      status.textContent = (e as Error).message;
      return;
    }
    create.disabled = true;
    const done = await ctx.files.newProject(content);
    if (done) dialog.close();
    else refresh();
  };

  name.addEventListener('input', refresh);
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  create.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
  name.select();
}
