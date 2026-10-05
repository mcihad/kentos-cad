import { describe, expect, it } from 'vitest';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import type { LibraryItem } from '../model/style';
import { createStyles } from './styles';

/**
 * The style library's project part follows the drawing (as the desktop's `Styles::follow_project`): a drawing opened
 * or a change taken in brings its own symbols and object templates (docs/adr/0176 §2), and what the library writes
 * into the drawing is not read back as a change.
 */

const TEMPLATE: LibraryItem = { kind: 'template', id: 'p-parsel', name: 'Parsel sınırı', path: ['Şablonlar'], template: { tool: 'polygon', layer: { path: ['Kadastro'], name: 'Parsel' } } };

describe('the project part of the style library', () => {
  it('follows the drawing’s own library when it changes', () => {
    const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
    const { library } = createStyles(doc, { items: [] });
    expect(library.template('p-parsel')).toBeUndefined();
    doc.styles.set({ items: [TEMPLATE], categories: [{ path: ['Şablonlar'] }] });
    expect(library.template('p-parsel')?.source).toBe('project');
    expect(library.template('p-parsel')?.template.layer.name).toBe('Parsel');
    // Another drawing without one: it goes.
    doc.styles.set({ items: [], categories: [] });
    expect(library.template('p-parsel')).toBeUndefined();
  });

  it('writes its own edits into the drawing, not back into itself', () => {
    const doc = new CadDocument({ name: 'Deneme', layers: new LayerStore([{ id: 'a', name: 'A' }], 'a'), origin: { x: 0, y: 0 } });
    const { library } = createStyles(doc, { items: [] });
    const version = library.version.value;
    library.add('project', TEMPLATE);
    expect(doc.styles.value.items.map((i) => i.id)).toEqual(['p-parsel']);
    // One change: the add itself, no reload after it.
    expect(library.version.value).toBe(version + 1);
  });
});
