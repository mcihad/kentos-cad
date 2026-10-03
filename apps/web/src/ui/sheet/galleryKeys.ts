/**
 * The template gallery's cards from the keyboard (TemplateGallery.ts): the
 * arrows move among them as they are laid out (rows by their place on the
 * screen), Enter uses the one in focus, Delete deletes it (the gallery asks
 * first and says why it cannot).
 */
export function onGalleryKey(grid: HTMLElement, e: KeyboardEvent, act: { choose(id: string): void; use(): void; remove(): void }): void {
  const cards = [...grid.querySelectorAll<HTMLElement>('.tcard')];
  const at = cards.indexOf(document.activeElement as HTMLElement);
  if (at < 0) return;
  const top = cards[at].offsetTop;
  const perRow = cards.filter((x) => x.offsetTop === top).length || 1;
  const to = e.key === 'ArrowRight' ? at + 1 : e.key === 'ArrowLeft' ? at - 1 : e.key === 'ArrowDown' ? at + perRow : e.key === 'ArrowUp' ? at - perRow : -1;
  if (to !== -1 && e.key.startsWith('Arrow')) {
    e.preventDefault();
    const next = cards[Math.max(0, Math.min(cards.length - 1, to))];
    next.focus();
    act.choose(next.dataset.id!);
  } else if (e.key === 'Enter') {
    e.preventDefault();
    act.choose(cards[at].dataset.id!);
    act.use();
  } else if (e.key === 'Delete') {
    e.preventDefault();
    act.choose(cards[at].dataset.id!);
    act.remove();
  }
}
