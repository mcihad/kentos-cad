// Smoke run of the sheet core's WASM package (docs/sheet/tasks-rust.md Step 2):
// loads apps/web/src/product/sheet/pkg with `initSync` and holds its answers to
// what the Rust tests expect of the same fixtures (fixtures/sheet/v1/README.md):
// every case of `ops/` (the book's digest, the inverse and the label, or the
// error code; then the inverse takes the book back), every display list of
// `display/`, every plan of `sync/`, and the snapping, hit, relayout, atlas,
// preflight, template and profile families besides; and the golden PDF, byte for byte.
//
//   pnpm wasm && node crates/wasm/sheet-wasm/tests/smoke.mjs
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = fileURLToPath(new URL('../../../..', import.meta.url));
const PKG = join(ROOT, 'apps/web/src/product/sheet/pkg');
const FX = join(ROOT, 'fixtures/sheet/v1');

const engine = await import(pathToFileURL(join(PKG, 'kentos_sheet_wasm.js')).href);
engine.initSync({ module: readFileSync(join(PKG, 'kentos_sheet_wasm_bg.wasm')) });

const text = (path) => readFileSync(join(FX, path), 'utf8');
const json = (path) => JSON.parse(text(path));
const files = (dir) => readdirSync(join(FX, dir)).filter((f) => f.endsWith('.json')).sort();

/** The value of an envelope; a refusal fails the run with its code and message. */
function ok(answer, where) {
  const r = JSON.parse(answer);
  assert.equal(r.ok, true, `${where}: ${r.error?.code} ${r.error?.message}`);
  return r.value;
}

/** The error of an envelope that must be a refusal. */
function refused(answer, where) {
  const r = JSON.parse(answer);
  assert.equal(r.ok, false, `${where}: kabul edildi`);
  return r.error;
}

/** A book as the Rust tests read it: checked and normalised (`read_book`), as JSON. */
const bookText = (book, where) => JSON.stringify(ok(engine.readBook(typeof book === 'string' ? book : JSON.stringify(book)), where));
const digest = (book, where) => ok(engine.bookDigest(typeof book === 'string' ? book : JSON.stringify(book)), where);

const counts = {};
const count = (family, n = 1) => {
  counts[family] = (counts[family] ?? 0) + n;
};

// ── ops: digest, inverse and label of every case (tests/ops.rs) ──────────
{
  const fx = json('ops/cases.json');
  const base = bookText(text(`ops/${fx.book}`), 'ops/book.json');
  const baseDigest = digest(base, 'ops/book.json');
  for (const c of fx.cases) {
    const answer = engine.applyOp(base, JSON.stringify(c.op));
    if (c.expect.error) {
      assert.equal(refused(answer, c.name).code, c.expect.error, c.name);
      count('ops (hata)');
      continue;
    }
    const a = ok(answer, c.name);
    assert.equal(digest(a.book, c.name), c.expect.bookSha256, `${c.name}: kitap`);
    assert.deepStrictEqual(a.inverse, c.expect.inverse, `${c.name}: ters işlem`);
    assert.equal(a.label, c.expect.label, `${c.name}: etiket`);
    const back = ok(engine.applyOps(JSON.stringify(a.book), JSON.stringify(a.inverse)), `${c.name} (ters)`);
    assert.equal(digest(back.book, c.name), baseDigest, `${c.name}: ters işlem kitabı geri vermedi`);
    count('ops');
  }
}

// ── display: the recorded lists (tests/fixtures.rs `display`) ────────────
for (const f of files('display')) {
  const fx = json(`display/${f}`);
  const book = bookText(typeof fx.book === 'string' ? text(`display/${fx.book}`) : fx.book, f);
  const list = ok(engine.displayList(book, fx.sheet, JSON.stringify(fx.inputs)), f);
  assert.deepStrictEqual(list, fx.expect, `display/${f}`);
  const svg = ok(engine.toSvg(JSON.stringify(list), '{}'), `${f} svg`);
  assert.ok(svg.startsWith('<?xml'), `${f}: SVG`);
  count('display');
}

