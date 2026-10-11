import type { Attachment } from '../contracts/generated/Attachment';

/**
 * Nesne ekleri's rules (docs/adr/0225 §1), the web's copy of `kentos_contracts::attachment`: the readers', the
 * commands' and the server's words. Whether an embedded photo's asset is in the project's library is the command's.
 */

/** The most attachments an object carries. */
export const MAX_ATTACHMENTS = 64;
/** The longest name, and the longest asset id, in characters. */
export const MAX_ATTACHMENT_NAME = 256;
/** The longest note, in characters. */
export const MAX_ATTACHMENT_NOTE = 1_000;
/** The longest path or address, in characters. */
export const MAX_ATTACHMENT_LINK = 4_096;

/** What an attachment is, by its source: an embedded photo or a JPEG's or PNG's file or address, another file, another address. */
export type AttachmentKind = 'photo' | 'document' | 'link';

const CONTROL = /\p{Cc}/u;
const CONTROL_BUT_LINE = /[^\P{Cc}\n]/u;
const chars = (s: string): number => [...s].length;
/** Rust's `trim` (the White_Space property; JavaScript's `trim` takes U+FEFF too). */
const trim = (s: string): string => s.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');

/** A link's path without its query or fragment, lower case: what its kind is read from. */
function pathPart(link: string): string {
  const q = link.search(/[?#]/);
  return (q < 0 ? link : link.slice(0, q)).toLowerCase();
}

/** Whether a path or address names a photo: it ends in `.jpg`, `.jpeg` or `.png`. */
export function isPhotoLink(link: string): boolean {
  const p = pathPart(link);
  return p.endsWith('.jpg') || p.endsWith('.jpeg') || p.endsWith('.png');
}

/** Whether an address is one an attachment may be: `http://` or `https://` (the scheme in any case), then a character. */
export function isWebAddress(url: string): boolean {
  const lower = url.toLowerCase();
  const bytes = new TextEncoder().encode(url).length;
  return (lower.startsWith('http://') && bytes > 7) || (lower.startsWith('https://') && bytes > 8);
}

/** What it is, by its source (`Attachment::kind`). */
export function attachmentKind(a: Attachment): AttachmentKind {
  if (a.asset != null) return 'photo';
  if (a.file != null) return isPhotoLink(a.file) ? 'photo' : 'document';
  if (a.url != null && isPhotoLink(a.url)) return 'photo';
  return 'link';
}

/** What is wrong with one attachment, when anything is (`Attachment::problem`). */
export function attachmentProblem(a: Attachment): string | null {
  const name = a.name;
  if (!trim(name)) return 'ekin adı boş';
  if (chars(name) > MAX_ATTACHMENT_NAME) return `ekin adı ${MAX_ATTACHMENT_NAME} karakterden uzun`;
  if (CONTROL.test(name)) return `“${trim(name)}” ekinin adında satır sonu ya da denetim karakteri var`;
  const sources = [a.asset, a.file, a.url].filter((s) => s != null).length;
  if (sources !== 1) return `“${name}” ekinin kaynağı tek olmalı: gömülü varlık (asset), bağlı dosya (file) ya da adres (url)`;
  if (a.asset != null && (!trim(a.asset) || chars(a.asset) > MAX_ATTACHMENT_NAME || CONTROL.test(a.asset))) return `“${name}” ekinin varlık kimliği geçersiz`;
  for (const link of [a.file, a.url]) {
    if (link == null) continue;
    if (!trim(link)) return `“${name}” ekinin yolu ya da adresi boş`;
    if (chars(link) > MAX_ATTACHMENT_LINK) return `“${name}” ekinin yolu ya da adresi ${MAX_ATTACHMENT_LINK} karakterden uzun`;
    if (CONTROL.test(link)) return `“${name}” ekinin yolunda ya da adresinde denetim karakteri var`;
  }
  if (a.url != null && (!isWebAddress(a.url) || /\p{White_Space}/u.test(a.url))) return `“${name}” ekinin adresi http:// ya da https:// ile başlamalı ve boşluk içermemeli`;
  if (a.note != null) {
    if (chars(a.note) > MAX_ATTACHMENT_NOTE) return `“${name}” ekinin notu ${MAX_ATTACHMENT_NOTE} karakterden uzun`;
    if (CONTROL_BUT_LINE.test(a.note)) return `“${name}” ekinin notunda denetim karakteri var (yalnız satır sonu olur)`;
  }
  return null;
}

/** What is wrong with an object's attachments, when anything is (`attachments_problem`): too many, or one of them. */
export function attachmentsProblem(list: readonly Attachment[]): string | null {
  if (list.length > MAX_ATTACHMENTS) return `nesnenin ${list.length} eki var; en çok ${MAX_ATTACHMENTS}`;
  for (const a of list) {
    const p = attachmentProblem(a);
    if (p) return p;
  }
  return null;
}

/** An attachment as an object keeps it: a copy with only the fields it has, in the contract's order. */
export function attachmentOf(a: Attachment): Attachment {
  return {
    name: a.name,
    ...(a.asset != null && { asset: a.asset }),
    ...(a.file != null && { file: a.file }),
    ...(a.url != null && { url: a.url }),
    ...(a.note != null && { note: a.note }),
  };
}
