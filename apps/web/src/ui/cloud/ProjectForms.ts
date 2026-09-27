import type { AppContext } from '../../app/context';
import { PROJECT_TYPES, TYPE_HINT, TYPE_LABEL, parseTags } from '../../app/cloud/catalog';
import {
  DESCRIPTION_MAX,
  FORM_TEXTS as T,
  NAME_MAX,
  convertForm,
  convertReason,
  convertedLine,
  copyName,
  creatablePlaces,
  duplicateSavable,
  metadataPatch,
  metadataSavable,
  type Place,
} from '../../app/cloud/formsPlan';
import type { ProjectDuplicated } from '../../contracts/generated/ProjectDuplicated';
import type { ProjectSummary } from '../../contracts/generated/ProjectSummary';
import type { ProjectType } from '../../contracts/generated/ProjectType';
import { h } from '../dom';
import { Dialog } from '../widgets/Dialog';
import { reason } from './ProjectActions';

/**
 * The catalog's forms (docs/adr/0028, TODOS.md CLOUD-02, CLOUD-03,
 * CLOUD-05): a project's catalog information (name, type, description,
 * tags), a copy of a project, and a new project in the other storage mode
 * (docs/adr/0039). The type is a label for finding work: the form says it
 * opens no module and claims no compliance. The server checks and
 * normalizes every value; the form only offers them. What the forms say and
 * offer is formsPlan.ts's; this file draws them.
 */

