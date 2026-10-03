import type { AssetWithBytes } from '../../contracts/generated/sheet/AssetWithBytes';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { InstanceIds } from '../../contracts/generated/sheet/InstanceIds';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { SheetBook } from '../../contracts/generated/sheet/SheetBook';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateMeta } from '../../contracts/generated/sheet/TemplateMeta';
import type { VariableValue } from '../../contracts/generated/sheet/VariableValue';
import { uuidv7 } from '../../core/uuid';
import { bookText, errorText, type BookText, type SheetEngine } from '../../product/sheet/engine';
import { fromBase64, toBase64 } from '../../product/sheet/store';
import type { TemplateCard } from '../../product/sheet/templates';
import type { TemplateAction } from '../../ui/sheet/galleryPlan';
import type { TemplateWords } from '../../ui/sheet/SaveTemplateDialog';
import { askRemove } from '../../ui/widgets/confirm';
import type { AppContext } from '../context';
import type { SheetService } from './service';

/**
 * Sheets from templates and templates from sheets (docs/sheet/design.md §12):
 * a template is made into a sheet by the engine with new ids from here, the
 * paper chosen (its items follow their constraints there), the drawing
 * area's centre and the project's plot scale for its maps; the sheet, its
 * master page and its pictures join the book as one undo step. A sheet is
 * saved as a template by the engine too (maps lose their place, keep their
 * scale; pictures go along; the sheet's values become questions) and kept
 * on this device. What a template needs that the project lacks (a
 * coordinate system, attribute layers) is the engine's preflight of it.
 */

/** The ids a sheet from this template gets: one for the sheet, one per item, one for its master page and each of its items. */
export function instanceIds(t: Template, newId: () => string): InstanceIds {
  return {
    sheet: newId(),
    items: t.sheet.items.map(() => newId()),
    masterItems: (t.master?.items ?? []).map(() => newId()),
    ...(t.master ? { master: newId() } : {}),
  };
}

/** The codes of the preflight that say the project lacks what an item needs. */
const NEEDS = /^needs_/;

/**
 * What a template needs that the project does not offer, as the engine's
 * preflight says it of a sheet made from it (design §11a: said before it is
 * used); empty when it needs nothing more.
 */
export function templateNeeds(engine: SheetEngine, s: SheetService, template: Template): Finding[] {
  const made = templatePreview(engine, s, template);
  if (!made) return [];
  const found = s.paint.preflight(engine, made.book, made.sheet);
  const seen = new Set<string>();
  return found.filter((f) => NEEDS.test(f.code) && !seen.has(f.code) && !!seen.add(f.code));
}

const previews = new Map<string, { book: BookText; sheet: string }>();

/**
 * A sheet made from a template as Kullan would make it (the drawing area's
 * centre, the plot scale), in a book of its own: the gallery's picture of it
 * and what its preflight finds. Kept per template revision and place.
 */
export function templatePreview(engine: SheetEngine, s: SheetService, template: Template): { book: BookText; sheet: string } | null {
  const place = s.mapPlace();
  const key = `${template.meta.id}|${template.meta.revision}|${template.meta.updated}|${place.center.x}|${place.center.y}|${place.scale}`;
  const hit = previews.get(key);
  if (hit) return hit;
  try {
    // Fixed ids: a preview's map pictures keep their place in the map frames' cache from one opening to the next.
    let n = 0;
    const base = template.meta.id.replace(/[^A-Za-z0-9-]+/g, '-');
    const ids = instanceIds(template, () => `onizleme-${base}-${n++}`);
    const inst = engine.instantiateTemplate(template, ids, { center: place.center, scale: place.scale, values: [] });
    const book: SheetBook = { schema: engine.info.bookSchema, sheets: [inst.sheet], masters: inst.master ? [inst.master] : [], assets: inst.assets, variables: [] };
    const made = { book: bookText(book), sheet: inst.sheet.id };
    previews.set(key, made);
    while (previews.size > 40) previews.delete(previews.keys().next().value!);
    return made;
  } catch {
    return null;
  }
}

/** Asks a template's questions before a sheet is made from it (design §12); null when the user left. */
export type AskValues = (template: Template) => Promise<VariableValue[] | null>;

/** The window that asks them (ui/sheet/TemplateQuestions.ts), loaded when first needed. */
export const askInWindow: AskValues = async (t) => (t.variables.length ? (await import('../../ui/sheet/TemplateQuestions')).askTemplateValues(t, { stack: true }) : []);

