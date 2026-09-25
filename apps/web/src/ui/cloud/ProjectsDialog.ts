import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { workspaceName } from '../../app/cloud/session';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import type { ProjectPermission } from '../../contracts/generated/ProjectPermission';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import { crsBySrid } from '../../geo/crs';
import { h, replaceChildren } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { openDeleteDialog, openRenameDialog } from './ProjectActions';

/**
 * Cloud projects of a workspace (an organisation, or the personal space):
 * open one, or upload the current drawing as a new one. The list holds the
 * projects this account may see there: its own, the ones shared with it and,
 * for an organisation's admins, every one (docs/adr/0015). Opening shows its
 * progress and can be cancelled; the drawing on screen is replaced only when
 * every object has arrived. The selected project can also be renamed
 * (project.edit) or deleted (project.delete), as that project's own access
 * allows; a button the account may not use says why.
 */
export function openProjectsDialog(ctx: AppContext, mode: 'open' | 'upload', pick?: { tenantId: string; projectId: string }): void {
  const cloud = ctx.cloud;
  const tenants = (cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat);
  if (!tenants.length) {
    ctx.log.warn('Hiçbir kurumda etkin üyeliğiniz ya da koltuğunuz yok; kurum yöneticinize başvurun.');
    return;
  }
  let tenant: MembershipView = tenants.find((t) => t.tenantId === (pick?.tenantId ?? cloud.project.value?.tenantId)) ?? tenants[0];
  let picked: ProjectSummary | null = null;
  let abort: AbortController | null = null;
  const label = (t: MembershipView) => workspaceName(t.tenantKind, t.tenantName, true);

  const tenantSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Çalışma alanı', disabled: tenants.length < 2 },
    tenants.map((t) => h('option', { value: t.tenantId, selected: t.tenantId === tenant.tenantId }, label(t))),
  );
  const list = h('div', { class: 'cloud-list', role: 'listbox', 'aria-label': 'Projeler' });
  const nameField = h('input', { class: 'field', value: ctx.doc.name.value, 'aria-label': 'Proje adı', spellcheck: 'false' });
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const primary = h('button', { class: 'btn btn--primary', type: 'button' }, mode === 'open' ? 'Aç' : 'Buluta yükle');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const rename = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Yeniden adlandır…');
  const remove = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Sil…');

  const body =
    mode === 'open'
      ? [h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), tenantSelect), list, progress, status]
      : [
          h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), tenantSelect),
          h('label', { class: 'cloud-field' }, h('span', null, 'Proje adı'), nameField),
          h('p', { class: 'cloud-hint' }, `${ctx.doc.size} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir. Sonra her değişiklik kendiliğinden kaydedilir.`),
          progress,
          status,
        ];
  const dialog = new Dialog({
    title: mode === 'open' ? 'Bulut projesi aç' : 'Buluta yükle',
    width: 520,
    className: 'dialog--cloud',
    content: body,
    footer: mode === 'open' ? [rename, remove, h('div', { class: 'dialog__foot-spacer' }), cancel, primary] : [h('div', { class: 'dialog__foot-spacer' }), cancel, primary],
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
    action(rename, 'project.edit', 'adlandırma');
    action(remove, 'project.delete', 'silme');
    primary.disabled = mode === 'open' ? !picked : !canCreate() || !nameField.value.trim();
    if (mode === 'upload' && !canCreate()) say(`“${label(tenant)}” kurumunda proje açma yetkiniz yok (project.create).`, 'error');
    else if (mode === 'upload') say('');
  };

  const load = async () => {
    picked = null;
    refreshButton();
    replaceChildren(list, h('p', { class: 'cloud-empty' }, 'Projeler yükleniyor…'));
    try {
      const { projects } = await cloud.projects(tenant.tenantId);
      if (!projects.length) {
        const empty =
          tenant.tenantKind === 'personal'
            ? 'Kişisel alanınızda henüz proje yok. Açık çizimi Dosya → Buluta yükle ile buraya gönderebilirsiniz.'
            : 'Bu kurumda size açık bir proje yok: sizin açtıklarınız ve sizinle paylaşılanlar burada görünür. Açık çizimi Dosya → Buluta yükle ile gönderebilirsiniz.';
        replaceChildren(list, h('p', { class: 'cloud-empty' }, empty));
        return;
      }
      replaceChildren(
        list,
        projects.map((p) => {
          const row = h(
            'button',
            { class: 'cloud-row', type: 'button', role: 'option', 'aria-selected': 'false', dataset: { id: p.id } },
            h('span', { class: 'cloud-row__name' }, p.name),
            h('span', { class: 'cloud-row__meta' }, crsBySrid(p.srid)?.name ?? `EPSG:${p.srid}`),
            h('span', { class: 'cloud-row__meta' }, new Date(p.updatedAt).toLocaleString('tr-TR', { dateStyle: 'short', timeStyle: 'short' })),
          );
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
      const chosen = pick?.tenantId === tenant.tenantId ? list.querySelector<HTMLButtonElement>(`.cloud-row[data-id="${CSS.escape(pick.projectId)}"]`) : null;
      if (chosen) {
        chosen.click();
        chosen.scrollIntoView({ block: 'nearest' });
        primary.focus();
      }
    } catch (e) {
      replaceChildren(list, h('p', { class: 'cloud-empty' }, e instanceof ApiFailure ? e.message : 'Projeler okunamadı.'));
    }
  };

  const run = async () => {
    if (primary.disabled) return;
    primary.disabled = true;
    tenantSelect.disabled = true;
    abort = new AbortController();
    try {
      const ok =
        mode === 'open'
          ? await cloud.open(tenant.tenantId, picked!.id, showProgress, abort.signal)
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

  const target = () => picked && { tenantId: tenant.tenantId, tenantName: label(tenant), projectId: picked.id, name: picked.name };
  rename.addEventListener('click', () => {
    const t = target();
    if (t) openRenameDialog(ctx, t, () => void load());
  });
  remove.addEventListener('click', () => {
    const t = target();
    if (t) openDeleteDialog(ctx, t, () => void load());
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
