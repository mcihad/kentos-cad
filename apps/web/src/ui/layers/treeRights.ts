import type { AppContext } from '../../app/context';

/**
 * Whether the layer tree may change in the open drawing. In a cloud
 * database project without project.edit (a project Editor: objects, not
 * the tree), a new, renamed or removed layer could not be saved, and an
 * object drawn on a new one would never reach the server (it refuses an
 * object on a layer it does not have). So those changes are refused where
 * they start. The eye, the lock, the open state, the active layer and the
 * layer style stay the user's own, as before. A file project and a local
 * drawing save the tree with the file.
 */

/** The refusal, in the words the Katmanlar panel and the commands say it. */
export const TREE_LOCKED = 'Bu projede katman ağacını değiştirme yetkiniz yok (project.edit); proje sahibinden ya da yöneticisinden isteyin.';

/** Why the layer tree may not change here, or null when it may. */
export function treeLocked(ctx: AppContext): string | null {
  const p = ctx.cloud.project.value;
  return p && p.storage === 'database' && !p.canEditMeta ? TREE_LOCKED : null;
}
