import type { AppContext } from '../../app/context';
import { openCatalog } from './CatalogDialog';
import { openUploadDialog } from './UploadDialog';

/**
 * The cloud's two project windows (`cloud.open`, `cloud.upload`): the
 * catalog, where projects are found, opened and managed (CatalogDialog.ts,
 * docs/adr/0028), and the upload of the open drawing as a new project
 * (UploadDialog.ts). `pick`: a project to select in the catalog (the
 * application menu's recent projects).
 */
export function openProjectsDialog(ctx: AppContext, mode: 'open' | 'upload', pick?: { tenantId: string; projectId: string }): void {
  if (mode === 'upload') openUploadDialog(ctx);
  else openCatalog(ctx, pick);
}
