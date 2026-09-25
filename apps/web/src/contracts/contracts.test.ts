import { describe, expect, it } from 'vitest';
import contractsLib from '../../../../crates/shared/contracts/src/lib.rs?raw';
import type { DefaultsContext } from '../processing/types';
import type { RunJob } from '../processing/job';
import type { Entity } from '../model/entities';
import type { LayerNode } from '../model/layers';
import type { ProjectSettingsData } from '../model/projectSettings';
import type { StyleFile } from '../style/file';
import type { Entity as ContractEntity } from './generated/Entity';
import type { LayerNode as ContractLayerNode } from './generated/LayerNode';
import type { ProjectSettings as ContractProjectSettings } from './generated/ProjectSettings';
import type { RunJob as ContractRunJob } from './generated/RunJob';
import type { DefaultsContext as ContractDefaultsContext } from './generated/DefaultsContext';
import type { StyleFile as ContractStyleFile } from './generated/StyleFile';
import { CONTRACTS_VERSION } from './version';

/**
 * The app's own types checked against the versioned contracts generated from
 * Rust (crates/shared/contracts → src/contracts/generated, docs/adr/0002). The
 * checks are type assignments: if a field is added, renamed or retyped on
 * one side only, `tsc` fails here. Readonly lists become plain lists on the
 * wire, so the internal side is compared through `Wire<…>`.
 */

type Wire<T> = T extends readonly [infer A, infer B]
  ? [Wire<A>, Wire<B>]
  : T extends readonly (infer U)[]
    ? Wire<U>[]
    : T extends object
      ? { -readonly [K in keyof T]: Wire<T[K]> }
      : T;

// Everything the app writes fits the wire format …
const entityOut = (e: Entity): ContractEntity => e;
const layerOut = (n: LayerNode): ContractLayerNode => n;
const settingsOut = (s: ProjectSettingsData): ContractProjectSettings => s;
const styleFileOut = (f: Wire<StyleFile>): ContractStyleFile => f;
const unitsOut = (u: DefaultsContext): ContractDefaultsContext => u;
const jobOut = (j: Wire<RunJob>): ContractRunJob => j;
// … and every object the wire can carry is one the app knows (after the reader's checks).
const entityIn = (e: ContractEntity): Entity => e;
// A file from before work modes has none; the reader reads it as hybrid (model/snapshot.ts).
const settingsIn = (s: ContractProjectSettings): ProjectSettingsData => ({ ...s, workspace: s.workspace ?? 'hybrid' });

describe('versioned contracts', () => {
  it('match the app types (checked by tsc; this keeps the checks referenced)', () => {
    expect([entityOut, layerOut, settingsOut, styleFileOut, unitsOut, jobOut, entityIn, settingsIn].every((f) => typeof f === 'function')).toBe(true);
  });
  it('are the version the Rust crate speaks', () => {
    expect(Number(/pub const CONTRACTS_VERSION: u32 = (\d+);/.exec(contractsLib)?.[1])).toBe(CONTRACTS_VERSION);
  });
});
