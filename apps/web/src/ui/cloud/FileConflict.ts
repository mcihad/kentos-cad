import { offer, REVISION_TEXTS, type Offer, type RevisionAnswer } from '../../app/cloud/fileRevisionsPlan';
import type { AppContext } from '../../app/context';
import { confirmDialog } from '../widgets/confirm';
import { openUploadDialog } from './UploadDialog';

/**
 * A file project's Kaydet that met a newer revision, and the newer
 * revision someone else saved while the project is open (docs/adr/0038,
 * docs/specs/file-revisions.md; TODOS.md SYNC-06). Two files are never
 * merged byte by byte: the user chooses where the drawing goes — a separate
 * copy (a new file project, or a local file) — or opens the newest revision
 * and drops the drawing's changes. Nothing is reloaded by itself. Which
 * question comes, its words and what each answer does are
 * app/cloud/fileRevisionsPlan.ts's; this file asks and does it.
 */

/** Asks what the plan offers and does the answer. True when the drawing ended up saved or replaced by the newest revision. */
async function run(ctx: AppContext, o: Offer): Promise<boolean> {
  if (o.kind === 'none') return false;
  if (o.kind === 'say') {
    ctx.log[o.tone](o.line);
    return false;
  }
  const q = o.question;
  const value = await confirmDialog({
    title: q.title,
    message: q.message,
    details: q.details,
    answers: q.answers.map(({ value, label, kind, aside }) => ({ value, label, kind, aside })),
    cancel: q.cancel,
  });
  const answer = q.answers.find((a) => a.value === value);
  return answer ? act(ctx, answer) : false;
}

/** What an answer does (fileRevisionsPlan.ts `AnswerDoes`, `AnswerWork`). */
async function act(ctx: AppContext, a: RevisionAnswer): Promise<boolean> {
  const cloud = ctx.cloud;
  const file = cloud.file.value;
  const p = cloud.project.value;
  if (!file || !p) return false;
  switch (a.does) {
    case 'copy':
      // The upload window, on file storage, named as a copy: it says how far it is and what went wrong.
      openUploadDialog(ctx, { storage: 'file', name: `${p.name} (kopya)`, tenantId: p.tenantId });
      return false;
    case 'local': {
      const saved = await ctx.files.saveAs();
      if (saved && cloud.file.value === file) {
        cloud.detach();
        ctx.log.info(REVISION_TEXTS.detached(p.name));
      }
      return saved;
    }
    case 'latest':
      // Dropped on purpose: their recovery copy goes too (app/recovery.ts).
      if (a.work === 'dropped' && ctx.doc.dirty.value) ctx.recovery.discard();
      try {
        return await cloud.open(p.tenantId, p.projectId);
      } catch (e) {
        ctx.log.error(REVISION_TEXTS.openFailed(p.name, e instanceof Error ? e.message : String(e)));
        return false;
      }
    default:
      return false;
  }
}

/** The question after a refused Kaydet, or Kayıt çakışmalarını çöz… on a file project. True when the drawing ended up saved. */
export async function resolveFileConflict(ctx: AppContext): Promise<boolean> {
  const file = ctx.cloud.file.value;
  const p = ctx.cloud.project.value;
  if (!file || !p) return false;
  return run(ctx, offer({ name: p.name, s: file.revisions, busy: file.busy, via: 'conflict' }));
}

/**
 * The newest revision offered: the save cell's click on a newer revision,
 * or Son revizyonu aç…. Over unsaved work it is the conflict's question
 * (the work needs a place first); over unsaved work with no newer revision
 * known, the unsaved question; over a clean drawing, a plain question.
 */
export async function offerNewest(ctx: AppContext): Promise<boolean> {
  const file = ctx.cloud.file.value;
  const p = ctx.cloud.project.value;
  if (!file || !p) return false;
  return run(ctx, offer({ name: p.name, s: file.revisions, busy: file.busy, via: 'newest' }));
}
