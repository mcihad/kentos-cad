/**
 * The plot scale or the project's annotation heights changed from the interface (docs/adr/0205 §3): the ribbon's
 * Ölçek, Proje ayarları' Kaydet and Başka çizimden al. The texts, leaders, dimensions and tables still at the old
 * height take the new one (`followAnnotationScale`, the shared rule), in one undo step “Yazı yüksekliklerini uydur”
 * (`cad.entities.edit`'s `annotationScale`); what was changed by hand stays; those on a locked layer are not written
 * and are counted. The setting itself is an edit of the project, not an undo step (as the style tables are). The
 * desktop's is `annotation_scale.rs`.
 */
import type { AnnotationHeights } from '../model/annotationScale';
import { followAnnotationScale, scaleChangeIsNone, type ScaleChange } from '../model/annotationScale';
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import { geometryOf } from '../product/entitiesEdit';
import { uidOf, writeEdit } from '../tools/editCommand';
import type { AppContext } from './context';

/** A scale as the interface writes it: “1:25.000”. */
export const scaleText = (scale: number): string => `1:${scale.toLocaleString('tr-TR')}`;

/** What following a change did: the objects written and those a locked layer kept. */
export interface Followed {
  readonly written: number;
  readonly locked: number;
}

/**
 * The annotations at the old height given the new one, in one step (none: nothing to follow). Said in the log: how
 * many followed, and how many a locked layer kept.
 */
export function followAnnotationChange(ctx: AppContext, change: ScaleChange): Followed {
  if (scaleChangeIsNone(change)) return { written: 0, locked: 0 };
  const doc = ctx.doc;
  const textStyles = doc.settings.textStyles.value;
  const dimensionStyles = doc.settings.dimensionStyles.value;
  const changes: EntityEdit[] = [];
  let locked = 0;
  for (const e of doc.all()) {
    const next = followAnnotationScale(e, change, textStyles, dimensionStyles);
    if (!next) continue;
    if (doc.layers.isLocked(e.layerId)) {
      locked++;
      continue;
    }
    changes.push({ kind: 'update', uid: uidOf(ctx, e), geometry: geometryOf(next as unknown as EditGeometry) as unknown as EditGeometry });
  }
  const written = changes.length && writeEdit(ctx, 'annotationScale', changes) ? changes.length : 0;
  const where = change.fromScale === change.toScale ? '' : ` (${scaleText(change.fromScale)} → ${scaleText(change.toScale)})`;
  if (written) ctx.log.success(`Yazı yükseklikleri uyduruldu${where}: ${written} nesne yeni boyunda.`);
  if (locked) ctx.log.warn(`Kilitli katmanlardaki ${locked} nesnenin yüksekliği değişmedi; kilidi açıp ölçeği yeniden seçebilirsiniz.`);
  return { written, locked };
}

/** The project's plot scale set to `scale` (the ribbon's Ölçek, Ölçek yaz…), its annotations following it. */
export function setPlotScale(ctx: AppContext, scale: number): Followed {
  const s = ctx.doc.settings;
  const from = s.plotScale.value;
  if (!(Number.isFinite(scale) && scale > 0) || from === scale) return { written: 0, locked: 0 };
  const heights: AnnotationHeights | undefined = s.annotation.value ?? undefined;
  s.assign({ plotScale: scale });
  return followAnnotationChange(ctx, { fromScale: from, toScale: scale, from: heights, to: heights });
}
