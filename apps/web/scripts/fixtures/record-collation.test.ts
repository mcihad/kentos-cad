// Writes the Turkish text order of the expression language
// (crates/shared/style-core/src/js/collation_tr.rs) from ICU's Turkish
// collation, as Node ships it. Runs only on purpose:
//   GOLDEN_WRITE=1 pnpm -C apps/web exec vitest run scripts/fixtures/record-collation.test.ts
// The TypeScript compared text with localeCompare(…, 'tr'), so the order
// depended on the browser's ICU; the core carries a fixed table instead
// (docs/adr/0008 “İfade dili”). Each character gets ICU's rank at the three
// strengths (letter, accent, case); a character the rank model cannot
// reproduce against ICU on random strings (ß, æ and the other expansions)
// is left out and ordered by code point after every letter. Rewriting the
// table is a deliberate change of the language's order, to be read in the diff.
import { writeFileSync } from 'node:fs';
import { expect, it } from 'vitest';

const OUT = new URL('../../../../crates/shared/style-core/src/js/collation_tr.rs', import.meta.url);

/** Blocks the table covers: controls that occur in text, ASCII, Latin, Greek, Cyrillic, general punctuation, currency. */
const BLOCKS: readonly [number, number][] = [
  [0x09, 0x0d],
  [0x20, 0x7e],
  [0xa0, 0x24f],
  [0x370, 0x3ff],
  [0x400, 0x4ff],
  [0x2000, 0x205e],
  [0xfeff, 0xfeff],
  [0x20a0, 0x20c0],
];

const full = new Intl.Collator('tr');
const accent = new Intl.Collator('tr', { sensitivity: 'accent' });
const base = new Intl.Collator('tr', { sensitivity: 'base' });

interface Rank {
  p: number;
  s: number;
  t: number;
}

/** Ranks of the kept characters; ignorable ones get none (skipped at every level). */
function ranks(chars: readonly string[]): Map<string, Rank | null> {
  const out = new Map<string, Rank | null>();
  const counted: string[] = [];
  for (const c of chars) {
    if (full.compare(c, '') === 0) out.set(c, null);
    else counted.push(c);
  }
  counted.sort(full.compare);
  let p = 0;
  let s = 0;
  let t = 0;
  counted.forEach((c, i) => {
    if (i > 0) {
      const prev = counted[i - 1];
      if (base.compare(prev, c) !== 0) {
        p++;
        s = 0;
        t = 0;
      } else if (accent.compare(prev, c) !== 0) {
        s++;
        t = 0;
      } else if (full.compare(prev, c) !== 0) t++;
    }
    // Primaries start at 1: 0 is no character.
    out.set(c, { p: p + 1, s, t });
  });
  return out;
}

/** The core's rule (collation_tr.rs `compare_tr`): letters, then accents, then case, over the whole text. */
function modelCompare(table: Map<string, Rank | null>, a: string, b: string): number {
  const keys = (x: string) =>
    [...x].flatMap((c) => {
      const r = table.get(c);
      if (r === null) return [];
      return [r ?? { p: 0x100000 + c.codePointAt(0)!, s: 0, t: 0 }];
    });
  const ka = keys(a);
  const kb = keys(b);
  for (const level of ['p', 's', 't'] as const) {
    for (let i = 0; i < Math.min(ka.length, kb.length); i++) if (ka[i][level] !== kb[i][level]) return Math.sign(ka[i][level] - kb[i][level]);
    if (ka.length !== kb.length) return Math.sign(ka.length - kb.length);
  }
  return 0;
}

