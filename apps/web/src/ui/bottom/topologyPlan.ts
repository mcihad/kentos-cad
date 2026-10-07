import type { TopologyRule } from '../../contracts/generated/TopologyRule';
import type { Bounds } from '../../model/geometry';
import type { TopologyFinding, TopologyKind } from '../../model/ops/topologyRules';

/**
 * The Topoloji tab's rules (docs/adr/0202 §5), apart from the DOM: its words, the rows' cells, which rows a filter
 * shows, the count line and the box a row zooms to. The desktop's are apps/desktop/src/topology/plan.rs, word for word;
 * the trace `topology-rules.json` reads the rows on both.
 */

export const TOPOLOGY_TEXTS = {
  tab: 'Topoloji',
  check: 'Denetle',
  checkHint: 'Projenin topoloji kurallarını çizimde denetler; bulgular tabloda, satıra tıklamak bulguya gider.',
  rules: 'Kurallar…',
  rulesHint: 'Topoloji kurallarını, toleransı ve istisnaları düzenler (proje ayarı).',
  rulePick: 'Kural',
  allRules: 'Bütün kurallar',
  open: 'Açık',
  exceptions: 'İstisna',
  all: 'Hepsi',
  showPick: 'Gösterilen bulgular',
  fix: 'Düzelt',
  fixHint: 'Seçili bulguyu düzeltir: çizim tek geri alma adımında değişir, denetim yinelenir.',
  fixNone: 'Bu bulgunun düzeltmesi yok.',
  mark: 'İstisna yap',
  markHint: 'Seçili bulguları bilerek bırakılmış sayar; projede saklanır, İstisna süzgecinde görünür.',
  unmark: 'İstisnayı kaldır',
  unmarkHint: 'Seçili bulguların istisnasını kaldırır.',
  noRules: 'Projede topoloji kuralı yok. Kurallar… ile katmanlara kural ekleyin.',
  stale: 'Çizim son denetimden sonra değişti; sonuçlar eski olabilir. Denetle ile yenileyin.',
  noneShown: 'Bu süzgeçte bulgu yok.',
} as const;

/** What the rows show: the findings left open, those marked as exceptions, or all. */
export type TopologyFilter = 'open' | 'exception' | 'all';

/** The table's columns. */
export const TOPOLOGY_COLUMNS = ['Sıra', 'Katman', 'Kural', 'Sorun', 'Nesneler', 'Ölçü'] as const;

/** Before a check: how many rules a check runs. */
export const notChecked = (rules: number): string => `Denetle'ye basın: ${rules} kural denetlenecek.`;

/** After a check without findings: how many rules and objects it looked at. */
export const noFindings = (rules: number, objects: number): string => `Bulgu yok: ${rules} kural, ${objects} nesne denetlendi.`;

/** Rules whose layer (or other layer) the project no longer has: not checked. */
export const missingLayers = (n: number): string => `${n} kuralın katmanı projede yok; denetlenmedi.`;

/** A rule as a sentence: its kind's name, the other layer's name for “…”. */
export function ruleText(rule: TopologyRule, kinds: readonly TopologyKind[], layerName: (id: string) => string): string {
  const kind = kinds.find((k) => k.key === rule.kind);
  const label = kind?.label ?? rule.kind;
  return kind?.between && rule.other !== undefined ? label.replace('…', layerName(rule.other)) : label;
}

/** The objects a finding names, as the drawing shows their ids. */
export const objectsText = (f: TopologyFinding, ids: readonly number[]): string => f.objects.map((k) => `#${ids[k]}`).join(', ');

/** A finding's measure in the project's units; empty when it has none or it is a distance of nothing (a T junction). */
export function measureText(f: TopologyFinding, fmt: { area(m2: number): string; length(m: number): string; angle(rad: number): string }): string {
  if (f.measure === undefined || (f.measureKind === 'distance' && f.measure === 0)) return '';
  switch (f.measureKind) {
    case 'area':
      return fmt.area(f.measure);
    case 'angle':
      return fmt.angle(f.measure);
    default:
      return fmt.length(f.measure);
  }
}

/**
 * A fix as Düzelt ▾ lists it: its name and what it acts on (the object an area is subtracted from or merged into, the
 * one a vertex is added to or deleted) or how far an end or a point moves.
 */
export function fixLabel(f: TopologyFinding, fix: { key: string; label: string }, ids: readonly number[], length: (m: number) => string): string {
  const id = (k: number | undefined) => (k === undefined || ids[k] === undefined ? '' : ` (#${ids[k]})`);
  switch (fix.key) {
    case 'subtractFirst':
    case 'addVertex':
      return fix.label + id(f.objects[0]);
    case 'subtractSecond':
      return fix.label + id(f.objects[1]);
    case 'mergeNeighbour':
    case 'deleteDuplicate':
      return fix.label + id(f.subject);
    case 'snapEnd':
    case 'snapToEnd':
      return f.measure === undefined ? fix.label : `${fix.label} (${length(f.measure)})`;
    default:
      return fix.label;
  }
}

/** The findings a filter and a rule (its place in the rules; null: every rule) show, by their places. */
export function shownFindings(findings: readonly TopologyFinding[], filter: TopologyFilter, rule: number | null): number[] {
  const out: number[] = [];
  findings.forEach((f, i) => {
    if (rule !== null && f.rule !== rule) return;
    if (filter === 'open' && f.exception) return;
    if (filter === 'exception' && !f.exception) return;
    out.push(i);
  });
  return out;
}

/** The count line: the rows shown, then how many are open and how many exceptions. */
export function countText(shown: number, open: number, exceptions: number): string {
  return `${shown} bulgu (açık ${open}, istisna ${exceptions})`;
}

/** The least side of the box a row zooms to (m): a point's problem shows its neighbourhood. */
export const FINDING_VIEW = 10;

/**
 * The box a row zooms to: the finding's, at least `FINDING_VIEW` on each side, with a fifth of it again on each side
 * for its neighbourhood, about its middle.
 */
export function findingView(b: Bounds): Bounds {
  const cx = (b.minX + b.maxX) / 2;
  const cy = (b.minY + b.maxY) / 2;
  const w = Math.max(b.maxX - b.minX, FINDING_VIEW) * 0.7;
  const h = Math.max(b.maxY - b.minY, FINDING_VIEW) * 0.7;
  return { minX: cx - w, minY: cy - h, maxX: cx + w, maxY: cy + h };
}
