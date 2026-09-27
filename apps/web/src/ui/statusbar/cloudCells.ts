import type { FileProjectSave } from '../../app/cloud/fileProject';
import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { DisposableStore } from '../../core/disposable';
import { h } from '../dom';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { FILE_SAVE_TEXT, accountRows, databaseTip, fileTip, saveCellAction, saveCellView, type FileSave, type SaveCellInput } from './cellsPlan';

/**
 * Status bar cells of the cloud (CLAUDE.md §21.1): the save state of an open
 * cloud project, with what is waiting, and the account menu behind the
 * server cell. "Buluta kaydedildi" is shown only after the server's answer.
 * A file project (docs/adr/0038, TODOS.md SYNC-04) shows its Kaydet's
 * stages apart: the drawing being written, the upload with how far, the
 * server checking and committing, and the revision once it is saved. What
 * they say and do is cellsPlan.ts's; this file draws it.
 */

/** A file project's Kaydet as the cell reads it now. */
const fileSaveOf = (f: FileProjectSave): FileSave => ({
  kind: 'file',
  state: f.state.value,
  base: f.base.value,
  progress: f.progress.value,
  conflictActual: f.conflict.value?.actual ?? null,
  newerRevision: f.newer.value?.revision ?? null,
});

/** The open file project's Kaydet state in words (the app menu uses it too). */
export const fileSaveText = (f: FileProjectSave): string => {
  const s = fileSaveOf(f);
  return FILE_SAVE_TEXT[s.state](s);
};

/** What the save cell reads: the open file project's Kaydet, or the database project's save, or nothing. */
function saveInput(ctx: AppContext): SaveCellInput {
  const file = ctx.cloud.file.value;
  if (file) return fileSaveOf(file);
  const sync = ctx.cloud.sync.value;
  if (!sync) return { kind: 'none' };
  return { kind: 'database', state: sync.state.value, pending: sync.pending.value, conflicts: sync.conflicts.value.length };
}

/** The save cell: hidden without a cloud project; a click does the next useful thing. */
export function saveCell(ctx: AppContext, d: DisposableStore): HTMLElement {
  const text = h('span', { class: 'status__save-text' });
  const cell = h('button', { class: 'status__cell status__btn status__save', type: 'button', hidden: true }, h('span', { class: 'status__lamp', 'aria-hidden': 'true' }), text);
  // Subscriptions to the current project's sync; replaced when another project opens.
  let per: (() => void)[] = [];
  const drop = () => {
    for (const u of per) u();
    per = [];
  };
  d.add(drop);
  const render = () => {
    const view = saveCellView(saveInput(ctx));
    cell.hidden = view.hidden;
    if (view.hidden) return;
    text.textContent = view.text;
    cell.dataset.state = view.state ?? '';
  };
  d.add(
    ctx.cloud.sync.subscribe((sync) => {
      drop();
      if (sync) per = [sync.state.subscribe(render), sync.pending.subscribe(render), sync.conflicts.subscribe(render)];
      render();
    }, true),
  );
  // A database project has an autosave, a file project a Kaydet; never both at once.
  d.add(
    ctx.cloud.file.subscribe((file) => {
      drop();
      if (file) per = [file.state.subscribe(render), file.progress.subscribe(render), file.base.subscribe(render), file.newer.subscribe(render)];
      render();
    }, true),
  );
  cell.addEventListener('click', () => {
    const command = saveCellAction(saveInput(ctx));
    if (command) ctx.commands.execute(command);
  });
  d.add(
    tooltip(
      cell,
      () => {
        const sync = ctx.cloud.sync.value;
        const file = ctx.cloud.file.value;
        const p = ctx.cloud.project.value;
        const where = p ? `${p.tenantName} › ${p.name}` : '';
        const link = ctx.cloud.link.value;
        if (file && p) {
          const last = file.lastSaved.value;
          return fileTip({
            where,
            base: file.base.value,
            lastSaved: last && { revision: last.revision, at: last.at },
            now: Date.now(),
            newer: file.newer.value,
            error: file.error.value,
            link,
            dirty: ctx.doc.dirty.value,
          });
        }
        if (!sync || !p) return null;
        return databaseTip({ state: sync.state.value, where, lastSaved: sync.lastSaved.value, now: Date.now(), link, error: sync.error.value, durableDrafts: ctx.cloud.durableDrafts });
      },
      'top',
    ),
  );
  return cell;
}

/** The server cell's menu: the account and the cloud commands (cellsPlan.ts `accountRows`). */
export function accountMenu(ctx: AppContext, anchor: HTMLElement): void {
  const me = ctx.cloud.me.value;
  const p = ctx.cloud.project.value;
  const rows = accountRows({
    user: me?.user.displayName ?? null,
    project: p,
    disabled: (id) => !ctx.commands.isEnabled(id),
    may: (permission) => ctx.cloud.may(permission),
  });
  PopupMenu.open(
    rows.map((r): MenuItem => ('command' in r ? { ...commandItem(ctx, r.command), ...(r.detail ? { detail: r.detail } : {}) } : r.kind === 'header' ? { kind: 'header', label: r.label } : { kind: 'separator' })),
    anchor.getBoundingClientRect(),
    { placement: 'below', owner: anchor },
  );
}
