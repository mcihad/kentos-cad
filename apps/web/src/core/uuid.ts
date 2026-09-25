/**
 * UUIDs as KentOS writes them (RFC 9562): lowercase, with hyphens. A new
 * object gets a version 7 id (docs/adr/0014): 48 bits of Unix time in
 * milliseconds, 12 bits that count up within the millisecond (RFC 9562
 * §6.2, method 1), then 62 random bits. So the ids one page makes sort in
 * the order they were made, which keeps the server's B-tree index compact,
 * and the random bits make a clash with another device practically
 * impossible. The counter starts each millisecond at a random value below
 * 2048; a millisecond that runs out of values borrows the next one, and a
 * clock that steps back counts on from where it was, so the order holds.
 */

const DIGITS = [...'0123456789abcdef'].map((c) => c.charCodeAt(0));
/** Where each of the 16 bytes' two digits go in the 36 characters. */
const AT = [0, 2, 4, 6, 9, 11, 14, 16, 19, 21, 24, 26, 28, 30, 32, 34];
/**
 * The text being made, as character codes: made into a string in one call,
 * so the id is one flat string (a bulk load puts 10⁵ of them into the
 * drawing's index, and joined pieces would each be flattened there first).
 */
const text = new Uint16Array(36);
text[8] = text[13] = text[18] = text[23] = 0x2d;

function put(i: number, byte: number): void {
  text[AT[i]] = DIGITS[(byte >>> 4) & 15];
  text[AT[i] + 1] = DIGITS[byte & 15];
}

/** Random bytes, drawn 4 KB at a time: one call to the system generator per ~400 ids. */
const pool = new Uint8Array(4096);
let drawn = pool.length;

/** Where `n` fresh random bytes start in `pool`. */
function take(n: number): number {
  if (drawn + n > pool.length) {
    crypto.getRandomValues(pool);
    drawn = 0;
  }
  const at = drawn;
  drawn += n;
  return at;
}

function seed(): number {
  const r = take(2);
  return ((pool[r] << 8) | pool[r + 1]) & 0x7ff;
}

/** A maker of UUIDv7s with its own clock and counter (tests give it a clock; the app has `uuidv7`). */
export function uuidv7Source(clock: () => number = Date.now): () => string {
  let last = -Infinity;
  let counter = 0;
  return () => {
    let ms = Math.floor(clock());
    if (ms > last) counter = seed();
    else {
      ms = last;
      if (++counter > 0xfff) {
        ms++;
        counter = seed();
      }
    }
    last = ms;
    const hi = Math.floor(ms / 0x100000000);
    const lo = ms >>> 0;
    put(0, hi >>> 8);
    put(1, hi);
    put(2, lo >>> 24);
    put(3, lo >>> 16);
    put(4, lo >>> 8);
    put(5, lo);
    put(6, 0x70 | (counter >>> 8));
    put(7, counter);
    const r = take(8);
    put(8, 0x80 | (pool[r] & 0x3f));
    for (let i = 1; i < 8; i++) put(8 + i, pool[r + i]);
    return String.fromCharCode.apply(null, text as unknown as number[]);
  };
}

/** A new UUIDv7, in creation order within this page. */
export const uuidv7 = uuidv7Source();

const TEXT = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

/** Whether `v` is a UUID as KentOS writes it, lowercase with hyphens. Any version: no rule depends on it (ADR 0014). */
export const isUuid = (v: unknown): v is string => typeof v === 'string' && TEXT.test(v);
