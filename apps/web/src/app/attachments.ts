// Nesne ekleri (docs/adr/0225 §1, §2; the desktop's `attachments.rs`): an object's photos and documents as Öznitelikler,
// the photo viewer, the hover card and the Bilgi tool show them, and the writes that add, rename, embed and remove them
// through `cad.entities.set` (operation `attachments`, one step “Ekler”). A photo is kept in the project's library under
// Ekler (an edit, no undo step: docs/adr/0092), as a picture's image is (docs/adr/0192 §2). The browser keeps no file's
// path: a photo is embedded, a document is given by its address.

import type { Attachment } from '../contracts/generated/Attachment';
import { attachmentKind, attachmentOf, attachmentsProblem, type AttachmentKind } from '../model/attachmentRules';
import type { Entity } from '../model/entities';
import { hasPicture } from '../product/entitiesEdit';
import { pictureItem, PICTURE_FILES } from '../tools/pictureFile';
import { setProperties, uidsOf } from '../ui/properties/write';
import type { AppContext } from './context';

/** The library folder an object's embedded photos go under. */
export const PHOTO_PATH = ['Ekler'] as const;

/** An attachment as the views show it. */
export interface AttachmentView {
  readonly index: number;
  readonly attachment: Attachment;
  readonly kind: AttachmentKind;
  /** An image the browser can show: an embedded photo's data address or a photo's web address; null for a linked file. */
  readonly src: string | null;
  /** Where it is: “Gömülü fotoğraf (1200×800)”, “Bağlı dosya: tapu.pdf”, the address. */
  readonly source: string;
}

const KIND_WORD: Record<AttachmentKind, string> = { photo: 'Fotoğraf', document: 'Belge', link: 'Bağlantı' };

/** A kind's word: Fotoğraf, Belge, Bağlantı. */
export const attachmentKindWord = (k: AttachmentKind): string => KIND_WORD[k];

/** A path's last part: a file's name. */
const baseName = (path: string): string => path.split(/[\\/]/).pop() || path;

/** The object's attachments as the views show them. */
export function attachmentViews(ctx: AppContext, e: Entity): AttachmentView[] {
  return (e.attachments ?? []).map((attachment, index) => {
    const kind = attachmentKind(attachment);
    let src: string | null = null;
    let source: string;
    if (attachment.asset != null) {
      const it = ctx.doc.styles.value.items.find((i) => i.kind === 'asset' && i.id === attachment.asset);
      if (it?.kind === 'asset') {
        src = it.data;
        source = `Gömülü fotoğraf (${it.width}×${it.height} piksel)`;
      } else source = 'Gömülü fotoğraf (kitaplıkta yok)';
    } else if (attachment.file != null) source = `Bağlı dosya: ${baseName(attachment.file)} (tarayıcıda okunamaz)`;
    else {
      source = attachment.url ?? '';
      if (kind === 'photo') src = attachment.url ?? null;
    }
    return { index, attachment, kind, src, source };
  });
}

/** The one object selected, when exactly one is. */
export function selectedObject(ctx: AppContext): Entity | null {
  const ids = ctx.selection.ids.value;
  if (ids.size !== 1) return null;
  const [id] = ids;
  return ctx.doc.get(id) ?? null;
}

/** Whether `e` takes edits: its layer is not locked. */
export const attachable = (ctx: AppContext, e: Entity): boolean => !ctx.doc.layers.isLocked(e.layerId);

/** Writes `e`'s whole list of attachments, one step “Ekler”; whether it was written. */
export function writeAttachments(ctx: AppContext, e: Entity, list: readonly Attachment[]): boolean {
  const out = setProperties(ctx, { uids: uidsOf(ctx, [e.id]), attachments: list.map(attachmentOf), operation: 'attachments' });
  return out !== null;
}

/** Photos chosen from the computer, embedded in the project's library and attached to `e` after its own. */
export async function addPhotos(ctx: AppContext, e: Entity): Promise<void> {
  const picked = await ctx.files.pickManyForImport(PICTURE_FILES);
  if (!picked?.length) return;
  const added: Attachment[] = [];
  for (const f of picked) {
    const got = await pictureItem(f.name, f.bytes, PHOTO_PATH);
    if (!got.ok) {
      ctx.log.warn(got.error);
      continue;
    }
    const doc = ctx.doc;
    if (!hasPicture(doc, got.id)) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, got.item] });
    added.push({ name: f.name, asset: got.id });
  }
  if (!added.length) return;
  const now = ctx.doc.get(e.id);
  if (!now) return;
  const list = [...(now.attachments ?? []), ...added];
  const problem = attachmentsProblem(list);
  if (problem) return void ctx.log.warn(`Eklenemedi: ${problem}.`);
  if (writeAttachments(ctx, now, list)) ctx.log.success(added.length === 1 ? `“${added[0].name}” eklendi.` : `${added.length} fotoğraf eklendi.`);
}

/** An address attached to `e` (Adres ekle…): the window asks its name, address and note. */
export function addLink(ctx: AppContext, e: Entity): void {
  void import('../ui/attachments/AttachmentDialog').then(
    (m) =>
      m.openAttachmentDialog(ctx, { mode: 'link' }, (a) => {
        const now = ctx.doc.get(e.id);
        return now ? writeAttachments(ctx, now, [...(now.attachments ?? []), a]) : false;
      }),
    failed(ctx),
  );
}

