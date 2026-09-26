import '../../styles/catalog.css';
import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { CatalogPager, PROJECT_TYPES, SORT_LABEL, TYPE_LABEL, VIEWS, viewDef } from '../../app/cloud/catalog';
import type { ProjectRef } from '../../app/cloud/lifecycle';
import type { CatalogSort } from '../../contracts/generated/CatalogSort';
import type { CatalogView } from '../../contracts/generated/CatalogView';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { renderDetails, type DetailActions, type DetailsState, type DetailsTab, type FileDetails } from './catalogDetails';
import { catalogRow } from './catalogRows';
import { downloadKcad } from './downloads';
import { HistoryPanel } from './historyPanel';
import { archiveProject, purgeProject, reason, targetOf, trashProject } from './ProjectActions';
import { openConvertDialog, openDuplicateDialog, openMetadataDialog } from './ProjectForms';
import { openShareDialog } from './ShareDialog';

/**
 * “Bulut projeleri” (docs/adr/0028, TODOS.md CLOUD-04): the account's
 * catalog. On the left its lists (recently opened, favourites, its own, an
 * organisation's, shared with it, archived, the trash); in the middle the
 * chosen list, searched, filtered by type, sorted and paged by the server
 * after its access check; on the right the selected project with every
 * action on it. The list's main action is the window's one amber button:
 * open (an archived project opens read-only), or restore in the trash.
 * Opening shows its progress and can be cancelled; the drawing on screen is
 * replaced only when every object has arrived. A file project opens from its
 * newest revision in the open's window (docs/adr/0038). The selected
 * project's second tab is its history; a project made from it here (a
 * restored point, a conversion) is shown in “Projelerim” and opened.
 */

/** The list this window opens on: the one shown last in this session. */
let lastView: CatalogView = 'mine';
const PAGE = 50;
const SEARCH_MS = 250;
const DETAILS_MS = 120;

/** What each list says above it, besides its name. */
const NOTE: Partial<Record<CatalogView, string>> = {
  shared: 'Başkalarının sizinle paylaştığı projeler; sahibi ve rolünüz yanında yazar.',
  archived: 'Arşivlenmiş projeler salt okunurdur: açılır, kopyalanır; arşivden çıkarmak proje sahibinin ya da yöneticisinindir.',
  favorites: 'Favorileriniz yalnız size görünür.',
};

