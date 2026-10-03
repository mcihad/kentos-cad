import { TYPE_LABEL } from '../../app/cloud/catalog';
import { WORKSPACES } from '../../app/workspaces';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { DisposableStore } from '../../core/disposable';
import { fixed } from '../../core/displayNumber';
import { BADGE_TIP, badgeLabel, categoryLabel, FIT_LABEL, type ArrangedCard, type TemplateCard } from '../../product/sheet/templates';
import { h, type Child } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';
import { Dropdown } from '../widgets/Dropdown';
import { tooltip } from '../widgets/tooltip';
import { actionsOf, paperChoices, paperText, type GalleryAbilities, type TemplateAction } from './galleryPlan';
import { conditionText } from './inspector/variantsSection';

/**
 * The gallery's right side (docs/sheet/design.md §11, §11a, §12): the chosen
 * template's picture (its own plan, as Kullan would make it), its badges,
 * how it stands with the project, its actions, what it needs that the
 * project lacks (the engine's preflight of it), and what it is: the papers
 * it suits, its kind, its modes and project types, its revision and author.
 * Apart from the window (TemplateGallery.ts).
 */

const ACTION_ICON: Record<Exclude<TemplateAction, 'use'>, string> = { duplicate: 'copy', edit: 'edit', publish: 'cloudUpload', share: 'share', sync: 'cloud', delete: 'trash' };

const ACTION_TIP: Record<Exclude<TemplateAction, 'use'>, string> = {
  duplicate: 'Şablonun bir kopyasını şablonlarınıza (Benim, bu cihazda) alır.',
  edit: 'Şablonu pafta olarak açar; Şablon olarak kaydet ile geri yazılır. Sistem şablonu değişmez: kopyası kaydedilir.',
  publish: 'Şablonunuzu kurumun şablon kitaplığına kopyalar: kurumun etkin üyeleri görür ve kullanır. Kurumdaki kopyasını sonra buradan güncelleyebilirsiniz.',
  share: 'Şablonu kurumunuzdaki kişilerle paylaşır: görüntüleyebilir ya da düzenleyebilir.',
  sync: 'Bu cihazdaki şablonu bulut hesabınıza eşitler: web ve masaüstünde aynı olur, paylaşılabilir. Eşitlenmiş şablonlar kendiliğinden eşitlenir; bu, hemen eşitler.',
  delete: 'Şablonu siler; ondan yapılmış paftalar kalır.',
};

export function badgesOf(a: ArrangedCard): Child[] {
  const out: Child[] = a.card.badges.map((k) => h('span', { class: `tbadge tbadge--${k}`, title: BADGE_TIP[k] }, badgeLabel(k, a.card)));
  if (a.badge) out.push(h('span', { class: 'tbadge tbadge--mode' }, a.badge));
  return out;
}

/** The template's layouts (design §3.2a): each with the papers it is for; it is laid out on any paper by its constraints. */
function layoutsOf(c: TemplateCard): Child {
  const vs = c.template.sheet.variants;
  if (!vs.length) return 'Temel düzen: her kâğıtta kısıtlarıyla yerleşir.';
  return h('ul', { class: 'tgal__layouts' }, vs.map((v) => h('li', null, h('b', null, v.name), ` · ${conditionText(v.when).toLocaleLowerCase('tr')}`)));
}

/** Where a template is kept, and the account's place in it. */
function cloudText(c: TemplateCard): string {
  if (c.source === 'system') return 'Uygulamayla gelir; web ve masaüstünde aynı.';
  if (c.source === 'device') return 'Yalnız bu cihazda. Buluta eşitleyince web ve masaüstünde aynı olur ve paylaşılabilir.';
  if (c.source === 'shared') return `${c.sharedBy ? `${c.sharedBy} paylaştı` : 'Sizinle paylaşıldı'} · ${c.role === 'editor' ? 'düzenleyebilirsiniz' : 'görüntüleyebilirsiniz'}. Son indirilen sürümü bu cihazda; bağlantı yokken de kullanılır.`;
  if (c.source === 'org')
    return `“${c.organization?.name ?? ''}” kurumunun şablonu, revizyon ${c.revision}${c.role === 'owner' ? ' · siz yayımladınız' : c.sharedBy ? ` · ${c.sharedBy} yayımladı` : ''}. Kurumun etkin üyeleri görür ve kullanır; yayımlayan ve kurum yöneticileri düzenler. Son sürümü bu cihazda; bağlantı yokken de kullanılır.`;
  return `Bulut hesabınızda (kişisel alan), revizyon ${c.revision}${c.badges.includes('shared') ? ' · paylaşıldı' : ''}. Son sürümü bu cihazda; bağlantı yokken de kullanılır.`;
}

const modesText = (c: TemplateCard) =>
  c.workspaces.length === 0 || (c.workspaces.includes('cad') && c.workspaces.includes('gis')) ? 'Ortak (bütün kipler)' : c.workspaces.map((m) => WORKSPACES.find((w) => w.id === m)?.label ?? m).join(', ');

