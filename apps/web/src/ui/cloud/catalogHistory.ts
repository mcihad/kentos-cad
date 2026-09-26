import type { AppContext } from '../../app/context';
import { when } from '../../app/cloud/catalog';
import { KIND_LABEL, pointText, whyNotCreate, whyNotDelete, whyNotDownload, whyNotTake, type HistoryData } from '../../app/cloud/history';
import { sizeText } from '../../app/cloud/transfer';
import type { Checkpoint } from '../../contracts/generated/Checkpoint';
import type { FileRevision } from '../../contracts/generated/FileRevision';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { h, type Child } from '../dom';
import { icon } from '../icons';

/**
 * The selected project's history in the catalog (docs/adr/0034, 0038;
 * TODOS.md SYNC-11, CLOUD-07): a file project's revisions — number, who,
 * when, size and objects — and every project's named checkpoints — name,
 * note, kind, the revision it shows, who, when and objects. Each can be
 * downloaded and restored as a new project; a checkpoint is removed by its
 * maker or someone who manages the project, and a new one is named here.
 * An action the account may not take stays visible, disabled, and says
 * why. Nothing here asks the server: the catalog does, and redraws (also
 * when a `project.checkpoint` or `project.file` event arrives).
 */

/** The history while it is on its way, or why it is not there. */
export type HistoryState = HistoryData | 'loading' | 'none' | Error;

export interface HistoryActions {
  downloadRevision(r: FileRevision): void;
  restoreRevision(r: FileRevision): void;
  createCheckpoint(): void;
  downloadCheckpoint(c: Checkpoint): void;
  restoreCheckpoint(c: Checkpoint): void;
  deleteCheckpoint(c: Checkpoint): void;
  retry(): void;
}

/** A small button of a history row; an action the account may not take stays visible, disabled, and says why. */
export function rowButton(label: string, iconName: string, run: () => void, why: string | null, danger = false): HTMLButtonElement {
  const b = h('button', { class: `btn btn--small${danger ? ' btn--danger' : ''}`, type: 'button', disabled: !!why, title: why ?? null }, icon(iconName, 14), label);
  b.addEventListener('click', run);
  return b;
}

const objectsText = (n?: string) => (n ? ` · ${Number(n).toLocaleString('tr-TR')} nesne` : '');

function revisionRow(p: ProjectSummary, r: FileRevision, newest: boolean, actions: HistoryActions): HTMLElement {
  const perms = p.access.permissions;
  return h(
    'div',
    { class: 'catalog-history__row', role: 'listitem', dataset: { revision: r.revision } },
    h(
      'div',
      { class: 'catalog-history__main' },
      h('span', { class: 'catalog-history__title' }, `Revizyon ${r.revision}`, newest ? h('span', { class: 'catalog-chip catalog-chip--open' }, 'En yeni') : null),
      h('span', { class: 'catalog-history__meta' }, `${r.createdByName || 'görünmüyor'} · ${when(r.createdAt)}`),
      h('span', { class: 'catalog-history__meta num' }, `${sizeText(r.size)}${objectsText(r.objects)}`),
    ),
    h(
      'div',
      { class: 'catalog-history__acts' },
      rowButton('İndir', 'export', () => actions.downloadRevision(r), whyNotDownload(perms)),
      rowButton('Yeni proje olarak geri yükle…', 'history', () => actions.restoreRevision(r), whyNotTake(perms, 'geri yükleme')),
    ),
  );
}

