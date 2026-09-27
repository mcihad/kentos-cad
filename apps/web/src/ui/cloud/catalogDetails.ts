import type { AppContext } from '../../app/context';
import { day, when } from '../../app/cloud/catalog';
import { detailPlan } from '../../app/cloud/catalogPlan';
import { ROLE_LABEL, STORAGE_TEXT } from '../../app/cloud/sharing';
import type { ProjectDetails } from '../../contracts/generated/ProjectDetails';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { crsBySrid } from '../../geo/crs';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { renderHistory, type HistoryActions, type HistoryState } from './catalogHistory';
import { placeOf } from './catalogRows';

/**
 * The selected project of the catalog (docs/adr/0028): what describes it
 * (type, description, tags), where it is and whose, the account's role,
 * its coordinate system and units, what the server counts on asking (its
 * objects, layers and extent), and every action the catalog offers on it.
 * An action the account may not take stays visible, disabled, and says
 * which right it needs; an archived project says it must be unarchived
 * first. Its history (a file project's revisions, docs/adr/0038) is the
 * second tab. Nothing here asks the server: the dialog does, and redraws.
 */

export interface DetailActions {
  share(): void;
  edit(): void;
  duplicate(): void;
  archive(): void;
  unarchive(): void;
  trash(): void;
  purge(): void;
  favorite(): void;
  /** “.kcad olarak indir”: a file project's newest revision, a database project's snapshot. */
  download(): void;
  /** A new project in the other storage mode (“PostGIS'e aktar”, “Dosya projesine çevir”; docs/adr/0039). */
  convert(): void;
}

/** The pane's two tabs. */
export type DetailsTab = 'info' | 'history';

/** The history tab: which tab shows, the history as it is, what its rows do. */
export interface DetailsView {
  tab: DetailsTab;
  onTab(tab: DetailsTab): void;
  history: HistoryState;
  historyActions: HistoryActions;
}

/**
 * A file project's content as its newest revision holds it: the server
 * counts rows of a database project only (a file project has none), so
 * its objects are the revision's and its extent is not worked out.
 */
export interface FileDetails {
  kind: 'file';
  project: ProjectSummary;
  /** The newest revision; null before the first Kaydet. */
  revision: string | null;
  objects?: string;
}

/** What the server worked out on asking, while it is on its way, or why it is not there. */
export type DetailsState = ProjectDetails | FileDetails | 'loading' | 'none' | Error;

const AREA_UNIT = { m2: 'm²', donum: 'dönüm', ha: 'hektar' } as const;

