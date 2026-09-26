import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { V1Identities } from '../contracts/generated/V1Identities';
import type { EncodedDrawing } from '../io/client';
import { sniffDrawing, type Dropped } from '../io/kcad';
import type { DocumentContent } from '../model/document';
import { DOCUMENT_FORMAT, DOCUMENT_VERSION, attachV1Identities, readSnapshot, readSnapshotV2 } from '../model/snapshot';
import { projectStylesProblem } from './cloud/incoming';

/**
 * A drawing file's bytes to document content, whatever version it is
 * (docs/specs/kcad-v2.md §8, docs/adr/0025): told apart by content, never by
 * name. A v2 file is decoded by the shared Rust codec in the formats worker
 * and brings every object's persistent id; a v1 file (JSON) is read here as
 * before and its ids, the project's id and the source record are derived
 * from its content in the worker meanwhile (docs/adr/0014). Either way the
 * drawing is checked like any file someone sent before anything is shown.
 */

/** The `.kcad` v2 codec: the formats worker (tests run the module in process). `encode` copies the drawing before it returns. */
export interface DrawingCodec {
  encode(snapshot: DocumentSnapshotV2): Promise<EncodedDrawing>;
  decode(bytes: Uint8Array): Promise<DocumentSnapshotV2>;
}

export interface ReadDeps {
  codec: () => Promise<DrawingCodec>;
  identities: (text: string) => Promise<V1Identities>;
}

/** A drawing read from a file: its content, the version the file was in, and a warning worth saying. */
export type ReadDrawing = { ok: true; content: DocumentContent; format: 'v1' | 'v2'; warning?: string } | { ok: false; error: string };

const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

/** Reads a file's bytes into document content, or says why it cannot (the caller names the file). */
export async function readDrawing(bytes: Uint8Array, deps: ReadDeps): Promise<ReadDrawing> {
  switch (sniffDrawing(bytes)) {
    case 'kcad':
    case 'kcad-damaged':
      return readV2(bytes, deps);
    case 'json':
      return readV1(new TextDecoder().decode(bytes), deps);
    case 'empty':
      return { ok: false, error: 'Dosya boş; içinde çizim yok. Başka bir dosya seçin ya da bir yedeği açın.' };
    case 'foreign':
      return { ok: false, error: 'KentOS çizim dosyası değil (ne KCAD v2 ne de v1). DXF ve koordinat listeleri Dosya → İçe aktar ile açılır.' };
  }
}

async function readV2(bytes: Uint8Array, deps: ReadDeps): Promise<ReadDrawing> {
  let snapshot: DocumentSnapshotV2;
  try {
    snapshot = await (await deps.codec()).decode(bytes);
  } catch (e) {
    return { ok: false, error: message(e) };
  }
  const read = readSnapshotV2(snapshot);
  if (!read.ok) return read;
  const styles = projectStylesProblem(read.content.styles);
  return styles ? { ok: false, error: `${styles}.` } : { ok: true, content: read.content, format: 'v2' };
}

async function readV1(text: string, deps: ReadDeps): Promise<ReadDrawing> {
  // Worked out in the formats worker while the page reads the drawing.
  const identities = deps.identities(text).then(
    (ids) => ({ ok: true as const, ids }),
    (e: unknown) => ({ ok: false as const, error: message(e) }),
  );
  const read = readSnapshot(text);
  if (!read.ok) return read;
  // The project's own symbols are checked like any shared style file (untrusted data).
  const styles = projectStylesProblem(read.content.styles);
  if (styles) return { ok: false, error: `${styles}.` };
  const got = await identities;
  const problem = got.ok ? attachV1Identities(read.content, got.ids) : got.error;
  if (problem)
    return {
      ok: true,
      content: read.content,
      format: 'v1',
      warning: `nesnelerin kalıcı kimlikleri dosyadan türetilemedi (${problem}); bu açılış için yeni kimlik verildi ve dosya yeniden açılınca kimlikler değişir. Dosyayı yeniden açmayı deneyin.`,
    };
  if (got.ok) {
    read.content.projectId = got.ids.project;
    read.content.migratedFrom = { format: DOCUMENT_FORMAT, version: DOCUMENT_VERSION, sourceSha256: got.ids.sourceSha256 };
  }
  return { ok: true, content: read.content, format: 'v1' };
}

/** What a save left out, in words: fields KCAD v2 does not know, by where they were (`polyline.not` × 3). */
export function describeDropped(dropped: Dropped): string | null {
  const entries = Object.entries(dropped);
  if (!entries.length) return null;
  return entries.map(([where, n]) => (n > 1 ? `${where} (${n} kez)` : where)).join(', ');
}
