import type { AppContext } from '../../app/context';
import { Formatter } from '../../app/format';
import { Signal } from '../../core/signal';
import { crsBySrid, crsTitle, LOCAL_SRID } from '../../geo/crs';
import { PROJECT_SETTINGS_DEFAULTS, secondAllowed, type DrawingUnit, type ProjectSettingsData } from '../../model/projectSettings';
import { definitionTitle, DEFINITION_CODE } from '../../model/projectCrs';
import { secondChoices, secondTitle } from '../../model/secondCrs';
import { h, type Child } from '../dom';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { PLOT_SCALES } from '../ribbon/fields';
import { note, segmented, settingRow, stepper, textField } from '../widgets/controls';
import { crsPicker } from './crsPicker';
import { workspacePicker } from './workspacePicker';
import { effectiveWorkspace } from '../../app/workspaces';
import { drawingFontPicker } from './appearancePickers';
import { group, SettingsShell, type DraftApi, type SectionDef } from './SettingsShell';
import { fixed } from '../../core/displayNumber';

/** Project settings: stored in the project file, shared by everyone who opens it. */
interface ProjectDraft extends ProjectSettingsData {
  name: string;
}

export type ProjectSettingsSection = 'general' | 'crs' | 'units';

export function openProjectSettings(ctx: AppContext, section?: ProjectSettingsSection): void {
  const doc = ctx.doc;
  const crsState = { query: '' };
  const initial: ProjectDraft = { ...doc.settings.toJSON(), name: doc.name.value };

  const sections: SectionDef<ProjectDraft>[] = [
    {
      id: 'general',
      label: 'Genel',
      icon: 'folder',
      title: 'Genel',
      lead: 'Projenin adı, türü, çizim ölçeği ve çizimdeki yazıların yazı tipi.',
      keys: ['plotScale', 'workspace', 'drawingFont'],
      render: (api) => [
        group(
          'Proje',
          settingRow('Proje adı', 'Dosya adı olarak da kullanılır.', textField({ label: 'Proje adı', value: api.draft.name, onChange: (v) => api.set('name', v, false) })),
          settingRow(
            'Çizim ölçeği',
            'Yazı yükseklikleri ve pafta çıktıları bu ölçeğe göre hesaplanır.',
            segmented({
              label: 'Çizim ölçeği',
              value: String(api.draft.plotScale),
              options: PLOT_SCALES.map((s) => ({ value: String(s), label: `1:${s.toLocaleString('tr-TR')}` })),
              onChange: (v) => api.set('plotScale', Number(v)),
            }),
          ),
        ),
        group(
          'Proje türü',
          h('p', { class: 'sgroup__note' }, 'CAD ya da CBS: sahnesi, eksen ve açı düzeni ve şeridi türe göredir; veri değişmez. Şeritte olmayan komutlar komut satırından yine çalışır.'),
          workspacePicker({ value: effectiveWorkspace(api.draft.workspace ?? null).id, compact: true, onChange: (id) => api.set('workspace', id, false) }),
        ),
        group(
          'Çizim yazı tipi',
          h('p', { class: 'sgroup__note' }, 'Çizimdeki yazılar, ölçü değerleri ve etiketler bu yazı tipiyle çizilir; projeyi açan herkes aynısını görür. Arayüzün yazı tipi Uygulama ayarlarındadır.'),
          drawingFontPicker({ value: api.draft.drawingFont, onChange: (id) => api.set('drawingFont', id) }),
        ),
        group(
          'Özet',
          settingRow('Nesne sayısı', null, h('span', { class: 'srow__value num' }, String(doc.size))),
          settingRow('Katman sayısı', null, h('span', { class: 'srow__value num' }, String(doc.layers.leaves().length))),
          settingRow(
            'Yerel çizim orijini',
            'Büyük TM koordinatları ekran kartında bu noktaya göre çizilir; hassasiyet kaybını önler.',
            // East and north as the project's type names them (docs/adr/0165 §4).
            h('span', { class: 'srow__value num' }, `${ctx.format.eastLabel} ${fixed(doc.origin.x, 0)}  ${ctx.format.northLabel} ${fixed(doc.origin.y, 0)}`),
          ),
        ),
      ],
    },
    {
      id: 'crs',
      label: 'Koordinat sistemi',
      icon: 'crs',
      title: 'Koordinat sistemi',
      lead: 'Bu projenin konum referansı. Koordinatlar bu sistemde saklanır, ölçülür ve dışa aktarılır.',
      keys: [],
      render: (api) => {
        const def = crsBySrid(ctx.prefs.defaultSrid.value);
        const openApp = h('button', { class: 'btn btn--small', type: 'button' }, 'Uygulama ayarlarını aç');
        openApp.addEventListener('click', () => ctx.commands.execute('tools.options'));
        return [
          crsPicker({
            value: api.draft.srid,
            initial: api.initial.srid,
            defaultSrid: ctx.prefs.defaultSrid.value,
            mode: 'assign',
            state: crsState,
            onChange: (srid, rerender) => api.set('srid', srid, rerender),
          }),
          secondGroup(api),
          group('Yeni projeler', settingRow('Yeni projelerin varsayılanı', def ? `${crsTitle(def)}. Uygulama ayarlarından değiştirilir.` : null, openApp)),
        ];
      },
    },
    {
      id: 'units',
      label: 'Birimler ve hassasiyet',
      icon: 'units',
      title: 'Birimler ve hassasiyet',
      lead: 'Bu projede panellerde, komut satırında ve ölçüm etiketlerinde sayıların nasıl gösterileceği.',
      keys: ['drawingUnit', 'lengthDecimals', 'areaDecimals', 'areaUnit', 'angleUnit'],
      render: (api) => unitsSection(api),
    },
  ];

  new SettingsShell<ProjectDraft>({
    title: 'Proje ayarları',
    scope: { icon: 'save', title: 'Proje dosyasına kaydedilir', detail: doc.name.value },
    sections,
    initial,
    defaults: { ...PROJECT_SETTINGS_DEFAULTS, drawingUnit: 'm', srid: initial.srid, name: initial.name },
    section,
    onSave: (draft, init) => {
      const name = draft.name.trim() || init.name;
      if (name !== init.name) doc.name.set(name);
      const { name: _n, ...settings } = draft;
      // A drawing unit is a local project's (docs/adr/0165 §2): one given a coordinate system is in metres. The second
      // system is the draft's, none when it has none (docs/adr/0167 §1, 0168 §1); the settings drop one that is not
      // another system.
      doc.settings.assign({
        ...settings,
        drawingUnit: settings.srid === LOCAL_SRID && !settings.customCrs ? (settings.drawingUnit ?? 'm') : 'm',
        secondSrid: settings.secondSrid ?? null,
        secondCustomCrs: settings.secondCustomCrs ?? null,
      });
      if (draft.srid !== init.srid) {
        const c = crsBySrid(draft.srid)!;
        ctx.log.success(`Proje koordinat sistemi ${crsTitle(c)} olarak atandı. Koordinat değerleri değiştirilmedi.`);
      }
      const second = doc.settings.secondSrid.value;
      const defined = doc.settings.secondCustomCrs.value;
      if (second !== (init.secondSrid ?? null) || JSON.stringify(defined) !== JSON.stringify(init.secondCustomCrs ?? null))
        ctx.log.success(
          second !== null
            ? `İkinci koordinat sistemi: ${secondTitle(second)}. Çizim dönüştürülmedi.`
            : defined
              ? `İkinci koordinat sistemi: ${definitionTitle(defined)}. Çizim dönüştürülmedi.`
              : 'İkinci koordinat sistemi kaldırıldı.',
        );
      ctx.log.success('Proje ayarları kaydedildi. Proje dosyasıyla birlikte saklanacak.');
    },
  });
}

