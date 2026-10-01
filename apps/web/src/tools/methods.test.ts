import { describe, expect, it } from 'vitest';
import { methodByAlias } from './methods';

/** A tool's method by its own typed name (docs/adr/0147 §7), as the command line starts it. */
describe('methods by their typed names', () => {
  it('finds Ölçülendirme’s methods by their names, in any case, and nothing for the tool’s own', () => {
    expect(methodByAlias('DOR')).toMatchObject({ command: 'tool.dimension', option: 'O', label: 'Koordinat' });
    expect(methodByAlias('dimordinate')).toMatchObject({ option: 'O' });
    expect(methodByAlias('KOORDINATOLCU')).toMatchObject({ option: 'O' });
    expect(methodByAlias('DAR')).toMatchObject({ command: 'tool.dimension', option: 'U', label: 'Yay uzunluğu' });
    expect(methodByAlias('yayuzunlugu')).toMatchObject({ option: 'U' });
    expect(methodByAlias('DIMLIN')).toMatchObject({ option: 'D' });
    expect(methodByAlias('DIMRAD')).toMatchObject({ option: 'R' });
    expect(methodByAlias('DAL')).toMatchObject({ option: 'H' });
    expect(methodByAlias('DIM')).toBeUndefined();
    expect(methodByAlias('CIRCLE')).toBeUndefined();
  });
});
