import type { PolylineEntity } from '../../model/entities';
import { pathCounts } from '../../model/pathCounts';
import type { PropRow } from '../widgets/PropertyGrid';

/**
 * Öznitelikler' rows that count a path or an area (docs/adr/0143): a multi-part area's corners, parts and islands
 * are every part's, its own ring being the first part. Length and area come from the core, part-aware already.
 */

/** 'Köşe sayısı' (every part's outer vertices) and, right after it, 'Parça sayısı' when the area has parts. */
export function cornerRows(e: Pick<PolylineEntity, 'pts' | 'holes' | 'parts'>): PropRow[] {
  const { corners, parts } = pathCounts(e);
  const rows: PropRow[] = [{ label: 'Köşe sayısı', value: String(corners), numeric: true }];
  if (e.parts?.length) rows.push({ label: 'Parça sayısı', value: String(parts), numeric: true });
  return rows;
}

/** 'Ada (delik)': the holes of every part; none when there are none. */
export function holeRows(e: Pick<PolylineEntity, 'pts' | 'holes' | 'parts'>): PropRow[] {
  const { holes } = pathCounts(e);
  return holes ? [{ label: 'Ada (delik)', value: String(holes), numeric: true }] : [];
}
