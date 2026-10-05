import '../../styles/io.css';
import type { AppContext } from '../../app/context';
import { TEXT_ALIGNS, textAlignName, type TextAlign } from '../../model/entities';
import { LINE_TYPE_LABEL, type LayerNode, type LineType } from '../../model/layers';
import { TEMPLATE_METHODS, TEMPLATE_TOOL_LABEL, TEMPLATE_TOOLS, type TemplateTool } from '../../model/objectTemplate';
import type { LibraryTemplate } from '../../model/style';
import { EMPTY_TEMPLATE_FORM, formOfTemplate, templateFromForm, type TemplateForm } from '../../model/templateForm';
import { newItemId } from '../../style/library';
import { templateSymbol } from '../../style/templateSymbol';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { field, select, summaryLine, type Choice } from '../io/common';
import { DRAW_COLORS } from '../ribbon/fields';
import { drawNow } from '../style/thumbs';
import { segmented } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { PopupMenu } from '../widgets/PopupMenu';

/**
 * Şablon düzenleyici (docs/adr/0176 §4): a template's name, category and description, the tool it draws with and its
 * method, the layer it draws on (typed, or chosen from the drawing's, with the look it is opened with), the objects'
 * own colour, weight and symbol (chosen in the Stil yöneticisi), their attributes, label, and the point's, text's or
 * block's own fields; a picture of what it draws. The form's rules are model/templateForm.ts's: the problems show
 * under the fields as they are typed, and Kaydet waits until there are none. A new template goes to Kitaplığım or the
 * project; an edited one stays where it is; a system template is copied to Kitaplığım. The desktop's window
 * (`apps/desktop/src/template_editor.rs`) is the same.
 */
