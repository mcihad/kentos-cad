import { describe, expect, it } from 'vitest';
import { rasterKey } from './rasterRules';

/**
 * A raster's scene key (docs/adr/0243 §5) as the style core writes it (`crates/shared/style-core/src/style/build.rs`
 * `raster_key`, its test `a_datasets_part_is_its_variable_mesh_and_slice`): the same text for the same slice, so that the
 * tiles, the statistics and Koordinat oku open one reader.
 */
describe('rasterKey', () => {
  const base = { affine: [500000, 0.125, 0, 4420040, 0, -0.125], width: 320, height: 320, bands: 1, style: { render: 'ramp' as const, bands: [1] } };

  it('is the file alone without a dataset', () => {
    expect(rasterKey({ ...base, file: 'orto.tif' })).toBe('file:orto.tif');
    expect(rasterKey({ ...base, asset: 'raster-00' })).toBe('asset:raster-00');
  });

  it("carries a grid's variable and slice", () => {
    const dataset = {
      variable: 't2m',
      dims: [
        { name: 'time', index: 1, values: [0, 3600000, 7200000], time: true },
        { name: 'level', index: 0, values: [850, 500] },
      ],
      followTime: true,
    };
    expect(rasterKey({ ...base, file: 'iklim.nc', dataset })).toBe('file:iklim.nc#{"variable":"t2m","slice":[1,0]}');
  });

  it("carries a mesh's vector, mesh and grid", () => {
    const dataset = { variable: 'ucx', vector: 'ucy', mesh: 'mesh', dims: [{ name: 'time', index: 0, values: [0, 1800000], time: true }] };
    expect(rasterKey({ ...base, file: 'taskin.nc', dataset })).toBe(
      'file:taskin.nc#{"variable":"ucx","vector":"ucy","mesh":"mesh","slice":[0],"affine":[500000,0.125,0,4420040,0,-0.125],"size":[320,320]}',
    );
  });
});
