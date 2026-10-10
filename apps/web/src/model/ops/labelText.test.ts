import { describe, expect, it } from 'vitest';
import { fillTemplate } from './labelText';

/** A label's template as the core fills it (docs/adr/0175 §1); the placing's cases are viewport/labelEngine.test.ts'. */
describe('Etiketleri yazıya çevir', () => {
  it('fills a template as the core does: the first {label}, literally', () => {
    expect(fillTemplate(undefined, '101')).toBe('101');
    expect(fillTemplate('', '101')).toBe('101');
    expect(fillTemplate('No: {label}', '101')).toBe('No: 101');
    expect(fillTemplate('{label}/{label}', '7')).toBe('7/{label}');
    expect(fillTemplate('Ada', '7')).toBe('Ada');
    // String.replace would write the template's text for `$&`.
    expect(fillTemplate('[{label}]', 'a$&b')).toBe('[a$&b]');
  });
});
