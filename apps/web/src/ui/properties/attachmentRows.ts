import { addLink, addPhotos, attachmentKindWord, attachmentViews, editAttachment, embedAttachment, openAttachment, openViewer, removeAttachment } from '../../app/attachments';
import type { AppContext } from '../../app/context';
import { objectActions, runAction } from '../../app/objectActions';
import type { Entity } from '../../model/entities';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';

const ICON = { photo: 'photo', document: 'document', link: 'attachmentLink' } as const;

/**
 * Öznitelikler's Ekler (docs/adr/0225 §2; the desktop's `properties::rows::attachments`): a row each attachment, its
 * kind and number, its name, a photo's thumbnail (it opens the viewer) and its menu: Aç, Düzenle…, Göm (a linked photo),
 * Kaldır; then Ekle with Dosya ekle… and Adres ekle…. On a locked layer only Aç.
 */
export function attachmentRows(ctx: AppContext, e: Entity, locked: boolean): PropRow[] {
  const rows: PropRow[] = attachmentViews(ctx, e).map((v, k) => {
    const items = (): MenuItem[] => [
      { label: v.kind === 'photo' ? 'Görüntüle' : 'Aç', icon: v.kind === 'photo' ? 'photo' : 'externalOpen', run: () => openAttachment(ctx, e, v.index) },
      ...(locked
        ? []
        : [
            { label: 'Düzenle…', icon: 'edit', run: () => editAttachment(ctx, e, v.index) },
            ...(v.kind === 'photo' && v.attachment.file != null ? [{ label: 'Göm', icon: 'attachment', run: () => void embedAttachment(ctx, e, v.index) }] : []),
            { label: 'Kaldır', icon: 'trash', run: () => removeAttachment(ctx, e, v.index) },
          ]),
    ];
    return {
      label: `${attachmentKindWord(v.kind)} ${k + 1}`,
      value: v.attachment.name,
      note: v.attachment.note ?? undefined,
      thumb: { src: v.kind === 'photo' ? v.src : null, icon: ICON[v.kind], title: v.source, open: () => (v.kind === 'photo' ? openViewer(ctx, e, v.index) : openAttachment(ctx, e, v.index)) },
      editor: { type: 'select', display: () => ({ text: v.attachment.name }), items },
    };
  });
  if (!locked)
    rows.push({
      label: 'Ekle',
      value: 'Dosya ya da adres',
      editor: {
        type: 'select',
        display: () => ({ text: 'Dosya ya da adres…', icon: 'attachment' }),
        items: () => [
          { label: 'Dosya ekle…', icon: 'attachment', run: () => void addPhotos(ctx, e) },
          { label: 'Adres ekle…', icon: 'attachmentLink', run: () => addLink(ctx, e) },
        ],
      },
    });
  return rows;
}

/** Öznitelikler's Eylemler (docs/adr/0225 §4): a row each action of the object's layer, its filled target, Aç in its menu. */
export function actionRows(ctx: AppContext, e: Entity): PropRow[] {
  return objectActions(ctx, e).map((a) => ({
    label: a.name,
    value: a.target ?? '—',
    note: a.error ?? undefined,
    warn: a.error !== null,
    editor: {
      type: 'select',
      display: () => ({ text: a.target ?? 'Şablon bozuk', icon: a.kind === 'file' ? 'document' : 'externalOpen' }),
      items: () => [
        { label: 'Aç', icon: 'externalOpen', run: () => runAction(ctx, a), disabled: a.error !== null },
        ...(a.target ? [{ label: 'Kopyala', icon: 'copy', run: () => void navigator.clipboard?.writeText(a.target ?? '') }] : []),
      ],
    },
  }));
}
