import type { CommandEnvelope } from '../../contracts/generated/CommandEnvelope';
import type { DrawingEntity } from '../../model/entities';
import { ApiFailure, type CloudApi } from './api';
import { wire } from './tracker';

/**
 * The objects of a drawing sent into a new cloud project (`CloudSession.upload`),
 * in batches. Each object goes under its persistent id (ADR 0014 slice 3,
 * docs/adr/0026): the same drawing uploaded twice makes two projects holding
 * the same ids (the server keys objects per project), and a batch sent again
 * after its answer was lost is the same command with the same idempotency
 * key, answered from the server's log instead of committed twice.
 */

export const UPLOAD_BATCH = 2000;

/** How long to wait before each next try while the server does not answer (then the upload fails and says why). */
export const RETRY_WAITS_MS = [500, 1000, 2000, 4000, 8000];

const uuid = () => crypto.randomUUID();

/**
 * `call` again, unchanged, while the server does not answer (no network, a
 * timeout, a restart); any other refusal ends it at once. Safe only for a
 * call that carries its own idempotency key.
 */
export async function again<T>(call: () => Promise<T>, waits: readonly number[] = RETRY_WAITS_MS): Promise<T> {
  for (let i = 0; ; i++) {
    try {
      return await call();
    } catch (e) {
      if (!(e instanceof ApiFailure) || !e.transient || i >= waits.length) throw e;
      await new Promise((resolve) => setTimeout(resolve, waits[i]));
    }
  }
}

export interface Uploaded {
  /** Each object's id (its `uid`) and the version the server gave it. */
  records: { id: string; version: string }[];
  /** The event cursor after the last batch. */
  cursor: string;
}

/** Creates `entities` in the project, batch by batch; `cursor` is the project's event cursor before them. */
export async function uploadObjects(
  api: CloudApi,
  target: { tenantId: string; projectId: string; cursor: string },
  entities: readonly DrawingEntity[],
  progress: (done: number, total: number) => void = () => {},
  waits: readonly number[] = RETRY_WAITS_MS,
): Promise<Uploaded> {
  const records: Uploaded['records'] = [];
  let cursor = target.cursor;
  progress(0, entities.length);
  for (let i = 0; i < entities.length; i += UPLOAD_BATCH) {
    const part = entities.slice(i, i + UPLOAD_BATCH);
    // Built once: a try after a lost answer must be the very same request.
    const envelope: CommandEnvelope = {
      commandName: 'project.changes',
      version: 1,
      tenantId: target.tenantId,
      projectId: target.projectId,
      requestId: `web-${uuid()}`,
      idempotencyKey: uuid(),
      expectedVersions: {},
      input: { features: part.map((e) => ({ op: 'create', id: e.uid, entity: structuredClone(wire(e)) })) },
    };
    const result = await again(() => api.command(envelope), waits);
    for (const e of part) {
      const version = result.versions[e.uid];
      if (version !== undefined) records.push({ id: e.uid, version });
    }
    cursor = result.eventSeq;
    progress(Math.min(entities.length, i + UPLOAD_BATCH), entities.length);
  }
  return { records, cursor };
}
