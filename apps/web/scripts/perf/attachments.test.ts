// Nesne ekleri on the web (docs/adr/0225 §7), the same work as the core's: a thousand photos' EXIF headers read
// (`formats::exif`, through the formats WASM, `crates/shared/formats/tests/all/exif.rs`'s `timing`), 10 000 parcels'
// card texts by a template of four placeholders (the application crate's `info_templates::timing`), and one card
// opened (its title, rows and text and two actions filled, the desktop's `a_card_opens_in_a_frame`). p50 and p95 of
// repeated runs after a warm-up against the ADR's budgets, 2.5 times the core's. Runs only on purpose, with the WASM
// that ships:
//   KENTOS_WASM_PROFILE=wasm pnpm -s rust:wasm; ATTACH_BENCH=1 pnpm -C apps/web exec vitest run scripts/perf/attachments.test.ts --disable-console-intercept; rm apps/web/src/wasm/pkg/.stamp
import { cpus, release } from 'node:os';
import { expect, it } from 'vitest';
import type { Entity } from '../../src/model/entities';
import { compileTemplate, templateTexts } from '../../src/model/infoTemplates';
import { CoreStore } from '../../src/wasm/core';

const RUNS = Number(process.env.ATTACH_RUNS ?? 20);
const WARM = 3;

interface Formats {
  initSync(o: { module: Uint8Array }): unknown;
  readExif(bytes: Uint8Array): Uint8Array;
}
const glue = import.meta.glob<Formats>('../../src/io/pkg/kentos_formats_wasm.js');
const loader = Object.values(glue)[0];
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL): Uint8Array<ArrayBuffer> } } }).process.getBuiltinModule('node:fs');

/** p50 and p95 in milliseconds. */
function quantiles(ms: number[]): [number, number] {
  const s = [...ms].sort((a, b) => a - b);
  const q = (p: number) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return [q(0.5), q(0.95)];
}

/** Runs a work; its p50 against `budget`, said. */
function measure(name: string, work: () => void, budget: number): boolean {
  const ms: number[] = [];
  for (let run = 0; run < WARM + RUNS; run++) {
    const t = performance.now();
    work();
    if (run >= WARM) ms.push(performance.now() - t);
  }
  const [p50, p95] = quantiles(ms);
  const ok = p50 <= budget;
  console.log(`${name.padEnd(46)} p50 ${p50.toFixed(3).padStart(8)} ms (p95 ${p95.toFixed(3).padStart(8)}), bütçe ${budget.toFixed(1).padStart(6)} ms: ${ok ? 'bütçede' : 'BÜTÇEYİ AŞIYOR'}`);
  return ok;
}

/** The core's test's parcels: a 100 × 100 grid of 28 × 23 m, Ada, Parsel and Mahalle. */
const parcel = (i: number): Entity => {
  const [x, y] = [487_000 + (i % 100) * 30, 4_420_000 + Math.floor(i / 100) * 25];
  return {
    id: i + 1,
    kind: 'polygon',
    layerId: 'parsel',
    attrs: { Ada: String(100 + Math.floor(i / 100)), Parsel: String(1 + (i % 100)), Mahalle: i % 2 === 0 ? 'Kızılay' : 'Bahçelievler' },
    pts: [
      { x, y },
      { x: x + 28, y },
      { x: x + 28, y: y + 23 },
      { x, y: y + 23 },
    ],
  };
};

it.runIf(!!process.env.ATTACH_BENCH && !!loader)('measures the attachments’ work against its budgets', async () => {
  const over: string[] = [];
  const w = await loader!();
  w.initSync({ module: fs.readFileSync(new URL('../../src/io/pkg/kentos_formats_wasm_bg.wasm', import.meta.url)) });
  const photo = fs.readFileSync(new URL('../../../../fixtures/exif/v1/konum.jpg', import.meta.url));
  let placed = 0;
  if (
    !measure(
      'EXIF: 1 000 fotoğrafın başlığı',
      () => {
        placed = 0;
        for (let i = 0; i < 1000; i++) if ((JSON.parse(new TextDecoder().decode(w.readExif(photo))) as { latitude?: number }).latitude != null) placed++;
      },
      125,
    )
  )
    over.push('EXIF');
  expect(placed).toBe(1000);
  const parcels = Array.from({ length: 10_000 }, (_, i) => parcel(i));
  const store = new CoreStore();
  store.put(JSON.stringify(parcels));
  const t = compileTemplate('Parsel {Ada}/{Parsel}, {Mahalle}: {yuvarla($alan, 2)} m²');
  if (!t.ok) throw new Error(t.error);
  let texts: string[] = [];
  if (!measure('Bilgi kartı: 10 000 nesnenin metni', () => (texts = templateTexts(t.template, parcels, () => 'Parsel', store)), 500)) over.push('kart metinleri');
  expect(texts[0]).toBe('Parsel 100/1, Kızılay: 644 m²');
  expect(texts[9999]).toBe('Parsel 199/100, Bahçelievler: 644 m²');
  // One card: its title and text, two actions (an address with the place, a file).
  const one = [parcels[0]];
  const card = () => {
    for (const source of ['Parsel {Ada}/{Parsel}', 'Hesaplanan alan: {yuvarla($alan, 2)} m²\nÇevre: {yuvarla($çevre, 2)} m', 'https://www.openstreetmap.org/?mlat={$enlem}&mlon={$boylam}', 'belgeler/tapu-{Ada}-{Parsel}.pdf']) {
      const c = compileTemplate(source);
      if (!c.ok) throw new Error(c.error);
      templateTexts(c.template, one, () => 'Parsel', store, () => [39.92, 32.85]);
    }
  };
  if (!measure('Bilgi kartı: bir kartın açılması', card, 40)) over.push('kart');
  store.dispose();
  console.log(`\n${cpus()[0]?.model ?? '?'}, ${cpus().length} iş parçacığı; Linux ${release()}; Node ${process.version}`);
  expect(over, `bütçeyi aşanlar: ${over.join(', ')}`).toEqual([]);
}, 600_000);