/**
 * İkinci koordinat sistemi (docs/adr/0167 §1): the project's second definition when it has one (docs/adr/0168 §1), the
 * systems of the registry but the project's own, grouped by datum, and Yok. Its values show beside the project's in the
 * status bar and Koordinat oku; the drawing is not transformed.
 */
function secondGroup(api: DraftApi<ProjectDraft>): Child {
  const project = crsBySrid(api.draft.srid);
  const custom = !!api.draft.customCrs && api.draft.srid === LOCAL_SRID;
  const what = 'Durum çubuğunda ve Koordinat oku’da projeninkilerin yanında bu sistemin değerleri de gösterilir; çizim dönüştürülmez. ED50 değerleri EPSG’nin ±2 m’lik dönüşümüyledir, resmî dönüşüm değildir.';
  if (!project || (project.kind === 'local' && !custom))
    return group('İkinci koordinat sistemi', settingRow('İkinci sistem', 'Yerel projenin ikinci sistemi olmaz; önce projeye bir koordinat sistemi atayın.', h('span', { class: 'srow__value' }, 'Yok')));
  const shown = secondAllowed(project, api.draft.secondSrid, custom) ? api.draft.secondSrid : null;
  const defined = shown === null ? (api.draft.secondCustomCrs ?? null) : null;
  // A system of the registry, or Yok, takes the place of a second definition.
  const choose = (srid: number | undefined) => {
    api.set('secondCustomCrs', undefined, false);
    api.set('secondSrid', srid);
  };
  const pick = new Dropdown({
    ariaLabel: 'İkinci koordinat sistemi',
    width: 260,
    items: (): MenuItem[] => [
      { label: 'Yok', radio: true, checked: shown === null && defined === null, run: () => choose(undefined) },
      ...(defined ? [{ label: defined.name, hint: DEFINITION_CODE, radio: true, checked: true, run: () => {} } satisfies MenuItem] : []),
      ...secondChoices(project).flatMap(({ datum, systems }): MenuItem[] => [
        { kind: 'header', label: datum },
        ...systems.map((c) => ({ label: c.name, hint: `EPSG:${c.srid}`, radio: true, checked: c.srid === shown, run: () => choose(c.srid) })),
      ]),
    ],
  });
  pick.set(shown !== null ? secondTitle(shown) : defined ? definitionTitle(defined) : 'Yok');
  return group('İkinci koordinat sistemi', settingRow('İkinci sistem', what, pick.el));
}

