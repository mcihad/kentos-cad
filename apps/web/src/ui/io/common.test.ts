import { describe, expect, it } from 'vitest';
import { reportText } from './common';

describe('reportText', () => {
  it('reads as the desktop’s sentences: a count and the first lines, none for a note about the whole file', () => {
    expect(reportText({ what: 'IMAGE', count: 3, reason: 'raster görüntüler alınmaz', lines: [120, 488] })).toBe('IMAGE: 3, raster görüntüler alınmaz (satır 120, 488 …).');
    expect(reportText({ what: 'IMAGE', count: 1, reason: 'raster görüntüler alınmaz', lines: [] })).toBe('IMAGE: 1, raster görüntüler alınmaz.');
    // Its unit (docs/adr/0165 §2).
    expect(reportText({ what: 'Birim', count: 0, reason: 'dosya inç biriminde; değerler çizimin birimine, milimetreye çevrildi', lines: [] })).toBe(
      'Birim: dosya inç biriminde; değerler çizimin birimine, milimetreye çevrildi.',
    );
  });
});
