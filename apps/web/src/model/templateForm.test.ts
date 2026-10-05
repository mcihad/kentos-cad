import { describe, expect, it } from 'vitest';
import type { ObjectTemplate } from './objectTemplate';
import { formOfTemplate, templateFromForm, type TemplateForm } from './templateForm';

/**
 * The Şablon düzenleyici's form rules (docs/adr/0176 §4) as fixtures/style/v1/template-form.json has them, written from
 * the rules by scripts/fixtures/template_form_cases.py (the desktop's `kentos_native_style::template_form` runs the
 * same cases).
 */

interface Item {
  name: string;
  path: string[];
  description?: string;
  template: ObjectTemplate;
}

const files = import.meta.glob<string>('../../../../fixtures/style/v1/template-form.json', { query: '?raw', import: 'default', eager: true });
const fixture = JSON.parse(Object.values(files)[0]) as {
  format: string;
  version: number;
  toTemplate: { name: string; form: TemplateForm; result: unknown }[];
  toForm: { name: string; item: Item; form: TemplateForm }[];
};

describe('the template form (fixtures/style/v1/template-form.json)', () => {
  it('is a template-form-cases v1 file', () => {
    expect([fixture.format, fixture.version]).toEqual(['kentos.template-form-cases', 1]);
    expect(fixture.toTemplate.length).toBeGreaterThanOrEqual(15);
  });
  for (const c of fixture.toTemplate) it(`form → template: ${c.name}`, () => expect(templateFromForm(c.form)).toEqual(c.result));
  for (const c of fixture.toForm)
    it(`template → form: ${c.name}`, () => {
      const form = formOfTemplate(c.item);
      expect(form).toEqual(c.form);
      // And back: the form makes what it was made from.
      const back = templateFromForm(form);
      expect('template' in back && back.template).toEqual({ ...c.item.template, ...(c.item.template.attrs && { attrs: Object.fromEntries(Object.entries(c.item.template.attrs).sort(([a], [b]) => (a < b ? -1 : 1))) }) });
    });
});