function unitsSection(api: DraftApi<ProjectDraft>): Child {
  const d = api.draft;
  // A local project's unit (docs/adr/0165 §2); a project with a coordinate system, its own definition too, is in its
  // system's metres.
  const local = d.srid === LOCAL_SRID && !d.customCrs;
  const f = new Formatter({
    lengthDecimals: new Signal(d.lengthDecimals),
    areaDecimals: new Signal(d.areaDecimals),
    areaUnit: new Signal(d.areaUnit),
    angleUnit: new Signal(d.angleUnit),
    unit: local ? (d.drawingUnit ?? 'm') : 'm',
  });
  return [
    group(
      'Uzunluk ve koordinat',
      local
        ? settingRow(
            'Çizim birimi',
            'Uzunluklar, koordinatlar ve alanlar bu birimle yazılır ve gösterilir; çizimin kendisi değişmez.',
            segmented({
              label: 'Çizim birimi',
              value: d.drawingUnit ?? 'm',
              options: [
                { value: 'mm', label: 'mm' },
                { value: 'cm', label: 'cm' },
                { value: 'm', label: 'm' },
              ],
              onChange: (v) => api.set('drawingUnit', v as DrawingUnit),
            }),
          )
        : null,
      settingRow('Ondalık basamak', 'Koordinatlar, kenar uzunlukları ve mesafeler.', stepper({ label: 'Uzunluk basamağı', value: d.lengthDecimals, min: 0, max: 4, onChange: (v) => api.set('lengthDecimals', v) })),
    ),
    group(
      'Alan',
      // A local project in millimetres or centimetres reads its areas in the unit squared: no area unit to choose.
      f.unit !== 'm'
        ? null
        : settingRow(
            'Alan birimi',
            'Parsel ve kapalı alanlarda gösterilen birim. Metrekare her zaman öznitelik panelinde de yer alır.',
            segmented({
              label: 'Alan birimi',
              value: d.areaUnit,
              options: [
                { value: 'm2', label: 'm²' },
                { value: 'donum', label: 'Dönüm' },
                { value: 'ha', label: 'Hektar' },
              ],
              onChange: (v) => api.set('areaUnit', v),
            }),
          ),
      settingRow('Ondalık basamak', null, stepper({ label: 'Alan basamağı', value: d.areaDecimals, min: 0, max: 4, onChange: (v) => api.set('areaDecimals', v) })),
    ),
    group(
      'Açı',
      settingRow(
        'Açı birimi',
        'Semt açıları kuzeyden saat yönünde ölçülür.',
        segmented({
          label: 'Açı birimi',
          value: d.angleUnit,
          options: [
            { value: 'grad', label: 'Grad' },
            { value: 'deg', label: 'Derece' },
          ],
          onChange: (v) => api.set('angleUnit', v),
        }),
      ),
    ),
    h(
      'div',
      { class: 'preview-card' },
      h('div', { class: 'preview-card__title' }, 'Önizleme'),
      h(
        'dl',
        { class: 'preview-card__grid num' },
        h('dt', null, 'Koordinat'),
        h('dd', null, f.point({ x: 486512.34567, y: 4420118.92061 })),
        h('dt', null, 'Kenar'),
        h('dd', null, f.length(23.41234)),
        h('dt', null, 'Alan'),
        h('dd', null, f.area(12997.304)),
        h('dt', null, 'Semt'),
        h('dd', null, f.bearing(132.452137)),
      ),
    ),
    note('info', 'Ondalık ayırıcı her zaman noktadır; komut satırına aynı biçimde yazılabilir (Y,X virgülle ayrılır).'),
  ];
}
