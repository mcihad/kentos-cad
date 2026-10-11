import { describe, expect, it } from 'vitest';
import { attachmentKind, attachmentsProblem, isWebAddress } from './attachmentRules';
import { actionsProblem, cardOf, cardProblem, infosProblem } from './layerInfoRules';
import type { LayerNode } from './layers';

/**
 * The rules of attachments, actions and cards (docs/adr/0225 §1, §4, §5), the web's copies of
 * `kentos_contracts::attachment` and `::layer_info`; the shared command cases (fixtures/commands/v1) hold their words
 * equal on both platforms.
 */
describe('attachment rules', () => {
  it('reads the kind from the source', () => {
    expect(attachmentKind({ name: 'a', asset: 'resim-1' })).toBe('photo');
    expect(attachmentKind({ name: 'a', file: 'foto/A.JPG' })).toBe('photo');
    expect(attachmentKind({ name: 'a', file: 'belge.pdf' })).toBe('document');
    expect(attachmentKind({ name: 'a', url: 'https://a.org/f.png?x=1' })).toBe('photo');
    expect(attachmentKind({ name: 'a', url: 'https://a.org/sayfa' })).toBe('link');
  });

  it('keeps one source, a name and an http address', () => {
    expect(attachmentsProblem([{ name: 'Tapu', file: 'a.pdf', note: 'iki\nsatır' }])).toBeNull();
    expect(attachmentsProblem([{ name: 'Tapu' }])).toMatch(/kaynağı tek olmalı/);
    // White space is Rust's (Unicode White_Space): U+FEFF is not one, as it is for JavaScript's trim.
    expect(attachmentsProblem([{ name: '\ufeff', file: 'a.pdf' }])).toBeNull();
    expect(attachmentsProblem([{ name: '\u3000', file: 'a.pdf' }])).toBe('ekin adı boş');
    expect(attachmentsProblem([{ name: 'a\tb', file: 'a.pdf' }])).toMatch(/denetim karakteri/);
    expect(attachmentsProblem([{ name: 'Tapu', url: 'ftp://a.org' }])).toMatch(/http/);
    expect(isWebAddress('https://')).toBe(false);
    expect(isWebAddress('HTTPS://a')).toBe(true);
  });
});

describe('action and card rules', () => {
  it('names actions once whatever their case, each with a target', () => {
    expect(actionsProblem([{ name: 'Sorgu', target: 'https://a.org' }])).toBeNull();
    expect(actionsProblem([{ name: 'Sorgu', target: 'a' }, { name: 'SORGU', target: 'b' }])).toBe('“SORGU” adlı iki eylem var');
    expect(actionsProblem([{ name: 'Sorgu', target: ' ' }])).toBe('“Sorgu” eyleminin hedefi boş');
  });

  it('keeps a card to its rules and writes its defaults away', () => {
    expect(cardProblem({ title: 'a\nb' })).toMatch(/başlığı/);
    expect(cardProblem({ fields: ['A', 'A'] })).toBe('kart “A” alanını iki kez gösteriyor');
    expect(cardOf({ title: 'T', fields: [], attachments: true, actions: false })).toEqual({ title: 'T', actions: false });
  });

  it('puts actions and cards only on layers that hold objects', () => {
    const node = (type: 'group' | 'layer', extra: Partial<LayerNode> = {}): LayerNode => ({
      id: 'x',
      name: 'X',
      type,
      visible: true,
      locked: false,
      expanded: true,
      style: { color: 'fg', lineType: 'continuous', lineWeight: 0.25 },
      children: [],
      ...extra,
    });
    expect(infosProblem([node('layer', { card: {} })])).toBeNull();
    expect(infosProblem([node('group', { card: {} })])).toMatch(/yalnız nesneleri olan katmanda/);
    expect(infosProblem([node('layer', { actions: [{ name: 'A', target: 'b' }], service: { kind: 'xyz', url: 'https://t/{z}/{x}/{y}.png' } as LayerNode['service'] })])).toMatch(/yalnız nesneleri olan katmanda/);
  });
});
