import { CommandRegistry } from '../core/commands';
import { Keymap } from '../core/keymap';
import type { StartContent } from './startContent';
import { Selection } from '../model/selection';
import { TOOL_CATALOG } from '../tools/catalog';
import { ToolManager } from '../tools/ToolManager';
import { openAboutDialog, openShortcutsDialog } from '../ui/dialogs';
import type { AppSettingsSection } from '../ui/settings/AppSettingsDialog';
import type { ProjectSettingsSection } from '../ui/settings/ProjectSettingsDialog';
import { AppShell } from '../ui/shell/AppShell';
import { ViewportController } from '../viewport/ViewportController';
import { Clipboard } from './clipboard';
import { registerCoreCommands } from './commands';
import type { AppContext } from './context';
import { registerCloudCommands } from './cloud/commands';
import { CloudSession } from './cloud/session';
import { registerCalcCommands } from './calc';
import { registerFileExchangeCommands } from './fileExchange';
import { DocumentFiles } from './fileIO';
import { RecoveryCopies } from './recovery';
import { ServerStatus } from './server';
import { registerDefaultKeybindings } from './keybindings';
import { createProcessing, registerProcessingCommands } from './processing';
import { createStyles, registerStyleCommands } from './styles';
import { Formatter } from './format';
import { applyUiScale } from './commands';
import { applyAccent, applyDrawingFont, applyUiFont } from './appearance';
import { createPreferences, createUiState, DraftingSettings, MessageLog } from './state';
import { openBrowserSettings, reportSettingsOpen } from './settings/browser';
import { onCoreFault } from '../wasm/core';

/** An OpenID sign-in that failed comes back as `?oidc=error&reason=…`: say why, then clean the address. */
function reportSignInError(ctx: AppContext): void {
  const q = new URLSearchParams(location.search);
  if (q.get('oidc') !== 'error') return;
  const why: Record<string, string> = {
    unauthenticated: 'kimlik doğrulanamadı',
    forbidden: 'hesabınız devre dışı',
    invalid: 'giriş isteğinin süresi doldu',
    unavailable: 'sunucuya ya da kimlik sağlayıcısına ulaşılamadı',
    not_configured: 'bu sunucuda OpenID girişi yok',
    access_denied: 'giriş reddedildi',
  };
  const reason = q.get('reason') ?? '';
  ctx.log.error(`Kurum hesabıyla giriş yapılamadı: ${why[reason] ?? reason}. Yeniden deneyin ya da yerel hesabınızla girin.`);
  q.delete('oidc');
  q.delete('reason');
  history.replaceState(null, '', `${location.pathname}${q.size ? `?${q}` : ''}${location.hash}`);
}

/**
 * Composition root: builds services, wires them into one AppContext and
 * mounts the shell. Nothing else in the app constructs services.
 */
