import type { Entity, EntityGeometry, NewEntity } from '../../model/entities';
import { checkValue } from '../../model/layerFields';
import { joinPlan } from '../../model/ops/statistics';
import { sameJson } from '../../model/sameJson';
import { defineTool, type FieldMap, type FieldMapRow, type Shown } from '../types';
import { missing, sourceTable } from './joinSource';

/**
 * Veri aktar (docs/adr/0216; ArcGIS Append with matching fields, QGIS Refactor fields and Join attributes by field value,
 * Netcad Veri Aktar): a source's rows (another layer's objects, or a CSV, TXT or Excel file's rows) go into the target
 * layer's objects whose key matches, through a field map (each target field's source column or default), typed fields
 * by their rules; optionally the source objects' geometry, and the rows no object took as new objects. A report says
 * what happened to each object and row; Yalnız raporla changes nothing. Keys match as Anahtarla birleştir's
 * (`ops::statistics::join_plan`). One undo step.
 */

/** The report's columns. */
const COLUMNS = ['Anahtar', 'Sonuç', 'Değişiklikler', 'Not'];

/** The most rows the report has: past it the run is refused. */
const MOST_ROWS = 1_000_000;

/** An attribute's value as a change shows it: none or empty is “(boş)”. */
const shown = (v: string | undefined): string => (v === undefined || v === '' ? '(boş)' : v);

/** An object's geometry without what is not geometry (its labels' pins too). */
function geometryOf(e: Entity): Record<string, unknown> {
  const { id: _i, uid: _u, layerId: _l, attrs: _a, color: _c, label: _t, symbol: _s, lineWeight: _w, labelPins: _p, kind: _k, ...g } = e as Entity & { labelPins?: unknown };
  return g;
}

/** The map the run uses: the one given, else every source column but the key under its own name. */
function mapOf(map: FieldMap, header: readonly string[], k: number): FieldMap {
  if (map.length) return map;
  return header.flatMap((h, i): FieldMapRow[] => (i !== k && h && header.indexOf(h) === i ? [{ target: h, source: h, default: '' }] : []));
}