// ── sync: §13's table (tests/fixtures.rs `sync`) ─────────────────────────
for (const f of files('sync')) {
  for (const c of json(`sync/${f}`).cases) {
    const plan = ok(engine.planSync(JSON.stringify(c.local), JSON.stringify(c.remote)), c.description);
    assert.deepStrictEqual(plan.actions, c.expect, c.description);
    count('sync');
  }
}

// ── relayout through setPage ─────────────────────────────────────────────
for (const f of files('relayout')) {
  const fx = json(`relayout/${f}`);
  const book = bookText(fx.book, f);
  for (const c of fx.cases) {
    const op = { op: 'setPage', owner: { kind: 'sheet', id: fx.sheet }, page: c.page, relayout: true };
    const a = ok(engine.applyOp(book, JSON.stringify(op)), c.description);
    const sheet = a.book.sheets.find((s) => s.id === fx.sheet);
    for (const [id, frame] of Object.entries(c.expect)) {
      assert.deepStrictEqual(sheet.items.find((i) => i.id === id).frame, frame, `${c.description}: ${id}`);
    }
    if ('expectVariant' in c) assert.equal(sheet.activeVariant ?? null, c.expectVariant, `${c.description}: düzen`);
    count('relayout');
  }
}

// ── snapping ─────────────────────────────────────────────────────────────
for (const f of files('snap')) {
  const fx = json(`snap/${f}`);
  const session = new engine.SnapSession(bookText(fx.book, f), fx.sheet, JSON.stringify(fx.moving), JSON.stringify(fx.options ?? {}));
  try {
    for (const q of fx.queries) {
      assert.deepStrictEqual(ok(session.query(q.delta[0], q.delta[1], q.tolerance), q.description), q.expect, q.description);
      count('snap');
    }
    for (const q of fx.resize) {
      assert.deepStrictEqual(ok(session.queryResize(q.handle, q.to[0], q.to[1], q.tolerance, false), q.description), q.expect, q.description);
      count('snap');
    }
  } finally {
    session.free();
  }
  for (const q of fx.rotation) {
    assert.equal(engine.snapRotation(q.angle, q.step), q.expect, JSON.stringify(q));
    count('snap');
  }
}

// ── hit test ─────────────────────────────────────────────────────────────
for (const f of files('hit')) {
  const fx = json(`hit/${f}`);
  const book = bookText(fx.book, f);
  for (const q of fx.queries) {
    assert.deepStrictEqual(ok(engine.hitTest(book, fx.sheet, JSON.stringify(q.query)), q.description), q.expect, q.description);
    count('hit');
  }
}

// ── atlas ────────────────────────────────────────────────────────────────
for (const f of files('atlas')) {
  const fx = json(`atlas/${f}`);
  const plan = ok(engine.atlasPlan(bookText(fx.book, f), fx.sheet, JSON.stringify(fx.features)), f);
  assert.deepStrictEqual(plan.warnings, fx.expect.warnings, `${f}: uyarılar`);
  assert.equal(plan.pages.length, fx.expect.pages.length, f);
  plan.pages.forEach((p, i) => {
    const want = fx.expect.pages[i];
    assert.deepStrictEqual([p.feature.id, p.index, p.count, p.name, p.maps], [want.feature, want.index, want.count, want.name, want.maps], `${f}: ${want.name}`);
    count('atlas');
  });
}

// ── preflight: (severity, code, item) sets ───────────────────────────────
for (const f of files('preflight')) {
  const fx = json(`preflight/${f}`);
  const found = ok(engine.preflight(bookText(fx.book, f), fx.sheet, JSON.stringify(fx.inputs)), f);
  const key = (x) => `${x.severity}|${x.code}|${x.item ?? ''}`;
  assert.deepStrictEqual([...new Set(found.map(key))].sort(), [...new Set(fx.expect.map(key))].sort(), f);
  count('preflight', found.length);
}

// ── templates: the valid one reads, every invalid one says its code ──────
for (const f of files('templates/valid')) {
  ok(engine.validateTemplate(text(`templates/valid/${f}`)), f);
  count('templates');
}
for (const f of files('templates/invalid')) {
  const fx = json(`templates/invalid/${f}`);
  assert.equal(refused(engine.validateTemplate(JSON.stringify(fx.template)), f).code, fx.expect.code, f);
  count('templates');
}

