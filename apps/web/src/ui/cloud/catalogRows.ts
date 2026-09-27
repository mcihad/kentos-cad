import type { AppContext } from '../../app/context';
import { rowPlan } from '../../app/cloud/catalogPlan';
import { workspaceName } from '../../app/cloud/session';
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

export function catalogRow(ctx: AppContext, p: ProjectSummary, view: CatalogView, sort: CatalogSort): HTMLElement {
  const open = ctx.cloud.project.value?.projectId === p.id;
  // The texts come from the plan (app/cloud/catalogPlan.ts, pinned by fixtures/cloud/v1/catalog.json); here they are drawn.
  const plan = rowPlan(p, view, sort, open, placeOf(ctx, p));
  const marks = plan.marks.map((m) =>
    m === 'Favorilerinizde' ? h('span', { class: 'catalog-row__fav', title: m }, icon('starOn', 14)) : h('span', { class: `catalog-chip${m === 'Açık' ? ' catalog-chip--open' : ''}` }, m),
  );
  const sub =
    view === 'trash'
      ? [h('span', null, plan.sub[0])]
      : [h('span', { class: 'catalog-chip catalog-chip--type' }, plan.sub[0]), h('span', { class: 'catalog-row__place' }, plan.sub[1])];
  const time = plan.side.at(-1)!;
  const side = [
    view === 'shared' ? h('span', { class: 'catalog-row__role', title: 'Bu projedeki rolünüz' }, plan.side[0]) : null,
    view === 'trash' && plan.side.length > 1 ? h('span', { class: 'catalog-row__when catalog-row__when--warn' }, plan.side[0]) : null,
    h('span', { class: 'catalog-row__when' }, time),
  ];
  return h(
    'div',
    { class: 'catalog-row', role: 'option', 'aria-selected': 'false', dataset: { id: p.id } },
    h('span', { class: 'catalog-row__main' }, h('span', { class: 'catalog-row__name' }, h('span', { class: 'catalog-row__title' }, p.name), marks), h('span', { class: 'catalog-row__sub' }, sub)),
    h('span', { class: 'catalog-row__side' }, side),
  );
}
