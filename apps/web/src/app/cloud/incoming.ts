import type { Bounds as ContractBounds } from '../../contracts/generated/Bounds';
import type { LayerNode as ContractLayerNode } from '../../contracts/generated/LayerNode';
import type { ProjectSettings as ContractSettings } from '../../contracts/generated/ProjectSettings';
import type { ProjectStyles as ContractStyles } from '../../contracts/generated/ProjectStyles';
import type { CadDocument, DocumentContent } from '../../model/document';
import type { Entity } from '../../model/entities';
import { DOCUMENT_FORMAT, DOCUMENT_VERSION, readSnapshot } from '../../model/snapshot';
import { STYLE_FORMAT, STYLE_VERSION, parseStyleFile } from '../../style/file';

/**
 * Whatever arrives from the server was written by some client and is
 * checked like a file someone sent (CLAUDE.md §9.7): the same reader as a
 * `.kcad` file, and project styles like a shared `.kstil`. Nothing here
 * trusts the contract types alone.
 */

export interface IncomingMeta {
  name: string;
  settings: ContractSettings;
  origin: { x: number; y: number };
  homeView?: ContractBounds | null;
  layers: ContractLayerNode[] | readonly unknown[];
  activeLayer: string;
  styles: ContractStyles | { items: readonly unknown[]; categories: readonly unknown[] };
}

export type Checked = { ok: true; content: DocumentContent } | { ok: false; error: string };

/** The problem with a project's own styles, or null (they are checked as a shared style file). */
export function projectStylesProblem(styles: { items: readonly unknown[]; categories: readonly unknown[] }): string | null {
  const check = parseStyleFile(JSON.stringify({ format: STYLE_FORMAT, version: STYLE_VERSION, exported: '', items: styles.items, categories: styles.categories }));
  return check.issues.length ? `proje stillerinde sorun var (${check.issues[0]})` : null;
}

/** Metadata and objects as document content, numbered 1…n in the given order, or the reason they are unreadable. */
export function readIncoming(meta: IncomingMeta, entities: readonly unknown[]): Checked {
  const snapshot = {
    format: DOCUMENT_FORMAT,
    version: DOCUMENT_VERSION,
    name: meta.name,
    settings: meta.settings,
    origin: meta.origin,
    ...(meta.homeView ? { homeView: meta.homeView } : {}),
    layers: meta.layers,
    activeLayer: meta.activeLayer,
    entities: entities.map((e, i) => ({ ...(e as object), id: i + 1 })),
    styles: meta.styles,
  };
  const read = readSnapshot(JSON.stringify(snapshot));
  if (!read.ok) return read;
  const styles = projectStylesProblem(read.content.styles);
  return styles ? { ok: false, error: styles } : read;
}

/** The drawing's current metadata, to check incoming objects against its layers. */
export function metaOf(doc: CadDocument): IncomingMeta {
  return {
    name: doc.name.value,
    settings: doc.settings.toJSON(),
    origin: doc.origin,
    layers: [...doc.layers.tree],
    activeLayer: doc.layers.active.value,
    styles: doc.styles.value,
  };
}

/** Objects checked against the drawing's layers, in order; or the reason. */
export function readEntities(doc: CadDocument, entities: readonly unknown[]): { ok: true; entities: Entity[] } | { ok: false; error: string } {
  const read = readIncoming(metaOf(doc), entities);
  return read.ok ? { ok: true, entities: [...read.content.entities] } : read;
}