/**
 * Makes a sheet from a template on the paper chosen (null: its own) with the
 * answers to its questions (`values`; none: its own values) and brings it
 * forward. Its id, or null when the engine refused (said in the log).
 */
export async function sheetFromTemplate(s: SheetService, template: Template, paper: PaperChoice | null, label?: string, values: readonly VariableValue[] = []): Promise<string | null> {
  const engine = await s.ensureEngine();
  const place = s.mapPlace();
  let inst;
  try {
    inst = engine.instantiateTemplate(template, instanceIds(template, () => s.newId()), { ...(paper ? { paper } : {}), center: place.center, scale: place.scale, values: [...values] });
  } catch (e) {
    s.report(`“${template.meta.name}” şablonundan pafta yapılamadı`, e);
    return null;
  }
  // The pictures' bytes first: the sheet shows them as soon as it is drawn.
  for (const a of inst.assetBytes) await s.assets.put(a.meta, fromBase64(a.data)).catch((e: Error) => s.report(`“${a.meta.name}” resmi saklanamadı`, e));
  const ops: Op[] = [...(inst.master ? [{ op: 'addMaster' as const, master: inst.master }] : []), { op: 'addSheet', sheet: inst.sheet }, ...(inst.assets.length ? [{ op: 'addAssets' as const, assets: inst.assets }] : [])];
  if (!s.apply(ops, label ?? `Şablondan pafta: ${inst.sheet.name}`)) return null;
  if (inst.assetBytes.length) void s.paint.refreshAssets();
  s.openSheet(inst.sheet.id);
  return inst.sheet.id;
}

/** A sheet from a template its questions asked first (Kullan, Yeni pafta); null when the user left or the engine refused. */
export async function useTemplate(s: SheetService, template: Template, paper: PaperChoice | null, ask: AskValues, label?: string): Promise<string | null> {
  const values = await ask(template);
  return values ? sheetFromTemplate(s, template, paper, label, values) : null;
}

/** A new sheet from the work mode's default template (design §11a), its questions asked; its id, or null. */
export async function newSheet(s: SheetService, ask: AskValues = askInWindow): Promise<string | null> {
  const engine = await s.ensureEngine();
  const id = s.profile.value.defaultTemplate;
  const t = engine.systemTemplates().find((x) => x.meta.id === id) ?? engine.systemTemplates()[0];
  if (!t) return null;
  return useTemplate(s, t, null, ask, 'Yeni pafta');
}

/** The pictures a sheet and its master page use, with their bytes (what `extractTemplate` takes along). */
async function usedAssets(s: SheetService, book: SheetBook, sheetId: string): Promise<AssetWithBytes[]> {
  const sheet = book.sheets.find((x) => x.id === sheetId);
  if (!sheet) return [];
  const master = sheet.master ? book.masters.find((m) => m.id === sheet.master) : undefined;
  const text = JSON.stringify([sheet.items, master?.items ?? []]);
  const out: AssetWithBytes[] = [];
  for (const meta of book.assets) {
    if (!text.includes(meta.sha256)) continue;
    const rec = await s.assets.get(meta.sha256);
    if (rec) out.push({ meta, data: toBase64(rec.bytes) });
  }
  return out;
}

/**
 * Saves a sheet as a template on this device. `replace` keeps an existing
 * template's id with its revision one higher (Düzenle, then Kaydet);
 * otherwise a new id. The saved template, or null when it was refused.
 */
export async function saveAsTemplate(ctx: AppContext, s: SheetService, sheetId: string, words: TemplateWords, replace: TemplateCard | null): Promise<Template | null> {
  const engine = await s.ensureEngine();
  const book = s.book();
  if (!book) return null;
  const now = new Date().toISOString();
  const meta: TemplateMeta = {
    ...words,
    id: replace?.id ?? uuidv7(),
    revision: replace ? replace.revision + 1 : 1,
    created: replace?.template.meta.created ?? now,
    updated: now,
    author: ctx.cloud.me.value?.user.displayName ?? '',
  };
  try {
    const t = engine.extractTemplate(book, sheetId, meta, await usedAssets(s, book.book, sheetId));
    const lib = s.shelf.library;
    // One of the cloud library (the account's own, or shared with it to edit): changed here, it goes up as soon as it can.
    if (replace && (replace.source === 'cloud' || replace.source === 'shared') && lib) {
      await lib.saved(replace.id, t);
      ctx.log.success(`“${t.meta.name}” güncellendi; bulut hesabınıza eşitleniyor (bağlantı yoksa gelince gider).`);
      return t;
    }
    await s.templates.save(t.meta.id, t);
    s.shelf.libraryChanged();
    ctx.log.success(`“${t.meta.name}” bu cihaza şablon olarak kaydedildi${replace ? ` (revizyon ${t.meta.revision})` : ''}. Pafta şablonları → Benim'de.`);
    return t;
  } catch (e) {
    s.report('Şablon olarak kaydedilemedi', e);
    return null;
  }
}

