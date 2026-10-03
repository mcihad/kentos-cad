import type { AppContext } from '../../app/context';
import { initials, SHARE_TEXTS } from '../../app/cloud/sharePlan';
import { failureText } from '../../app/cloud/sharing';
import type { SheetTemplateAccess } from '../../contracts/generated/sheet/SheetTemplateAccess';
import type { TemplateGrantRole } from '../../contracts/generated/sheet/TemplateGrantRole';
import type { TemplateCard } from '../../product/sheet/templates';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { askRemove } from '../widgets/confirm';
import { note } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { SHARE_NEEDS_SYNC } from './galleryPlan';
import type { SheetHost, TemplateCloud } from './host';
import { createTemplateFinder } from './templateFinder';

/**
 * Şablonu paylaş (docs/sheet/design.md §13), in “Projeyi paylaş”'s family
 * (ui/cloud/ShareDialog.ts): the template and where it is kept; a person
 * found among those who share an organisation with you (the server's
 * search, ADR 0024: no open e-mail search), given a role (görüntüleyebilir,
 * düzenleyebilir); who has it, each role changed in its row or taken away
 * after a question. Only the owner shares. A template only on this device
 * cannot be shared: the window says so first and offers to sync it, and
 * goes on with the synced one. A refusal is said in the server's words.
 */

export interface ShareTemplateOptions {
  readonly stack?: boolean;
  /** Why it cannot be shared now (only on this device: “Paylaşmak için önce buluta eşitleyin.”); null when it can. */
  readonly reason: string | null;
}

export const TEMPLATE_ROLES: readonly { readonly value: TemplateGrantRole; readonly label: string; readonly hint: string }[] = [
  { value: 'viewer', label: 'Görüntüleyebilir', hint: 'Şablonu kullanır ve kopyalar; değiştiremez.' },
  { value: 'editor', label: 'Düzenleyebilir', hint: 'Şablonun yeni sürümünü de kaydeder; paylaşamaz ve silemez.' },
];
const roleLabel = (r: TemplateGrantRole) => TEMPLATE_ROLES.find((x) => x.value === r)!.label.toLocaleLowerCase('tr');

export const SHARE_TEMPLATE_TEXTS = {
  title: 'Şablonu paylaş',
  inCloud: 'Bulutta: kişisel alanınızda; web ve masaüstünde aynı.',
  onDevice: 'Bu cihazda: yalnız bu tarayıcıda saklanıyor, bulutta kopyası yok.',
  sync: 'Buluta eşitle',
  syncing: 'Buluta eşitleniyor…',
  notSynced: 'Şablon şimdi eşitlenemedi; bağlantı gelince kendiliğinden gider.',
  notOwner: 'Yalnız şablonun sahibi paylaşır.',
  pick: 'Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.',
  search: 'Yalnız ortak kurumlarınızın etkin üyeleri bulunur; genel e-posta araması yoktur. Kurum dışındaki kişiye e-postayla davet sonraki aşamada gelecek.',
  policy: 'Paylaşım kaldırılınca şablon karşı tarafın “Benimle paylaşılanlar” listesinden kalkar; ondan yapılmış paftalar kalır.',
  shared: (name: string, role: TemplateGrantRole, changed: boolean) => (changed ? `${name} artık bu şablonu ${roleLabel(role)}.` : `${name} zaten ${roleLabel(role)}; değişen bir şey yok.`),
  revoked: (name: string) => `${name} artık bu şablonu göremiyor.`,
  owner: 'şablonun sahibi',
} as const;