function checkpointRow(ctx: AppContext, p: ProjectSummary, c: Checkpoint, actions: HistoryActions): HTMLElement {
  const perms = p.access.permissions;
  return h(
    'div',
    { class: 'catalog-history__row', role: 'listitem', dataset: { checkpoint: c.id } },
    h(
      'div',
      { class: 'catalog-history__main' },
      h('span', { class: 'catalog-history__title' }, c.name, h('span', { class: 'catalog-chip' }, KIND_LABEL[c.kind])),
      c.note ? h('span', { class: 'catalog-history__note' }, c.note) : null,
      h('span', { class: 'catalog-history__meta' }, `${pointText(c)} · ${c.createdByName || 'görünmüyor'} · ${when(c.createdAt)}`),
      h('span', { class: 'catalog-history__meta num' }, `${sizeText(Number(c.size))}${objectsText(c.objects)}`),
    ),
    h(
      'div',
      { class: 'catalog-history__acts' },
      rowButton('İndir', 'export', () => actions.downloadCheckpoint(c), whyNotTake(perms, 'indirme')),
      rowButton('Yeni proje olarak geri yükle…', 'history', () => actions.restoreCheckpoint(c), whyNotTake(perms, 'geri yükleme')),
      rowButton('Sil…', 'trash', () => actions.deleteCheckpoint(c), whyNotDelete(c, ctx.cloud.me.value?.user.id, perms, p.state === 'archived'), true),
    ),
  );
}

/** The history tab's content for `p`. */
export function renderHistory(ctx: AppContext, p: ProjectSummary, state: HistoryState, actions: HistoryActions): Child[] {
  if (state === 'loading') return [h('p', { class: 'catalog-details__muted' }, 'Geçmiş yükleniyor…')];
  if (state instanceof Error) {
    const retry = h('button', { class: 'btn btn--small', type: 'button' }, 'Yeniden dene');
    retry.addEventListener('click', () => actions.retry());
    return [h('div', { class: 'catalog-history__error' }, h('p', null, `Geçmiş okunamadı: ${state.message}`), retry)];
  }
  if (state === 'none') return [h('p', { class: 'catalog-details__muted' }, 'Bu projenin geçmişi gösterilemiyor.')];
  const perms = p.access.permissions;
  const out: Child[] = [
    h(
      'div',
      { class: 'catalog-history__tools' },
      rowButton('Kontrol noktası oluştur…', 'plus', () => actions.createCheckpoint(), whyNotCreate(perms, p.state === 'archived', state.storage, state.revisions)),
    ),
  ];
  // The checkpoints: named points of the history (docs/adr/0034).
  const checkpoints = state.checkpoints;
  out.push(h('h4', { class: 'catalog-history__head' }, 'Kontrol noktaları', checkpoints ? h('span', { class: 'catalog-history__count' }, String(checkpoints.length)) : null));
  if (!checkpoints) out.push(h('p', { class: 'catalog-details__muted' }, whyNotTake(perms, 'geçmişi görme') ?? 'Kontrol noktaları okunamadı.'));
  else if (!checkpoints.length)
    out.push(
      h(
        'p',
        { class: 'catalog-details__muted' },
        state.storage === 'file'
          ? 'Henüz kontrol noktası yok: bir revizyona ad vermek için “Kontrol noktası oluştur”.'
          : 'Henüz kontrol noktası yok: projenin şimdiki hâlini saklamak için “Kontrol noktası oluştur”.',
      ),
    );
  else out.push(h('div', { class: 'catalog-history__list', role: 'list', 'aria-label': 'Kontrol noktaları' }, checkpoints.map((c) => checkpointRow(ctx, p, c, actions))));
  // A file project's revisions; a database project keeps none.
  if (state.revisions) {
    const list = state.revisions.revisions;
    out.push(
      h('h4', { class: 'catalog-history__head' }, 'Revizyonlar', h('span', { class: 'catalog-history__count' }, String(list.length))),
      list.length
        ? h('div', { class: 'catalog-history__list', role: 'list', 'aria-label': 'Revizyonlar' }, list.map((r) => revisionRow(p, r, r.revision === state.revisions!.current, actions)))
        : h('p', { class: 'catalog-details__muted' }, 'Henüz kaydedilmiş revizyon yok: ilk Kaydet 1. revizyonu yazar.'),
    );
  } else
    out.push(
      h(
        'p',
        { class: 'catalog-history__note' },
        'Bu proje nesne nesne veritabanında saklanıyor; revizyon dosyaları yoktur. Geçmişi kontrol noktalarıdır; şimdiki hâlini tek bir dosya olarak almak için “.kcad olarak indir”.',
      ),
    );
  return out;
}
