import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { DxfReadOptions } from '../contracts/generated/DxfReadOptions';
import type { DxfWriteInput } from '../contracts/generated/DxfWriteInput';

/**
 * Messages between the page and the formats worker (formatsWorker.ts).
 * Files cross as ArrayBuffers, transferred (the page keeps its own copy
 * when it still needs one); results come back as UTF-8 JSON bytes, also
 * transferred, and the page parses them (CLAUDE.md §6.2 rule 6).
 */

export type FormatsRequest =
  | { id: number; op: 'readCoords'; bytes: ArrayBuffer; options: CoordReadOptions }
  | { id: number; op: 'writeCoords'; input: CoordWriteInput }
  | { id: number; op: 'readDxf'; bytes: ArrayBuffer; options: DxfReadOptions }
  | { id: number; op: 'writeDxf'; input: DxfWriteInput }
  /** The persistent ids of a v1 drawing's objects (`V1Identities`, docs/adr/0014), from the drawing's text. */
  | { id: number; op: 'v1Identities'; text: string };

export type FormatsReply =
  /** A reader's result: JSON. */
  | { id: number; ok: true; json: ArrayBuffer }
  /** A writer's file and its report (ExportReport JSON). */
  | { id: number; ok: true; file: ArrayBuffer; report: string }
  /** `fatal`: the module trapped or did not load; the page starts a fresh worker. */
  | { id: number; ok: false; message: string; fatal: boolean };