// ── .kpafta: the one codec (kentos_sheet::kpafta) ───────────────────────
for (const f of readdirSync(join(FX, 'kpafta/valid')).filter((x) => x.endsWith('.kpafta')).sort()) {
  const file = ok(engine.decodeKpafta(text(`kpafta/valid/${f}`)), `kpafta/${f}`);
  assert.equal(file.format, 'kentos.sheet.file', f);
  const written = ok(engine.encodeKpafta(JSON.stringify(file.book), JSON.stringify(file.assets)), `kpafta/${f} yazılırken`);
  assert.deepEqual(ok(engine.decodeKpafta(written), `kpafta/${f} yeniden`), file, f);
  count('kpafta');
}
for (const f of files('kpafta/invalid')) {
  const fx = json(`kpafta/invalid/${f}`);
  const input = typeof fx.file === 'string' ? fx.file : JSON.stringify(fx.file);
  assert.equal(refused(engine.decodeKpafta(input), `kpafta/${f}`).code, fx.expect.code, f);
  count('kpafta');
}

// ── work modes and the gallery ───────────────────────────────────────────
{
  const fx = json('profiles/modes.json');
  for (const c of fx.profiles) {
    const ws = JSON.stringify(c.workspace);
    const caps = JSON.stringify(c.capabilities);
    const p = ok(engine.profileFor(ws, caps), c.description);
    assert.equal(p.id, c.expect.profile, c.description);
    assert.equal(p.defaultTemplate, c.expect.defaultTemplate, c.description);
    const tools = ok(engine.toolAvailability(ws, caps), c.description);
    for (const [id, want] of Object.entries(c.expect.tools)) {
      const t = tools.find((x) => x.id === id);
      assert.ok(t, `${c.description}: ${id}`);
      for (const [k, v] of Object.entries(want)) {
        const have = k === 'presets' ? t.presets.map((x) => x.id) : t[k];
        assert.deepStrictEqual(have, v, `${c.description}: ${id}.${k}`);
      }
    }
    for (const id of c.expect.absent) assert.ok(!tools.some((t) => t.id === id), `${c.description}: ${id}`);
    count('profiles');
  }
  const metas = JSON.stringify(ok(engine.systemTemplates(), 'systemTemplates').map((t) => t.meta));
  for (const c of fx.ranking) {
    const ranked = ok(engine.rankTemplates(metas, JSON.stringify(c.workspace), JSON.stringify(c.projectType)), c.description);
    assert.deepStrictEqual(ranked, c.expect, c.description);
    count('profiles');
  }
}