export async function createApp(root: HTMLElement, start: Promise<StartContent>): Promise<AppContext> {
  const commands = new CommandRegistry();
  const keymap = new Keymap(commands);
  const ui = createUiState();
  // The typed settings (docs/adr/0023): opened (and migrated once) before anything reads a preference.
  const settingsStore = openBrowserSettings();
  const prefs = createPreferences(settingsStore);
  // The system symbol library and the demo drawing are chunks of their own, fetched beside the geometry core (main.ts).
  const { system, sampleProject } = await start;
  const doc = sampleProject(prefs.defaultSrid.value);
  // Theme, accent, typeface and type scale before any service reads CSS tokens (canvas palette).
  document.documentElement.dataset.theme = ui.theme.value;
  applyAccent(prefs.accent.value);
  const fontReady = applyUiFont(prefs.uiFont.value);
  applyUiScale(prefs.uiScale.value);

  const selection = new Selection();
  // Services that need the context are attached right after it exists.
  const ctx = {
    commands,
    keymap,
    doc,
    selection,
    settings: new DraftingSettings(),
    log: new MessageLog(),
    ui,
    prefs,
    settingsStore,
    format: new Formatter(doc.settings),
    clipboard: new Clipboard(),
    // The visible area and the geometry store are read lazily: the viewport exists only after the context.
    processing: createProcessing(doc, selection, () => ctx.view.camera.visibleBounds(), { inBox: (r) => ctx.view.inBox(r), measures: (ids) => ctx.view.measures(ids) }),
    styles: createStyles(doc, system),
    server: new ServerStatus(),
  } as AppContext & { tools: ToolManager; view: ViewportController; files: DocumentFiles; cloud: CloudSession; recovery: RecoveryCopies };
  ctx.tools = new ToolManager(ctx);
  ctx.view = new ViewportController(ctx);
  ctx.files = new DocumentFiles(ctx);
  ctx.cloud = new CloudSession(ctx);
  // Unsaved work is kept on this device as it changes, apart from any file (docs/adr/0030).
  ctx.recovery = new RecoveryCopies(ctx);
  ctx.files.discarded = () => ctx.recovery.discard();
  ctx.recovery.start();
  // Closing the tab with unsaved changes asks first. Not in development, where Vite reloads the page on every edit.
  if (import.meta.env.PROD) window.addEventListener('beforeunload', (e) => doc.dirty.value && e.preventDefault());
  TOOL_CATALOG.forEach((d) => ctx.tools.register(d));

  let shell: AppShell | null = null;
  registerCoreCommands(ctx, {
    openShortcuts: () => openShortcutsDialog(ctx),
    openAbout: () => openAboutDialog(ctx),
    // Windows are loaded when first opened (CLAUDE.md §20): most sessions open few of them.
    openAppSettings: (section) => lazy(ctx, import('../ui/settings/AppSettingsDialog'), (m) => m.openAppSettings(ctx, section as AppSettingsSection | undefined)),
    openProjectSettings: (section) => lazy(ctx, import('../ui/settings/ProjectSettingsDialog'), (m) => m.openProjectSettings(ctx, section as ProjectSettingsSection | undefined)),
    openNewProject: () => lazy(ctx, import('../ui/settings/NewProjectDialog'), (m) => m.openNewProjectDialog(ctx)),
    focusCommandLine: () => shell?.bottom.commandLine.focus(),
    searchCommands: () => shell?.searchCommands(),
    keyTips: () => shell?.keyTips(),
    openStart: () => void openStart(ctx),
  });
  registerProcessingCommands(ctx, {
    open: (id, values) => lazy(ctx, import('../ui/processing/ToolDialog'), (m) => m.openToolDialog(ctx, id, values)),
    openModel: (id, values) => lazy(ctx, import('../ui/processing/ToolDialog'), (m) => m.openModelDialog(ctx, id, values)),
    // The designer is loaded when first opened: most sessions never need it.
    design: (id) => void import('../ui/processing/model/ModelDesigner').then((m) => m.openModelDesigner(ctx, id)),
    show: (tab) => shell?.showProcessing(tab),
  });
  registerStyleCommands(ctx);
  registerFileExchangeCommands(ctx);
  registerCalcCommands(ctx);
  // The open cloud project as the rename and delete dialogs name it.
  const openTarget = () => {
    const p = ctx.cloud.project.value;
    return p && { tenantId: p.tenantId, tenantName: p.tenantName, projectId: p.projectId, name: p.name };
  };
  registerCloudCommands(ctx, {
    signIn: (then) => lazy(ctx, import('../ui/cloud/LoginDialog'), (m) => m.openLoginDialog(ctx, then)),
    projects: (mode, pick) => lazy(ctx, import('../ui/cloud/ProjectsDialog'), (m) => m.openProjectsDialog(ctx, mode, pick)),
    conflicts: () => lazy(ctx, import('../ui/cloud/ConflictDialog'), (m) => m.openConflictDialog(ctx)),
    rename: () => {
      const t = openTarget();
      if (t) lazy(ctx, import('../ui/cloud/ProjectActions'), (m) => m.openRenameDialog(ctx, t));
    },
    remove: () => {
      const t = openTarget();
      if (t) lazy(ctx, import('../ui/cloud/ProjectActions'), (m) => m.openDeleteDialog(ctx, t));
    },
    share: () => {
      const t = openTarget();
      if (t) lazy(ctx, import('../ui/cloud/ShareDialog'), (m) => m.openShareDialog(ctx, t));
    },
  });
  // The open cloud project's access taken away: say so, and offer a local copy (TODOS.md CLOUD-13).
  ctx.cloud.accessLost.subscribe((lost) => lost && lazy(ctx, import('../ui/cloud/AccessLostNotice'), (m) => m.openAccessLostNotice(ctx, lost)));
  registerDefaultKeybindings(ctx);
  commands.events.on('missing', ({ id }) => ctx.log.error(`Komut bulunamadı: ${id}`));
  // A trap in the geometry core is a bug in it; the drawing itself is safe (saving needs no core).
  onCoreFault(() => ctx.log.error('Geometri çekirdeği beklenmedik biçimde durdu. Çizimi kaydedip sayfayı yenileyin; hata sürerse bildirin.'));

  shell = new AppShell(ctx);
  root.replaceChildren(shell.el);
  keymap.attach(window);

  ctx.tools.activate('select');
  await ctx.view.mount(shell.viewportHost);
  // The overlay was drawn with fallbacks until the bundled faces arrived (a canvas does not ask for a face by itself).
  void fontReady.then(() => ctx.view.requestOverlay());
  // The drawing's typeface is the project's: set again whenever a project is opened or its setting changes.
  doc.settings.drawingFont.subscribe((f) => void applyDrawingFont(f).then(() => ctx.view.refreshFonts()), true);

  // The server is asked only once the app is idle, so the check never slows the start.
  ctx.server.watch(window);
  const idle = window.requestIdleCallback ?? ((f: () => void) => window.setTimeout(f, 300));
  idle(() => void ctx.server.check());
  // Who is signed in, once the server answers (and again whenever it comes back).
  ctx.server.state.subscribe((s) => s === 'online' && ctx.cloud.auth.value !== 'signedIn' && void ctx.cloud.refresh());
  reportSignInError(ctx);

  // Drop selection entries whose entities disappeared (undo, erase).
  doc.events.on('changed', () => ctx.selection.retain((id) => !!doc.get(id)));

  ctx.log.info(`${doc.name.value} açıldı: ${doc.size} nesne, ${doc.layers.leaves().length} katman.`);
  reportSettingsOpen(settingsStore, ctx.log);
  if (startScreenOnOpen(ctx)) void openStart(ctx);
  // Work a crash or a closed tab left unsaved is offered once the app is up (over the start screen).
  void ctx.recovery.offer();
  return ctx;
}

/** Runs `open` with a window's module once it has loaded; a failed load says so (the drawing is untouched). */
function lazy<M>(ctx: AppContext, module: Promise<M>, open: (m: M) => void): void {
  module.then(open, (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`));
}

/** The start screen is loaded on first use (CLAUDE.md §20); a failed load says so and the drawing stays. */
function openStart(ctx: AppContext): Promise<void> {
  return import('../ui/start/StartScreen').then(
    (m) => m.openStartScreen(ctx),
    () => ctx.log.error('Başlangıç ekranı yüklenemedi; bağlantıyı denetleyip Dosya → Başlangıç ekranı ile yeniden deneyin.'),
  );
}

/**
 * Whether the start screen opens with the app: the user's setting, except
 * under automation (tests drive the drawing at once; `?start=1` still asks for it) and with `?start=0`.
 */
function startScreenOnOpen(ctx: AppContext): boolean {
  const param = new URLSearchParams(location.search).get('start');
  if (param === '0') return false;
  if (param === '1') return true;
  return ctx.prefs.startScreen.value && !navigator.webdriver;
}
