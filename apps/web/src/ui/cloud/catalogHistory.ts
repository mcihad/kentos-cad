import type { AppContext } from '../../app/context';
import { when } from '../../app/cloud/catalog';
import { sizeText } from '../../app/cloud/transfer';
import type { FileRevision } from '../../contracts/generated/FileRevision';
import type { FileRevisions } from '../../contracts/generated/FileRevisions';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { h, type Child } from '../dom';
import { icon } from '../icons';

/**
 * The selected project's history in the catalog (docs/adr/0038, TODOS.md
 * CLOUD-07): a file project's revisions, newest first — number, who, when,
 * size and objects — each downloadable. A database project keeps no
 * revision files; its `.kcad` of one moment is “.kcad olarak indir”.
 * Nothing here asks the server: the catalog does, and redraws.
 */

export interface HistoryData {
  /** A file project's revisions; null for a database project. */
  revisions: FileRevisions | null;
}

/** The history while it is on its way, or why it is not there. */
export type HistoryState = HistoryData | 'loading' | 'none' | Error;

export interface HistoryActions {
  downloadRevision(r: FileRevision): void;
  retry(): void;
}

const may = (p: ProjectSummary, permission: ProjectPermission) => p.access.permissions.includes(permission);

/** A small button of a history row; an action the account may not take stays visible, disabled, and says why. */
export function rowButton(label: string, iconName: string, run: () => void, why: string | null, danger = false): HTMLButtonElement {
  const b = h('button', { class: `btn btn--small${danger ? ' btn--danger' : ''}`, type: 'button', disabled: !!why, title: why ?? null, 'aria-label': label }, icon(iconName, 14), label);
  b.addEventListener('click', run);
  return b;
}

/** Why `permission` is missing in `p`, or null. */
export function needs(p: ProjectSummary, permission: ProjectPermission, what: string): string | null {
  return may(p, permission) ? null : `“${p.name}” projesinde ${what} yetkiniz yok (${permission}); proje sahibine ya da yöneticisine başvurun.`;
}

function revisionRow(p: ProjectSummary, r: FileRevision, newest: boolean, actions: HistoryActions): HTMLElement {
  const objects = r.objects ? ` · ${Number(r.objects).toLocaleString('tr-TR')} nesne` : '';
  return h(
    'div',
    { class: 'catalog-history__row', role: 'listitem', dataset: { revision: r.revision } },
    h(
      'div',
      { class: 'catalog-history__main' },
      h('span', { class: 'catalog-history__title' }, `Revizyon ${r.revision}`, newest ? h('span', { class: 'catalog-chip catalog-chip--open' }, 'En yeni') : null),
      h('span', { class: 'catalog-history__meta' }, `${r.createdByName || 'görünmüyor'} · ${when(r.createdAt)}`),
      h('span', { class: 'catalog-history__meta num' }, `${sizeText(r.size)}${objects}`),
    ),
    h('div', { class: 'catalog-history__acts' }, rowButton('İndir', 'export', () => actions.downloadRevision(r), needs(p, 'project.download', 'indirme'))),
  );
}

/** The history tab's content for `p`. */
export function renderHistory(_ctx: AppContext, p: ProjectSummary, state: HistoryState, actions: HistoryActions): Child[] {
  if (state === 'loading') return [h('p', { class: 'catalog-details__muted' }, 'Geçmiş yükleniyor…')];
  if (state instanceof Error) {
    const retry = h('button', { class: 'btn btn--small', type: 'button' }, 'Yeniden dene');
    retry.addEventListener('click', () => actions.retry());
    return [h('div', { class: 'catalog-history__error' }, h('p', null, `Geçmiş okunamadı: ${state.message}`), retry)];
  }
  if (state === 'none') return [h('p', { class: 'catalog-details__muted' }, 'Bu projenin geçmişi gösterilemiyor.')];
  const out: Child[] = [];
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
        'Bu proje nesne nesne veritabanında saklanıyor; revizyon dosyaları yoktur. Projenin o anki hâlini tek bir dosya olarak almak için “.kcad olarak indir”i kullanın.',
      ),
    );
  return out;
}