/** `e`'s attachment `index` renamed and its note changed (Düzenle…). */
export function editAttachment(ctx: AppContext, e: Entity, index: number): void {
  const a = e.attachments?.[index];
  if (!a) return;
  void import('../ui/attachments/AttachmentDialog').then(
    (m) =>
      m.openAttachmentDialog(ctx, { mode: 'edit', attachment: a }, (b) => {
        const now = ctx.doc.get(e.id);
        if (!now?.attachments?.[index]) return false;
        return writeAttachments(ctx, now, now.attachments.map((x, i) => (i === index ? b : x)));
      }),
    failed(ctx),
  );
}

/** `e`'s attachment `index` taken away (the photo stays in the library: Kullanılmayanları temizle takes it). */
export function removeAttachment(ctx: AppContext, e: Entity, index: number): void {
  const list = e.attachments ?? [];
  if (!list[index]) return;
  writeAttachments(
    ctx,
    e,
    list.filter((_, i) => i !== index),
  );
}

/**
 * Göm (docs/adr/0225 §2): the browser cannot read a linked photo's file, so it asks for it; the photo chosen is kept in
 * the project's library and the attachment made embedded, one step “Ekler”.
 */
export async function embedAttachment(ctx: AppContext, e: Entity, index: number): Promise<void> {
  const a = e.attachments?.[index];
  if (!a?.file) return;
  const picked = await ctx.files.pickForImport(PICTURE_FILES);
  if (!picked) return;
  const got = await pictureItem(picked.name, picked.bytes, PHOTO_PATH);
  if (!got.ok) return void ctx.log.warn(got.error);
  const doc = ctx.doc;
  if (!hasPicture(doc, got.id)) doc.styles.set({ ...doc.styles.value, items: [...doc.styles.value.items, got.item] });
  const now = doc.get(e.id);
  if (!now?.attachments?.[index]) return;
  writeAttachments(
    ctx,
    now,
    now.attachments.map((x, i) => (i === index ? { name: x.name, asset: got.id, ...(x.note != null && { note: x.note }) } : x)),
  );
}

/**
 * Aç: a photo in the viewer, an address in a new tab; a linked file cannot be opened from the browser (said). A photo's
 * address opens in the viewer too.
 */
export function openAttachment(ctx: AppContext, e: Entity, index: number): void {
  const views = attachmentViews(ctx, e);
  const v = views[index];
  if (!v) return;
  if (v.kind === 'photo' && v.src) return void openViewer(ctx, e, index);
  if (v.attachment.url != null) {
    window.open(v.attachment.url, '_blank', 'noopener');
    return;
  }
  ctx.log.warn(`“${v.attachment.name}” bağlı bir dosya; tarayıcı bilgisayardaki dosyaları açamaz. Masaüstü uygulamasında açın.`);
}

/** The photo viewer on `e`'s photos, at its attachment `index`. */
export function openViewer(ctx: AppContext, e: Entity, index: number): void {
  void import('../ui/attachments/PhotoViewer').then((m) => m.openPhotoViewer(ctx, e.id, index), failed(ctx));
}

const failed = (ctx: AppContext) => (err: Error) => ctx.log.error(`Pencere yüklenemedi: ${err.message}. Bağlantıyı denetleyip yeniden deneyin.`);

/** Ekler's commands: Dosya ekle… and Adres ekle… on the one object selected (docs/adr/0225 §2). */
export function registerAttachmentCommands(ctx: AppContext): void {
  const { log, selection } = ctx;
  const target = (): Entity | null => {
    const e = selectedObject(ctx);
    if (!e) {
      log.warn('Ek tek bir nesneye eklenir: bir nesne seçin.');
      return null;
    }
    if (!attachable(ctx, e)) {
      log.warn(`“${ctx.doc.layers.get(e.layerId)?.name ?? e.layerId}” katmanı kilitli; nesnesine ek eklenemez. Kilidi Katmanlar panelinden açın.`);
      return null;
    }
    return e;
  };
  ctx.commands.registerAll([
    {
      id: 'attachment.add',
      title: 'Dosya ekle…',
      short: 'Dosya ekle',
      category: 'Ekler',
      icon: 'attachment',
      description:
        'Seçili nesneye fotoğraf ya da belge ekler: Öznitelikler’in Ekler bölümünde, üzerine gelme kartında ve Bilgi aracında görünür. Fotoğraf projenin kitaplığına gömülür; web’de belge dosyası bağlanamaz, belgeyi Adres ekle… ile ekleyin.',
      aliases: ['EKEKLE', 'DOSYAEKLE', 'ATTACH'],
      isEnabled: () => selection.ids.value.size === 1,
      watch: [selection.ids],
      run: () => {
        const e = target();
        if (e) void addPhotos(ctx, e);
      },
    },
    {
      id: 'attachment.addLink',
      title: 'Adres ekle…',
      short: 'Adres ekle',
      category: 'Ekler',
      icon: 'attachmentLink',
      description: 'Seçili nesneye bir web adresi ekler (http ya da https): kurumun sayfası, bir belgenin adresi, bir fotoğrafın adresi.',
      aliases: ['ADRESEKLE', 'BAGLANTIEKLE'],
      isEnabled: () => selection.ids.value.size === 1,
      watch: [selection.ids],
      run: () => {
        const e = target();
        if (e) addLink(ctx, e);
      },
    },
  ]);
}
