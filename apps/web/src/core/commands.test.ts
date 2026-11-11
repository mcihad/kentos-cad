import { describe, expect, it } from 'vitest';
import { CommandRegistry } from './commands';

describe('CommandRegistry.search', () => {
  // An alias equal first, then a title equal, then an alias beginning with the text (the shorter first), then a title
  // beginning with it, then one holding it.
  it('puts a title equal to the text before an alias that only begins with it', () => {
    const r = new CommandRegistry();
    const run = () => undefined;
    r.register({ id: 'cad.sections.edit', title: 'Kesit çıkar', aliases: ['KESITCIKAR'], run });
    r.register({ id: 'tool.viewSection', title: 'Kesit', aliases: ['KESITISARETI', 'SECTIONMARK'], run });
    r.register({ id: 'tool.section', title: 'Kesit takımı', run });
    r.register({ id: 'tool.profile', title: 'Boy kesit', aliases: ['KESIT'], run });
    expect(r.search('kesit').map((c) => c.id)).toEqual([
      'tool.profile',
      'tool.viewSection',
      'cad.sections.edit',
      'tool.section',
    ]);
    expect(r.search('KESİT ÇIKAR').map((c) => c.id)).toEqual(['cad.sections.edit']);
  });
});
