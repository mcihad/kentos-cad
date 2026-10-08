import { describe, expect, it } from 'vitest';
import source from '../../../../../crates/shared/formats/src/raster/style.rs?raw';
import { RASTER_RAMPS } from '../../model/rasterRules';
import { RAMP_STOPS } from './ramps';

/** The windows' ramp samples are the core's stops (docs/adr/0204 §4): read from the Rust source itself. */
describe('raster ramps', () => {
  it('match the formats core’s stops', () => {
    const body = source.slice(source.indexOf('pub fn ramp_stops'), source.indexOf('pub fn ramp_at'));
    const core: Record<string, number[][]> = {};
    // Each arm from its name to the next arm, however rustfmt lays its stops out.
    const arms = [...body.matchAll(/"([^"]+)" => &\[/g)];
    arms.forEach((m, i) => {
      const end = i + 1 < arms.length ? arms[i + 1].index : body.indexOf('_ =>');
      const text = body.slice(m.index, end);
      core[m[1]] = [...text.matchAll(/\[0x(\w\w), 0x(\w\w), 0x(\w\w)\]/g)].map((c) => [1, 2, 3].map((k) => parseInt(c[k], 16)));
    });
    for (const name of RASTER_RAMPS.filter((r) => r !== 'Gri')) expect(RAMP_STOPS[name], name).toEqual(core[name]);
    expect(Object.keys(core).sort()).toEqual(RASTER_RAMPS.filter((r) => r !== 'Gri').sort());
    expect(body).toContain('_ => &[[0, 0, 0], [255, 255, 255]]');
  });
});
