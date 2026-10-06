import type { AppContext } from '../../app/context';
import { fixed } from '../../core/displayNumber';
import { turnOf } from '../../model/blocks';
import type { Entity } from '../../model/entities';
import { hasPicture } from '../../product/entitiesEdit';
import { pictureItem, PICTURE_FILES } from '../../tools/pictureFile';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometry } from './write';

type ImageEntity = Extract<Entity, { kind: 'image' }>;

/**
 * A picture's rows in Öznitelikler (docs/adr/0192 §4), the desktop's `properties::rows` the same: its source (Göm for
 * a linked one), its place, width and height (each keeping its shape), turn (degrees typed), see-through share (0 to
 * 90 %), clip and mirror. They write through `cad.entities.edit`'s properties.
 */
export function imageRows(ctx: AppContext, e: ImageEntity, locked: boolean): PropRow[] {
  const f = ctx.format;
  const numEdit = (patch: (x: number) => Record<string, unknown> | null) =>
    locked
      ? undefined
      : ({
          type: 'number',
          commit: (t: string) => {
            const x = parseFloat(t.replace(',', '.'));
            const changed = Number.isFinite(x) ? patch(x) : null;
            if (changed) setGeometry(ctx, e, changed);
          },
        } as const);
  const num = (label: string, v: number, unit?: string): PropRow => ({ label, value: f.length(v, false), numeric: true, unit: unit === 'm' ? f.lengthUnitLabel : unit });
  const linked = e.asset === undefined && e.file !== undefined;
  const source = sourceWords(ctx, e);
  // A picture keeps its shape: its width sets its height and the other way round.
  const sized = (k: number) => (k > 0 && Number.isFinite(k) ? { width: e.width * k, height: e.height * k } : null);
  return [
    {
      label: 'Kaynak',
      value: source,
      editor:
        linked && !locked
          ? { type: 'select', display: () => ({ text: source }), items: () => [{ label: 'Göm', run: () => void embed(ctx, e) }] }
          : undefined,
    },
    { ...num('Konum Y', e.p.x), editor: numEdit((x) => ({ p: { ...e.p, x: f.toMetres(x) } })) },
    { ...num('Konum X', e.p.y), editor: numEdit((y) => ({ p: { ...e.p, y: f.toMetres(y) } })) },
    { ...num('Genişlik', e.width, 'm'), editor: numEdit((x) => (x > 0 ? sized(f.toMetres(x) / e.width) : null)) },
    { ...num('Yükseklik', e.height, 'm'), editor: numEdit((x) => (x > 0 ? sized(f.toMetres(x) / e.height) : null)) },
    { label: 'Dönüş', value: fixed((e.rotation * 180) / Math.PI, 4), numeric: true, unit: '°', editor: numEdit((x) => ({ rotation: turnOf(x) })) },
    {
      label: 'Saydamlık',
      value: fixed((1 - (e.opacity ?? 1)) * 100, 0),
      numeric: true,
      unit: '%',
      editor: numEdit((x) => (x >= 0 && x <= 90 ? { opacity: x > 0 ? 1 - x / 100 : undefined } : null)),
    },
    { label: 'Kırpma', value: e.clip ? 'Var' : 'Yok' },
    { label: 'Aynalı', value: e.mirror ? 'Evet' : 'Hayır' },
  ];
}

/** Kaynak: an embedded picture's name and size in pixels, a linked one's file (the browser cannot read it). */
function sourceWords(ctx: AppContext, e: ImageEntity): string {
  if (e.asset !== undefined) {
    const it = ctx.doc.styles.value.items.find((i) => i.kind === 'asset' && i.id === e.asset);
    return it?.kind === 'asset' ? `Gömülü: ${it.name} (${it.width}×${it.height} piksel)` : 'Gömülü (kitaplıkta yok)';
  }
  if (e.file !== undefined) return `Bağlı: ${e.file.split(/[\\/]/).pop() || e.file} (tarayıcıda okunamaz)`;
  return '—';
}

/**
 * Göm (docs/adr/0192 §2): the browser cannot read a linked picture's file, so it asks for it; the picture chosen is
 * kept in the project's library and the picture made embedded, one step “Değiştir”.
 */
async function embed(ctx: AppContext, e: ImageEntity): Promise<void> {
  const picked = await ctx.files.pickForImport(PICTURE_FILES);
  if (!picked) return;
  const got = await pictureItem(picked.name, picked.bytes);
  if (!got.ok) return void ctx.log.warn(got.error);
  const doc = ctx.doc;
  if (!hasPicture(doc, got.id)) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, got.item] });
  const now = doc.get(e.id);
  if (now?.kind === 'image') setGeometry(ctx, now, { asset: got.id, file: undefined });
}
