import type { AppContext } from '../../app/context';
import { ApiFailure } from '../../app/cloud/api';
import { workspaceName } from '../../app/cloud/session';
import type { MembershipView } from '../../contracts/generated/MembershipView';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { catalogFields } from './ProjectForms';

/**
 * “Buluta yükle”: the open drawing as a new cloud project in a workspace
 * the account may open projects in (its personal space or an organisation),
 * with the catalog's type, description and tags (docs/adr/0028). The
 * upload shows its progress; the project is the account's until it shares
 * it, and every change is saved by itself from then on.
 */
export function openUploadDialog(ctx: AppContext): void {
  const cloud = ctx.cloud;
  const tenants = (cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat);
  if (!tenants.length) {
    ctx.log.warn('Hiçbir çalışma alanında etkin üyeliğiniz ya da koltuğunuz yok; kurum yöneticinize başvurun.');
    return;
  }
  let tenant: MembershipView = tenants.find((t) => t.tenantId === cloud.project.value?.tenantId && t.capabilities.includes('project.create')) ?? tenants.find((t) => t.capabilities.includes('project.create')) ?? tenants[0];
  const label = (t: MembershipView) => workspaceName(t.tenantKind, t.tenantName, true);
  const tenantSelect = h(
    'select',
    { class: 'field', 'aria-label': 'Çalışma alanı', disabled: tenants.length < 2 },
    tenants.map((t) => h('option', { value: t.tenantId, selected: t.tenantId === tenant.tenantId }, label(t))),
  );
  const nameField = h('input', { class: 'field', value: ctx.doc.name.value, 'aria-label': 'Proje adı', spellcheck: 'false', maxlength: '200' });
  const fields = catalogFields();
  const status = h('p', { class: 'cloud-status', role: 'status' });
  const bar = h('span');
  const progress = h('div', { class: 'cloud-progress', hidden: true }, bar);
  const primary = h('button', { class: 'btn btn--primary', type: 'button' }, 'Buluta yükle');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Buluta yükle',
    width: 560,
    className: 'dialog--cloud',
    content: [
      h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), tenantSelect),
      h('label', { class: 'cloud-field' }, h('span', null, 'Proje adı'), nameField),
      ...fields.elements,
      h('p', { class: 'cloud-hint' }, `${ctx.doc.size} nesne, katman ağacı, proje ayarları ve proje stilleri yüklenir. Proje sizin olur; başkaları paylaşımla eklenir. Sonra her değişiklik kendiliğinden kaydedilir.`),
      progress,
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, primary],
  });
  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  const canCreate = () => tenant.capabilities.includes('project.create');
  const refresh = () => {
    primary.disabled = !canCreate() || !nameField.value.trim();
    if (!canCreate()) say(`“${label(tenant)}” çalışma alanında proje açma yetkiniz yok (project.create); başka bir çalışma alanı seçin ya da kurum yöneticinize başvurun.`, 'error');
    else say('');
  };
  const run = async () => {
    if (primary.disabled) return;
    primary.disabled = true;
    tenantSelect.disabled = true;
    try {
      const ok = await cloud.upload(
        tenant.tenantId,
        nameField.value.trim(),
        (done, total) => {
          progress.hidden = false;
          bar.style.width = `${total ? Math.round((done / total) * 100) : 100}%`;
          say(`${done} / ${total} nesne`);
        },
        fields.read(),
      );
      if (ok) dialog.close();
    } catch (e) {
      say(e instanceof ApiFailure || e instanceof Error ? e.message : 'Yükleme tamamlanamadı.', 'error');
      primary.disabled = false;
      tenantSelect.disabled = tenants.length < 2;
      progress.hidden = true;
    }
  };
  tenantSelect.addEventListener('change', () => {
    tenant = tenants.find((t) => t.tenantId === tenantSelect.value) ?? tenant;
    refresh();
  });
  nameField.addEventListener('input', refresh);
  primary.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  refresh();
  nameField.select();
}
