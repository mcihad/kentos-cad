import { describe, expect, it } from 'vitest';
import { basemapCommand, layerOf, perHost, PRESET_GROUPS, PRESETS, preset, serviceIcon } from './servicePresets';
import { serviceProblem } from './serviceRules';

/** The ready basemaps (docs/adr/0208 §1): fixtures/services/v1/presets.json, which the desktop reads too. */
describe('ready basemaps', () => {
  it('are all valid services in known groups, each with its own icon and command', () => {
    const groups = new Set(PRESET_GROUPS.map((g) => g.id));
    const icons = new Set<string>();
    const commands = new Set<string>();
    for (const p of PRESETS) {
      expect(groups.has(p.group), p.id).toBe(true);
      expect(serviceProblem(layerOf(p)), p.id).toBeNull();
      icons.add(p.icon);
      commands.add(basemapCommand(p.id));
    }
    expect(icons.size).toBe(PRESETS.length);
    expect(commands.size).toBe(PRESETS.length);
  });

  it('name their commands in camel case, as the desktop does', () => {
    expect(basemapCommand('osm-standard')).toBe('basemap.osmStandard');
    expect(basemapCommand('hgm-ortofoto')).toBe('basemap.hgmOrtofoto');
    expect(basemapCommand('ofm-liberty')).toBe('basemap.ofmLiberty');
  });

  it('add a layer that names its preset and its connection; a key stays out of it', () => {
    const osm = preset('osm-standard')!;
    expect(layerOf(osm).preset).toBe('osm-standard');
    expect(layerOf(osm).connection).toBeUndefined();
    expect(perHost(layerOf(osm))).toBe(2);
    const keyed = PRESETS.find((p) => p.connection)!;
    expect(layerOf(keyed).connection).toBe(keyed.connection!.id);
    expect(JSON.stringify(keyed.connection)).not.toMatch(/"(values|token|password)"/);
  });

  it("give a service layer its preset's icon, or its kind's", () => {
    expect(serviceIcon(layerOf(preset('esri-imagery')!))).toBe('basemapImagery');
    expect(serviceIcon({ kind: 'wms', url: 'https://ornek.org/wms', layers: ['a'], srid: 3857, version: '1.3.0' })).toBe('serviceAdd');
    expect(serviceIcon({ kind: 'vector', url: 'https://ornek.org/style.json' })).toBe('basemapVector');
  });
});
