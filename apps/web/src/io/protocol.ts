import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { DocumentSnapshotV2 } from '../contracts/generated/DocumentSnapshotV2';
import type { DxfReadOptions } from '../contracts/generated/DxfReadOptions';
import type { DxfWriteInput } from '../contracts/generated/DxfWriteInput';
import type { Dropped } from './kcad';

/**
 * Messages between the page and the formats worker (formatsWorker.ts).
 * Files cross as ArrayBuffers, transferred (the page keeps its own copy
 * when it still needs one); results come back as UTF-8 JSON bytes, also
 * transferred, and the page parses them (CLAUDE.md §6.2 rule 6). A drawing
 * to save goes as a structured copy made when the message is posted, so it
 * is the drawing of that moment (its revision); a read drawing comes back
 * the same way, parsed in the worker.
 */

export type FormatsRequest =
  | { id: number; op: 'readCoords'; bytes: ArrayBuffer; options: CoordReadOptions }
  | { id: number; op: 'writeCoords'; input: CoordWriteInput }
  | { id: number; op: 'readDxf'; bytes: ArrayBuffer; options: DxfReadOptions }
  | { id: number; op: 'writeDxf'; input: DxfWriteInput }
  /** The persistent ids of a v1 drawing's objects (`V1Identities`, docs/adr/0014), from the drawing's text. */
  | { id: number; op: 'v1Identities'; text: string }
  /** A drawing to write as `.kcad` v2 (docs/specs/kcad-v2.md): its bytes come back verified. */
  | { id: number; op: 'encodeKcad'; snapshot: DocumentSnapshotV2 }
  /** A `.kcad` v2 file's bytes to read. */
  | { id: number; op: 'decodeKcad'; bytes: ArrayBuffer };

export type FormatsReply =
  /** A reader's result: JSON. */
  | { id: number; ok: true; json: ArrayBuffer }
  /** A writer's file and its report (ExportReport JSON). */
  | { id: number; ok: true; file: ArrayBuffer; report: string }
  /** A drawing's `.kcad` v2 bytes, read back and compared; `dropped` is what KCAD v2 did not keep. */
  | { id: number; ok: true; kcad: ArrayBuffer; dropped: Dropped }
  /** A `.kcad` v2 file's drawing. */
  | { id: number; ok: true; snapshot: DocumentSnapshotV2 }
  /** `fatal`: the module trapped or did not load; the page starts a fresh worker. `code`: a KCAD error's (spec §9). */
  | { id: number; ok: false; message: string; fatal: boolean; code?: string };
