import type { AppContext } from '../../app/context';
import type { Command } from '../../core/commands';
import { listen } from '../../core/disposable';
import { formatChord, isTextInput } from '../../core/keymap';
import { looksLikeCoordinate } from '../../tools/coordinateInput';
import { CALC_KINDS, canCalcPoint, startPointCalc } from '../../tools/pointCalc';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { optionButtons, optionForKey, parsePrompt, runPromptOption } from '../promptOptions';

/**
 * AutoCAD/Netcad-style command line. Accepts command aliases (L, PL,
 * PARSEL…), coordinates and tool options. Digits typed anywhere on the
 * drawing jump here so coordinates can be entered without clicking.
 */
export class CommandLine extends Component {
  readonly el: HTMLElement;
  readonly input: HTMLInputElement;
  private readonly prompt: HTMLElement;
  private readonly list: HTMLElement;
  private suggestions: Command[] = [];
  private active = -1;
  private history: string[] = [];
  private historyIndex = -1;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.prompt = h('span', { class: 'cmdline__prompt' });
    this.input = h('input', {
      class: 'cmdline__input',
      type: 'text',
      spellcheck: 'false',
      autocomplete: 'off',
      'aria-label': 'Komut satırı',
      'aria-autocomplete': 'list',
      'aria-controls': 'cmd-suggest',
      placeholder: 'Komut ya da koordinat yazın. Boşluk tuşu buraya getirir',
    });
    this.list = h('ul', { class: 'cmdline__suggest', id: 'cmd-suggest', role: 'listbox', hidden: true });
    this.el = h(
      'div',
      { class: 'cmdline' },
      h('span', { class: 'cmdline__icon' }, icon('terminal', 16)),
      this.prompt,
      h('div', { class: 'cmdline__field' }, this.input, this.list),
    );

    this.d.add(ctx.tools.prompt.subscribe((p) => this.setPrompt(p), true));
    this.d.add(listen(this.input, 'input', () => this.suggest()));
    this.d.add(listen<KeyboardEvent>(this.input, 'keydown', (e) => this.onKey(e)));
    this.d.add(listen(this.input, 'blur', () => setTimeout(() => this.hideList(), 120)));

    // Route coordinate-looking keystrokes from the drawing into the field.
    // The browser then types the key into the field it focused: the first
    // character arrives exactly once. + and − reach here only while a
    // command runs (their zoom bindings wait for the select tool).
    ctx.keymap.fallback = (e) => {
      if (e.ctrlKey || e.altKey || e.metaKey || e.key.length !== 1) return;
      if (isTextInput(document.activeElement)) return;
      if (!/[\d@.+-]/.test(e.key)) return;
      if (this.direct?.accepts()) this.direct.show();
      else this.focus();
    };
    this.d.add(() => (ctx.keymap.fallback = null));