// ── PDF: the golden file, byte for byte (tests/pdf.rs) ───────────────────
{
  const FONTS = join(ROOT, 'apps/desktop/assets/fonts/drawing');
  const file = (f) => ({
    'architects-daughter': 'ArchitectsDaughter',
    arimo: 'Arimo',
    barlow: 'Barlow',
    'courier-prime': 'CourierPrime',
    overpass: 'Overpass',
    'plex-mono': 'IBMPlexMono',
    quicksand: 'Quicksand',
  })[f.font] + `-${f.weight}${f.italic ? '-italic' : ''}.ttf`;
  const inputs = json('pdf/ifraz-inputs.json');
  const book = text('pdf/ifraz-book.json');
  // What a host does: ask which faces, then give their TrueType files.
  const faces = ok(engine.pdfFonts(book, JSON.stringify(inputs), '{}'), 'pdfFonts');
  assert.ok(faces.some((f) => f.font === 'barlow' && f.weight === 500), 'pdfFonts');
  for (const f of inputs.fonts) f.data = readFileSync(join(FONTS, file(f))).toString('base64');
  const pdf = engine.toPdf(book, JSON.stringify(inputs), '{}');
  const { createHash } = await import('node:crypto');
  const golden = json('pdf/ifraz.json');
  assert.equal(pdf.length, golden.size, 'PDF boyu');
  assert.equal(createHash('sha256').update(pdf).digest('hex'), golden.sha256, 'PDF baytları Rust\'unkiyle aynı değil');
  assert.throws(() => engine.toPdf(book, JSON.stringify({ ...inputs, fonts: [] }), '{}'), /^Error: pdf_font_missing: /);
  const wkt = ok(engine.tmWkt(JSON.stringify({ name: 'TUREF / TM36', datum: 'TUREF', ellipsoid: 'GRS80', epsg: 5256, tm: { centralMeridian: 36, scaleFactor: 1, falseEasting: 500000, falseNorthing: 0, semiMajor: 6378137, inverseFlattening: 298.257222101 } })), 'tmWkt');
  assert.ok(wkt.endsWith('AUTHORITY["EPSG","5256"]]'), wkt);
  // The export name (the sheet's `[% @pafta_adi %]`) and the paper's colours, from the core.
  const sheetName = JSON.parse(book).sheets[0].name;
  assert.equal(ok(engine.exportName(book, 's1', JSON.stringify(inputs.render)), 'exportName'), sheetName);
  const paper = ok(engine.paperPalette(), 'paperPalette');
  assert.deepStrictEqual(paper, { paper: '#ffffff', ink: '#000000', fg: '#111111', fgDim: '#555555', label: '#111111', labelHalo: '#ffffff' });
  // An SVG picture: the size the host draws it at, the warning without its PNG, the note with it
  // (tests/pdf.rs `an_svg_picture_goes_in_as_the_png_the_host_drew`).
  const withSvg = JSON.parse(book);
  const sha = 'a'.repeat(64);
  withSvg.assets = [...(withSvg.assets ?? []), { sha256: sha, kind: 'svg', name: 'logo.svg', width: 24, height: 24, bytes: 60 }];
  withSvg.sheets[0].items.push({ ...withSvg.sheets[0].items.find((i) => i.kind.type !== 'map'), id: 'logo', name: 'Kurum logosu', kind: { type: 'picture', asset: sha, fit: 'contain', clip: true }, frame: { left: 310000, top: 230000, width: 20000, height: 20000 }, bindings: [], rotation: 0 });
  const svgBook = JSON.stringify(ok(engine.readBook(JSON.stringify(withSvg)), 'readBook (SVG)'));
  const svgInputs = { ...inputs, assets: [{ sha256: sha, data: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"/>').toString('base64') }] };
  assert.deepStrictEqual(ok(engine.pdfSvgSizes(svgBook, JSON.stringify(svgInputs), '{}', 150), 'pdfSvgSizes'), [{ sha256: sha, width: 119, height: 119 }]);
  const warned = ok(engine.pdfFindings(svgBook, JSON.stringify(svgInputs), '{}'), 'pdfFindings');
  assert.deepStrictEqual(warned.map((f) => [f.severity, f.code, f.item]), [['warning', 'svg_not_in_pdf', 'logo']]);
  // A 1 × 1 PNG drawn by the host.
  svgInputs.assets[0].raster = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==';
  const said = ok(engine.pdfFindings(svgBook, JSON.stringify(svgInputs), '{}'), 'pdfFindings');
  assert.deepStrictEqual(said.map((f) => [f.severity, f.code]), [['info', 'svg_as_picture']]);
  assert.ok(said[0].message.endsWith("SVG resim PDF'e resim olarak gömülür (1 × 1 piksel)."), said[0].message);
  count('pdf', 9);
}

// ── The magnetic model against NOAA's test values (tests/wmm.rs): the page's 12 and the package's 100 ──
{
  const rows = (file) =>
    readFileSync(join(ROOT, 'fixtures/sheet/v1/wmm', file), 'utf8')
      .split('\n')
      .filter((l) => l.trim() && !l.trim().startsWith('#'))
      .map((l) => l.trim().split(/\s+/).map(Number));
  for (const r of rows('WMM2025_TEST_VALUES.txt')) {
    const f = ok(engine.magneticField(r[2], r[3], r[1], r[0]), 'magneticField');
    assert.ok(Math.abs(f.declination - r[10]) <= 0.01 && Math.abs(f.inclination - r[9]) <= 0.01, `WMM ${r.slice(0, 4)}`);
    count('wmm');
  }
  for (const r of rows('WMM2025_TestValues.txt')) {
    const f = ok(engine.magneticField(r[2], r[3], r[1], r[0]), 'magneticField');
    assert.ok(Math.abs(f.declination - r[4]) <= 0.01 && Math.abs(f.inclination - r[5]) <= 0.01, `WMM ${r.slice(0, 4)}`);
    assert.ok(Math.abs(f.x - r[7]) <= 0.01 && Math.abs(f.y - r[8]) <= 0.01 && Math.abs(f.z - r[9]) <= 0.01, `WMM XYZ ${r.slice(0, 4)}`);
    count('wmm');
  }
  const info = ok(engine.wmmInfo(), 'wmmInfo');
  assert.deepStrictEqual(info, { model: 'WMM2025', released: '2024-12-17', validFrom: 2025, validUntil: 2030 });
  assert.equal(ok(engine.decimalYear('2026-10-03'), 'decimalYear'), 2026 + 275 / 365);
  count('wmm', 2);
  // northInfo: the ifraz sheet's arrow; its map's centre on the ground from the core, the
  // declination the model's there on the project's date (today's: the sheet has no “tarih”).
  const book = text('pdf/ifraz-book.json');
  const render = json('pdf/ifraz-inputs.json').render;
  const n = ok(engine.northInfo(book, 's1', 'kuzey', JSON.stringify(render)), 'northInfo');
  assert.ok(n.lat > 36 && n.lat < 42 && n.lon > 26 && n.lon < 45, `northInfo yer: ${n.lat} ${n.lon}`);
  const year = ok(engine.decimalYear(render.project.date), 'decimalYear');
  const f = ok(engine.magneticField(n.lat, n.lon, 0, year), 'magneticField');
  assert.deepStrictEqual(
    [n.declination, n.source, n.model, n.date, n.dateSource, n.year, n.inModel, n.validFrom, n.validUntil],
    [f.declination, 'model', 'WMM2025', render.project.date, 'today', year, true, 2025, 2030],
    'northInfo',
  );
  assert.equal(typeof n.convergence, 'number');
  assert.equal(refused(engine.northInfo(book, 's1', 'harita', JSON.stringify(render)), 'northInfo').code, 'not_north_arrow');
  // The angles are written with the apostrophe and the quotation mark, never the wide prime.
  const arrow = ok(engine.displayList(book, 's1', JSON.stringify(render)), 'displayList').prims.filter((p) => p.type === 'text' && p.item === 'kuzey');
  assert.ok(arrow.some((p) => /^Yakınsama [−+]\d°\d\d'\d\d"$/.test(p.text)) && !arrow.some((p) => /[′″]/.test(p.text)), arrow.map((p) => p.text).join(' | '));
  count('wmm', 3);
}

// ── Missing glyphs: the pieces each face draws, and the faces a PDF embeds (fixtures::glyphs) ──
{
  for (const c of json('glyphs/runs.json').cases) {
    assert.deepStrictEqual(ok(engine.textRuns(c.font, c.weight, c.italic, c.text), c.why), c.runs, c.why);
    count('glyphs');
  }
  const fx = json('glyphs/sheet.json');
  const book = JSON.stringify(fx.book);
  const inputs = JSON.stringify({ render: fx.inputs, fonts: [], assets: [], maps: [] });
  const faces = ok(engine.pdfFonts(book, inputs, '{}'), 'pdfFonts');
  const key = (f) => `${f.font} ${f.weight}${f.italic ? ' italik' : ''}`;
  assert.deepStrictEqual(faces.map(key).sort(), fx.expectFaces.map(key).sort(), 'glyphs: pdfFonts');
  const list = ok(engine.displayList(book, fx.sheet, JSON.stringify(fx.inputs)), 'displayList');
  for (const [item, want] of Object.entries(fx.expectTexts)) {
    const have = list.prims.filter((p) => p.type === 'text' && p.item === item).map((p) => p.text).join('');
    assert.equal(have, want, `glyphs: ${item}`);
  }
  const found = ok(engine.preflight(book, fx.sheet, JSON.stringify(fx.inputs)), 'preflight');
  const tri = (f) => `${f.severity} ${f.code} ${f.item ?? ''}`;
  assert.deepStrictEqual([...new Set(found.map(tri))].sort(), fx.expect.map(tri).sort(), 'glyphs: preflight');
  count('glyphs', 3);
}

const info = ok(engine.engineInfo(), 'engineInfo');
console.log(
  `Pafta çekirdeği WASM ${info.version} (${info.bookSchema}): ` +
    Object.entries(counts)
      .map(([k, n]) => `${k} ${n}`)
      .join(', ') +
    ' — hepsi Rust testlerinin beklediğiyle aynı.',
);
