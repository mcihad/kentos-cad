/**
 * The raster ramps' stops (docs/adr/0204 §4) for the windows' wide samples: what the formats core colours a tile by
 * (`raster::style::ramp_stops`, crates/shared/formats/src/raster/style.rs; ramps.test.ts keeps the two equal). The
 * drawing's colours are the core's; these only draw the samples.
 */

/** Each ramp's stops (sRGB), evenly spaced. */
export const RAMP_STOPS: Readonly<Record<string, readonly (readonly [number, number, number])[]>> = {
  Gri: [
    [0, 0, 0],
    [255, 255, 255],
  ],
  Arazi: [
    [0x2e, 0x7d, 0x32],
    [0x9c, 0xcc, 0x65],
    [0xff, 0xf5, 0x9d],
    [0xa1, 0x88, 0x7f],
    [0xfa, 0xfa, 0xfa],
  ],
  Spektral: [
    [0x2b, 0x83, 0xba],
    [0xab, 0xdd, 0xa4],
    [0xff, 0xff, 0xbf],
    [0xfd, 0xae, 0x61],
    [0xd7, 0x19, 0x1c],
  ],
  Viridis: [
    [0x44, 0x01, 0x54],
    [0x3b, 0x52, 0x8b],
    [0x21, 0x91, 0x8c],
    [0x5e, 0xc9, 0x62],
    [0xfd, 0xe7, 0x25],
  ],
  'Mavi-kırmızı': [
    [0x21, 0x66, 0xac],
    [0x92, 0xc5, 0xde],
    [0xf7, 0xf7, 0xf7],
    [0xf4, 0xa5, 0x82],
    [0xb2, 0x18, 0x2b],
  ],
  Sıcaklık: [
    [0xff, 0xff, 0xb2],
    [0xfe, 0xcc, 0x5c],
    [0xfd, 0x8d, 0x3c],
    [0xf0, 0x3b, 0x20],
    [0xbd, 0x00, 0x26],
  ],
};

/** A ramp as a CSS gradient from left to right (turned round when `invert`). */
export function rampCss(name: string, invert: boolean): string {
  const stops = [...(RAMP_STOPS[name] ?? RAMP_STOPS.Gri)];
  if (invert) stops.reverse();
  const n = stops.length - 1;
  return `linear-gradient(to right, ${stops.map(([r, g, b], i) => `rgb(${r} ${g} ${b}) ${Math.round((i / n) * 100)}%`).join(', ')})`;
}
