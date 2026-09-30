/**
 * Metin dosyası yerleştir's file (docs/adr/0145 §6): its lines, each to become a text. A file of more than 1 MB, one
 * that is not UTF-8, one of more than 10 000 lines or one of nothing but empty lines is refused, and why is said.
 * Lines end at \r\n, \n or \r; a last line break ends the last line; a byte order mark is no letter; each line is
 * trimmed, and an empty one keeps its place and becomes no text. The desktop's is
 * `kentos_interaction::text_file::lines`; fixtures/text/v1/file.json holds both to the rule
 * (scripts/fixtures/text_cases.py).
 */
export const MAX_TEXT_FILE_BYTES = 1024 * 1024;
export const MAX_TEXT_FILE_LINES = 10_000;

export type TextFileRefusal = 'tooBig' | 'notUtf8' | 'tooMany' | 'empty';

export type TextFileLines = { ok: true; lines: string[] } | { ok: false; kind: TextFileRefusal; error: string };

export function textFileLines(name: string, bytes: Uint8Array): TextFileLines {
  if (bytes.length > MAX_TEXT_FILE_BYTES)
    return {
      ok: false,
      kind: 'tooBig',
      error: `“${name}” 1 MB'tan büyük (${(bytes.length / MAX_TEXT_FILE_BYTES).toFixed(1)} MB); en çok 1 MB okunur. Dosyayı bölüp yeniden deneyin.`,
    };
  let text: string;
  try {
    // The decoder takes a byte order mark off.
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return { ok: false, kind: 'notUtf8', error: `“${name}” UTF-8 değil. Dosyayı UTF-8 olarak kaydedip yeniden deneyin.` };
  }
  const lines = text.split(/\r\n|\n|\r/);
  if (lines.at(-1) === '') lines.pop();
  if (lines.length > MAX_TEXT_FILE_LINES)
    return { ok: false, kind: 'tooMany', error: `“${name}” ${lines.length} satır; en çok ${MAX_TEXT_FILE_LINES} satır yerleştirilir. Dosyayı bölüp yeniden deneyin.` };
  const trimmed = lines.map((l) => l.trim());
  if (!trimmed.some(Boolean)) return { ok: false, kind: 'empty', error: `“${name}” boş: yerleştirilecek satır yok.` };
  return { ok: true, lines: trimmed };
}
