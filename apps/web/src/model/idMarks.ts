// Ids of objects as a growing byte map (docs/adr/0211 §6).

/**
 * A set of objects' ids as a growing byte map (docs/adr/0211 §6): the document gives ids in turn from 1, so a filter's
 * marks over a large layer are array reads and writes, no hashing. An id beyond the map's reach goes to a set.
 */
export class IdMarks {
  private static readonly REACH = 1 << 24;
  private bytes = new Uint8Array(1024);
  private readonly far = new Set<number>();

  has(id: number): boolean {
    return id >= 0 && id < this.bytes.length ? this.bytes[id] === 1 : this.far.has(id);
  }

  add(id: number): void {
    if (!Number.isInteger(id) || id < 0 || id >= IdMarks.REACH) return void this.far.add(id);
    if (id >= this.bytes.length) {
      let n = this.bytes.length;
      while (n <= id) n *= 2;
      const grown = new Uint8Array(n);
      grown.set(this.bytes);
      this.bytes = grown;
    }
    this.bytes[id] = 1;
  }

  delete(id: number): void {
    if (id >= 0 && id < this.bytes.length) this.bytes[id] = 0;
    else this.far.delete(id);
  }

  clear(): void {
    this.bytes.fill(0);
    this.far.clear();
  }
}
