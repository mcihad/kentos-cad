import type { AppContext } from '../../app/context';
import { WGS84_SYSTEM, crsTransformIn, type DatumChoice, type System, type Transformed, type Unreached } from '../../model/geom/crsTransform';
import { crsSettings, datumChoices, ownSystem } from '../../model/projectCrs';

/**
 * The move of an export's objects into WGS 84 (docs/adr/0222 §4, §5, §7; the desktop's `wgs84_from` and `wgs84_way`):
 * the project's system as the transforms read it with its datum choices, and how the move goes at the middle of the
 * objects written, which the windows say before writing.
 */

/** The project's system as the transforms read it and its datum choices; null without one. */
export function projectSystem(ctx: AppContext): { readonly from: System; readonly choices: DatumChoice[] } | null {
  const settings = crsSettings(ctx.doc.settings);
  const from = ownSystem(settings)?.system ?? null;
  return from ? { from, choices: datumChoices(settings) } : null;
}

/** How the move into WGS 84 goes at the middle of the objects `ids`: its accuracy, or why not; null without a system or objects. */
export function wayToWgs84(ctx: AppContext, ids: readonly number[]): Transformed | { readonly error: Unreached } | null {
  const own = projectSystem(ctx);
  const b = ids.length ? ctx.doc.bounds(ids) : null;
  if (!own || !b) return null;
  return crsTransformIn(own.from, WGS84_SYSTEM, { x: (b.minX + b.maxX) / 2, y: (b.minY + b.maxY) / 2 }, own.choices);
}
