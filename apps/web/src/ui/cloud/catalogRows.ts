import type { AppContext } from '../../app/context';
import { STATE_LABEL, TYPE_LABEL, day, when } from '../../app/cloud/catalog';
import { workspaceName } from '../../app/cloud/session';
import { ROLE_LABEL } from '../../app/cloud/sharing';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { h } from '../dom';
import { icon } from '../icons';

/**
 * One project in a catalog list (docs/adr/0028): its name, marked when it is
 * a favourite, open here or archived; under it its type and where it is
 * (or whose it is); on the right the time the list is ordered by, and the
 * account's role where the list is about roles. A row is an option of the
 * list: a click selects it, a double click does the list's main action.
 */

/** Where a project is, as the account names it: an organisation, “Kişisel”, or someone's personal space. */
export function placeOf(ctx: AppContext, p: ProjectSummary): string {
  return workspaceName(p.tenantKind, p.tenantName, !!ctx.cloud.membership(p.tenantId));
}

/** The time a row shows: the one its list is ordered by. */
function timeOf(p: ProjectSummary, sort: CatalogSort): string {
  switch (sort) {
    case 'opened':
      return p.openedAt ? when(p.openedAt) : '';
    case 'created':
      return when(p.createdAt);
    case 'trashed':
      return p.trashedAt ? `Silinme: ${day(p.trashedAt)}` : '';
    default:
      return when(p.updatedAt);
  }
}

export function catalogRow(ctx: AppContext, p: ProjectSummary, view: CatalogView, sort: CatalogSort): HTMLElement {
  const open = ctx.cloud.project.value?.projectId === p.id;
  const marks = [
    p.favorite ? h('span', { class: 'catalog-row__fav', title: 'Favorilerinizde' }, icon('starOn', 14)) : null,
    open ? h('span', { class: 'catalog-chip catalog-chip--open' }, 'Açık') : null,
    p.state === 'archived' && view !== 'archived' ? h('span', { class: 'catalog-chip' }, STATE_LABEL.archived) : null,
  ];
  const whose = view === 'organization' || view === 'shared' ? `Sahibi: ${p.ownerName || 'görünmüyor'}` : placeOf(ctx, p);
  const sub =
    view === 'trash'
      ? [h('span', null, p.trashedByName ? `Çöpe taşıyan: ${p.trashedByName}` : placeOf(ctx, p))]
      : [h('span', { class: 'catalog-chip catalog-chip--type' }, TYPE_LABEL[p.projectType]), h('span', { class: 'catalog-row__place' }, whose)];
  const side = [
    view === 'shared' ? h('span', { class: 'catalog-row__role', title: 'Bu projedeki rolünüz' }, ROLE_LABEL[p.access.role]) : null,
    view === 'trash' && p.purgeAfter ? h('span', { class: 'catalog-row__when catalog-row__when--warn' }, `${day(p.purgeAfter)} tarihinde silinir`) : null,
    h('span', { class: 'catalog-row__when' }, timeOf(p, sort)),
  ];
  return h(
    'div',
    { class: 'catalog-row', role: 'option', 'aria-selected': 'false', dataset: { id: p.id } },
    h('span', { class: 'catalog-row__main' }, h('span', { class: 'catalog-row__name' }, h('span', { class: 'catalog-row__title' }, p.name), marks), h('span', { class: 'catalog-row__sub' }, sub)),
    h('span', { class: 'catalog-row__side' }, side),
  );
}