export function renderDetails(ctx: AppContext, host: HTMLElement, p: ProjectSummary | null, d: DetailsState, actions: DetailActions, view: DetailsView): void {
  if (!p) {
    replaceChildren(host, h('div', { class: 'catalog-details__body' }, h('p', { class: 'catalog-details__empty' }, 'Bilgilerini görmek ve üzerinde işlem yapmak için listeden bir proje seçin.')));
    return;
  }
  const button = (label: string, iconName: string, run: () => void, why: string | null, danger = false) => {
    const b = h('button', { class: `btn btn--small${danger ? ' btn--danger' : ''}`, type: 'button', disabled: !!why, title: why ?? null }, icon(iconName, 14), label);
    b.addEventListener('click', run);
    return b;
  };
  // What shows and what can be done comes from the plan (app/cloud/catalogPlan.ts, pinned by fixtures/cloud/v1/catalog.json); here it is drawn.
  const plan = detailPlan(p, ctx.cloud.project.value?.projectId === p.id);
  const favorite =
    plan.favorite === null
      ? null
      : (() => {
          const b = h(
            'button',
            { class: 'btn btn--ghost btn--small catalog-details__fav', type: 'button', 'aria-pressed': String(p.favorite), title: 'Favorileriniz yalnız size görünür.' },
            icon(p.favorite ? 'starOn' : 'star', 14),
            plan.favorite,
          );
          b.addEventListener('click', () => actions.favorite());
          return b;
        })();
  // The type first, then the state (when not active), then “Şu anda açık”.
  const chips = plan.chips.map((text, i) =>
    h('span', { class: `catalog-chip ${i === 0 ? 'catalog-chip--type' : text === 'Şu anda açık' ? 'catalog-chip--open' : `catalog-chip--${p.state}`}` }, text),
  );
  const crs = crsBySrid(p.srid);
  const row = (term: string, value: Child) => [h('dt', null, term), h('dd', null, value)];
  const muted = (text: string) => h('span', { class: 'catalog-details__muted' }, text);
  const counted = (f: (x: ProjectDetails) => Child, file: (x: FileDetails) => Child): Child =>
    d === 'loading' ? muted('Hesaplanıyor…') : d instanceof Error ? muted('Okunamadı') : d === 'none' ? '—' : 'kind' in d ? file(d) : f(d);
  const fileObjects = (x: FileDetails): Child =>
    x.revision === null
      ? muted('Henüz kaydedilmiş revizyon yok')
      : h('span', { class: 'num' }, `${x.objects === undefined ? 'Nesne sayısı bilinmiyor' : `${Number(x.objects).toLocaleString('tr-TR')} nesne`} (revizyon ${x.revision})`);
  const extent = (x: ProjectDetails): Child => {
    const b = x.bounds;
    if (!b) return h('span', { class: 'catalog-details__muted' }, 'Geometrili nesne yok');
    const f = ctx.format;
    return h('span', { class: 'num catalog-details__extent' }, h('span', null, `Y ${f.coord(b.minX)}–${f.coord(b.maxX)}`), h('span', null, `X ${f.coord(b.minY)}–${f.coord(b.maxY)}`));
  };
  // Each fact the plan shows, by its name.
  const value = (term: string): Child => {
    switch (term) {
      case 'Çalışma alanı':
        return placeOf(ctx, p);
      case 'Sahibi':
        return p.ownerName || muted('görünmüyor');
      case 'Rolünüz':
        return `${ROLE_LABEL[p.access.role]}${p.access.via === 'policy' ? ' (kurum politikası)' : ''}`;
      case 'Koordinat sistemi':
        return crs ? `${crs.name} (EPSG:${p.srid})` : `EPSG:${p.srid}`;
      case 'Alan birimi':
        return AREA_UNIT[p.areaUnit];
      case 'Nesne':
        return counted((x) => h('span', { class: 'num' }, `${x.featureCount} nesne, ${x.layerCount} katman`), fileObjects);
      case 'Kapsam':
        return counted(extent, () => muted('Dosya projesinde hesaplanmaz'));
      case 'Oluşturan':
        return `${p.creatorName || 'görünmüyor'}, ${when(p.createdAt)}`;
      case 'Son değişiklik':
        return when(p.updatedAt);
      case 'Revizyon':
        return h('span', { class: 'num' }, p.dataRevision);
      case 'Saklama':
        return STORAGE_TEXT[p.storage].title;
      case 'Arşivlenme':
        return p.archivedAt ? when(p.archivedAt) : '';
      case 'Çöpe taşınma':
        return p.trashedAt ? `${when(p.trashedAt)}${p.trashedByName ? `, ${p.trashedByName}` : ''}` : '';
      default:
        return p.purgeAfter ? day(p.purgeAfter) : 'Elle silinene kadar kalır';
    }
  };
  const facts: Child[] = plan.facts.map((term) => row(term, value(term)));
  const buttons = plan.actions.map((a) => button(a.label, a.icon, () => actions[a.id](), a.why, a.danger));
  // Nothing of a project in the trash has a history to show (it does not open).
  const TAB_IDS: readonly DetailsTab[] = ['info', 'history'];
  const tabs = plan.tabs
    ? h(
        'div',
        { class: 'catalog-details__tabs', role: 'tablist', 'aria-label': 'Proje bilgileri' },
        plan.tabs.map((text, i) => {
          const id = TAB_IDS[i];
          const b = h('button', { class: 'catalog-details__tab', type: 'button', role: 'tab', 'aria-selected': String(view.tab === id), dataset: { tab: id } }, text);
          b.addEventListener('click', () => view.onTab(id));
          return b;
        }),
      )
    : null;
  const history = view.tab === 'history' && !!plan.tabs;
  // The facts scroll; the actions stay in view below them.
  replaceChildren(
    host,
    h(
      'div',
      { class: 'catalog-details__body' },
      h('header', { class: 'catalog-details__head' }, h('h3', { class: 'catalog-details__name' }, p.name), favorite),
      h('div', { class: 'catalog-details__chips' }, chips),
      tabs,
      ...(history
        ? renderHistory(ctx, p, view.history, view.historyActions)
        : [
            p.description ? h('p', { class: 'catalog-details__desc' }, p.description) : h('p', { class: 'catalog-details__desc catalog-details__muted' }, 'Açıklama yok.'),
            p.tags.length ? h('div', { class: 'catalog-details__tags', 'aria-label': 'Etiketler' }, p.tags.map((t) => h('span', { class: 'catalog-tag' }, t))) : null,
            h('dl', { class: 'catalog-details__facts' }, facts),
          ]),
    ),
    h('div', { class: 'catalog-details__actions' }, buttons),
  );
}