it.runIf(!!process.env.GOLDEN_WRITE)('records ICU’s Turkish order for the expression language', () => {
  let chars: string[] = [];
  for (const [a, b] of BLOCKS) for (let c = a; c <= b; c++) if (!(c >= 0xd800 && c <= 0xdfff)) chars.push(String.fromCodePoint(c));
  // Left out: characters whose order the one-rank-per-character model gets wrong.
  const left: string[] = [];
  // Marks with an accent but no letter of their own cannot be one rank: left out.
  for (const c of chars) if (base.compare(c, '') === 0 && full.compare(c, '') !== 0) left.push(c);
  chars = chars.filter((c) => !left.includes(c));
  // Expansions (ß = ss, Ǳ = DZ): a character of one rank is greater than the letter below it
  // followed by anything; one that expands flips where its second letter falls. Expansions
  // next to each other hide one another (ȸ = db lies between d and Ǳ), so until none is left.
  for (;;) {
    const first = ranks(chars);
    // One character per letter, in the order of the letters.
    const letters = [...new Map([...first].flatMap(([c, r]) => (r ? [[r.p, c] as const] : [])))].sort((x, y) => x[0] - y[0]).map(([, c]) => c);
    const found = chars.filter((c) => {
      if (!first.get(c)) return false;
      // The letter just below it: the first element of an expansion (ß → s, ″ → ′).
      const own = letters.filter((l) => base.compare(l, c) < 0).at(-1);
      return !!own && new Set(letters.map((y) => Math.sign(full.compare(c, own + y)))).size > 1;
    });
    if (!found.length) break;
    left.push(...found);
    chars = chars.filter((c) => !found.includes(c));
  }
  // Contractions with a letter ("L·" is one element): every character after every ASCII letter and digit.
  {
    const table = ranks(chars);
    const alnum = chars.filter((c) => /[A-Za-z0-9]/.test(c));
    const found = chars.filter(
      (y) =>
        !alnum.includes(y) &&
        alnum.some((x) =>
          [
            [x + y, x],
            [x + y, x + 'a'],
            [x + y + 'a', x + 'b'],
            [x + y, x + '\u00a0'],
          ].some(([a, b]) => Math.sign(full.compare(a, b)) !== modelCompare(table, a, b)),
        ),
    );
    left.push(...found);
    chars = chars.filter((c) => !found.includes(c));
  }
  let seed = 7;
  const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647;
  let clean = 0;
  for (let round = 0; round < 1000; round++) {
    const table = ranks(chars);
    // Strings of letters with their accented and cased relatives, so every level is reached.
    const pick = () => chars[Math.floor(rnd() * chars.length)];
    const kin = new Map<number, string[]>();
    for (const c of chars) {
      const r = table.get(c);
      if (r) kin.set(r.p, [...(kin.get(r.p) ?? []), c]);
    }
    const near = (c: string) => {
      const r = table.get(c);
      const same = r ? kin.get(r.p)! : [c];
      return same[Math.floor(rnd() * same.length)];
    };
    const blame = new Map<string, number>();
    let wrong = 0;
    for (let i = 0; i < 100000; i++) {
      const n = 1 + Math.floor(rnd() * 5);
      const a: string[] = [];
      for (let k = 0; k < n; k++) a.push(pick());
      // Half the pairs are relatives (every level), half are unrelated strings (expansions show there).
      const b = i % 2 ? a.map((c) => (rnd() < 0.5 ? near(c) : rnd() < 0.2 ? pick() : c)) : Array.from({ length: Math.floor(rnd() * 6) }, pick);
      if (i % 2 && rnd() < 0.2) b.splice(Math.floor(rnd() * b.length), 1);
      const sa = a.join('');
      const sb = b.join('');
      if (Math.sign(full.compare(sa, sb)) !== modelCompare(table, sa, sb)) {
        wrong++;
        // Blamed: the characters whose removal from both strings makes the model agree.
        const present = [...new Set([...a, ...b])];
        const fixes = present.filter((c) => {
          const x = a.filter((d) => d !== c).join('');
          const y = b.filter((d) => d !== c).join('');
          return Math.sign(full.compare(x, y)) === modelCompare(table, x, y);
        });
        // A contraction ("L·" is one element) is fixed by removing either character: never an ASCII
        // letter or digit, which every text has.
        const rare = (fixes.length ? fixes : present).filter((c) => !/[A-Za-z0-9]/.test(c));
        for (const c of rare) blame.set(c, (blame.get(c) ?? 0) + (fixes.length ? 1 : 0.01));
      }
    }
    if (!wrong && ++clean === 5) break;
    if (!wrong) continue;
    clean = 0;
    const worst = [...blame].sort((x, y) => y[1] - x[1])[0][0];
    left.push(worst);
    chars = chars.filter((c) => c !== worst);
  }
  const table = ranks(chars);
  // A last, independent check on fresh strings.
  let seed2 = 99;
  const rnd2 = () => (seed2 = (seed2 * 48271) % 2147483647) / 2147483647;
  for (let i = 0; i < 300000; i++) {
    const s = () => Array.from({ length: Math.floor(rnd2() * 6) }, () => chars[Math.floor(rnd2() * chars.length)]).join('');
    const a = s();
    const b = s();
    expect(modelCompare(table, a, b), `${JSON.stringify(a)} ${JSON.stringify(b)}`).toBe(Math.sign(full.compare(a, b)));
  }
  // The core packs a row into six bytes (code point, letter: u16; accent, case: u8).
  for (const [c, r] of table) expect(c.codePointAt(0)! <= 0xffff && (!r || (r.p <= 0xffff && r.s <= 0xff && r.t <= 0xff))).toBe(true);
  const rows = [...table]
    .map(([c, r]) => [c.codePointAt(0)!, r] as const)
    .sort((x, y) => x[0] - y[0])
    .map(([cp, r]) => (r ? `    (0x${cp.toString(16)}, ${r.p}, ${r.s}, ${r.t}),` : `    (0x${cp.toString(16)}, 0, 0, 0),`));
  const hex = (c: string) => `U+${c.codePointAt(0)!.toString(16).toUpperCase().padStart(4, '0')}`;
  writeFileSync(
    OUT,
    `// Generated by apps/web/scripts/fixtures/record-collation.test.ts from ICU ${process.versions.icu}
// (Node ${process.version}, Unicode ${process.versions.unicode}); do not edit.
// Turkish collation ranks: (character, letter, accent, case); letter 0 is
// ignorable. Left out (ordered by code point after every letter): ${left.map(hex).join(', ') || 'none'}.

pub(super) static TABLE: &[(u16, u16, u8, u8)] = &[
${rows.join('\n')}
];
`,
  );
});
