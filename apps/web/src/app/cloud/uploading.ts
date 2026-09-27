import type { FileUpload } from '../../contracts/generated/FileUpload';
import type { LayerNode } from '../../contracts/generated/LayerNode';
import type { ProjectCreate } from '../../contracts/generated/ProjectCreate';
import type { ProjectInfo } from '../../contracts/generated/ProjectInfo';
import type { ProjectStorage } from '../../contracts/generated/ProjectStorage';
import type { CadDocument } from '../../model/document';
import { encodeDrawing, type DrawingCodec, type EncodedDrawing } from '../drawingFile';
import { ApiFailure, type CloudApi, type Transfer } from './api';
import { catalogEnvelope } from './catalog';
import { sha256Hex, uploadBytes, type UploadTarget } from './transfer';
import { again } from './upload';

/**
 * What a new cloud project made from the drawing on screen needs, whatever
 * it is kept as (docs/adr/0031, 0036, 0038): the project's metadata from the
 * drawing, the drawing's `.kcad` bytes sent as an upload with its stages
 * said, and the failure that keeps the drawing local. As on the desktop
 * (kentos-cloud `upload_new`), a drawing refused for good leaves no project
 * behind: the empty project goes to the trash. With no answer it is kept,
 * so the same upload tried again (the same idempotency key) finds it and
 * fills it.
 */

/** Where an upload of the drawing is: the stages the upload window says. */
export type FileStage = 'creating' | 'encoding' | 'uploading' | 'verifying' | 'importing';

/** What the catalog says of a drawing uploaded as a new project. */
export interface NewProjectCatalog {
  projectType?: ProjectCreate['projectType'];
  description?: string;
  tags?: readonly string[];
}

/** The layer tree with every layer unlocked (a locked layer refuses objects written one by one). */
export function unlocked(nodes: readonly LayerNode[]): LayerNode[] {
  return nodes.map((n) => ({ ...n, locked: false, children: unlocked(n.children) }));
}

/** A new project's metadata from the drawing: its settings, origin, view, layers (unlocked) and styles. */
export function createInput(doc: CadDocument, name: string, catalog: NewProjectCatalog, storage?: ProjectStorage): ProjectCreate {
  const tree = structuredClone([...doc.layers.tree]) as LayerNode[];
  return {
    name,
    settings: doc.settings.toJSON(),
    origin: { ...doc.origin },
    homeView: doc.homeView ? { ...doc.homeView } : undefined,
    layers: unlocked(tree),
    activeLayer: doc.layers.active.value,
    styles: { items: structuredClone([...doc.styles.value.items]), categories: structuredClone([...doc.styles.value.categories]) },
    ...(catalog.projectType ? { projectType: catalog.projectType } : {}),
    ...(catalog.description?.trim() ? { description: catalog.description.trim() } : {}),
    ...(catalog.tags?.length ? { tags: [...catalog.tags] } : {}),
    ...(storage === 'file' ? { storage } : {}),
  };
}

/** What became of the new project the drawing did not get into: moved to the trash, or kept on the server (empty). */
export type LeftProject = 'trashed' | 'kept';

/**
 * The new project was created, but the drawing did not get into it; the
 * drawing on screen is still local. `refused`: the object the server did
 * not take (`entities[i]` of the file, the drawing's objects in order),
 * when that was the reason. `left`: what became of the project.
 */
export class UploadFailed extends Error {
  readonly project: ProjectInfo;
  /** The failure itself (the server's refusal, a dead network). */
  readonly failure: unknown;
  readonly refused: { index: number; path: string } | null;
  readonly left: LeftProject;

  constructor(project: ProjectInfo, failure: unknown, left: LeftProject = 'kept') {
    super(failure instanceof Error ? failure.message : String(failure));
    this.project = project;
    this.failure = failure;
    this.left = left;
    const path = failure instanceof ApiFailure ? failure.path : undefined;
    const m = /^entities\[(\d+)\]/.exec(path ?? '');
    this.refused = m && path ? { index: Number(m[1]), path } : null;
  }
}

/**
 * The drawing did not get into the new project `info`: refused for good
 * (or failing here, as an encoding can), the empty project is moved to the
 * trash; with no answer from the server it is kept, for the same upload
 * tried again. A trash that does not go through keeps it too.
 */
export async function failedUpload(api: CloudApi, info: ProjectInfo, failure: unknown, waits?: readonly number[]): Promise<UploadFailed> {
  if (failure instanceof ApiFailure && failure.transient) return new UploadFailed(info, failure, 'kept');
  // Built once: its tries keep the idempotency key.
  const trash = catalogEnvelope('project.trash', info.tenantId, info.id, {});
  try {
    await again(() => api.lifecycle(trash), waits);
    return new UploadFailed(info, failure, 'trashed');
  } catch {
    return new UploadFailed(info, failure, 'kept');
  }
}

/** The drawing's bytes, their hash and the verified upload that holds them. */
export interface SentDrawing {
  encoded: EncodedDrawing;
  sha256: string;
  upload: FileUpload;
}

/**
 * Sends the drawing on screen as an upload of the project: encoded (the
 * drawing of this moment, in the formats worker), hashed, uploaded; the
 * server verifies it before it answers. `stage` and `progress` hear where it is.
 */
export async function sendDrawing(
  api: CloudApi,
  doc: CadDocument,
  codec: () => Promise<DrawingCodec>,
  target: UploadTarget,
  o: { stage?: (s: FileStage) => void; progress?: Transfer; warn?: (text: string) => void; waits?: readonly number[]; part?: number } = {},
): Promise<SentDrawing> {
  o.stage?.('encoding');
  const encoded = await encodeDrawing(doc, codec);
  if (encoded.dropped) o.warn?.(`KCAD v2'nin tanımadığı alanlar yüklenmedi: ${encoded.dropped}.`);
  const sha256 = await sha256Hex(encoded.bytes);
  o.stage?.('uploading');
  const upload = await uploadBytes(api, target, encoded.bytes, sha256, { progress: o.progress, verifying: () => o.stage?.('verifying'), waits: o.waits, part: o.part });
  o.stage?.('verifying');
  return { encoded, sha256, upload };
}
