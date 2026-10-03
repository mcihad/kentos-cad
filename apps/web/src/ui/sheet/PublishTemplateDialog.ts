import { DisposableStore } from '../../core/disposable';
import { errorText } from '../../product/sheet/engine';
import type { TemplateCard } from '../../product/sheet/templates';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import type { SheetHost, TemplateCloud } from './host';
import { choice } from './widgets/form';

/**
 * Kuruma yayımla (docs/sheet/design.md §13 “Kurum şablonları”): one's own
 * template, as the cloud has it, copied into an organisation's template
 * library, where every active member sees and uses it and its publisher
 * and the organisation's administrators edit it. Only the organisations
 * the account may publish to are offered. Where this template was
 * published before, the window says so and offers to put its content into
 * that copy instead (“Kurumdakini güncelle”: a new revision there). A
 * refusal is said in the server's words and the window stays open; done,
 * it closes and the gallery shows the organisation's template.
 */

export const PUBLISH_TEXTS = {
  title: 'Kuruma yayımla',
  intro: (name: string) => `“${name}” seçtiğiniz kurumun şablon kitaplığına kopyalanır: kurumun etkin, koltuklu üyeleri görür ve kullanır; siz ve kurum yöneticileri düzenler. Kendi şablonunuz olduğu gibi kalır.`,
  organisation: 'Kurum',
  already: (name: string, revision: number) => `Bu şablon bu kurumda “${name}” adıyla yayımlı (${revision}. revizyon). Kurumdakini güncellerseniz bu şablonun içeriği oradaki kopyanın yeni revizyonu olur.`,
  publish: 'Yayımla',
  update: 'Kurumdakini güncelle',
  cancel: 'Vazgeç',
  busy: 'Yayımlanıyor…',
  failed: (why: string) => `Kuruma yayımlanamadı: ${why}`,
  none: 'Yayımlayabileceğiniz bir kurum yok: kurum sahibi, yöneticiler ve proje açabilen üyeler yayımlar.',
  loading: 'Kurumlar okunuyor…',
} as const;

type Target = Awaited<ReturnType<TemplateCloud['publishTargets']>>[number];

export interface PublishOptions {
  /** Done: the organisation's template (a new one, or the copy brought up to date). */
  readonly done: (id: string) => void;
}

export function openPublishDialog(host: SheetHost, card: TemplateCard, opts: PublishOptions): void {
  const cloud = host.cloud();
  if (!cloud) return;
  const d = new DisposableStore();
  let targets: readonly Target[] | null = null;
  let chosen: string | null = null;
  let busy = false;
  let failed: string | null = null;
  const body = h('div', { class: 'sheet-form sheet-publish' });
  const cancel = h('button', { class: 'btn', type: 'button' }, PUBLISH_TEXTS.cancel);
  const publish = h('button', { class: 'btn btn--primary', type: 'button' }, PUBLISH_TEXTS.publish);
  const update = h('button', { class: 'btn btn--primary', type: 'button', hidden: true }, PUBLISH_TEXTS.update);
  const target = () => targets?.find((t) => t.tenantId === chosen) ?? null;
  const render = () => {
    d.dispose();
    const t = target();
    const copy = t?.copy;
    // One primary: “Kurumdakini güncelle” where a copy is there, else “Yayımla”.
    update.hidden = !copy;
    publish.className = copy ? 'btn' : 'btn btn--primary';
    publish.textContent = busy && !copy ? PUBLISH_TEXTS.busy : PUBLISH_TEXTS.publish;
    update.textContent = busy && copy ? PUBLISH_TEXTS.busy : PUBLISH_TEXTS.update;
    publish.disabled = update.disabled = busy || !t;
    replaceChildren(
      body,
      h('p', { class: 'sheet-insp__hint sheet-publish__intro' }, PUBLISH_TEXTS.intro(card.name)),
      targets === null
        ? h('p', { class: 'sheet-insp__hint' }, PUBLISH_TEXTS.loading)
        : targets.length
          ? choice({ label: PUBLISH_TEXTS.organisation, key: 'publish.org', value: chosen, options: targets.map((x) => ({ value: x.tenantId, label: x.name, detail: x.copy ? `yayımlı: ${x.copy.revision}. revizyon` : undefined })), readOnly: busy ? PUBLISH_TEXTS.busy : null, onChange: (v) => ((chosen = v), (failed = null), render()) }, d)
          : note('warn', PUBLISH_TEXTS.none),
      copy ? note('info', PUBLISH_TEXTS.already(copy.name, copy.revision)) : null,
      failed ? note('warn', PUBLISH_TEXTS.failed(failed)) : null,
    );
  };
  const run = async (how: 'publish' | 'update') => {
    const t = target();
    if (!t || busy) return;
    busy = true;
    failed = null;
    render();
    try {
      if (how === 'update' && t.copy) {
        await cloud.republish(card.id, t.copy.id);
        dialog.close();
        opts.done(t.copy.id);
      } else {
        const id = await cloud.publish(card.id, t.tenantId);
        dialog.close();
        opts.done(id);
      }
    } catch (e) {
      busy = false;
      failed = errorText(e);
      render();
    }
  };
  cancel.addEventListener('click', () => dialog.close());
  publish.addEventListener('click', () => void run('publish'));
  update.addEventListener('click', () => void run('update'));
  const dialog = new Dialog({
    title: PUBLISH_TEXTS.title,
    width: 520,
    className: 'sheet-dialog dialog--publish',
    stack: true,
    content: [h('div', { class: 'sheet-publish__head' }, icon('cloudUpload', 18), h('b', null, card.name)), body],
    footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, publish, update],
    onClose: () => d.dispose(),
  });
  render();
  void cloud.publishTargets(card.id).then((list) => {
    targets = list;
    chosen = list.find((x) => x.copy)?.tenantId ?? list[0]?.tenantId ?? null;
    render();
  });
}