export function openTemplateEditor(ctx: AppContext, opts: { id?: string; form?: TemplateForm; to?: 'user' | 'project' } = {}): void {
  const lib = ctx.styles.library;
  const editing = opts.id ? lib.template(opts.id) : undefined;
  // A system template is not changed: its copy goes to Kitaplığım.
  const inPlace = editing && editing.source !== 'system' ? editing : undefined;
  const form: TemplateForm = structuredClone(opts.form ?? (editing ? formOfTemplate(editing) : newForm(ctx)));
  let to: 'user' | 'project' = inPlace ? (inPlace.source as 'user' | 'project') : (opts.to ?? 'user');

  const body = h('div', { class: 'tpl-editor' });
  const issues = h('div', { role: 'status' });
  const picture = h('canvas', { class: 'tpl-editor__picture', width: '72', height: '72', style: 'width:72px;height:72px', 'aria-hidden': 'true' });
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const title = inPlace ? 'Şablonu düzenle' : editing ? 'Şablonun kopyasını düzenle' : 'Yeni şablon';
  const dialog = new Dialog({ title, width: 640, className: 'dialog--io', content: [body, issues], footer: [h('div', { class: 'dialog__spacer' }), cancel, save] });

  /** The problems under the fields, the picture of what it draws and Kaydet, after every change. */
  const refresh = () => {
    const made = templateFromForm(form);
    save.disabled = 'issues' in made;
    replaceChildren(issues, ...('issues' in made ? made.issues.map((i) => summaryLine('error', i)) : []));
    if ('template' in made) drawNow(ctx, picture, templateSymbol(made.template, lib));
  };
  const text = (label: string, key: keyof TemplateForm, placeholder = '', size: 'auto' | 'grow' | 'wide' = 'grow') => {
    const input = h('input', { class: 'field', value: String(form[key]), placeholder, 'aria-label': label, spellcheck: 'false', dataset: { key } });
    input.addEventListener('input', () => {
      (form as unknown as Record<string, string>)[key] = input.value;
      refresh();
    });
    return field(label, input, undefined, size);
  };
  const choice = <T extends string>(label: string, key: keyof TemplateForm, choices: readonly Choice<T>[], rebuild = false) => {
    const value = String(form[key]) as T;
    // A value the list does not have (a template's own colour) is shown as itself.
    const all = choices.some((c) => c.value === value) ? choices : [...choices, { value, label: value.toUpperCase() }];
    return field(
      label,
      select(label, all, value, (v) => {
        (form as unknown as Record<string, string>)[key] = v;
        if (rebuild) render();
        else refresh();
      }, String(key)),
    );
  };
  const colours = (none: string): Choice<string>[] => [{ value: '', label: none }, ...DRAW_COLORS.map((c) => ({ value: c.value, label: c.name }))];

  /** The layer typed, or chosen from the drawing's with the look it has. */
  const layerFrom = (button: HTMLElement) => {
    const r = button.getBoundingClientRect();
    const items = ctx.doc.layers.leaves().map((n) => {
      const groups = groupsOf(ctx, n);
      return {
        label: [...groups, n.name].join(' / '),
        icon: 'layers',
        run: () => {
          Object.assign(form, { layerGroups: groups.join(' / '), layerName: n.name, layerColor: n.style.color, layerLineType: n.style.lineType, layerWeight: String(n.style.lineWeight) });
          render();
        },
      };
    });
    PopupMenu.open(items.length ? items : [{ label: 'Çizimde katman yok', disabled: true, run: () => {} }], { x: r.left, y: r.bottom + 4 });
  };

  /** Seç: the Stil yöneticisi over the window, in pick mode for the tool's kind of symbol. */
  const pickSymbol = () =>
    void import('../style/StyleManager').then((m) =>
      m.openStyleManager(ctx, {
        stack: true,
        pick: {
          kind: kindOf(form.tool),
          title: 'Şablonun sembolünü seçin',
          current: form.symbol || undefined,
          onPick: (id) => {
            form.symbol = id;
            render();
          },
        },
      }),
    );

  const render = () => {
    const methods = ctx.tools.get(form.tool)?.methods;
    const symbolName = form.symbol ? (lib.get(form.symbol)?.name ?? form.symbol) : 'Katmanın stili';
    const layerButton = h('button', { class: 'btn btn--small', type: 'button', title: 'Çizimdeki bir katmanı seçin' }, icon('layers', 14), 'Çizimden', icon('chevronDown', 12));
    layerButton.addEventListener('click', () => layerFrom(layerButton));
    const symbolButton = h('button', { class: 'btn btn--small', type: 'button' }, icon('symbolAssign', 14), 'Seç…');
    symbolButton.addEventListener('click', pickSymbol);
    const clearSymbol = h('button', { class: 'btn btn--small', type: 'button', disabled: !form.symbol, title: 'Katmanın stiliyle çizilsin' }, icon('close', 14), 'Kaldır');
    clearSymbol.addEventListener('click', () => {
      form.symbol = '';
      render();
    });
    const rows = form.attrs.map(([key, value], i) => {
      const k = h('input', { class: 'field', value: key, placeholder: 'Ad', 'aria-label': `${i + 1}. özniteliğin adı`, spellcheck: 'false' });
      const v = h('input', { class: 'field', value, placeholder: 'Değer', 'aria-label': `${i + 1}. özniteliğin değeri`, spellcheck: 'false' });
      k.addEventListener('input', () => ((form.attrs[i][0] = k.value), refresh()));
      v.addEventListener('input', () => ((form.attrs[i][1] = v.value), refresh()));
      const remove = h('button', { class: 'ibtn', type: 'button', title: 'Satırı sil', 'aria-label': `${i + 1}. satırı sil` }, icon('trash', 14));
      remove.addEventListener('click', () => {
        form.attrs.splice(i, 1);
        render();
      });
      return h('div', { class: 'tpl-editor__attr' }, k, v, remove);
    });
    const addRow = h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), 'Satır ekle');
    addRow.addEventListener('click', () => {
      form.attrs.push(['', '']);
      render();
      body.querySelector<HTMLInputElement>('.tpl-editor__attr:last-of-type input')?.focus();
    });
    const toolChoices: Choice<TemplateTool>[] = TEMPLATE_TOOLS.map((t) => ({ value: t, label: TEMPLATE_TOOL_LABEL[t] }));
    const own: HTMLElement[] = [];
    if (form.tool === 'point') own.push(h('div', { class: 'io-row' }, text('İlk ad', 'pointName', 'Ad her noktada artar: P1, P2…'), text('Kod', 'pointCode', 'İsteğe bağlı')));
    if (form.tool === 'text') {
      const alignChoices: Choice<'' | TextAlign>[] = [{ value: '', label: 'sol taban' }, ...TEXT_ALIGNS.map((a) => ({ value: a, label: textAlignName(a) }))];
      const mask = h('input', { type: 'checkbox', checked: form.textMask, dataset: { key: 'textMask' } });
      mask.addEventListener('change', () => ((form.textMask = mask.checked), refresh()));
      own.push(h('div', { class: 'io-row' }, text('Yükseklik (mm)', 'textHeight', '2.5', 'auto'), choice('Hiza', 'textAlign', alignChoices), field('Zemin', h('label', { class: 'io-check' }, mask, 'Yazının arkası boyansın'))));
    }
    if (form.tool === 'blockInsert') {
      const names = ctx.doc.blocks.value.map((b) => b.name);
      own.push(choice('Blok', 'block', [{ value: '', label: names.length ? 'Blok seçin' : 'Çizimde blok yok' }, ...names.map((n) => ({ value: n, label: n }))]));
    }
    const where: HTMLElement[] = [];
    if (!inPlace) {
      const seg = segmented<'user' | 'project'>({
        label: 'Kaydet',
        options: [
          { value: 'user', label: 'Kitaplığım' },
          { value: 'project', label: 'Proje' },
        ],
        value: to,
        onChange: (v) => {
          to = v;
          render();
        },
      });
      where.push(field('Kaydet', seg, to === 'user' ? 'Bu tarayıcıda, bütün çizimlerde.' : 'Proje dosyasında; projeyi açan herkes görür.'));
    }
    replaceChildren(
      body,
      h('div', { class: 'tpl-editor__head' }, h('div', { class: 'tpl-editor__fields' }, h('div', { class: 'io-row' }, text('Ad', 'name'), text('Kategori', 'category', 'Kadastro / Sınırlar')), text('Açıklama', 'description', 'İsteğe bağlı: şablonun ne çizdiği')), picture),
      h(
        'div',
        { class: 'io-row' },
        choice('Araç', 'tool', toolChoices, true),
        TEMPLATE_METHODS[form.tool] && methods ? choice('Yöntem', 'method', methods.map((m) => ({ value: m.option ?? '', label: m.label }))) : null,
      ),
      h('h4', { class: 'tpl-editor__section' }, 'Katman'),
      h('div', { class: 'tpl-editor__layer' }, text('Gruplar', 'layerGroups', 'Kadastro / Tapu'), text('Katman', 'layerName'), field(' ', layerButton)),
      h(
        'div',
        { class: 'io-row' },
        choice('Rengi', 'layerColor', colours('Varsayılan')),
        choice('Çizgi tipi', 'layerLineType', [{ value: '' as '' | LineType, label: 'Varsayılan' }, ...(Object.keys(LINE_TYPE_LABEL) as LineType[]).map((t) => ({ value: t, label: LINE_TYPE_LABEL[t] }))]),
        text('Kalınlığı (mm)', 'layerWeight', 'Varsayılan', 'auto'),
      ),
      h('p', { class: 'io-field__hint' }, 'Katman çizimde yoksa bu görünüşle, gruplarının içinde açılır.'),
      h('h4', { class: 'tpl-editor__section' }, 'Görünüş'),
      h(
        'div',
        { class: 'io-row' },
        choice('Renk', 'color', colours('Katmana göre')),
        text('Kalınlık (mm)', 'weight', 'Katmana göre', 'auto'),
        field('Sembol', h('div', { class: 'tpl-editor__symbol' }, h('span', { class: 'tpl-editor__symbol-name', title: symbolName }, symbolName), symbolButton, clearSymbol)),
      ),
      h('h4', { class: 'tpl-editor__section' }, 'Öznitelikler ve etiket'),
      h('div', { class: 'tpl-editor__attrs' }, ...rows, addRow),
      text('Etiket', 'label', 'Nesnenin yanında yazılacak metin'),
      ...own,
      ...where,
    );
    refresh();
  };

  cancel.addEventListener('click', () => dialog.close());
  save.addEventListener('click', () => {
    const made = templateFromForm(form);
    if ('issues' in made) return;
    const fields = { name: made.name, path: [...made.path], description: made.description, template: made.template };
    if (inPlace) {
      lib.update(inPlace.id, fields);
      ctx.log.success(`“${made.name}” şablonu kaydedildi.`);
    } else {
      const item: LibraryTemplate = { kind: 'template', id: newItemId(to === 'project' ? 'p' : 'u'), name: made.name, path: made.path, template: made.template, ...(made.description && { description: made.description }) };
      lib.add(to, item);
      ctx.log.success(`“${made.name}” şablonu ${to === 'user' ? 'Kitaplığım’a' : 'projenin kitaplığına'} kaydedildi.`);
    }
    dialog.close();
  });
  render();
  body.querySelector<HTMLInputElement>('input[data-key="name"]')?.select();
}

/** A new template's form: Kapalı alan on the active layer. */
function newForm(ctx: AppContext): TemplateForm {
  const layer = ctx.doc.layers.get(ctx.doc.layers.active.value);
  return { ...structuredClone(EMPTY_TEMPLATE_FORM), name: 'Yeni şablon', layerGroups: layer ? groupsOf(ctx, layer).join(' / ') : '', layerName: layer?.name ?? '' };
}

/** The names of the groups above a layer, from the top. */
function groupsOf(ctx: AppContext, n: LayerNode): string[] {
  const out: string[] = [];
  for (let g = ctx.doc.layers.parentOf(n.id); g; g = ctx.doc.layers.parentOf(g.id)) out.unshift(g.name);
  return out;
}

/** The kind of symbol a tool's objects take: a mark for points and blocks, a line for lines, a fill for areas. */
function kindOf(tool: TemplateTool): 'marker' | 'line' | 'fill' | undefined {
  if (tool === 'point' || tool === 'blockInsert') return 'marker';
  if (tool === 'line' || tool === 'polyline') return 'line';
  if (tool === 'polygon' || tool === 'rectangle' || tool === 'rectangle3' || tool === 'circle') return 'fill';
  return undefined;
}
