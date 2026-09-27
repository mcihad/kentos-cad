import { describe, expect, it } from 'vitest';
import { deedAreaText } from './HoverCard';

/** The hover card's registered area: the deed's value as written, never parsed or rounded. */
describe('the hover card’s Tapu alanı', () => {
  it('shows the value as written, with m² after a plain decimal number, a comma or a point', () => {
    expect(deedAreaText({ 'Tapu alanı (m²)': '723,52' })).toBe('723,52 m²');
    expect(deedAreaText({ 'Tapu alanı (m²)': ' 723.525 ' })).toBe('723.525 m²');
    expect(deedAreaText({ 'Tapu alanı (m²)': '1200' })).toBe('1200 m²');
  });

  it('shows other text as it is, and nothing when the attribute is empty or absent', () => {
    expect(deedAreaText({ 'Tapu alanı (m²)': 'tapuda yok' })).toBe('tapuda yok');
    expect(deedAreaText({ 'Tapu alanı (m²)': '723abc' })).toBe('723abc');
    expect(deedAreaText({ 'Tapu alanı (m²)': '  ' })).toBeNull();
    expect(deedAreaText({})).toBeNull();
  });
});
