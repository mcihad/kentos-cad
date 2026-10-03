/**
 * Which project a book of sheets belongs to (docs/sheet/design.md §10,
 * tasks-web A2): the key the book is kept under on this device until `.kcad`
 * carries the sheets (integration.md §6). A cloud project is named by its
 * workspace and id; a drawing by its lasting project id (KCAD v2 keeps it, a
 * v1 file's is derived, docs/adr/0014); a file without one by its name; a new
 * drawing that has none of these by this session only, and the interface
 * says its sheets are kept for the session until the drawing is saved.
 */

export interface ProjectKey {
  /** The store's key. */
  readonly id: string;
  /** Whether the key names the project again after a reload (a session's does not). */
  readonly lasting: boolean;
  /** What the key is, in words, for the tab strip's tooltip. */
  readonly label: string;
}

export interface ProjectIdentity {
  readonly cloud: { readonly tenantId: string; readonly projectId: string; readonly name: string } | null;
  /** The drawing's lasting project id (CadDocument.projectId). */
  readonly projectId: string | null;
  /** The file the drawing was opened from or saved to. */
  readonly fileName: string | null;
  /** A key for this session, made once when the app starts. */
  readonly session: string;
}

export function projectKeyOf(p: ProjectIdentity): ProjectKey {
  if (p.cloud) return { id: `bulut/${p.cloud.tenantId}/${p.cloud.projectId}`, lasting: true, label: `bulut projesi “${p.cloud.name}”` };
  if (p.projectId) return { id: `proje/${p.projectId}`, lasting: true, label: 'çizimin proje kimliği' };
  if (p.fileName) return { id: `dosya/${p.fileName}`, lasting: true, label: `“${p.fileName}” dosyası` };
  return { id: `oturum/${p.session}`, lasting: false, label: 'bu oturum (çizim kaydedilmedi)' };
}

/**
 * What to do with the book when the key may have changed: another drawing
 * put on screen loads its own (even under the same key: it was read again);
 * a new drawing that got its lasting name takes its book along; a drawing
 * that got another lasting name (Farklı kaydet, uploaded as a cloud project)
 * takes a copy along and the old name keeps its own.
 */
export function keyChange(before: ProjectKey | null, after: ProjectKey, drawingReplaced: boolean): 'keep' | 'move' | 'copy' | 'load' {
  if (drawingReplaced || !before) return 'load';
  if (before.id === after.id) return 'keep';
  return before.lasting ? 'copy' : 'move';
}
