import type { AppContext } from '../../app/context';
import { Formatter } from '../../app/format';
import { Signal } from '../../core/signal';
import { crsBySrid } from '../../geo/crs';
import { PROJECT_SETTINGS_DEFAULTS, type ProjectSettingsData } from '../../model/projectSettings';
import { h, type Child } from '../dom';
import { PLOT_SCALES } from '../toolbar/fields';
import { note, segmented, settingRow, stepper, textField } from '../widgets/controls';
import { crsPicker } from './crsPicker';
import { workspacePicker } from './workspacePicker';
import { group, SettingsShell, type DraftApi, type SectionDef } from './SettingsShell';

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
      lead: 'Projenin adı, çalışma modu ve pafta çıktılarında kullanılacak çizim ölçeği.',
      keys: ['plotScale', 'workspace'],
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
          'Çalışma modu',
          h('p', { class: 'sgroup__note' }, 'Hangi menülerin, şerit sekmelerinin ve araçların görüneceğini seçer; veriyi değiştirmez. Gizlenen komutlar komut satırından yine çalışır.'),
          workspacePicker({ value: api.draft.workspace, compact: true, onChange: (id) => api.set('workspace', id, false) }),
        ),
        group(
          'Özet',
          settingRow('Nesne sayısı', null, h('span', { class: 'srow__value num' }, String(doc.size))),
          settingRow('Katman sayısı', null, h('span', { class: 'srow__value num' }, String(doc.layers.leaves().length))),
          settingRow(
            'Yerel çizim orijini',
            'Büyük TM koordinatları ekran kartında bu noktaya göre çizilir; hassasiyet kaybını önler.',
            h('span', { class: 'srow__value num' }, `Y ${doc.origin.x.toFixed(0)}  X ${doc.origin.y.toFixed(0)}`),
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
          group('Yeni projeler', settingRow('Yeni projelerin varsayılanı', def ? `${def.name} (EPSG:${def.srid}). Uygulama ayarlarından değiştirilir.` : null, openApp)),
        ];
      },
    },
    {
      id: 'units',
      label: 'Birimler ve hassasiyet',
      icon: 'units',
      title: 'Birimler ve hassasiyet',
      lead: 'Bu projede panellerde, komut satırında ve ölçüm etiketlerinde sayıların nasıl gösterileceği.',
      keys: ['lengthDecimals', 'areaDecimals', 'areaUnit', 'angleUnit'],
      render: (api) => unitsSection(api),
    },
  ];

  new SettingsShell<ProjectDraft>({
    title: 'Proje ayarları',
    scope: { icon: 'save', title: 'Proje dosyasına kaydedilir', detail: doc.name.value },
    sections,
    initial,
    defaults: { ...PROJECT_SETTINGS_DEFAULTS, srid: initial.srid, name: initial.name },
    section,
    onSave: (draft, init) => {
      const name = draft.name.trim() || init.name;
      if (name !== init.name) doc.name.set(name);
      const { name: _n, ...settings } = draft;
      doc.settings.assign(settings);
      if (draft.srid !== init.srid) {
        const c = crsBySrid(draft.srid)!;
        ctx.log.success(`Proje koordinat sistemi ${c.name} (EPSG:${c.srid}) olarak atandı. Koordinat değerleri değiştirilmedi.`);
      }
      ctx.log.success('Proje ayarları kaydedildi. Proje dosyasıyla birlikte saklanacak.');
    },
  });
}

function unitsSection(api: DraftApi<ProjectDraft>): Child {
  const d = api.draft;
  const f = new Formatter({
    lengthDecimals: new Signal(d.lengthDecimals),
    areaDecimals: new Signal(d.areaDecimals),
    areaUnit: new Signal(d.areaUnit),
    angleUnit: new Signal(d.angleUnit),
  });
  return [
    group(
      'Uzunluk ve koordinat',
      settingRow('Ondalık basamak', 'Koordinatlar, kenar uzunlukları ve mesafeler.', stepper({ label: 'Uzunluk basamağı', value: d.lengthDecimals, min: 0, max: 4, onChange: (v) => api.set('lengthDecimals', v) })),
    ),
    group(
      'Alan',
      settingRow(
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