export interface DetailsInput {
  readonly arranged: ArrangedCard;
  readonly can: GalleryAbilities;
  readonly picture: HTMLElement | null;
  readonly needs: readonly Finding[];
  readonly paperName: (p: PaperChoice['paper']) => string;
  readonly paperSize: (p: PaperChoice) => { width: number; height: number } | null;
  run(action: TemplateAction): void;
}

export function detailsOf(i: DetailsInput, d: DisposableStore): Child[] {
  const { arranged: a, can } = i;
  const c = a.card;
  const actions = actionsOf(c, can);
  const field = (label: string, value: Child) => h('div', { class: 'smgr__field' }, h('div', { class: 'smgr__flabel' }, label), h('div', { class: 'smgr__fvalue' }, value));
  const buttons = (['duplicate', 'edit', 'publish', 'share', 'sync', 'delete'] as const)
    .filter((k) => actions[k].shown)
    .map((k) => {
      const v = actions[k];
      // Paylaş… opens even when it cannot share yet (its window says why); every other action with a reason is closed (the desktop's rule).
      const open = k === 'share';
      const b = h('button', { class: `btn btn--small${k === 'delete' ? ' btn--danger' : ''}`, type: 'button', disabled: v.reason !== null && !open }, icon(ACTION_ICON[k], 14), v.label);
      b.addEventListener('click', () => i.run(k));
      d.add(tooltip(b, () => ({ title: v.label, description: ACTION_TIP[k], note: v.reason ?? undefined }), 'top'));
      return b;
    });
  const first = c.papers[0];
  const size = first ? i.paperSize(first) : null;
  const recommended = first ? `${paperText(first, i.paperName)}${size ? ` (${fixed(size.width, 0)} × ${fixed(size.height, 0)} mm)` : ''}` : '';
  return [
    h('div', { class: 'tgal__preview' }, i.picture),
    h(
      'div',
      { class: 'smgr__head' },
      h('h3', { class: 'smgr__name' }, c.name),
      h('div', { class: 'smgr__kindline' }, h('span', { class: 'tcard__badges' }, badgesOf(a))),
      h('div', { class: 'tgal__fit' }, icon(a.fit === 'other' ? 'info' : 'check', 14), FIT_LABEL[a.fit]),
    ),
    h('div', { class: 'tgal__actions' }, buttons),
    i.needs.length ? note('warn', h('strong', null, 'Projede eksik: '), i.needs.map((f) => f.message).join(' '), ' Kullanmadan önce sorulur.') : null,
    c.conflictOf ? note('info', h('strong', null, 'Çakışmada ayrılan kopya. '), 'Siz bu cihazda değiştirirken şablon bulutta da değişmişti: bu, sizin sürümünüz; bulutun sürümü asıl adıyla ayrıca duruyor. İkisini karşılaştırıp birini silebilirsiniz.') : null,
    h(
      'div',
      { class: 'smgr__fields' },
      c.description ? field('Açıklama', c.description) : null,
      first ? field(c.papers.length > 1 ? 'Önerilen kâğıtlar' : 'Önerilen kâğıt', h('span', { class: 'num' }, [recommended, ...c.papers.slice(1).map((p) => paperText(p, i.paperName))].join(', '))) : null,
      field('Yerleşim düzenleri', layoutsOf(c)),
      field('Tür', categoryLabel(c.category)),
      field('Çalışma modu', modesText(c)),
      c.projectTypes.length ? field('Proje türleri', c.projectTypes.map((t) => (TYPE_LABEL as Record<string, string>)[t] ?? t).join(', ')) : null,
      field('Sürüm', h('span', { class: 'num' }, `${c.revision}${c.updated ? ` · ${c.updated.slice(0, 10)}` : ''}`)),
      field('Saklandığı yer', cloudText(c)),
      c.author && c.source !== 'shared' && c.source !== 'org' ? field('Yazar', c.author) : null,
      field('Kimlik', h('code', { class: 'smgr__id' }, c.id)),
    ),
  ];
}

/** The paper Kullan lays the template out on: the ones it suits, the recommended one first. */
export function paperPick(c: TemplateCard, chosen: PaperChoice | null, name: (p: PaperChoice['paper']) => string, take: (p: PaperChoice | null) => void): Child[] {
  const choices = paperChoices(c, name);
  if (!choices.length) return [];
  const now = choices.find((x) => chosen && x.choice.paper === chosen.paper && x.choice.orientation === chosen.orientation) ?? choices[0];
  const dd = new Dropdown({
    ariaLabel: 'Kâğıt',
    items: () => choices.map((x) => ({ label: x.label, radio: true, checked: x === now, run: () => take(x === choices[0] ? null : x.choice) })),
  });
  dd.set(h('span', { class: 'dropdown__text' }, now.label));
  return [h('span', null, 'Kâğıt'), dd.el];
}