    // While a command runs, a plain letter that is one of its options (the
    // key shown on the option buttons) triggers it at once — one keystroke,
    // no Space or Enter — and wins over the tool shortcut with that letter.
    ctx.keymap.intercept = (e) => {
      if (e.ctrlKey || e.altKey || e.metaKey || e.shiftKey || e.key.length !== 1) return false;
      if (isTextInput(document.activeElement)) return false;
      if (ctx.tools.activeId.value === 'select' && !ctx.tools.nested) return false;
      const letter = e.key.toLocaleUpperCase('tr-TR');
      if (!/\p{L}/u.test(letter)) return false;
      const opt = optionForKey(parsePrompt(ctx.tools.prompt.value).options, letter);
      if (!opt) return false;
      runPromptOption(ctx, opt.key);
      return true;
    };
    this.d.add(() => (ctx.keymap.intercept = null));
  }

  /** Field beside the cursor that takes typed values while the mouse is on the drawing. */
  direct: { accepts(): boolean; show(): void } | null = null;

  focus(): void {
    this.input.focus();
  }

  private setPrompt(prompt: string): void {
    const p = parsePrompt(prompt);
    if (!p.tool) return replaceChildren(this.prompt, h('b', null, p.step));
    const notes = p.notes.length ? ` (${p.notes.join('; ')})` : '';
    // Options are buttons here too; typing their letter still works.
    replaceChildren(this.prompt, h('span', { class: 'cmdline__text' }, h('b', null, p.tool), `: ${p.step}${notes}`), ...optionButtons(this.ctx, p.options, 'cmdline__chip'));
  }

  private suggest(): void {
    const text = this.input.value.trim();
    this.historyIndex = -1;
    if (!text || looksLikeCoordinate(text) || this.ctx.tools.activeId.value !== 'select') return this.hideList();
    this.suggestions = this.ctx.commands.search(text, 7);
    this.active = this.suggestions.length ? 0 : -1;
    this.renderList();
  }

  private renderList(): void {
    if (!this.suggestions.length) return this.hideList();
    this.list.hidden = false;
    replaceChildren(
      this.list,
      this.suggestions.map((c, i) => {
        const chord = this.ctx.keymap.chordFor(c.id);
        const li = h(
          'li',
          { class: 'cmdline__opt', role: 'option', 'aria-selected': String(i === this.active) },
          h('span', { class: 'cmdline__opt-icon' }, c.icon ? icon(c.icon, 16) : null),
          h('span', { class: 'cmdline__opt-title' }, c.title),
          c.aliases?.length ? h('span', { class: 'cmdline__opt-alias' }, c.aliases[0]) : null,
          chord ? h('kbd', { class: 'kbd' }, formatChord(chord)) : null,
        );
        li.addEventListener('pointerdown', (e) => {
          e.preventDefault();
          this.run(c);
        });
        return li;
      }),
    );
  }

  private hideList(): void {
    this.list.hidden = true;
    this.suggestions = [];
    this.active = -1;
  }

  private run(c: Command): void {
    this.remember(this.input.value.trim() || c.aliases?.[0] || c.title);
    this.input.value = '';
    this.hideList();
    this.ctx.commands.execute(c.id);
    if (c.id.startsWith('tool.')) this.ctx.view.focus();
  }

  private remember(text: string): void {
    if (text && this.history.at(-1) !== text) this.history.push(text);
    this.historyIndex = -1;
  }

  private submit(): void {
    const text = this.input.value.trim();
    const { tools, commands, log } = this.ctx;
    if (!text) {
      commands.execute('tool.confirm');
      return;
    }
    if (this.active >= 0 && this.suggestions[this.active]) return this.run(this.suggestions[this.active]);
    const tool = tools.active;
    if (tools.activeId.value !== 'select' && tool.input) {
      log.command(`› ${text}`);
      this.remember(text);
      this.input.value = '';
      if (tool.input(text)) return;
      // Point calculator by its alias (YAN, KKES, DKES, HAT, AM, ORTA) while a point is expected.
      const calc = CALC_KINDS.find((k) => k.alias === text.toLocaleUpperCase('tr-TR'));
      if (calc && canCalcPoint(this.ctx)) return startPointCalc(this.ctx, calc.kind);
      log.warn(`“${text}” anlaşılamadı. Koordinatı Y,X ya da @dY,dX biçiminde yazın.`);
      return;
    }
    const cmd = commands.byAlias(text) ?? commands.search(text, 1)[0];
    if (cmd) return this.run(cmd);
    log.error(`“${text}” adında bir komut yok. Tüm komutlar ve kısayollar için F1’e basın.`);
    this.input.select();
  }

  private onKey(e: KeyboardEvent): void {
    switch (e.key) {
      case 'Enter':
        e.preventDefault();
        return this.submit();
      case 'Escape':
        e.preventDefault();
        if (this.input.value) {
          this.input.value = '';
          this.hideList();
        } else {
          this.input.blur();
          this.ctx.tools.exit();
          this.ctx.view.focus();
        }
        return;
      case 'ArrowDown':
      case 'ArrowUp': {
        e.preventDefault();
        const dir = e.key === 'ArrowDown' ? 1 : -1;
        if (this.suggestions.length) {
          this.active = (this.active + dir + this.suggestions.length) % this.suggestions.length;
          return this.renderList();
        }
        if (!this.history.length) return;
        if (this.historyIndex === -1) this.historyIndex = this.history.length;
        this.historyIndex = Math.max(0, Math.min(this.history.length, this.historyIndex + (dir === -1 ? -1 : 1)));
        this.input.value = this.history[this.historyIndex] ?? '';
        return;
      }
      case 'Tab':
        if (this.suggestions[this.active]) {
          e.preventDefault();
          this.input.value = this.suggestions[this.active].aliases?.[0] ?? this.suggestions[this.active].title;
        }
        return;
    }
  }
}
