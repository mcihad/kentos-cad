import type { AppContext } from '../../app/context';
import { PROJECT_TYPES, TYPE_HINT, TYPE_LABEL, parseTags } from '../../app/cloud/catalog';
import type { MetadataPatch } from '../../app/cloud/lifecycle';
import { workspaceName } from '../../app/cloud/session';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { reason } from './ProjectActions';

/**
 * The catalog's forms (docs/adr/0028, TODOS.md CLOUD-02, CLOUD-03,
 * CLOUD-05): a project's catalog information (name, type, description,
 * tags) and a copy of a project. The type is a label for finding work: the
 * form says it opens no module and claims no compliance. The server checks
 * and normalizes every value; the form only offers them.
 */

/** Type, description and tags as fields, shared by these forms and the upload. */
export function catalogFields(initial: { projectType?: ProjectType; description?: string; tags?: readonly string[] } = {}) {
  const type = h(
    'select',
    { class: 'field', 'aria-label': 'Proje türü' },
    PROJECT_TYPES.map((t) => h('option', { value: t, selected: t === (initial.projectType ?? 'cad') }, TYPE_LABEL[t])),
  );
  const description = h('textarea', { class: 'field cloud-textarea', rows: '3', maxlength: '2000', 'aria-label': 'Açıklama', placeholder: 'İsteğe bağlı: işin konusu, yeri, dayanağı' });
  description.value = initial.description ?? '';
  const tags = h('input', { class: 'field', value: (initial.tags ?? []).join(', '), 'aria-label': 'Etiketler', placeholder: 'Virgülle ayırın: Kadıköy, 2026', spellcheck: 'false' });
  const elements = [
    h('label', { class: 'cloud-field' }, h('span', null, 'Tür'), type),
    h('p', { class: 'cloud-hint cloud-hint--tight' }, TYPE_HINT),
    h('label', { class: 'cloud-field' }, h('span', null, 'Açıklama'), description),
    h('label', { class: 'cloud-field' }, h('span', null, 'Etiketler'), tags),
  ];
  return {
    elements,
    /** The values now (tags split, spaces trimmed). */
    read: () => ({ projectType: type.value as ProjectType, description: description.value.trim(), tags: parseTags(tags.value) }),
    inputs: [type, description, tags] as HTMLElement[],
  };
}

/** “Proje bilgileri”: the name, type, description and tags of a project, from the catalog version shown. */
export function openMetadataDialog(ctx: AppContext, p: ProjectSummary, done?: () => void): void {
  const name = h('input', { class: 'field', value: p.name, 'aria-label': 'Proje adı', spellcheck: 'false', maxlength: '200' });
  const fields = catalogFields(p);
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const save = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Proje bilgileri',
    width: 520,
    className: 'dialog--cloud',
    stack: true,
    content: [h('label', { class: 'cloud-field' }, h('span', null, 'Proje adı'), name), ...fields.elements, status],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  const patch = (): MetadataPatch => {
    const now = fields.read();
    const out: MetadataPatch = {};
    if (name.value.trim() !== p.name) out.name = name.value.trim();
    if (now.projectType !== p.projectType) out.projectType = now.projectType;
    if (now.description !== p.description) out.description = now.description;
    if (now.tags.join('\n') !== p.tags.join('\n')) out.tags = now.tags;
    return out;
  };
  const refresh = () => {
    save.disabled = !name.value.trim() || !Object.keys(patch()).length;
  };
  const run = async () => {
    if (save.disabled) return;
    save.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = 'Kaydediliyor…';
    try {
      await ctx.cloud.lifecycle.updateMetadata({ tenantId: p.tenantId, projectId: p.id, name: p.name }, patch(), p.catalogVersion);
      dialog.close();
      ctx.log.success(`“${name.value.trim()}” projesinin bilgileri kaydedildi.`);
      done?.();
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      refresh();
    }
  };
  for (const el of [name, ...fields.inputs]) {
    el.addEventListener('input', refresh);
    el.addEventListener('change', refresh);
  }
  save.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
}

/** “Kopyasını oluştur”: a new project of the account's from this one, in a workspace it may open projects in. */
export function openDuplicateDialog(ctx: AppContext, p: ProjectSummary, done?: (copy: ProjectDuplicated) => void): void {
  const places = (ctx.cloud.me.value?.memberships ?? []).filter((m) => m.active && m.seat && m.capabilities.includes('project.create'));
  // The source's workspace first when the account may open projects there, then the rest (the personal space last).
  places.sort((a, b) => Number(b.tenantId === p.tenantId) - Number(a.tenantId === p.tenantId));
  const name = h('input', { class: 'field', value: `${p.name} (kopya)`, 'aria-label': 'Kopyanın adı', spellcheck: 'false', maxlength: '200' });
  const place = h(
    'select',
    { class: 'field', 'aria-label': 'Kopyanın çalışma alanı', disabled: places.length < 2 },
    places.map((m) => h('option', { value: m.tenantId }, workspaceName(m.tenantKind, m.tenantName, true))),
  );
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const make = h('button', { class: 'btn btn--primary', type: 'button', disabled: !places.length }, 'Kopyasını oluştur');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({
    title: 'Projenin kopyasını oluştur',
    width: 520,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('p', null, `“${p.name}” yeni bir proje olarak kopyalanır.`),
      h(
        'ul',
        { class: 'cloud-consequences' },
        h('li', null, 'Katmanlar, ayarlar, stiller, açıklama, tür, etiketler ve bütün nesneler kalıcı kimlikleriyle kopyalanır.'),
        h('li', null, 'Geçmiş (komut günlüğü, olaylar), paylaşımlar ve favoriler kopyalanmaz; arşivlenmiş proje etkin bir kopya olur.'),
        h('li', null, 'Kopya sizin olur: siz paylaşana kadar yalnız size ve kurum politikasıyla kurum yöneticilerine görünür.'),
      ),
      h('label', { class: 'cloud-field' }, h('span', null, 'Kopyanın adı'), name),
      h('label', { class: 'cloud-field' }, h('span', null, 'Çalışma alanı'), place),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });
  if (!places.length) {
    status.dataset.kind = 'error';
    status.textContent = 'Proje açabileceğiniz bir çalışma alanınız yok (project.create); kurum yöneticinize başvurun.';
  }
  const run = async () => {
    if (make.disabled || !name.value.trim()) return;
    make.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = 'Kopyalanıyor…';
    try {
      const copy = await ctx.cloud.lifecycle.duplicate({ tenantId: p.tenantId, projectId: p.id, name: p.name }, { name: name.value, tenantId: place.value });
      dialog.close();
      ctx.log.success(`“${copy.project.name}” oluşturuldu: ${copy.objects} nesne kopyalandı.`);
      done?.(copy);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      make.disabled = false;
    }
  };
  name.addEventListener('input', () => (make.disabled = !places.length || !name.value.trim()));
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  make.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
  name.select();
}
