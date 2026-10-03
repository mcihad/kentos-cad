import type { AssetMeta } from '../../contracts/generated/sheet/AssetMeta';
import type { Capabilities } from '../../contracts/generated/sheet/Capabilities';
import type { DisplayList } from '../../contracts/generated/sheet/DisplayList';
import type { Finding } from '../../contracts/generated/sheet/Finding';
import type { NorthInfo } from '../../contracts/generated/sheet/NorthInfo';
import type { GroundPoint } from '../../contracts/generated/sheet/GroundPoint';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { PaperChoice } from '../../contracts/generated/sheet/PaperChoice';
import type { SheetTemplateAccess } from '../../contracts/generated/sheet/SheetTemplateAccess';
import type { SheetTemplateCandidates } from '../../contracts/generated/sheet/SheetTemplateCandidates';
import type { SheetTemplateChanged } from '../../contracts/generated/sheet/SheetTemplateChanged';
import type { Template } from '../../contracts/generated/sheet/Template';
import type { TemplateGrantRole } from '../../contracts/generated/sheet/TemplateGrantRole';
import type { Workspace } from '../../contracts/generated/Workspace';
import type { ReadonlySignal } from '../../core/signal';
import type { BookText, SheetEngine } from '../../product/sheet/engine';
import type { SheetHistory } from '../../product/sheet/history';
import type { SheetProfile } from '../../product/sheet/profile';
import type { SheetState } from '../../product/sheet/state';
import type { ProjectTraits, TemplateCard, TemplateProvider } from '../../product/sheet/templates';
import type { PaintSources } from '../../render/sheet/painter';
import type { GalleryAbilities, TemplateAction } from './galleryPlan';

/**
 * What the sheet interface needs besides the app's context
 * (docs/sheet/design.md §11): the sheet mode's state, the work mode's
 * profile, the engine and the book's history, what the paper is painted
 * with, the template providers and the project as the gallery sorts for it.
 * The app's sheet service (app/sheet/service.ts) gives it; the panels never
 * reach the stores or the drawing's pipeline themselves. Every change of the
 * book is a list of the engine's operations given to `apply`, one undo step.
 */
export interface SheetHost {
  readonly state: SheetState;
  readonly profile: ReadonlySignal<SheetProfile>;
  /** Sistem, Benim, Kurumum, Benimle paylaşılanlar. */
  readonly providers: readonly TemplateProvider[];
  /** Bumped when the paper shows something new while the book stays (the drawing changed, a picture was drawn, a typeface arrived). */
  readonly painted: ReadonlySignal<number>;
  /** The preflight of the sheet in front (as designed), errors first; empty while none is in front. */
  readonly findings: ReadonlySignal<readonly Finding[]>;
  /** The book's undo history, once the engine has read a book. */
  readonly history: ReadonlySignal<SheetHistory | null>;
  /** The pictures and the typefaces the paper is painted with. */
  readonly sources: PaintSources;
  /** The engine once fetched; null before (it is fetched when a sheet first comes forward or the gallery opens). */
  engine(): SheetEngine | null;
  /** The book as it is now; null before the engine read one. */
  book(): BookText | null;
  /** Brings a sheet forward (null: Model); a sheet kept on this device is read by the engine first. */
  openSheet(id: string | null): void;
  /** What a sheet of a book looks like (the engine's plan with the project's inputs); null when it cannot be drawn (said in the log). */
  plan(book: BookText, sheet: string): DisplayList | null;
  /** The preflight of a sheet for an export at `dpi` (none: the sheet's own), errors first. */
  preflight(sheet: string, dpi?: number): Finding[];
  /** A north arrow's lines: its map's centre, the declination and its source, the date and where it came from (the engine's `northInfo`). */
  northInfo(sheet: string, item: string): NorthInfo | null;
  /** The project's work mode and what it offers the tools (design §11a). */
  workspace(): Workspace;
  capabilities(): Capabilities;
  /** The open project as the gallery sorts and checks templates for it. */
  traits(): ProjectTraits;
  /** Why the book cannot be changed now; null when it can. */
  whyReadOnly(): string | null;
  /**
   * Applies operations as one undo step named `label` (else the engine's
   * name); false when the engine refused them, which the message log says
   * with its code. The book is then as it was.
   */
  apply(ops: readonly Op[], label?: string): boolean;
  /** A new lasting id (an item, a sheet, a guide). */
  newId(): string;
  /** Keeps a picture file's bytes on this device (PNG, JPEG, SVG; checked and measured); its metadata, or null (said why). */
  addPicture(file: File): Promise<AssetMeta | null>;
  /** Where a new map looks and at what scale: the drawing area's centre and the project's plot scale. */
  mapPlace(): { readonly center: GroundPoint; readonly scale: number };
  /** What the gallery can do now: the engine (use, copy, edit) and the cloud (sync, share). */
  galleryAbilities(): GalleryAbilities;
  /** What a template needs that the project lacks (the engine's preflight of a sheet made from it), said before it is used. */
  templateNeeds(template: Template): Finding[];
  /** A sheet made from a template for its picture (as Kullan would make it: the drawing's centre, the plot scale); null when refused. */
  templatePreview(template: Template): { book: BookText; sheet: string } | null;
  /**
   * Does what the gallery offers for a template (galleryPlan.ts `actionsOf`):
   * Kullan makes a sheet from it on the paper chosen (null: its own). False,
   * said in the message log, when it could not.
   */
  templateAction(card: TemplateCard, action: TemplateAction, paper?: PaperChoice | null): Promise<boolean>;
  /** What the gallery and the status bar say of the cloud library now; state 'none' (and no text) when nothing. */
  readonly cloudLine: ReadonlySignal<LibraryLine>;
  /** The account's cloud library, once it is there (it comes with the engine); null before or without one. */
  cloud(): TemplateCloud | null;
}

/** The cloud library's line: how its sync stands, in words. */
export interface LibraryLine {
  readonly state: 'none' | 'signedOut' | 'offline' | 'syncing' | 'synced' | 'failed';
  readonly text: string;
}

/** What the share window and the gallery ask of the account's cloud library (app/sheet/cloudLibrary.ts). */
export interface TemplateCloud {
  /** Why cloud actions cannot run now (not signed in, the server away); null when they can. */
  why(): string | null;
  /** “Buluta eşitle”: the id the cloud gave the template, or null when it could not go now (said why). */
  upload(id: string, name: string): Promise<string | null>;
  access(id: string, signal?: AbortSignal): Promise<SheetTemplateAccess>;
  candidates(id: string, query: string, signal?: AbortSignal): Promise<SheetTemplateCandidates>;
  share(id: string, userId: string, role: TemplateGrantRole): Promise<SheetTemplateChanged>;
  unshare(id: string, userId: string): Promise<SheetTemplateChanged>;
  /** The organisations of the account's library, by name (the last list): their names group “Kurumum”; where `canPublish`, “Kuruma yayımla…” may go. */
  organizations(): readonly { readonly tenantId: string; readonly name: string; readonly canPublish: boolean }[];
  /** Where a template of the account's own may be published, each with the copy published from it there before. */
  publishTargets(id: string): Promise<readonly { readonly tenantId: string; readonly name: string; readonly copy?: { readonly id: string; readonly name: string; readonly revision: number } }[]>;
  /** Publishes one's own template into an organisation; the new template's id there (thrown with the server's words when it cannot). */
  publish(id: string, tenantId: string): Promise<string>;
  /** Puts this template's content into the organisation's copy published from it (a new revision there). */
  republish(id: string, copyId: string): Promise<void>;
}