export function openShareTemplateDialog(ctx: AppContext, host: SheetHost, first: TemplateCard, opts: ShareTemplateOptions): void {
  let card = first;
  let access: SheetTemplateAccess | null = null;
  let busy = false;
  let open = true;
  const me = ctx.cloud.me.value?.user;
  const cloud = (): TemplateCloud | null => host.cloud();
  /** The owner's template of the cloud, with the cloud reachable: the window shares it. */
  const live = () => card.source === 'cloud' && !!cloud() && !cloud()!.why();

  const status = h('p', { class: 'cloud-status', role: 'status' });
  const say = (text: string, kind: 'info' | 'error' = 'info') => {
    status.textContent = text;
    status.dataset.kind = kind;
    status.setAttribute('role', kind === 'error' ? 'alert' : 'status');
  };
  const finder = createTemplateFinder({
    open: () => open,
    search: (q, signal) => cloud()!.candidates(card.id, q, signal),
    has: (userId) => access?.grants.find((g) => g.userId === userId)?.role ?? null,
    roleLabel: (r) => TEMPLATE_ROLES.find((x) => x.value === r)!.label,
    say,
    picked: () => refresh(),
    submit: () => void share(),
  });
  const role = h('select', { class: 'field', 'aria-label': SHARE_TEXTS.people.role }, TEMPLATE_ROLES.map((r) => h('option', { value: r.value, title: r.hint, selected: r.value === 'viewer' }, r.label)));
  const roleHint = h('p', { class: 'cloud-hint share-rolehint' }, TEMPLATE_ROLES[0].hint);
  role.addEventListener('change', () => (roleHint.textContent = TEMPLATE_ROLES.find((r) => r.value === role.value)?.hint ?? ''));
  const add = h('button', { class: 'btn btn--primary', type: 'button' }, SHARE_TEXTS.people.share);
  const head = h('p', { class: 'share-head' });
  const where = h('p', { class: 'share-storage' });
  const gate = h('div', { class: 'share-template__sync' });
  const count = h('span', { class: 'share-count' });
  const people = h('div', { class: 'share-list', role: 'list', 'aria-label': SHARE_TEXTS.people.listLabel });
  const close = h('button', { class: 'btn', type: 'button' }, SHARE_TEXTS.close);

  const refresh = () => {
    const can = live();
    finder.input.disabled = !can || busy;
    role.disabled = !can || busy;
    add.disabled = !can || busy || !finder.chosen();
    add.title = !can ? (opts.reason ?? cloud()?.why() ?? SHARE_TEMPLATE_TEXTS.notOwner) : finder.chosen() ? '' : SHARE_TEMPLATE_TEXTS.pick;
  };

  const ownerRow = () => {
    const theirs = card.source === 'shared';
    const name = access?.ownerName || (theirs ? (card.sharedBy ?? '') : (me?.displayName ?? 'Siz'));
    return h(
      'div',
      { class: 'share-row', role: 'listitem' },
      h('span', { class: 'share-avatar', 'aria-hidden': 'true' }, initials(name)),
      h('div', { class: 'share-who' }, h('span', { class: 'share-name' }, name, theirs || !me ? null : h('span', { class: 'share-you' }, SHARE_TEXTS.people.you)), h('span', { class: 'share-sub' }, !theirs && me?.email ? `${me.email} · ${SHARE_TEMPLATE_TEXTS.owner}` : SHARE_TEMPLATE_TEXTS.owner)),
      h('span', { class: 'share-fixed', title: 'Sahibin erişimi değişmez.', 'aria-label': 'Sahibin erişimi değişmez.' }, icon('lock', 14)),
    );
  };

  const grantRow = (g: SheetTemplateAccess['grants'][number]) => {
    const select = h('select', { class: 'field share-rolesel', 'aria-label': SHARE_TEXTS.people.roleLabel(g.displayName), disabled: busy || !live() }, TEMPLATE_ROLES.map((r) => h('option', { value: r.value, title: r.hint, selected: r.value === g.role }, r.label)));
    select.addEventListener('change', () => void change(g.userId, g.displayName, select.value as TemplateGrantRole, select));
    const remove = h('button', { class: 'btn btn--ghost btn--small share-remove', type: 'button', disabled: busy || !live(), 'aria-label': SHARE_TEXTS.people.removeLabel(g.displayName) }, SHARE_TEXTS.people.remove);
    remove.addEventListener('click', () => void revoke(g.userId, g.displayName));
    return h(
      'div',
      { class: 'share-row', role: 'listitem', dataset: { user: g.userId } },
      h('span', { class: 'share-avatar', 'aria-hidden': 'true' }, initials(g.displayName)),
      h('div', { class: 'share-who' }, h('span', { class: 'share-name' }, g.displayName), h('span', { class: 'share-sub' }, [g.email, `paylaşan: ${g.grantedByName}`].filter(Boolean).join(' · '))),
      select,
      remove,
    );
  };

  const render = () => {
    const local = card.source === 'device';
    const why = local ? SHARE_NEEDS_SYNC : card.source === 'shared' ? SHARE_TEMPLATE_TEXTS.notOwner : (cloud()?.why() ?? null);
    replaceChildren(head, h('b', null, `“${card.name}”`), h('span', null, ` · Sürüm ${card.revision}`));
    replaceChildren(where, icon(local ? 'save' : 'cloud', 16), h('span', null, local ? SHARE_TEMPLATE_TEXTS.onDevice : SHARE_TEMPLATE_TEXTS.inCloud));
    const away = cloud()?.why() ?? null;
    const sync = h('button', { class: 'btn btn--small', type: 'button', disabled: busy || !cloud() || !!away }, icon('cloud', 14), busy ? SHARE_TEMPLATE_TEXTS.syncing : SHARE_TEMPLATE_TEXTS.sync);
    sync.addEventListener('click', () => void upload());
    replaceChildren(
      gate,
      why ? note('warn', h('strong', null, local ? 'Önce eşitleyin.' : 'Şimdi paylaşılamıyor.'), ` ${local ? `${why} Paylaşım bulut hesabındaki şablonla yapılır; eşitlenince bu pencere kişileri ve rolleri gösterir.` : why}`) : null,
      local ? h('div', { class: 'share-template__actions' }, sync, away ? h('span', { class: 'cloud-hint' }, away) : null) : null,
    );
    const rows = [ownerRow(), ...(access?.grants ?? []).map(grantRow)];
    count.textContent = ` ${rows.length}`;
    replaceChildren(people, ...rows);
    refresh();
  };

  const load = async () => {
    if (!live()) return render();
    replaceChildren(people, h('p', { class: 'cloud-empty' }, SHARE_TEXTS.people.loading));
    try {
      const next = await cloud()!.access(card.id);
      if (!open) return;
      access = next;
      render();
    } catch (e) {
      if (!open) return;
      render();
      say(failureText(e, SHARE_TEXTS.people.readFailed), 'error');
    }
  };

  /** One change of the sharing: said while it goes, then the list read again and what it did said (or the server's refusal). */
  const act = async (doing: string, run: () => Promise<string>, failed: string) => {
    busy = true;
    render();
    say(doing);
    try {
      const text = await run();
      await load();
      say(text);
    } catch (e) {
      say(failureText(e, failed), 'error');
    } finally {
      busy = false;
      if (open) render();
    }
  };

  const share = async () => {
    const who = finder.chosen();
    if (!who || busy || !live()) return;
    const r = role.value as TemplateGrantRole;
    await act(
      SHARE_TEXTS.people.adding(who.displayName),
      async () => {
        const answer = await cloud()!.share(card.id, who.userId, r);
        const text = SHARE_TEMPLATE_TEXTS.shared(who.displayName, r, answer.changed);
        if (answer.changed) ctx.log.success(`“${card.name}”: ${text}`);
        finder.clear();
        return text;
      },
      SHARE_TEXTS.people.shareFailed,
    );
    if (open && live()) finder.input.focus();
  };

  const change = (userId: string, name: string, r: TemplateGrantRole, select: HTMLSelectElement) => {
    select.disabled = true;
    return act(
      SHARE_TEXTS.people.changing(name),
      async () => {
        await cloud()!.share(card.id, userId, r);
        const text = SHARE_TEMPLATE_TEXTS.shared(name, r, true);
        ctx.log.success(`“${card.name}”: ${text}`);
        return text;
      },
      SHARE_TEXTS.people.changeFailed,
    );
  };

  const revoke = async (userId: string, name: string) => {
    const sure = await askRemove({ title: 'Paylaşımı kaldır', message: `${name}, “${card.name}” şablonunu artık görmesin mi?`, details: ['Şablon “Benimle paylaşılanlar” listesinden kalkar.', 'Ondan yaptığı paftalar kalır; onlar şablonun kopyasıdır.'], action: 'Paylaşımı kaldır' });
    if (!sure || !open) return;
    await act(
      SHARE_TEXTS.people.revoking(name),
      async () => {
        await cloud()!.unshare(card.id, userId);
        const text = SHARE_TEMPLATE_TEXTS.revoked(name);
        ctx.log.success(`“${card.name}”: ${text}`);
        return text;
      },
      SHARE_TEXTS.people.revokeFailed,
    );
  };

  /** “Buluta eşitle” from here: the window goes on with the template the cloud now has. */
  const upload = async () => {
    const lib = cloud();
    if (!lib || busy) return;
    busy = true;
    render();
    say(SHARE_TEMPLATE_TEXTS.syncing);
    const id = await lib.upload(card.id, card.name);
    busy = false;
    if (!open) return;
    // The list has the template under its cloud id once it is read again.
    const mine = host.providers.find((p) => p.section === 'mine');
    await mine?.refresh();
    const found = id ? host.providers.flatMap((p) => p.cards.value).find((c) => c.id === id) : undefined;
    if (found) {
      card = found;
      say('');
      void load().then(() => open && live() && finder.input.focus());
    } else {
      render();
      say(cloud()?.why() ?? SHARE_TEMPLATE_TEXTS.notSynced, 'error');
    }
  };

  add.addEventListener('click', () => void share());
  close.addEventListener('click', () => dialog.close());
  const dialog = new Dialog({
    title: SHARE_TEMPLATE_TEXTS.title,
    width: 680,
    className: 'dialog--cloud dialog--share',
    stack: opts.stack,
    content: [
      head,
      where,
      gate,
      h('div', { class: 'share-add' }, h('div', { class: 'cloud-field share-add__who' }, h('span', null, SHARE_TEXTS.people.add), finder.el), h('label', { class: 'cloud-field' }, h('span', null, SHARE_TEXTS.people.role), role), add),
      roleHint,
      h('p', { class: 'cloud-hint' }, SHARE_TEMPLATE_TEXTS.search),
      h('h3', { class: 'share-title' }, h('span', null, SHARE_TEXTS.people.title), count),
      people,
      h('p', { class: 'cloud-hint share-policy' }, SHARE_TEMPLATE_TEXTS.policy),
      status,
    ],
    footer: [h('div', { class: 'dialog__foot-spacer' }), close],
    onClose: () => {
      open = false;
      finder.dispose();
    },
  });
  render();
  void load().then(() => open && live() && finder.input.focus());
}
