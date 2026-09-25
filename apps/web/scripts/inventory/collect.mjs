// The part of the feature inventory (TODOS.md BASE-04) that only the running
// app knows: commands as registered, keyboard bindings, where menus and the
// ribbon place each command, which work modes hide it, the tool catalog, the
// processing tools and models, and the settings with their defaults.
//
// `collectInPage` runs inside the page (it is sent as source text through the
// DevTools protocol), so it must not use anything from this module's scope.
// It reads the development diagnostics surface `window.kentos` (the
// AppContext) and imports the app's own modules from the Vite dev server.

export async function collectInPage() {
  const k = window.kentos;
  const [menus, ribbon, workspaces, processing, projectSettings] = await Promise.all([
    import('/src/app/menus.ts'),
    import('/src/app/ribbon.ts'),
    import('/src/app/workspaces.ts'),
    import('/src/app/processing.ts'),
    import('/src/model/projectSettings.ts'),
  ]);
  const tools = k.tools.list();
  const registry = k.processing.registry;
  const models = k.processing.models.value;
  const toolIds = registry.list().map((t) => processing.processingCommandId(t.id));
  const modelIds = ['processing.newModel', ...models.map((m) => processing.modelCommandId(m.id))];
  const push = (map, id, value) => {
    const list = (map[id] ??= []);
    if (!list.includes(value)) list.push(value);
  };

  // Menu entries, as “Menü › Bölüm › Alt menü” paths.
  const menuPaths = {};
  const walkMenu = (specs, trail, into, place) => {
    for (const block of menus.menuBlocks(specs, tools)) {
      const here = block.label ? [...trail, block.label] : trail;
      for (const e of block.items) {
        if (typeof e === 'object') walkMenu(e.items, [...here, e.label], into, place);
        else for (const id of e === '@processing' ? toolIds : e === '@models' ? modelIds : [e]) push(into, id, place ?? here.join(' › '));
      }
    }
  };
  for (const m of menus.MAIN_MENU) walkMenu(m.items, [m.label], menuPaths);

  // Ribbon places, as “Sekme › Panel”; a drop-down button's items count as its panel's.
  const ribbonPlaces = {};
  const tabs = ribbon.ribbonTabs({ tools, processing: registry.tree(), models, iconOf: () => undefined });
  for (const tab of tabs) {
    for (const panel of tab.panels) {
      const place = `${tab.label} › ${panel.label}`;
      for (const id of ribbon.panelCommands(panel)) push(ribbonPlaces, id, place);
      for (const item of panel.items) if (item.kind === 'menu') walkMenu(item.menu.items, [], ribbonPlaces, place);
    }
  }

  const modes = workspaces.WORKSPACES.filter((w) => w.status === 'ready').map((w) => ({ id: w.id, filter: workspaces.workspaceFilter(w, tools) }));
  const shortcuts = {};
  for (const b of k.keymap.all()) push(shortcuts, b.command, b.args === undefined ? b.chord : `${b.chord} ${JSON.stringify(b.args)}`);
  const toolCommands = new Set(tools.map((t) => `tool.${t.id}`));

  const commands = k.commands.all().map((c) => ({
    id: c.id,
    title: c.title,
    short: c.short,
    category: c.category,
    description: c.description,
    aliases: [...(c.aliases ?? [])],
    pending: !!c.pending,
    pendingNote: c.pendingNote,
    kind: typeof c.isChecked === 'function' ? 'toggle' : 'action',
    shortcuts: shortcuts[c.id] ?? [],
    menus: menuPaths[c.id] ?? [],
    ribbon: ribbonPlaces[c.id] ?? [],
    quickAccess: ribbon.QUICK_ACCESS.includes(c.id),
    toolbox: toolCommands.has(c.id),
    hiddenIn: modes.filter((m) => !m.filter.command(c.id)).map((m) => m.id),
  }));

  const toolItems = tools.map((t) => ({
    id: t.id,
    label: t.label,
    group: t.group,
    section: t.section,
    shortcut: t.shortcut,
    aliases: [...(t.aliases ?? [])],
    ready: t.ready,
    primary: !!t.primary,
    family: t.family,
    rare: !!t.rare,
    methods: (t.methods ?? []).map((m) => m.label),
    steps: (t.steps ?? []).length,
    description: t.description,
  }));

  const params = (list) => list.map((p) => ({ name: p.name, type: p.type, label: p.label, optional: !!p.optional, unit: p.unit || undefined }));
  const processingItems = registry.list().map((t) => ({
    id: t.id,
    label: t.label,
    category: registry.categoryPath(t.category),
    description: t.description,
    aliases: [...(t.aliases ?? [])],
    targets: [...t.targets],
    parameters: params(t.parameters),
    outputs: (t.outputs ?? []).map((o) => ({ name: o.name, type: o.type })),
    command: processing.processingCommandId(t.id),
  }));
  const modelItems = models.map((m) => ({
    id: m.id,
    label: m.label,
    category: registry.categoryPath(m.category) || m.category,
    description: m.description,
    inputs: params(m.inputs),
    steps: m.steps.length,
    command: processing.modelCommandId(m.id),
  }));

  const workspaceItems = workspaces.WORKSPACES.map((w) => ({ id: w.id, label: w.label, title: w.title, ready: w.status === 'ready', hide: w.hide ?? {} }));

  // Settings: one signal per field; the values a fresh browser profile starts with.
  const isSignal = (v) => !!v && typeof v === 'object' && 'value' in v && typeof v.subscribe === 'function';
  const typeOf = (v) => (v === null ? 'null' : Array.isArray(v) ? 'array' : typeof v);
  const fields = (owner, scope, persistence) =>
    Object.entries(owner)
      .filter(([, s]) => isSignal(s))
      .map(([key, s]) => ({ key, scope, persistence, type: typeOf(s.value), default: s.value }));
  const settings = [
    ...fields(k.prefs, 'user', 'localStorage kentos.prefs.v1'),
    ...fields(k.ui, 'layout', 'localStorage kentos.ui.v1'),
    ...Object.entries(projectSettings.PROJECT_SETTINGS_DEFAULTS).map(([key, v]) => ({ key, scope: 'project', persistence: '.kcad settings, cloud project', type: typeOf(v), default: v })),
    ...fields(k.settings, 'session', 'none'),
  ];

  return { commands, tools: toolItems, processing: processingItems, models: modelItems, workspaces: workspaceItems, settings };
}
