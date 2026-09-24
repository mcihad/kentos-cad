import { h, type Child } from '../dom';
import { Dialog } from './Dialog';

/**
 * The one way the app asks before it goes on (DESIGN.md §7.9.1): a small
 * window over the one that asked (it stacks, so every answer but the
 * leaving one returns there), a title, a sentence or two, and the answers
 * as buttons. Esc, × and the backdrop give the answer that changes
 * nothing. Nothing asks in a status line or a menu: a question there is
 * easy to miss and sits among the window's own buttons.
 */

export interface ConfirmAnswer<T extends string> {
  value: T;
  label: string;
  /** `primary`: the one amber button. `danger`: removes something (outlined, red text; never amber). */
  kind?: 'primary' | 'danger';
  /** On the left of the bar, apart from the others (the secondary answer, as in every window). */
  aside?: boolean;
}

export interface ConfirmOptions<T extends string> {
  title: string;
  message: Child;
  /** What will happen, point by point. */
  details?: readonly Child[];
  answers: readonly ConfirmAnswer<T>[];
  /** The answer of Esc, × and the backdrop: the one that changes nothing. */
  cancel: T;
  /** The answer whose button has the focus (Enter): the primary one when there is one, else `cancel`. */
  focus?: T;
}

let serial = 0;

export function confirmDialog<T extends string>(opts: ConfirmOptions<T>): Promise<T> {
  return new Promise((resolve) => {
    let answered = false;
    const answer = (v: T) => {
      if (answered) return;
      answered = true;
      dialog.close();
      resolve(v);
    };
    const buttons = opts.answers.map((a) => {
      const b = h('button', { class: `btn${a.kind ? ` btn--${a.kind}` : ''}`, type: 'button' }, a.label);
      b.addEventListener('click', () => answer(a.value));
      return { a, b };
    });
    const id = `confirm-${++serial}`;
    // No fixed width: the window is as wide as its answers need (controls.css), and the text wraps inside.
    const dialog = new Dialog({
      title: opts.title,
      className: 'dialog--confirm',
      stack: true,
      content: [
        h('p', { class: 'confirm__message', id }, opts.message),
        opts.details?.length ? h('ul', { class: 'confirm__details' }, opts.details.map((d) => h('li', null, d))) : null,
      ],
      footer: [...buttons.filter((x) => x.a.aside).map((x) => x.b), h('div', { class: 'dialog__spacer' }), ...buttons.filter((x) => !x.a.aside).map((x) => x.b)],
      onClose: () => answer(opts.cancel),
    });
    // A question, read out with its message.
    const card = dialog.el.querySelector('.dialog');
    card?.setAttribute('role', 'alertdialog');
    card?.setAttribute('aria-describedby', id);
    const focus = opts.focus ?? opts.answers.find((a) => a.kind === 'primary')?.value ?? opts.cancel;
    buttons.find((x) => x.a.value === focus)?.b.focus();
  });
}

export type UnsavedAnswer = 'save' | 'discard' | 'stay';

/**
 * “Kaydedilmemiş değişiklikler”: save and go on (Enter), go on without
 * saving (on the left, never the default), or stay (Esc).
 */
export function askUnsaved(opts: {
  /** What holds the changes (a drawing, a symbol, a model). */
  name: string;
  /** What leaving does to them: "Pencere kapanırsa bu değişiklikler kaybolur." */
  after: string;
  /** The verb of the two leaving answers: "kapat" gives "Kaydet ve kapat", "Kaydetmeden kapat". */
  verb: string;
  /** Changes that are applied rather than saved (a symbol inside a layer style). */
  apply?: boolean;
  /** False when saving cannot go on from here: only "…madan" and Vazgeç. */
  canSave?: boolean;
}): Promise<UnsavedAnswer> {
  const [save, without, state] = opts.apply ? ['Uygula', 'Uygulamadan', 'uygulanmamış'] : ['Kaydet', 'Kaydetmeden', 'kaydedilmemiş'];
  const answers: ConfirmAnswer<UnsavedAnswer>[] = [
    { value: 'discard', label: `${without} ${opts.verb}`, aside: true },
    { value: 'stay', label: 'Vazgeç' },
  ];
  if (opts.canSave !== false) answers.push({ value: 'save', label: `${save} ve ${opts.verb}`, kind: 'primary' });
  return confirmDialog({
    title: opts.apply ? 'Uygulanmamış değişiklikler' : 'Kaydedilmemiş değişiklikler',
    message: `“${opts.name}” içinde ${state} değişiklikler var. ${opts.after}`,
    answers,
    cancel: 'stay',
  });
}

/** Asks before removing something: Vazgeç has the focus, the removing button is marked, not amber. */
export async function askRemove(opts: { title: string; message: Child; details?: readonly Child[]; action: string }): Promise<boolean> {
  const a = await confirmDialog({
    title: opts.title,
    message: opts.message,
    details: opts.details,
    answers: [
      { value: 'stay', label: 'Vazgeç' },
      { value: 'remove', label: opts.action, kind: 'danger' },
    ],
    cancel: 'stay',
  });
  return a === 'remove';
}
