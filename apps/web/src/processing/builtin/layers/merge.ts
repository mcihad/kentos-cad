import type { NewEntity } from '../../../model/entities';
import { checkValue } from '../../../model/layerFields';
import { defineTool } from '../../types';
import { copyOf } from './shared';

/**
 * Vektör katmanlarını birleştir (docs/adr/0217 §3; QGIS Merge vector layers, ArcGIS Merge): the chosen layers' objects
 * copied into one layer, the layers in the tree's order, each in the drawing's order; a field map (docs/adr/0216 §2)
 * gives the copies' attributes, else they keep their own; the source layer's name goes into a field. Into a layer with
 * typed fields the values go by their rules, its defaults fill what is left empty, and an object with a required field
 * left empty is not added. One undo step.
 */

export const mergeLayers = defineTool({
  id: 'layers.merge',
  label: 'Vektör katmanlarını birleştir',
  category: 'layers',
  icon: 'layersMergeVector',
  description: 'Seçilen katmanların nesnelerini alan eşlemesiyle tek katmanda toplar; kaynak katmanın adını bir alana yazar.',
  help: [
    'Örnekler: paftalara bölünmüş parsel katmanlarını tek parsel katmanında toplamak; farklı ekiplerin bina katmanlarını ortak alan adlarıyla birleştirmek.',
    'Nesneler kopyalanır, girdiler değişmez: katmanlar ağaçtaki sırasıyla, her katmanda çizim sırasıyla. Alan eşleme boşsa nesneler bütün öznitelikleriyle gelir; doluysa yalnız eşlemenin alanlarıyla (hücre boşsa varsayılan). Kaynak katman alanı doluysa nesnenin geldiği katmanın adı oraya yazılır.',
    'Sonuç alanları tipli bir katmansa değerler alanların kuralıyla yazılır: uymayan değer yazılmaz, katmanın varsayılanları boş alanlara gelir, zorunlu alanı boş kalan nesne eklenmez. Kilitli katmanın nesneleri alınmaz.',
  ].join('\n\n'),
  keywords: ['birleştir', 'katman', 'topla', 'alan eşleme', 'merge', 'merge vector layers', 'append', 'pafta'],
  aliases: ['KATMANTOPLA'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'layers', label: 'Katmanlar', type: 'layers', min: 2, description: 'Birleştirilecek katmanlar; ağaçtaki sırasıyla okunur.' },
    {
      name: 'map',
      label: 'Alan eşleme',
      type: 'fieldMap',
      target: 'layers',
      source: ['layers'],
      description: 'Her sonuç alanına hangi kaynak alan ya da hücre boşken hangi değer; boşsa bütün öznitelikler kendi adlarıyla.',
    },
    { name: 'sourceField', label: 'Kaynak katman alanı', type: 'field', of: 'layers', allowNew: true, optional: true, default: 'Kaynak katman', description: 'Nesnenin geldiği katmanın adı bu alana yazılır; boşsa yazılmaz.' },
    { name: 'output', label: 'Sonuç', type: 'layer', default: { newName: 'Birleşik' }, newLayerStyle: { color: '#8E4EC6' } },
  ] as const,
  outputs: [
    { name: 'added', label: 'Birleşik nesneler', type: 'features' },
    { name: 'count', label: 'Nesne sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const out = v.output;
    const fields = out.isNew ? [] : ctx.fields(out.id);
    const fieldOf = new Map(fields.map((f) => [f.name, f]));
    const sourceField = (v.sourceField ?? '').trim();
    const add: NewEntity[] = [];
    let misfits = 0;
    let notAdded = 0;
    feedback.progress(0, 'Nesneler kopyalanıyor');
    for (const layer of v.layers) {
      for (const e of layer.entities) {
        let attrs: Record<string, string>;
        if (v.map.length) {
          attrs = {};
          for (const m of v.map) {
            const source = m.source.trim();
            const cell = source && Object.hasOwn(e.attrs, source) ? e.attrs[source] : '';
            const value = cell.trim() !== '' ? cell : m.default.trim() !== '' ? m.default : null;
            if (value !== null) attrs[m.target.trim()] = value;
          }
        } else attrs = { ...e.attrs };
        if (sourceField) attrs[sourceField] = layer.name;
        // Into typed fields: the values by their rules, the defaults, the required fields.
        const kept: Record<string, string> = {};
        for (const [name, value] of Object.entries(attrs)) {
          const f = fieldOf.get(name);
          if (!f) {
            kept[name] = value;
            continue;
          }
          const c = checkValue(f, value);
          if ('error' in c) misfits++;
          else kept[name] = c.value;
        }
        for (const f of fields) {
          if ((kept[f.name] ?? '').trim() !== '' || (f.default ?? '').trim() === '') continue;
          const c = checkValue(f, f.default ?? '');
          if ('value' in c && c.value !== '') kept[f.name] = c.value;
        }
        if (fields.some((f) => f.required && (kept[f.name] ?? '').trim() === '')) {
          notAdded++;
          continue;
        }
        add.push(copyOf(e, out.id, kept));
      }
    }
    if (misfits) feedback.warn(`${misfits} değer alanına uymadığı için yazılmadı.`);
    if (notAdded) feedback.warn(`${notAdded} nesne zorunlu alanı boş kaldığı için eklenmedi.`);
    return {
      changes: { add },
      outputs: { count: add.length },
      summary: `${v.layers.length} katmandan ${add.length} nesne “${out.name}” katmanına birleştirildi.`,
    };
  },
});