export const transfer = defineTool({
  id: 'attributes.transfer',
  label: 'Veri aktar',
  category: 'attributes',
  icon: 'dataTransfer',
  description: 'Başka bir katmandan ya da CSV, TXT, Excel dosyasından ortak anahtarla öznitelikleri alan eşlemesiyle aktarır; isterse geometriyi günceller, eşleşmeyenleri ekler.',
  help: [
    'Örnekler: tapu listesindeki malik, hisse ve yüzölçümünü parsel numarasıyla parsellere aktarmak; başka bir ekibin güncel ölçümünü mevcut katmana geometrisiyle işlemek, yeni parselleri eklemek; koordinat listesinden noktaları adlarıyla güncellemek ve yenilerini eklemek.',
    'Anahtarlar baştaki ve sondaki boşluklar atılarak karşılaştırılır; iki taraf da sayıysa sayı olarak (002 ile 2 aynı). Kaynakta yinelenen anahtarın ilk satırı alınır; hedefte aynı anahtarlı nesnelerin hepsi güncellenir.',
    'Alan eşleme her hedef alana bir kaynak sütun ya da varsayılan değer verir: hücre boşsa varsayılan yazılır. Eşleme boşsa anahtardan başka bütün sütunlar kendi adlarıyla aktarılır. Hedef katmanın alanı tipliyse değer alanın kuralıyla yazılır; uymayan değer yazılmaz ve raporda nedeni söylenir.',
    'Eşleşmeyenleri ekle açıkken hiçbir nesnenin almadığı satırlar hedef katmana eklenir: katmandan geometrisiyle, dosyadan Doğu ve Kuzey sütunlarından nokta olarak. Yalnız raporla açıkken çizim değişmez; rapor ne olacağını gösterir.',
  ].join('\n\n'),
  keywords: ['veri aktar', 'aktar', 'anahtar', 'alan eşleme', 'eşle', 'güncelle', 'ekle', 'append', 'upsert', 'field map', 'refactor', 'tapu', 'csv', 'excel'],
  aliases: ['VERIAKTAR', 'EKLEGUNCELLE'],
  targets: ['client', 'worker'],
  parameters: [
    { name: 'target', label: 'Hedef katman', type: 'features', scopes: ['layer'], writes: true, allowEmpty: true, description: 'Güncellenecek nesnelerin katmanı; eşleşmeyen satırlar da buna eklenir.' },
    { name: 'targetKey', label: 'Hedef anahtar', type: 'field', of: 'target', allowNew: true, description: 'Hedef nesnelerin anahtar alanı (parsel numarası, kimlik).' },
    {
      name: 'sourceKind',
      label: 'Kaynak',
      type: 'enum',
      options: [
        { value: 'file', label: 'Dosya', hint: 'CSV, TXT ya da Excel dosyası' },
        { value: 'layer', label: 'Katman', hint: 'Başka nesnelerin öznitelikleri ve geometrisi' },
      ],
      default: 'file',
    },
    { name: 'file', label: 'Dosya', type: 'file', accept: ['.csv', '.txt', '.xlsx', '.ods'], visibleWhen: (v: Shown) => v.sourceKind !== 'layer', description: 'İlk satırı sütun adları olan CSV, TXT, Excel (.xlsx) ya da ODS dosyası.' },
    { name: 'source', label: 'Kaynak nesneler', type: 'features', scopes: ['layer', 'all', 'visible', 'selection'], visibleWhen: (v: Shown) => v.sourceKind === 'layer', description: 'Özniteliklerin ve geometrinin alınacağı nesneler.' },
    { name: 'sourceKey', label: 'Kaynak anahtar', type: 'field', of: ['source', 'file'], optional: true, description: 'Boş bırakılırsa hedef anahtarla aynı ad.' },
    {
      name: 'map',
      label: 'Alan eşleme',
      type: 'fieldMap',
      target: 'target',
      source: ['source', 'file'],
      key: ['sourceKey', 'targetKey'],
      description: 'Her hedef alana hangi kaynak sütun ya da hücre boşken hangi değer; boşsa anahtardan başka bütün sütunlar kendi adlarıyla.',
    },
    {
      name: 'existing',
      label: 'Var olan değerler',
      type: 'enum',
      options: [
        { value: 'overwrite', label: 'Üzerine yaz', hint: 'Hedefteki değer kaynağınkiyle değişir; değeri olmayan alan boşalır' },
        { value: 'empty', label: 'Yalnız boşlara', hint: 'Hedefte dolu olan değer kalır' },
      ],
      default: 'overwrite',
    },
    { name: 'geometry', label: 'Geometriyi güncelle', type: 'boolean', default: false, visibleWhen: (v: Shown) => v.sourceKind === 'layer', description: 'Eşleşen nesne kaynağın geometrisini alır (türleri aynıysa).' },
    { name: 'append', label: 'Eşleşmeyenleri ekle', type: 'boolean', default: false, description: 'Hiçbir nesnenin almadığı satırlar hedef katmana yeni nesne olarak eklenir.' },
    { name: 'east', label: 'Doğu sütunu', type: 'field', of: 'file', optional: true, visibleWhen: (v: Shown) => v.sourceKind !== 'layer' && v.append === true, description: 'Eklenen noktanın doğu koordinatı (CBS projesinde Y).' },
    { name: 'north', label: 'Kuzey sütunu', type: 'field', of: 'file', optional: true, visibleWhen: (v: Shown) => v.sourceKind !== 'layer' && v.append === true, description: 'Eklenen noktanın kuzey koordinatı (CBS projesinde X).' },
    { name: 'reportOnly', label: 'Yalnız raporla', type: 'boolean', default: false, description: 'Çizim değişmez; rapor ne olacağını gösterir.' },
  ] as const,
  outputs: [
    { name: 'table', label: 'Aktarım raporu', type: 'table' },
    { name: 'changed', label: 'Güncellenen nesneler', type: 'features' },
    { name: 'count', label: 'Güncellenen nesne sayısı', type: 'number' },
    { name: 'added', label: 'Eklenen nesne sayısı', type: 'number' },
  ],
  run: async (v, ctx, feedback) => {
    const targets = v.target.entities;
    const layerId = v.target.layerId ?? targets[0]?.layerId ?? '';
    const fromFile = v.sourceKind !== 'layer';
    const objects = fromFile ? null : (v.source?.entities ?? []);
    const { header, rows } = sourceTable(objects, fromFile ? v.file : null);
    const key = (v.sourceKey ?? '').trim() || v.targetKey;
    const k = header.indexOf(key);
    if (k < 0) return { refused: missing(key, header) };
    const map = mapOf(v.map, header, k);
    for (const m of map) if (m.source.trim() && !header.includes(m.source.trim())) return { refused: missing(m.source.trim(), header) };
    const east = (v.east ?? '').trim();
    const north = (v.north ?? '').trim();
    if (v.append && fromFile) {
      if (!east || !north) return { refused: 'Dosyadan nesne eklemek için Doğu ve Kuzey sütunlarını seçin.' };
      for (const c of [east, north]) if (!header.includes(c)) return { refused: missing(c, header) };
    }
    feedback.progress(0, 'Anahtarlar eşleniyor');
    const cell = (r: number, name: string): string => rows[r][header.indexOf(name)] ?? '';
    const value = (r: number, m: FieldMapRow): string | null => {
      if (m.source.trim()) {
        const c = cell(r, m.source.trim());
        if (c.trim() !== '') return c;
      }
      return m.default.trim() !== '' ? m.default : null;
    };
    const plan = joinPlan(
      targets.map((t) => (Object.hasOwn(t.attrs, v.targetKey) ? t.attrs[v.targetKey] : null)),
      rows.map((r) => r[k] ?? ''),
    );
    const layerFields = ctx.fields(layerId);
    const fieldOf = (name: string) => layerFields.find((f) => f.name === name);
    const report: string[][] = [];
    const update: { id: number; patch: Partial<Entity> }[] = [];
    const add: NewEntity[] = [];
    let misfits = 0;
    let updated = 0;
    let unmatched = 0;
    targets.forEach((t, n) => {
      const keyText = (Object.hasOwn(t.attrs, v.targetKey) ? t.attrs[v.targetKey] : '').trim();
      const r = plan.matches[n];
      if (r === null || r === undefined) {
        unmatched++;
        report.push([keyText, 'Eşleşmedi', '', keyText ? '' : 'Anahtarı boş.']);
        return;
      }
      const attrs: Record<string, string> = { ...t.attrs };
      const changes: string[] = [];
      const notes: string[] = [];
      for (const m of map) {
        const name = m.target.trim();
        const old = Object.hasOwn(attrs, name) ? attrs[name] : undefined;
        if (v.existing === 'empty' && old !== undefined && old.trim() !== '') continue;
        let text = value(r, m);
        const f = fieldOf(name);
        if (text === null) {
          if (old === undefined) continue;
          if (f) {
            const c = checkValue(f, '');
            if ('error' in c) {
              notes.push(c.message);
              misfits++;
              continue;
            }
          }
          delete attrs[name];
          changes.push(`${name}: ${shown(old)} → (boş)`);
          continue;
        }
        if (f) {
          const c = checkValue(f, text);
          if ('error' in c) {
            notes.push(c.message);
            misfits++;
            continue;
          }
          text = c.value;
        }
        if (old === text) continue;
        attrs[name] = text;
        changes.push(`${name}: ${shown(old)} → ${text}`);
      }
      let geometry: Record<string, unknown> | null = null;
      if (v.geometry && objects) {
        const src = objects[r];
        if (src.kind !== t.kind) notes.push('Geometrinin türü farklı; geometri güncellenmedi.');
        else if (!sameJson(geometryOf(src), geometryOf(t))) {
          // The source's geometry whole: what the target had and the source has not goes.
          const g = geometryOf(src);
          geometry = Object.fromEntries([...Object.keys(geometryOf(t)).map((key) => [key, undefined] as const), ...Object.entries(g)]);
          changes.push('geometri');
        }
      }
      if (changes.length) {
        updated++;
        update.push({ id: t.id, patch: { ...(sameJson(attrs, t.attrs) ? {} : { attrs }), ...(geometry ?? {}) } as Partial<Entity> });
      }
      report.push([keyText, changes.length ? 'Güncellendi' : 'Değişmedi', changes.join('; '), notes.join(' ')]);
    });
    // The source rows no object took, in their order; a row's key matched against the rows themselves gives the first
    // row of that key (one pass, the core's rule).
    const used = new Set(plan.matches.filter((m): m is number => m !== null && m !== undefined));
    const keys = rows.map((row) => row[k] ?? '');
    const firsts = joinPlan(keys, keys).matches;
    let added = 0;
    let notAdded = 0;
    let unusedRows = 0;
    for (let r = 0; r < rows.length; r++) {
      if (used.has(r)) continue;
      const keyText = (rows[r][k] ?? '').trim();
      if (!keyText) {
        unusedRows++;
        report.push(['', 'Kullanılmadı', '', 'Anahtarı boş.']);
        continue;
      }
      if (firsts[r] !== r) {
        unusedRows++;
        report.push([keyText, 'Kullanılmadı', '', 'Anahtar kaynakta yineleniyor; ilk satır alındı.']);
        continue;
      }
      if (!v.append) {
        unusedRows++;
        report.push([keyText, 'Kullanılmadı', '', '']);
        continue;
      }
      // A new object, in this order: the key, the map's values, the layer fields' defaults, the required fields, the
      // geometry; a key its field does not take, a required field left empty or an unread point leaves it out.
      const attrs: Record<string, string> = {};
      const written: string[] = [];
      const notes: string[] = [];
      let ok = true;
      const keyField = fieldOf(v.targetKey);
      const keyCheck = keyField ? checkValue(keyField, keyText) : { value: keyText };
      if ('error' in keyCheck) {
        notes.push(keyCheck.message);
        ok = false;
      } else {
        attrs[v.targetKey] = keyCheck.value;
        written.push(`${v.targetKey}: ${keyCheck.value}`);
      }
      for (const m of map) {
        const name = m.target.trim();
        if (name === v.targetKey) continue;
        let text = value(r, m);
        if (text === null) continue;
        const f = fieldOf(name);
        if (f) {
          const c = checkValue(f, text);
          if ('error' in c) {
            notes.push(c.message);
            misfits++;
            continue;
          }
          text = c.value;
        }
        attrs[name] = text;
        written.push(`${name}: ${text}`);
      }
      for (const f of layerFields) {
        if (Object.hasOwn(attrs, f.name) || (f.default ?? '').trim() === '') continue;
        const c = checkValue(f, f.default ?? '');
        if ('value' in c && c.value !== '') {
          attrs[f.name] = c.value;
          written.push(`${f.name}: ${c.value}`);
        }
      }
      for (const f of layerFields) {
        if (f.required && f.name !== v.targetKey && (attrs[f.name] ?? '').trim() === '') {
          const c = checkValue(f, '');
          if ('error' in c) notes.push(c.message);
          ok = false;
        }
      }
      let shape: EntityGeometry | null = null;
      if (objects) shape = { kind: objects[r].kind, ...geometryOf(objects[r]) } as EntityGeometry;
      else {
        const x = checkValue({ name: 'Doğu', kind: 'decimal' }, cell(r, east).trim());
        const y = checkValue({ name: 'Kuzey', kind: 'decimal' }, cell(r, north).trim());
        if ('error' in x || 'error' in y || !cell(r, east).trim() || !cell(r, north).trim()) {
          notes.push('Doğu ya da kuzey okunamadı; nokta eklenmedi.');
          ok = false;
        } else shape = { kind: 'point', p: { x: Number(x.value), y: Number(y.value) } } as EntityGeometry;
      }
      if (!ok || !shape) {
        notAdded++;
        report.push([keyText, 'Eklenmedi', '', notes.join(' ')]);
        continue;
      }
      added++;
      add.push({ ...shape, layerId, attrs } as NewEntity);
      report.push([keyText, 'Eklendi', written.join('; '), notes.join(' ')]);
    }
    if (report.length > MOST_ROWS) return { refused: `Rapor ${report.length} satır olurdu (en çok ${MOST_ROWS}); kaynağı ya da hedefi daraltın.` };
    if (plan.repeated) feedback.warn(`${plan.repeated} anahtar kaynakta birden çok kez var; ilk satırları alındı.`);
    if (misfits) feedback.warn(`${misfits} değer alanına uymadığı için yazılmadı; nedenleri raporda.`);
    if (notAdded) feedback.warn(`${notAdded} kaynak satırı eklenemedi; nedenleri raporda.`);
    const summary = `${updated} nesne güncellendi, ${added} eklendi; ${unmatched} hedefin eşi yok, ${unusedRows + notAdded} kaynak satırı kullanılmadı.`;
    const table = { columns: COLUMNS, rows: report };
    if (v.reportOnly) return { outputs: { table, count: 0, added: 0 }, summary: `Yalnız rapor, çizim değişmedi: ${summary}` };
    return {
      changes: { update, add },
      outputs: { table, changed: update.map((u) => u.id), count: updated, added },
      summary,
    };
  },
});