/** Type, description and tags as fields, shared by these forms and the upload. */
export function catalogFields(initial: { projectType?: ProjectType; description?: string; tags?: readonly string[] } = {}) {
  const F = T.fields;
  const type = h(
    'select',
    { class: 'field', 'aria-label': F.typeLabel },
    PROJECT_TYPES.map((t) => h('option', { value: t, selected: t === (initial.projectType ?? 'cad') }, TYPE_LABEL[t])),
  );
  const description = h('textarea', { class: 'field cloud-textarea', rows: '3', maxlength: String(DESCRIPTION_MAX), 'aria-label': F.description, placeholder: F.descriptionPlaceholder });
  description.value = initial.description ?? '';
  const tags = h('input', { class: 'field', value: (initial.tags ?? []).join(', '), 'aria-label': F.tags, placeholder: F.tagsPlaceholder, spellcheck: 'false' });
  const elements = [
    h('label', { class: 'cloud-field' }, h('span', null, F.type), type),
    h('p', { class: 'cloud-hint cloud-hint--tight' }, TYPE_HINT),
    h('label', { class: 'cloud-field' }, h('span', null, F.description), description),
    h('label', { class: 'cloud-field' }, h('span', null, F.tags), tags),
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
  const M = T.metadata;
  const name = h('input', { class: 'field', value: p.name, 'aria-label': M.name, spellcheck: 'false', maxlength: String(NAME_MAX) });
  const fields = catalogFields(p);
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const save = h('button', { class: 'btn btn--primary', type: 'button', disabled: true }, M.save);
  const cancel = h('button', { class: 'btn', type: 'button' }, T.cancel);
  const dialog = new Dialog({
    title: M.title,
    width: 520,
    className: 'dialog--cloud',
    stack: true,
    content: [h('label', { class: 'cloud-field' }, h('span', null, M.name), name), ...fields.elements, status],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, save],
  });
  const patch = () => metadataPatch(p, { name: name.value, ...fields.read() });
  const refresh = () => {
    save.disabled = !metadataSavable(name.value, patch());
  };
  const run = async () => {
    if (save.disabled) return;
    save.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = T.saving;
    try {
      await ctx.cloud.lifecycle.updateMetadata({ tenantId: p.tenantId, projectId: p.id, name: p.name }, patch(), p.catalogVersion);
      dialog.close();
      ctx.log.success(M.saved(name.value.trim()));
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
  const D = T.duplicate;
  // The source's workspace first when the account may open projects there, then the rest (the personal space last).
  const places = creatableWorkspaces(ctx, p.tenantId);
  const name = h('input', { class: 'field', value: copyName(p.name), 'aria-label': D.nameLabel, spellcheck: 'false', maxlength: String(NAME_MAX) });
  const place = placeSelect(places, D.placeLabel);
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const make = h('button', { class: 'btn btn--primary', type: 'button', disabled: !duplicateSavable(places.length, name.value) }, D.make);
  const cancel = h('button', { class: 'btn', type: 'button' }, T.cancel);
  const dialog = new Dialog({
    title: D.title,
    width: 520,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('p', null, D.lead(p.name)),
      h('ul', { class: 'cloud-consequences' }, D.consequences.map((c) => h('li', null, c))),
      h('label', { class: 'cloud-field' }, h('span', null, D.name), name),
      h('label', { class: 'cloud-field' }, h('span', null, T.place), place),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });
  if (!places.length) {
    status.dataset.kind = 'error';
    status.textContent = T.noPlace;
  }
  const run = async () => {
    if (make.disabled || !name.value.trim()) return;
    make.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = D.running;
    try {
      const copy = await ctx.cloud.lifecycle.duplicate({ tenantId: p.tenantId, projectId: p.id, name: p.name }, { name: name.value, tenantId: place.value });
      dialog.close();
      ctx.log.success(D.done(copy.project.name, copy.objects));
      done?.(copy);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = reason(e);
      make.disabled = false;
    }
  };
  name.addEventListener('input', () => (make.disabled = !duplicateSavable(places.length, name.value)));
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

/** The workspaces the account may open projects in, the source's first (the personal space last): formsPlan.ts `creatablePlaces`. */
export function creatableWorkspaces(ctx: AppContext, first: string): Place[] {
  return creatablePlaces(ctx.cloud.me.value?.memberships ?? [], first);
}

/** The workspace list of a form: off while it offers fewer than two. */
export function placeSelect(places: readonly Place[], label: string): HTMLSelectElement {
  return h('select', { class: 'field', 'aria-label': label, disabled: places.length < 2 }, places.map((m) => h('option', { value: m.tenantId }, m.label)));
}

/**
 * “PostGIS'e aktar” (a file project) or “Dosya projesine çevir” (a database
 * project): a new project in the other storage mode from this one's present
 * state (docs/adr/0039), with an optional name, in a workspace the account
 * may open projects in. The source does not change. `done` gets the new
 * project and the mode it is kept as.
 */
export function openConvertDialog(ctx: AppContext, p: ProjectSummary, done?: (made: ProjectDuplicated) => void): void {
  const C = T.convert;
  const places = creatableWorkspaces(ctx, p.tenantId);
  const form = convertForm(p, !!ctx.cloud.openProject(p.tenantId, p.id) && ctx.doc.dirty.value);
  const name = h('input', { class: 'field', value: '', placeholder: form.placeholder, 'aria-label': C.nameLabel, spellcheck: 'false', maxlength: String(NAME_MAX) });
  const place = placeSelect(places, C.placeLabel);
  const status = h('p', { class: 'cloud-status', role: 'alert' });
  const make = h('button', { class: 'btn btn--primary', type: 'button', disabled: !places.length }, form.title);
  const cancel = h('button', { class: 'btn', type: 'button' }, T.cancel);
  const dialog = new Dialog({
    title: form.title,
    width: 540,
    className: 'dialog--cloud',
    stack: true,
    content: [
      h('p', null, form.lead),
      h('ul', { class: 'cloud-consequences' }, form.consequences.map((c) => h('li', null, c))),
      h('label', { class: 'cloud-field' }, h('span', null, C.name), name),
      h('label', { class: 'cloud-field' }, h('span', null, T.place), place),
      status,
    ],
    footer: [h('div', { class: 'dialog__spacer' }), cancel, make],
  });
  if (!places.length) {
    status.dataset.kind = 'error';
    status.textContent = T.noPlace;
  }
  const run = async () => {
    if (make.disabled) return;
    make.disabled = true;
    status.dataset.kind = 'info';
    status.textContent = form.running;
    try {
      const made = await ctx.cloud.lifecycle.convert({ tenantId: p.tenantId, projectId: p.id, name: p.name }, form.to, { name: name.value, tenantId: place.value });
      dialog.close();
      ctx.log.success(convertedLine(made.project.name, made.objects, form.to));
      done?.(made);
    } catch (e) {
      status.dataset.kind = 'error';
      status.textContent = convertReason(e);
      make.disabled = false;
    }
  };
  name.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      void run();
    }
  });
  make.addEventListener('click', () => void run());
  cancel.addEventListener('click', () => dialog.close());
  name.focus();
}
