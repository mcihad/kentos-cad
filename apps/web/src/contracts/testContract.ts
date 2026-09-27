/**
 * The contract as ts-rs writes it from the Rust types (crates/shared/contracts
 * → ./generated), read as text, for tests. The desktop reads the shared
 * fixture files into the contract's own types, strictly: a value or a field
 * the contract does not have fails there. Holding a file's values to the
 * generated text here makes the web fail on the same value, against the Rust
 * types rather than a copy of them.
 */

const CONTRACT = import.meta.glob<string>('./generated/*.ts', { query: '?raw', import: 'default', eager: true });

function contractText(type: string): string {
  const text = CONTRACT[`./generated/${type}.ts`];
  if (text === undefined) throw new Error(`contracts/generated/${type}.ts okunamadı`);
  // Doc comments carry words, not fields.
  return text.replace(/\/\*\*[\s\S]*?\*\//g, '');
}

/** A string union's members: `export type X = "a" | "b";`. */
export function membersOf(type: string): string[] {
  const body = /export type \w+ = ([^;]+);/.exec(contractText(type))?.[1] ?? '';
  const out = [...body.matchAll(/"([^"]*)"/g)].map((m) => m[1]);
  if (!out.length) throw new Error(`${type} bir metin birleşimi değil`);
  return out;
}

/** A flat object type's fields, name → optional: `export type X = { a: …, b?: … };`. */
export function fieldsOf(type: string): Map<string, boolean> {
  const text = contractText(type);
  const body = text.slice(text.indexOf('= {') + 3, text.lastIndexOf('}'));
  const out = new Map([...body.matchAll(/(?:^|[,{\s])(\w+)(\?)?\s*:/g)].map((m) => [m[1], m[2] === '?']));
  if (!out.size) throw new Error(`${type} bir nesne türü değil`);
  return out;
}

/** What a strict reader of `type` refuses in `value`'s fields: one the type has not, one it needs and is missing. */
export function fieldProblems(value: object, type: string): string[] {
  const fields = fieldsOf(type);
  return [
    ...Object.keys(value)
      .filter((k) => !fields.has(k))
      .map((k) => `${type} has no field “${k}”`),
    ...[...fields].filter(([k, optional]) => !optional && !(k in value)).map(([k]) => `${type} needs “${k}”`),
  ];
}

/** Whether `value` is one of a string union's members; the problem said, or null. */
export function memberProblem(value: unknown, type: string): string | null {
  const members = membersOf(type);
  return members.includes(value as string) ? null : `“${String(value)}” is not a ${type} (${members.join(', ')})`;
}
