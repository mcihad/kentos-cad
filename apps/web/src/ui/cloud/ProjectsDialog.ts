import type { AppContext } from '../../app/context';
import { openCatalog } from './CatalogDialog';
import { openUploadDialog } from './UploadDialog';

/**
 * The cloud's two project windows (`cloud.open`, `cloud.upload`,
 * `cloud.uploadFile`): the catalog, where projects are found, opened and
 * managed (CatalogDialog.ts, docs/adr/0028), and the upload of the open
 * drawing as a new project (UploadDialog.ts; as a file project from the
 * start with `uploadFile`). `pick`: a project to select in the catalog (the
 * application menu's recent projects, the open project's history);
 * `tab: 'history'` shows its history (docs/adr/0038).
 */
export function openProjectsDialog(ctx: AppContext, mode: 'open' | 'upload' | 'uploadFile', pick?: { tenantId: string; projectId: string }, tab?: 'history'): void {
  if (mode === 'upload') openUploadDialog(ctx);
  else if (mode === 'uploadFile') openUploadDialog(ctx, { storage: 'file' });
  else openCatalog(ctx, pick, tab);
}
