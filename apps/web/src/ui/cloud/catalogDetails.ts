import type { AppContext } from '../../app/context';
import { STATE_LABEL, TYPE_LABEL, day, when } from '../../app/cloud/catalog';
import { ROLE_LABEL, STORAGE_TEXT } from '../../app/cloud/sharing';
import type { ProjectDetails } from '../../contracts/generated/ProjectDetails';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
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

/** What the server worked out on asking, while it is on its way, or why it is not there. */
export type DetailsState = ProjectDetails | 'loading' | 'none' | Error;

const AREA_UNIT = { m2: 'm²', donum: 'dönüm', ha: 'hektar' } as const;

const may = (p: ProjectSummary, permission: ProjectPermission) => p.access.permissions.includes(permission);

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
  // Why an action cannot be taken: the right it needs, or the archive.
  const needs = (permission: ProjectPermission, what: string): string | null =>
    may(p, permission) ? null : `“${p.name}” projesinde ${what} yetkiniz yok (${permission}); proje sahibine ya da yöneticisine başvurun.`;
  const writable = (permission: ProjectPermission, what: string): string | null =>
    p.state === 'archived' ? 'Arşivlenmiş proje değiştirilemez; önce arşivden çıkarın.' : needs(permission, what);

  const favorite =
    p.state === 'trashed'
      ? null
      : (() => {
          const b = h(
            'button',
            { class: 'btn btn--ghost btn--small catalog-details__fav', type: 'button', 'aria-pressed': String(p.favorite), title: 'Favorileriniz yalnız size görünür.' },
            icon(p.favorite ? 'starOn' : 'star', 14),
            p.favorite ? 'Favorilerde' : 'Favorilere ekle',
          );
          b.addEventListener('click', () => actions.favorite());
          return b;
        })();
  const chips = [
    h('span', { class: 'catalog-chip catalog-chip--type' }, TYPE_LABEL[p.projectType]),
    p.state !== 'active' ? h('span', { class: `catalog-chip catalog-chip--${p.state}` }, STATE_LABEL[p.state]) : null,
    ctx.cloud.project.value?.projectId === p.id ? h('span', { class: 'catalog-chip catalog-chip--open' }, 'Şu anda açık') : null,
  ];
  const crs = crsBySrid(p.srid);
  const row = (term: string, value: Child) => [h('dt', null, term), h('dd', null, value)];
  const counted = (f: (x: ProjectDetails) => Child): Child =>
    d === 'loading' ? h('span', { class: 'catalog-details__muted' }, 'Hesaplanıyor…') : d instanceof Error ? h('span', { class: 'catalog-details__muted' }, 'Okunamadı') : d === 'none' ? '—' : f(d);
  const extent = (x: ProjectDetails): Child => {
    const b = x.bounds;
    if (!b) return h('span', { class: 'catalog-details__muted' }, 'Geometrili nesne yok');
    const f = ctx.format;
    return h('span', { class: 'num catalog-details__extent' }, h('span', null, `Y ${f.coord(b.minX)}–${f.coord(b.maxX)}`), h('span', null, `X ${f.coord(b.minY)}–${f.coord(b.maxY)}`));
  };
  const facts: Child[] = [
    row('Çalışma alanı', placeOf(ctx, p)),
    row('Sahibi', p.ownerName || h('span', { class: 'catalog-details__muted' }, 'görünmüyor')),
    row('Rolünüz', `${ROLE_LABEL[p.access.role]}${p.access.via === 'policy' ? ' (kurum politikası)' : ''}`),
    row('Koordinat sistemi', crs ? `${crs.name} (EPSG:${p.srid})` : `EPSG:${p.srid}`),
    row('Alan birimi', AREA_UNIT[p.areaUnit]),
    // Nothing of a project in the trash is counted (it does not open).
    p.state === 'trashed' ? null : row('Nesne', counted((x) => h('span', { class: 'num' }, `${x.featureCount} nesne, ${x.layerCount} katman`))),
    p.state === 'trashed' ? null : row('Kapsam', counted(extent)),
    row('Oluşturan', `${p.creatorName || 'görünmüyor'}, ${when(p.createdAt)}`),
    row('Son değişiklik', when(p.updatedAt)),
    row('Revizyon', h('span', { class: 'num' }, p.dataRevision)),
    row('Saklama', STORAGE_TEXT[p.storage].title),
    p.archivedAt ? row('Arşivlenme', when(p.archivedAt)) : null,
    p.trashedAt ? row('Çöpe taşınma', `${when(p.trashedAt)}${p.trashedByName ? `, ${p.trashedByName}` : ''}`) : null,
    p.state === 'trashed' ? row('Kalıcı silinme', p.purgeAfter ? day(p.purgeAfter) : 'Elle silinene kadar kalır') : null,
  ];
  const toDatabase = p.storage === 'file';
  const buttons =
    p.state === 'trashed'
      ? [button('Kalıcı olarak sil…', 'trash', actions.purge, needs('project.delete', 'kalıcı olarak silme'), true)]
      : [
          button('Paylaş…', 'share', actions.share, needs('project.share', 'paylaşma')),
          button('Bilgileri düzenle…', 'edit', actions.edit, writable('project.edit', 'bilgileri değiştirme')),
          button('.kcad olarak indir', 'export', actions.download, needs('project.download', 'indirme')),
          button('Kopyasını oluştur…', 'copy', actions.duplicate, needs('project.download', 'kopyalama')),
          button(toDatabase ? "PostGIS'e aktar…" : 'Dosya projesine çevir…', toDatabase ? 'server' : 'save', actions.convert, needs('project.download', 'dönüştürme')),
          p.state === 'archived'
            ? button('Arşivden çıkar', 'archive', actions.unarchive, needs('project.edit', 'arşivden çıkarma'))
            : button('Arşivle…', 'archive', actions.archive, needs('project.edit', 'arşivleme')),
          button('Çöpe taşı…', 'trash', actions.trash, needs('project.delete', 'çöpe taşıma'), true),
        ];
  // Nothing of a project in the trash has a history to show (it does not open).
  const tabs =
    p.state === 'trashed'
      ? null
      : h(
          'div',
          { class: 'catalog-details__tabs', role: 'tablist', 'aria-label': 'Proje bilgileri' },
          (
            [
              ['info', 'Bilgiler'],
              ['history', 'Geçmiş'],
            ] as const
          ).map(([id, text]) => {
            const b = h('button', { class: 'catalog-details__tab', type: 'button', role: 'tab', 'aria-selected': String(view.tab === id), dataset: { tab: id } }, text);
            b.addEventListener('click', () => view.onTab(id));
            return b;
          }),
        );
  const history = view.tab === 'history' && p.state !== 'trashed';
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