/** Does what the gallery offers for a card (ui/sheet/galleryPlan.ts `actionsOf`); Kullan asks the template's questions first. */
export async function templateAction(ctx: AppContext, s: SheetService, card: TemplateCard, action: TemplateAction, paper: PaperChoice | null, ask: AskValues = askInWindow): Promise<boolean> {
  const lib = s.shelf.library;
  try {
    switch (action) {
      case 'use':
        return (await useTemplate(s, card.template, paper, ask)) !== null;
      case 'edit': {
        // A template is edited as a sheet: “Şablon olarak kaydet” puts it back (a system one as the user's own copy).
        const id = await sheetFromTemplate(s, card.template, paper, `Şablonu düzenle: ${card.name}`);
        const own = card.source === 'device' || card.source === 'cloud' || card.role === 'editor' || (card.source === 'org' && (card.role === 'owner' || card.role === 'admin'));
        if (id) ctx.log.info(own ? `“${card.name}” paftada açıldı. Değişiklikleri Şablon olarak kaydet → “Bu şablonu güncelle” ile geri yazın.` : `“${card.name}” paftada açıldı. Şablon olarak kaydet ile kendi şablonunuz olarak saklayın; ${card.source === 'system' ? 'sistem şablonu' : card.source === 'org' ? 'kurum şablonu' : 'paylaşılan şablon'} değişmez.`);
        return id !== null;
      }
      case 'duplicate': {
        const engine = await s.ensureEngine();
        const now = new Date().toISOString();
        const copy: Template = { ...card.template, meta: { ...card.template.meta, id: uuidv7(), revision: 1, name: `${card.name} (kopya)`, created: now, updated: now, author: ctx.cloud.me.value?.user.displayName ?? '' } };
        const t = engine.validateTemplate(JSON.stringify(copy));
        await s.templates.save(t.meta.id, t);
        s.shelf.libraryChanged();
        ctx.log.success(`“${t.meta.name}” şablonlarınıza eklendi (Benim, bu cihazda).`);
        return true;
      }
      case 'delete': {
        const org = card.source === 'org' && (card.role === 'owner' || card.role === 'admin') ? card.organization : undefined;
        if (card.source !== 'device' && card.source !== 'cloud' && !org) break;
        const cloud = card.source === 'cloud';
        const ok = await askRemove({
          title: 'Şablonu sil',
          message: org ? `“${card.name}” “${org.name}” kurumunun şablonlarından silinsin mi?` : cloud ? `“${card.name}” bulut hesabınızdan silinsin mi?` : `“${card.name}” bu cihazdan silinsin mi?`,
          details: [...(org ? ['Kurumun bütün üyelerinin listesinden kalkar.'] : cloud ? ['Web ve masaüstündeki bütün cihazlarınızdan, paylaştığınız kişilerin listesinden de kalkar.'] : []), 'Ondan yapılmış paftalar kalır: onlar şablonun kopyasıdır.'],
          action: 'Sil',
        });
        if (!ok) return false;
        if (lib) return lib.remove(card.id, card.name);
        await s.templates.remove(card.id);
        s.shelf.libraryChanged();
        ctx.log.success(`“${card.name}” bu cihazdan silindi.`);
        return true;
      }
      case 'sync':
        if (!lib || s.galleryAbilities().cloud) break;
        if (card.source === 'device') return (await lib.upload(card.id, card.name)) !== null;
        await lib.sync.run();
        return true;
      case 'share':
      case 'publish':
        break;
    }
  } catch (e) {
    ctx.log.error(`Şablon: ${errorText(e)}`);
    return false;
  }
  ctx.log.warn(`${action === 'sync' ? 'Eşitle' : action === 'share' ? 'Paylaş' : 'Sil'}: ${s.galleryAbilities().cloud ?? 'bu şablonda yapılamaz.'}`);
  return false;
}
