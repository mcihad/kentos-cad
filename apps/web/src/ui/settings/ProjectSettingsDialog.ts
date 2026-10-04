import type { AppContext } from '../../app/context';
import { Formatter } from '../../app/format';
import { Signal } from '../../core/signal';
import { crsBySrid, crsTitle, LOCAL_SRID } from '../../geo/crs';
import { PROJECT_SETTINGS_DEFAULTS, secondAllowed, type DrawingUnit, type ProjectSettingsData } from '../../model/projectSettings';
import { datumChoices, definitionTitle, DEFINITION_CODE, ownSystem } from '../../model/projectCrs';
import { secondChoices, secondTitle } from '../../model/secondCrs';
import { h, replaceChildren, type Child } from '../dom';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { PLOT_SCALES } from '../ribbon/fields';
import { note, segmented, settingRow, stepper, textField } from '../widgets/controls';
import { askRemove } from '../widgets/confirm';
import { gridLine } from '../../app/gridLibrary';
import type { Convention } from '../../contracts/generated/Convention';
import type { DatumTransform } from '../../contracts/generated/DatumTransform';
import { buildChoice, datumName, epsgText, formOf, PAIRS, PARAMETERS, type ChoiceForm, type GridRef, type Method, type Parameter, type Problems } from '../../model/choiceForm';
import { crsPicker } from './crsPicker';
import { CONVENTIONS, openCustomCrs, PARAMETER_CAPTIONS, type TrialContext } from './CustomCrsDialog';
import type { CrsDefinition } from '../../contracts/generated/CrsDefinition';
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
  // Datum dönüşümleri's forms as typed, and what is wrong in each (docs/adr/0168 §3).
  const choiceState: ChoiceState = { forms: PAIRS.map((p) => formOf(p, initial.datumTransforms ?? [])), problems: PAIRS.map(() => ({})) };
  // The project's own definition and its second's (docs/adr/0168 §1), kept while another system is chosen: their rows
  // stay in the lists until Kaydet, which keeps only the chosen (the desktop's `State.defined`).
  const defined: Defined = { own: initial.srid === LOCAL_SRID ? (initial.customCrs ?? null) : null, second: initial.secondCustomCrs ?? null };

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
            onChange: (srid, rerender) => {
              // A system of the registry takes the definition's place (docs/adr/0168 §1).
              api.set('customCrs', undefined, false);
              api.set('srid', srid, rerender);
            },
            defined: ownDefined(ctx, api, defined),
          }),
          secondGroup(ctx, api, defined),
          datumGroup(ctx, api, choiceState),
          gridsGroup(ctx, api),
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
    // A datum choice with something to put right keeps Kaydet waiting.
    blocked: () => choiceState.problems.some((p) => Object.keys(p).length > 0),
    onSave: (draft, init) => {
      const name = draft.name.trim() || init.name;
      if (name !== init.name) doc.name.set(name);
      const { name: _n, ...settings } = draft;
      // A drawing unit is a local project's (docs/adr/0165 §2): one given a coordinate system is in metres. The second
      // system is the draft's, none when it has none (docs/adr/0167 §1, 0168 §1); the settings drop one that is not
      // another system.
      doc.settings.assign({
        ...settings,
        customCrs: ownOf(settings),
        drawingUnit: settings.srid === LOCAL_SRID && !settings.customCrs ? (settings.drawingUnit ?? 'm') : 'm',
        secondSrid: settings.secondSrid ?? null,
        secondCustomCrs: settings.secondCustomCrs ?? null,
        datumTransforms: settings.datumTransforms ?? [],
      });
      // The project's system, a definition's too (docs/adr/0168 §1).
      const own = ownOf(draft);
      if (draft.srid !== init.srid || JSON.stringify(own) !== JSON.stringify(ownOf(init)))
        ctx.log.success(`Proje koordinat sistemi ${own ? definitionTitle(own) : crsTitle(crsBySrid(draft.srid)!)} olarak atandı. Koordinat değerleri değiştirilmedi.`);
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

/** The project's own definition and its second's as the window knows them. */
interface Defined {
  own: CrsDefinition | null;
  second: CrsDefinition | null;
}

/** A draft's own definition: only without an EPSG code (docs/adr/0168 §1). */
const ownOf = (d: ProjectSettingsData): CrsDefinition | null => (d.srid === LOCAL_SRID ? (d.customCrs ?? null) : null);

/**
 * Where Özel koordinat sistemi's trial point is compared (the desktop's `custom_context`): the project's system as it was
 * saved for its own definition, the draft's own system for the second; the draft's datum choices; the project's way of
 * writing points.
 */
function trialContext(ctx: AppContext, api: DraftApi<ProjectDraft>, target: 'own' | 'second'): TrialContext {
  const named = target === 'own' ? (ownOf(api.initial) ? null : ownSystem(api.initial)) : ownSystem(api.draft);
  return {
    reference: named?.system ? { name: named.name, system: named.system } : null,
    choices: datumChoices(api.draft),
    format: { east: ctx.format.eastLabel, north: ctx.format.northLabel, decimals: api.draft.lengthDecimals, notation: ctx.prefs.geographic.value },
  };
}

/** Özel koordinat sistemi over Proje ayarları for the project's own system or its second. */
function openDefinition(ctx: AppContext, api: DraftApi<ProjectDraft>, target: 'own' | 'second', existing: CrsDefinition | null, onDone: (d: CrsDefinition) => void, onRegistry: (srid: number) => void): void {
  openCustomCrs({
    target,
    existing,
    trial: trialContext(ctx, api, target),
    onDone,
    onRegistry,
    say: (kind, text) => (kind === 'success' ? ctx.log.success(text) : ctx.log.error(text)),
  });
}

/** The picker's definition (docs/adr/0168 §6): its row, Özel sistem… under the list, Düzenle on its card. */
function ownDefined(ctx: AppContext, api: DraftApi<ProjectDraft>, defined: Defined) {
  const done = (d: CrsDefinition) => {
    defined.own = d;
    api.set('srid', LOCAL_SRID, false);
    api.set('customCrs', d);
  };
  // Kayıttakini seç: the registry's system takes the definition's place (docs/adr/0168 §1).
  const registry = (srid: number) => {
    api.set('customCrs', undefined, false);
    api.set('srid', srid);
  };
  return {
    definition: defined.own,
    chosen: ownOf(api.draft) !== null,
    was: ownOf(api.initial) !== null,
    changed: api.draft.srid !== api.initial.srid || JSON.stringify(ownOf(api.draft)) !== JSON.stringify(ownOf(api.initial)),
    onPick: () => defined.own && done(defined.own),
    onNew: () => openDefinition(ctx, api, 'own', null, done, registry),
    onEdit: () => openDefinition(ctx, api, 'own', defined.own, done, registry),
  };
}

/**
 * İkinci koordinat sistemi (docs/adr/0167 §1): the registry's systems but the project's own, grouped by datum, the
 * project's second definition (docs/adr/0168 §1) and Yok; Özel sistem… under the list defines one, Düzenle beside it
 * edits the one chosen. Its values show beside the project's in the status bar and Koordinat oku; the drawing is not
 * transformed (the desktop's `second_group`).
 */
function secondGroup(ctx: AppContext, api: DraftApi<ProjectDraft>, defined: Defined): Child {
  const project = crsBySrid(api.draft.srid);
  const custom = !!api.draft.customCrs && api.draft.srid === LOCAL_SRID;
  const what = 'Durum çubuğunda ve Koordinat oku’da projeninkilerin yanında bu sistemin değerleri de gösterilir; çizim dönüştürülmez. ED50 değerleri EPSG’nin ±2 m’lik dönüşümüyledir, resmî dönüşüm değildir.';
  if (!project || (project.kind === 'local' && !custom))
    return group('İkinci koordinat sistemi', settingRow('İkinci sistem', 'Yerel projenin ikinci sistemi olmaz; önce projeye bir koordinat sistemi atayın.', h('span', { class: 'srow__value' }, 'Yok')));
  const shown = secondAllowed(project, api.draft.secondSrid, custom) ? api.draft.secondSrid : null;
  const chosen = shown === null ? (api.draft.secondCustomCrs ?? null) : null;
  // A system of the registry, or Yok, takes the place of a second definition.
  const choose = (srid: number | undefined) => {
    api.set('secondCustomCrs', undefined, false);
    api.set('secondSrid', srid);
  };
  const done = (d: CrsDefinition) => {
    defined.second = d;
    api.set('secondSrid', undefined, false);
    api.set('secondCustomCrs', d);
  };
  const pick = new Dropdown({
    ariaLabel: 'İkinci koordinat sistemi',
    width: 260,
    items: (): MenuItem[] => [
      { label: 'Yok', radio: true, checked: shown === null && chosen === null, run: () => choose(undefined) },
      ...(defined.second ? [{ label: defined.second.name, hint: DEFINITION_CODE, radio: true, checked: chosen !== null, run: () => defined.second && done(defined.second) } satisfies MenuItem] : []),
      // At the head of the long list, where it is seen without scrolling (the desktop's list keeps it under the rows).
      { label: 'Özel sistem…', icon: 'plus', run: () => openDefinition(ctx, api, 'second', null, done, choose) },
      { kind: 'separator' },
      ...secondChoices(project).flatMap(({ datum, systems }): MenuItem[] => [
        { kind: 'header', label: datum },
        ...systems.map((c) => ({ label: c.name, hint: `EPSG:${c.srid}`, radio: true, checked: c.srid === shown, run: () => choose(c.srid) })),
      ]),
    ],
  });
  pick.set(shown !== null ? secondTitle(shown) : chosen ? definitionTitle(chosen) : 'Yok');
  let control: Child = pick.el;
  if (chosen) {
    const edit = h('button', { class: 'btn btn--small', type: 'button' }, 'Düzenle');
    edit.addEventListener('click', () => openDefinition(ctx, api, 'second', defined.second, done, choose));
    control = h('div', { class: 'second-crs__control' }, pick.el, edit);
  }
  return group('İkinci koordinat sistemi', settingRow('İkinci sistem', what, control));
}

/** Datum dönüşümleri's forms as typed, and what is wrong in each. */
interface ChoiceState {
  readonly forms: ChoiceForm[];
  readonly problems: Problems[];
}


/**
 * Datum dönüşümleri (docs/adr/0168 §3, §6): for each of the registry's three datum pairs, EPSG's way, the project's
 * seven parameters or an NTv2 grid of the device's library. What is typed is checked field by field (model/choiceForm.ts,
 * the shared cases'); while a form has something to put right, Kaydet waits (the desktop's `project/choices.rs`).
 */
function datumGroup(ctx: AppContext, api: DraftApi<ProjectDraft>, state: ChoiceState): Child {
  // The grids a choice may name: the device's library, and the ones the project's choices name that it does not have.
  const grids = (): GridRef[] => {
    const out: GridRef[] = ctx.grids.entries.value.map((e) => ({ id: e.id, file: e.file, size: e.size }));
    for (const t of api.initial.datumTransforms ?? []) if (t.grid && !out.some((g) => g.id === t.grid!.id)) out.push({ id: t.grid.id, file: t.grid.file, size: t.grid.size });
    return out;
  };
  const problemNodes: { pair: number; key: keyof Problems; el: HTMLElement }[] = [];
  const banner = note('warn', 'Datum dönüşümlerinde düzeltilecek alan var; düzeltilene dek Kaydet kapalı.');
  const paintProblems = () => {
    for (const p of problemNodes) p.el.textContent = state.problems[p.pair]![p.key] ?? '';
    banner.hidden = !state.problems.some((x) => Object.keys(x).length > 0);
  };
  // The draft's choices built again from the forms that build; a text typed keeps its field (no new page).
  const rebuild = (rerender: boolean) => {
    const list: DatumTransform[] = [];
    PAIRS.forEach((pair, i) => {
      const got = buildChoice(pair, state.forms[i]!, grids());
      if ('problems' in got) state.problems[i] = got.problems;
      else {
        state.problems[i] = {};
        if (got.choice) list.push(got.choice);
      }
    });
    api.set('datumTransforms', list, rerender);
    if (!rerender) paintProblems();
  };
  const field = (i: number, key: keyof Problems, caption: string, value: string, onChange: (v: string) => void, size: 'wide' | 'number' | 'scale' = 'number') => {
    const pairName = `${datumName(PAIRS[i]![0])} ↔ ${datumName(PAIRS[i]![1])}`;
    const input = textField({ label: `${pairName} ${caption}`, value, onChange: (v) => (onChange(v), rebuild(false)) });
    input.classList.add('datum-field__input');
    const problem = h('div', { class: 'datum-field__problem' });
    problemNodes.push({ pair: i, key, el: problem });
    return h('label', { class: `datum-field datum-field--${size}` }, h('span', { class: 'datum-field__caption' }, caption), input, problem);
  };
  const pairs = PAIRS.map((pair, i) => {
    const form = state.forms[i]!;
    const pairName = `${datumName(pair[0])} ↔ ${datumName(pair[1])}`;
    const ways = segmented<Method>({
      label: `${pairName} yöntemi`,
      value: form.method,
      options: [
        { value: 'epsg', label: 'EPSG' },
        { value: 'helmert', label: '7 parametre' },
        { value: 'grid', label: 'NTv2 ızgarası' },
      ],
      onChange: (m) => {
        // A new choice starts named by its direction, for the user to finish.
        if (form.method === 'epsg' && m !== 'epsg' && !form.name.trim()) {
          const [from, to] = form.reversed ? [pair[1], pair[0]] : pair;
          form.name = `${datumName(from)} → ${datumName(to)}: `;
        }
        form.method = m;
        rebuild(true);
      },
    });
    const head = h('div', { class: 'datum-pair__head' }, h('span', { class: 'datum-pair__name' }, pairName), ways);
    if (form.method === 'epsg') return h('div', { class: 'datum-pair' }, head, h('p', { class: 'sgroup__note' }, `EPSG'nin yolu: ${epsgText(pair)}`));
    const way = (reversed: boolean) => (reversed ? `${datumName(pair[1])} → ${datumName(pair[0])}` : `${datumName(pair[0])} → ${datumName(pair[1])}`);
    const direction = segmented<'a' | 'b'>({
      label: `${pairName} yönü`,
      value: form.reversed ? 'b' : 'a',
      options: [
        { value: 'a', label: way(false) },
        { value: 'b', label: way(true) },
      ],
      onChange: (v) => ((form.reversed = v === 'b'), rebuild(true)),
    });
    const top = h(
      'div',
      { class: 'datum-fields' },
      field(i, 'name', 'Ad', form.name, (v) => (form.name = v), 'wide'),
      h('div', { class: 'datum-field' }, h('span', { class: 'datum-field__caption' }, 'Yön'), direction),
    );
    const accuracy = field(i, 'accuracy', 'Doğruluk (m)', form.accuracy, (v) => (form.accuracy = v));
    if (form.method === 'helmert') {
      // The translations on a row, the rotations and the scale difference under them (the desktop's rows).
      const parameter = (k: Parameter) => field(i, k, PARAMETER_CAPTIONS[k], form.parameters[k], (v) => (form.parameters[k] = v), k === 'ds' ? 'scale' : 'number');
      const parameters = [h('div', { class: 'datum-fields' }, PARAMETERS.slice(0, 3).map(parameter)), h('div', { class: 'datum-fields' }, PARAMETERS.slice(3).map(parameter))];
      const rule = segmented<Convention>({
        label: `${pairName} dönüklüklerin kuralı`,
        value: form.convention,
        options: CONVENTIONS,
        onChange: (c) => ((form.convention = c), rebuild(true)),
      });
      const bottom = h('div', { class: 'datum-fields' }, h('div', { class: 'datum-field' }, h('span', { class: 'datum-field__caption' }, 'Dönüklüklerin kuralı'), rule), accuracy);
      return h('div', { class: 'datum-pair' }, head, top, parameters, bottom);
    }
    const list = grids();
    const chosen = list.find((g) => g.id === form.grid);
    const pick = new Dropdown({
      ariaLabel: `${pairName} ızgarası`,
      width: 300,
      items: (): MenuItem[] => list.map((g) => ({ label: g.file, hint: g.id.slice(0, 12), radio: true, checked: g.id === form.grid, run: () => ((form.grid = g.id), rebuild(true)) })),
    });
    pick.set(chosen ? chosen.file : 'Izgara seçin…');
    const gridProblem = h('div', { class: 'datum-field__problem' });
    problemNodes.push({ pair: i, key: 'grid', el: gridProblem });
    const bottom = h('div', { class: 'datum-fields' }, h('div', { class: 'datum-field datum-field--wide' }, h('span', { class: 'datum-field__caption' }, 'Izgara'), pick.el, gridProblem), accuracy);
    return h('div', { class: 'datum-pair' }, head, top, bottom);
  });
  paintProblems();
  return group(
    'Datum dönüşümleri',
    h('p', { class: 'sgroup__note' }, "Kayıttaki datumlar arasında EPSG'nin yolu yerine projenin seçimi: yedi parametre ya da bu cihazdaki bir NTv2 ızgarası. Seçim yalnız kendi çiftini değiştirir; ters yönü aynı dönüşümün tersidir."),
    banner,
    pairs,
  );
}

/**
 * Izgaralar (docs/adr/0168 §4, §6): the device's NTv2 grids, each with what its header says and Kaldır (asked), the
 * grids the project's datum choices name that this device does not have, and Ekle…. The grids are the device's, not
 * the project's; the rows are drawn again as the library changes (the desktop's `grids_group`).
 */
function gridsGroup(ctx: AppContext, api: DraftApi<ProjectDraft>): Child {
  const list = h('div', { class: 'grid-library' });
  const paint = () => {
    const entries = ctx.grids.entries.value;
    const missing = (api.draft.datumTransforms ?? []).flatMap((t) => (t.grid && !entries.some((e) => e.id === t.grid!.id) ? [t.grid.file] : []));
    replaceChildren(
      list,
      entries.map((e) => {
        const remove = h('button', { class: 'btn btn--small', type: 'button' }, 'Kaldır');
        remove.addEventListener('click', async () => {
          const yes = await askRemove({
            title: 'Izgara kaldırılsın mı?',
            message: `“${e.file}” bu cihazın ızgara kitaplığından silinecek.`,
            details: ['Onu anan projelerin datum seçimi, ızgara yeniden eklenene dek değer vermez.'],
            action: 'Kaldır',
          });
          if (!yes) return;
          await ctx.grids.remove(e.id);
          ctx.log.success(`“${e.file}” ızgara kitaplığından kaldırıldı.`);
          await ctx.grids.follow(api.draft.datumTransforms ?? [], (text) => ctx.log.warn(text));
          paint();
        });
        return settingRow(e.file, gridLine(e), remove);
      }),
      missing.map((file) => note('warn', `${file}: projenin datum seçimi bu ızgarayı istiyor, bu cihazda yok. Aynı dosyayı Ekle… ile ekleyin.`)),
      entries.length || missing.length ? null : h('p', { class: 'sgroup__note' }, 'Bu cihazda NTv2 ızgarası yok.'),
    );
  };
  const input = h('input', { type: 'file', accept: '.gsb,.GSB', hidden: true }) as HTMLInputElement;
  const add = h('button', { class: 'btn btn--small', type: 'button' }, 'Ekle…');
  add.addEventListener('click', () => input.click());
  input.addEventListener('change', async () => {
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    const got = await ctx.grids.add(file.name, new Uint8Array(await file.arrayBuffer()));
    if ('error' in got) ctx.log.error(got.error);
    else ctx.log.success(`“${got.file}” ızgara kitaplığına eklendi (${got.from} → ${got.to}); projelerin datum seçimleri onu kullanabilir.`);
    paint();
  });
  paint();
  void ctx.grids.refresh().then(paint);
  return group(
    'Izgaralar',
    h('p', { class: 'sgroup__note' }, "NTv2 ızgaraları bu cihazda saklanır, projeyle paylaşılmaz: projenin datum seçimi onları SHA-256'larıyla anar, projeyi açan başka cihaza da eklenmeleri gerekir."),
    list,
    h('div', { class: 'grid-library__actions' }, add, input),
  );
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
