import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { workspaceName } from '../../app/cloud/session';
import { ROLE_LABEL, sharedWithMe } from '../../app/cloud/sharing';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { crsBySrid } from '../../geo/crs';
import { h, replaceChildren } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { openDeleteDialog, openRenameDialog } from './ProjectActions';
import { openShareDialog } from './ShareDialog';

/**
 * Cloud projects: open one, or upload the current drawing as a new one.
 * Opening has two lists (docs/adr/0015, TODOS.md CLOUD-04): a workspace's
 * projects (an organisation, or the personal space), which holds the
 * account's own, the ones shared with it there and, for an organisation's
 * admins, every one; and “Benimle paylaşılanlar”, the projects others shared
 * with the account from any workspace, with their owner and the account's
 * role. Both come from the server's lists, already filtered by access.
 * Opening shows its progress and can be cancelled; the drawing on screen is
 * replaced only when every object has arrived. The selected project can
 * also be shared (project.share), renamed (project.edit) or deleted
 * (project.delete), as that project's own access allows; a button the
 * account may not use says why.
 */
export function openProjectsDialog(ctx: AppContext, mode: 'open' | 'upload', pick?: { tenantId: string; projectId: string }): void {
  const cloud = ctx.cloud;
  const tenants = (cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat);
  if (!tenants.length) {
    ctx.log.warn('Hiçbir kurumda etkin üyeliğiniz ya da koltuğunuz yok; kurum yöneticinize başvurun.');
    return;
  }
  let tenant: MembershipView = tenants.find((t) => t.tenantId === (pick?.tenantId ?? cloud.project.value?.tenantId)) ?? tenants[0];
  // A project picked elsewhere outside the account's workspaces was shared with it from someone's personal space.
  let view: 'workspace' | 'shared' = pick && !tenants.some((t) => t.tenantId === pick.tenantId) ? 'shared' : 'workspace';
  let picked: ProjectSummary | null = null;
  let abort: AbortController | null = null;
  let loads = 0;
  const label = (t: MembershipView) => workspaceName(t.tenantKind, t.tenantName, true);
  const placeOf = (p: ProjectSummary) => workspaceName(p.tenantKind, p.tenantName, !!cloud.membership(p.tenantId));
  const when = (iso: string) => new Date(iso).toLocaleString('tr-TR', { dateStyle: 'short', timeStyle: 'short' });

  const tenantSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Çalışma alanı', disabled: tenants.length < 2 },
    tenants.map((t) => h('option', { value: t.tenantId, selected: t.tenantId === tenant.tenantId }, label(t))),
  );
  const tenantField = h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), tenantSelect);
  const sharedHint = h('p', { class: 'cloud-hint', hidden: true }, 'Başkalarının sizinle paylaştığı projeler; sahibi ve rolünüz yanında yazar.');
  const list = h('div', { class: 'cloud-list', role: 'listbox', 'aria-label': 'Projeler' });
  const nameField = h('input', { class: 'field', value: ctx.doc.name.value, 'aria-label': 'Proje adı', spellcheck: 'false' });
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const primary = h('button', { class: 'btn btn--primary', type: 'button' }, mode === 'open' ? 'Aç' : 'Buluta yükle');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const share = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Paylaş…');
  const rename = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Yeniden adlandır…');
  const remove = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Sil…');
  const tab = (id: typeof view, text: string) => {
    const b = h('button', { class: 'tab', type: 'button', role: 'tab', 'aria-selected': String(view === id), dataset: { view: id } }, text);
    b.addEventListener('click', () => show(id));
    return b;
  };
  const tabs = h('div', { class: 'cloud-tabs', role: 'tablist', 'aria-label': 'Proje listeleri' }, tab('workspace', 'Çalışma alanı'), tab('shared', 'Benimle paylaşılanlar'));

  const body =
    mode === 'open'
      ? [tabs, tenantField, sharedHint, list, progress, status]
      : [
          tenantField,
          h('label', { class: 'cloud-field' }, h('span', null, 'Proje adı'), nameField),
          h('p', { class: 'cloud-hint' }, `${ctx.doc.size} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir. Sonra her değişiklik kendiliğinden kaydedilir.`),
          progress,
          status,
        ];
  const dialog = new Dialog({
    title: mode === 'open' ? 'Bulut projesi aç' : 'Buluta yükle',
    width: 560,
    className: 'dialog--cloud',
    content: body,
    footer: mode === 'open' ? [share, rename, remove, h('div', { class: 'dialog__foot-spacer' }), cancel, primary] : [h('div', { class: 'dialog__foot-spacer' }), cancel, primary],
    onClose: () => abort?.abort(),
  });

  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const showProgress = (done: number, total: number) => {
    progress.hidden = false;
    bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
    say(`${done} / ${total} nesne`);
  };
  const canCreate = () => tenant.capabilities.includes('project.create');
  // A disabled button says why: nothing picked, or the right it needs in the picked project.
  const action = (b: HTMLButtonElement, permission: ProjectPermission, what: string) => {
    const allowed = !!picked?.access.permissions.includes(permission);
    b.disabled = !allowed;
    b.title = !picked ? 'Önce listeden bir proje seçin.' : !allowed ? `“${picked.name}” projesinde ${what} yetkiniz yok (${permission}); proje sahibine ya da yöneticisine başvurun.` : '';
  };
  const refreshButton = () => {
    action(share, 'project.share', 'paylaşma');
    action(rename, 'project.edit', 'adlandırma');
    action(remove, 'project.delete', 'silme');
    primary.disabled = mode === 'open' ? !picked : !canCreate() || !nameField.value.trim();
    if (mode === 'upload' && !canCreate()) say(`“${label(tenant)}” kurumunda proje açma yetkiniz yok (project.create).`, 'error');
    else if (mode === 'upload') say('');
  };

  /** A row of a workspace's list: name, coordinate system, last change. */
  const workspaceRow = (p: ProjectSummary) =>
    h(
      'button',
      { class: 'cloud-row', type: 'button', role: 'option', 'aria-selected': 'false', dataset: { id: p.id } },
      h('span', { class: 'cloud-row__name' }, p.name),
      h('span', { class: 'cloud-row__meta' }, crsBySrid(p.srid)?.name ?? `EPSG:${p.srid}`),
      h('span', { class: 'cloud-row__meta' }, when(p.updatedAt)),
    );
  /** A row of “Benimle paylaşılanlar”: name, whose it is and where, the account's role, last change. */
  const sharedRow = (p: ProjectSummary) =>
    h(
      'button',
      { class: 'cloud-row cloud-row--shared', type: 'button', role: 'option', 'aria-selected': 'false', dataset: { id: p.id } },
      h(
        'span',
        { class: 'cloud-row__main' },
        h('span', { class: 'cloud-row__name' }, p.name),
        h('span', { class: 'cloud-row__sub' }, `Sahibi: ${p.ownerName || 'görünmüyor'} · ${placeOf(p)}`),
      ),
      h('span', { class: 'cloud-row__role', title: 'Bu projedeki rolünüz' }, ROLE_LABEL[p.access.role]),
      h('span', { class: 'cloud-row__meta' }, when(p.updatedAt)),
    );

  const load = async () => {
    const asked = ++loads;
    picked = null;
    refreshButton();
    tenantField.hidden = view === 'shared';
    sharedHint.hidden = view !== 'shared';
    list.setAttribute('aria-label', view === 'shared' ? 'Benimle paylaşılan projeler' : 'Projeler');
    replaceChildren(list, h('p', { class: 'cloud-empty' }, 'Projeler yükleniyor…'));
    try {
      const projects = view === 'shared' ? sharedWithMe(await cloud.myProjects()) : (await cloud.projects(tenant.tenantId)).projects;
      // Another list was asked for meanwhile: this answer is not shown.
      if (asked !== loads) return;
      if (!projects.length) {
        const empty =
          view === 'shared'
            ? 'Sizinle paylaşılmış bir proje yok. Biri bir projeyi sizinle paylaşınca burada, sahibinin adı ve rolünüzle görünür.'
            : tenant.tenantKind === 'personal'
              ? 'Kişisel alanınızda henüz proje yok. Açık çizimi Dosya → Buluta yükle ile buraya gönderebilirsiniz.'
              : 'Bu kurumda size açık bir proje yok: sizin açtıklarınız ve sizinle paylaşılanlar burada görünür. Açık çizimi Dosya → Buluta yükle ile gönderebilirsiniz.';
        replaceChildren(list, h('p', { class: 'cloud-empty' }, empty));
        return;
      }
      replaceChildren(
        list,
        projects.map((p) => {
          const row = view === 'shared' ? sharedRow(p) : workspaceRow(p);
          row.addEventListener('click', () => {
            picked = p;
            for (const r of list.querySelectorAll('.cloud-row')) r.setAttribute('aria-selected', String(r === row));
            refreshButton();
          });
          row.addEventListener('dblclick', () => void run());
          return row;
        }),
      );
      // A project picked elsewhere (the application menu) is selected, ready for Aç.
      const inView = view === 'shared' || pick?.tenantId === tenant.tenantId;
      const chosen = pick && inView ? list.querySelector<HTMLButtonElement>(`.cloud-row[data-id="${CSS.escape(pick.projectId)}"]`) : null;
      if (chosen) {
        chosen.click();
        chosen.scrollIntoView({ block: 'nearest' });
        primary.focus();
      }
    } catch (e) {
      if (asked !== loads) return;
      replaceChildren(list, h('p', { class: 'cloud-empty' }, e instanceof ApiFailure ? e.message : 'Projeler okunamadı.'));
    }
  };

  const show = (next: typeof view) => {
    if (next === view || abort) return;
    view = next;
    for (const b of tabs.querySelectorAll('.tab')) b.setAttribute('aria-selected', String((b as HTMLElement).dataset.view === view));
    say('');
    void load();
  };

  const run = async () => {
    if (primary.disabled) return;
    primary.disabled = true;
    tenantSelect.disabled = true;
    abort = new AbortController();
    try {
      const ok =
        mode === 'open'
          ? await cloud.open(picked!.tenantId, picked!.id, showProgress, abort.signal)
          : await cloud.upload(tenant.tenantId, nameField.value.trim(), showProgress);
      if (ok) dialog.close();
    } catch (e) {
      if (abort.signal.aborted) return;
      say(e instanceof ApiFailure || e instanceof Error ? e.message : 'İşlem tamamlanamadı.', 'error');
      primary.disabled = false;
      tenantSelect.disabled = tenants.length < 2;
      progress.hidden = true;
    } finally {
      abort = null;
    }
  };

  const target = () => picked && { tenantId: picked.tenantId, tenantName: placeOf(picked), projectId: picked.id, name: picked.name };
  share.addEventListener('click', () => {
    const t = target();
    if (t) openShareDialog(ctx, t, { stack: true, done: () => void load() });
  });
  rename.addEventListener('click', () => {
    const t = target();
    if (t) openRenameDialog(ctx, t, () => void load());
  });
  remove.addEventListener('click', () => {
    const t = target();
    if (t) openDeleteDialog(ctx, t, () => void load());
  });
  tabs.addEventListener('keydown', (e) => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    show(view === 'workspace' ? 'shared' : 'workspace');
    (tabs.querySelector('[aria-selected="true"]') as HTMLElement | null)?.focus();
  });
  tenantSelect.addEventListener('change', () => {
    tenant = tenants.find((t) => t.tenantId === tenantSelect.value) ?? tenant;
    if (mode === 'open') void load();
    else refreshButton();
  });
  nameField.addEventListener('input', refreshButton);
  primary.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => {
    abort?.abort();
    dialog.close();
  });
  if (mode === 'open') void load();
  else {
    refreshButton();
    nameField.select();
  }
}