export function openCatalog(ctx: AppContext, pick?: { tenantId: string; projectId: string }, tab?: DetailsTab): void {
  const cloud = ctx.cloud;
  const orgs = (cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat && m.tenantKind === 'organization');
  let view: CatalogView = pick ? 'recent' : lastView;
  let org = orgs.find((m) => m.tenantId === cloud.project.value?.tenantId)?.tenantId ?? orgs[0]?.tenantId;
  let sort: CatalogSort = viewDef(view).sorts[0];
  let picked: ProjectSummary | null = null;
  let wanted: string | null = pick?.projectId ?? null;
  let details: DetailsState = 'none';
  let detailsGen = 0;
  let detailsTimer = 0;
  let searchTimer = 0;
  let opening: AbortController | null = null;
  let detailsTab: DetailsTab = tab ?? 'info';
  /** A project made here (a restore, a conversion) is opened once the list shows it. */
  let openAfter = false;
  const pager = new CatalogPager(cloud.api);

  const navButtons = VIEWS.map((v) => {
    const b = h('button', { class: 'catalog-nav__item', type: 'button', role: 'tab', 'aria-selected': String(v.id === view), dataset: { view: v.id } }, icon(v.icon, 16), h('span', null, v.label));
    b.addEventListener('click', () => show(v.id));
    return b;
  });
  const nav = h('nav', { class: 'catalog-nav', role: 'tablist', 'aria-orientation': 'vertical', 'aria-label': 'Proje listeleri' }, navButtons);
  const title = h('h3', { class: 'catalog-main__title' });
  const count = h('span', { class: 'catalog-main__count', role: 'status' });
  const search = h('input', { class: 'field field--search', type: 'search', placeholder: 'Ara: ad, açıklama, etiket', 'aria-label': 'Projelerde ara', spellcheck: 'false' });
  const typeSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Proje türü' },
    h('option', { value: '' }, 'Tüm türler'),
    PROJECT_TYPES.map((t) => h('option', { value: t }, TYPE_LABEL[t])),
  );
  const sortSelect = h('select', { class: 'field', 'aria-label': 'Sıralama' });
  const orgSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Kurum', disabled: orgs.length < 2 },
    orgs.map((m) => h('option', { value: m.tenantId, selected: m.tenantId === org }, m.tenantName)),
  );
  const orgField = h('label', { class: 'catalog-org' }, h('span', null, 'Kurum'), orgSelect);
  const note = h('p', { class: 'catalog-note' });
  const list = h('div', { class: 'catalog-list', role: 'listbox', tabindex: '0', 'aria-label': 'Projeler' });
  const more = h('button', { class: 'btn btn--small catalog-more', type: 'button', hidden: true });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const pane = h('aside', { class: 'catalog-details', 'aria-label': 'Seçili proje' });
  const main = h(
    'section',
    { class: 'catalog-main' },
    h('header', { class: 'catalog-main__head' }, title, count),
    h('div', { class: 'catalog-tools' }, h('div', { class: 'catalog-search' }, h('span', { class: 'field-icon' }, icon('search', 14)), search), typeSelect, sortSelect),
    orgField,
    note,
    list,
    more,
    progress,
    status,
  );
  const primary = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Aç');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Bulut projeleri',
    width: 1040,
    className: 'dialog--catalog',
    content: [h('div', { class: 'catalog' }, nav, main, pane)],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, primary],
    onClose: () => {
      opening?.abort();
      pager.cancel();
      detailsGen++;
      history.hide();
      clearTimeout(detailsTimer);
      clearTimeout(searchTimer);
    },
  });

  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const ref = (p: ProjectSummary): ProjectRef => ({ tenantId: p.tenantId, projectId: p.id, name: p.name });

  const refreshPrimary = () => {
    if (view === 'trash') {
      primary.textContent = 'Geri yükle';
      const may = !!picked?.access.permissions.includes('project.delete');
      primary.disabled = !may;
      primary.title = !picked ? 'Önce listeden bir proje seçin.' : may ? '' : `“${picked.name}” projesini geri yükleme yetkiniz yok (project.delete).`;
    } else {
      primary.textContent = 'Aç';
      primary.disabled = !picked;
      primary.title = picked ? (picked.state === 'archived' ? 'Arşivlenmiş proje salt okunur açılır.' : '') : 'Önce listeden bir proje seçin.';
    }
  };

  const paintDetails = () =>
    renderDetails(ctx, pane, picked, details, actions, {
      tab: detailsTab,
      onTab: (t) => {
        detailsTab = t;
        showHistory();
      },
      history: history.state,
      historyActions: history.actions,
    });

  /** The selected project's history while its tab shows (asked again when the project's events say so). */
  const showHistory = () => {
    if (detailsTab === 'history' && picked) history.show(picked);
    else {
      history.hide();
      paintDetails();
    }
  };

  /** A download with its progress in the window's status line. */
  const download = (request: Parameters<typeof downloadKcad>[1]) =>
    void downloadKcad(ctx, {
      ...request,
      say: (text, fraction) => {
        progress.hidden = false;
        bar.style.width = `${Math.round(fraction * 100)}%`;
        say(text);
      },
    }).then((ok) => {
      progress.hidden = true;
      say(ok ? `“${request.name}” indirildi.` : '');
    });

  /** A project made here: shown in “Projelerim”, selected and opened. */
  const openMade = (made: ProjectDuplicated) => {
    wanted = made.project.id;
    openAfter = true;
    detailsTab = 'info';
    // Whatever was being searched: the new project is the one to show.
    search.value = '';
    typeSelect.value = '';
    show('mine');
  };
  const history = new HistoryPanel(ctx, { paint: () => paintDetails(), download, opened: openMade });

  /** The selected project's counts and extent, asked a moment after the selection settles. */
  const loadDetails = () => {
    clearTimeout(detailsTimer);
    const p = picked;
    const gen = ++detailsGen;
    const known = typeof details === 'object' && !(details instanceof Error) && p && details.project.id === p.id ? details : null;
    if (!p || p.state === 'trashed') details = 'none';
    else if (known) details = { ...known, project: p };
    else {
      details = 'loading';
      detailsTimer = setTimeout(() => {
        // A file project's content is its newest revision (the server counts no rows of it).
        const asked: Promise<DetailsState> =
          p.storage === 'file'
            ? cloud.api.fileRevisions(p.tenantId, p.id).then((r): FileDetails => {
                const objects = r.revisions.find((x) => x.revision === r.current)?.objects;
                return { kind: 'file', project: p, revision: r.current ?? null, ...(objects === undefined ? {} : { objects }) };
              })
            : cloud.api.details(p.tenantId, p.id);
        asked.then(
          (d) => {
            if (gen !== detailsGen) return;
            details = d;
            paintDetails();
          },
          (e: unknown) => {
            if (gen !== detailsGen) return;
            details = e instanceof Error ? e : new Error(String(e));
            paintDetails();
          },
        );
      }, DETAILS_MS) as unknown as number;
    }
    paintDetails();
  };

  const select = (p: ProjectSummary | null) => {
    const changed = picked?.id !== p?.id;
    picked = p;
    for (const r of list.querySelectorAll<HTMLElement>('.catalog-row')) r.setAttribute('aria-selected', String(r.dataset.id === p?.id));
    const row = p ? list.querySelector<HTMLElement>(`.catalog-row[data-id="${CSS.escape(p.id)}"]`) : null;
    if (row) {
      list.setAttribute('aria-activedescendant', row.id);
      row.scrollIntoView({ block: 'nearest' });
    } else list.removeAttribute('aria-activedescendant');
    refreshPrimary();
    loadDetails();
    if (changed) showHistory();
  };

  const paintList = () => {
    const def = viewDef(view);
    const filtered = !!search.value.trim() || !!typeSelect.value;
    if (!pager.projects.length) {
      const empty = filtered
        ? 'Aramanıza uyan proje yok. Başka sözcüklerle ya da tür süzgeci olmadan deneyin.'
        : view === 'organization' && !orgs.length
          ? 'Etkin üyeliğiniz olan bir kurum yok; kurum projeleri burada görünür.'
          : def.empty;
      replaceChildren(list, h('p', { class: 'cloud-empty' }, empty));
    } else {
      replaceChildren(
        list,
        pager.projects.map((p, i) => {
          const row = catalogRow(ctx, p, view, sort);
          row.id = `catalog-row-${i}`;
          row.addEventListener('click', () => select(p));
          row.addEventListener('dblclick', () => void runPrimary());
          return row;
        }),
      );
    }
    count.textContent = pager.total ? `${pager.total} proje` : '';
    more.hidden = !pager.hasMore;
    more.textContent = `Daha fazla göster (${pager.projects.length} / ${pager.total})`;
    if (view === 'trash')
      note.textContent = `Çöp kutusundaki projeler, taşındıktan ${pager.retentionDays || 30} gün sonra kalıcı olarak silinir; o zamana kadar proje sahibi ya da kurum yöneticisi geri yükleyebilir.`;
    const want = wanted ? pager.projects.find((x) => x.id === wanted) : null;
    const made = openAfter;
    wanted = null;
    openAfter = false;
    const keep = want ?? (picked ? pager.projects.find((x) => x.id === picked!.id) : null) ?? null;
    select(keep);
    if (want) primary.focus();
    // A project made here is opened as soon as the list shows it (only then: a later list opens nothing by itself).
    if (want && made) void runPrimary();
    else if (made) say('Yeni proje oluşturuldu ama bu listede görünmüyor; “Projelerim”de arayıp açın.');
  };

  const load = async () => {
    replaceChildren(list, h('p', { class: 'cloud-empty' }, 'Projeler yükleniyor…'));
    count.textContent = '';
    more.hidden = true;
    try {
      if (await pager.reset({ view, tenant: view === 'organization' ? org : undefined, q: search.value, type: (typeSelect.value || undefined) as ProjectSummary['projectType'] | undefined, sort, limit: PAGE }))
        paintList();
    } catch (e) {
      if (!dialog.el.isConnected) return;
      const retry = h('button', { class: 'btn btn--small', type: 'button' }, 'Yeniden dene');
      retry.addEventListener('click', () => void load());
      replaceChildren(list, h('div', { class: 'cloud-empty' }, h('p', null, e instanceof ApiFailure ? e.message : 'Projeler okunamadı.'), retry));
      select(null);
    }
  };

  const show = (next: CatalogView) => {
    if (opening) return;
    view = next;
    lastView = next;
    for (const b of navButtons) b.setAttribute('aria-selected', String(b.dataset.view === view));
    const def = viewDef(view);
    title.textContent = def.label;
    replaceChildren(sortSelect, def.sorts.map((s) => h('option', { value: s }, SORT_LABEL[s])));
    sort = def.sorts[0];
    sortSelect.value = sort;
    orgField.hidden = view !== 'organization' || !orgs.length;
    note.textContent = NOTE[view] ?? '';
    picked = null;
    details = 'none';
    history.hide();
    say('');
    void load();
  };

  /** After an action: the list again (the selection kept when it is still there), and a line about it. */
  const after = (text: string) => {
    say(text);
    void load();
  };
  const fail = (e: unknown) => say(reason(e), 'error');

  const actions: DetailActions = {
    share: () => picked && openShareDialog(ctx, targetOf(ctx, picked), { stack: true, done: () => void load() }),
    edit: () => picked && openMetadataDialog(ctx, picked, () => void load()),
    duplicate: () =>
      picked &&
      openDuplicateDialog(ctx, picked, (copy) => {
        // The copy is the account's own: shown selected in “Projelerim”.
        wanted = copy.project.id;
        show('mine');
      }),
    download: () => {
      const p = picked;
      if (!p) return;
      download({
        name: p.name,
        // A database project as one file of one moment; a file project's newest revision.
        fetch: async (step, signal) => {
          if (p.storage !== 'file') return cloud.api.snapshot(p.tenantId, p.id, step, signal);
          const revs = await cloud.api.fileRevisions(p.tenantId, p.id, signal);
          if (!revs.current) throw new Error('Projenin henüz kaydedilmiş revizyonu yok; indirilecek dosya yok.');
          return cloud.api.fileRevision(p.tenantId, p.id, revs.current, step, signal);
        },
      });
    },
    convert: () => picked && openConvertDialog(ctx, picked, openMade),
    archive: () => {
      const p = picked;
      if (p) void archiveProject(ctx, targetOf(ctx, p)).then((ok) => ok && after(`“${p.name}” arşivlendi; Arşivlenmişler listesinde duruyor.`));
    },
    unarchive: () => {
      const p = picked;
      if (!p) return;
      cloud.lifecycle.unarchive(ref(p)).then(() => {
        ctx.log.success(`“${p.name}” arşivden çıkarıldı.`);
        after(`“${p.name}” arşivden çıkarıldı; yeniden düzenlenebilir.`);
      }, fail);
    },
    trash: () => {
      const p = picked;
      if (p) void trashProject(ctx, targetOf(ctx, p), pager.retentionDays || undefined).then((ok) => ok && after(`“${p.name}” çöp kutusuna taşındı.`));
    },
    purge: () => {
      const p = picked;
      if (p) void purgeProject(ctx, targetOf(ctx, p)).then((ok) => ok && after(`“${p.name}” kalıcı olarak silindi.`));
    },
    favorite: () => {
      const p = picked;
      if (!p) return;
      cloud.lifecycle.setFavorite(ref(p), !p.favorite).then((r) => {
        const at = pager.projects.findIndex((x) => x.id === p.id);
        // Taken out of “Favoriler”, it leaves the list; anywhere else its row changes in place.
        if (view === 'favorites' && !r.project.favorite) return after(`“${p.name}” favorilerden çıkarıldı.`);
        if (at >= 0) pager.projects[at] = r.project;
        picked = r.project;
        paintList();
        say(r.project.favorite ? `“${p.name}” favorilere eklendi.` : `“${p.name}” favorilerden çıkarıldı.`);
      }, fail);
    },
  };

  /** Opens the selected project (kept as `storage` when the caller knows better than the list), or restores it in the trash. */
  const runPrimary = async () => {
    const p = picked;
    if (!p || primary.disabled) return;
    primary.disabled = true;
    if (view === 'trash') {
      try {
        await cloud.lifecycle.restore(ref(p));
        ctx.log.success(`“${p.name}” çöp kutusundan geri yüklendi.`);
        after(`“${p.name}” geri yüklendi; listelerinde yeniden görünür.`);
      } catch (e) {
        fail(e);
        refreshPrimary();
      }
      return;
    }
    const abort = new AbortController();
    opening = abort;
    try {
      const ok = await cloud.open(
        p.tenantId,
        p.id,
        (done, total) => {
          progress.hidden = false;
          bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
          say(`${done} / ${total} nesne`);
        },
        abort.signal,
      );
      if (ok) dialog.close();
      else if (!abort.signal.aborted) {
        // Stopped in the open's window, or it could not be read (said in the log): the list stays.
        refreshPrimary();
        progress.hidden = true;
      }
    } catch (e) {
      if (abort.signal.aborted) return;
      fail(e);
      refreshPrimary();
      progress.hidden = true;
    } finally {
      opening = null;
    }
  };

  nav.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    const at = VIEWS.findIndex((v) => v.id === view);
    const next = VIEWS[(at + (e.key === 'ArrowDown' ? 1 : VIEWS.length - 1)) % VIEWS.length];
    show(next.id);
    navButtons.find((b) => b.dataset.view === next.id)?.focus();
  });
  list.addEventListener('keydown', (e) => {
    const items = pager.projects;
    if (e.key === 'Enter') {
      e.preventDefault();
      void runPrimary();
      return;
    }
    if (!items.length) return;
    const at = picked ? items.findIndex((x) => x.id === picked!.id) : -1;
    const next =
      e.key === 'ArrowDown' ? Math.min(items.length - 1, at + 1) : e.key === 'ArrowUp' ? Math.max(0, at - 1) : e.key === 'Home' ? 0 : e.key === 'End' ? items.length - 1 : null;
    if (next === null) return;
    e.preventDefault();
    select(items[next]);
  });
  search.addEventListener('input', () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void load(), SEARCH_MS) as unknown as number;
  });
  typeSelect.addEventListener('change', () => void load());
  sortSelect.addEventListener('change', () => {
    sort = sortSelect.value as CatalogSort;
    void load();
  });
  orgSelect.addEventListener('change', () => {
    org = orgSelect.value;
    void load();
  });
  more.addEventListener('click', () => {
    more.disabled = true;
    pager.more().then(
      (ok) => {
        more.disabled = false;
        if (ok) paintList();
      },
      (e: unknown) => {
        more.disabled = false;
        fail(e);
      },
    );
  });
  primary.addEventListener('click', () => void runPrimary());
  cancel.addEventListener('click', () => {
    opening?.abort();
    dialog.close();
  });
  show(view);
  search.focus();
}
