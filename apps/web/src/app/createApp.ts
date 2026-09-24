import { CommandRegistry } from '../core/commands';
import { Keymap } from '../core/keymap';
import { createSampleProject } from '../model/sampleProject';
import { buildShowcase } from '../style/showcase';
import { SYSTEM_LIBRARY } from '../style/system';
import { Selection } from '../model/selection';
import { TOOL_CATALOG } from '../tools/catalog';
import { ToolManager } from '../tools/ToolManager';
import { openAboutDialog, openShortcutsDialog } from '../ui/dialogs';
import { openAppSettings, type AppSettingsSection } from '../ui/settings/AppSettingsDialog';
import { openNewProjectDialog } from '../ui/settings/NewProjectDialog';
import { openProjectSettings, type ProjectSettingsSection } from '../ui/settings/ProjectSettingsDialog';
import { AppShell } from '../ui/shell/AppShell';
import { openModelDialog, openToolDialog } from '../ui/processing/ToolDialog';
import { ViewportController } from '../viewport/ViewportController';
import { Clipboard } from './clipboard';
import { registerCoreCommands } from './commands';
import type { AppContext } from './context';
import { registerCloudCommands } from './cloud/commands';
import { openConflictDialog } from '../ui/cloud/ConflictDialog';
import { openLoginDialog } from '../ui/cloud/LoginDialog';
import { openDeleteDialog, openRenameDialog } from '../ui/cloud/ProjectActions';
import { openProjectsDialog } from '../ui/cloud/ProjectsDialog';
import { CloudSession } from './cloud/session';
import { registerFileExchangeCommands } from './fileExchange';
import { DocumentFiles } from './fileIO';
import { ServerStatus } from './server';
import { registerDefaultKeybindings } from './keybindings';
import { createProcessing, registerProcessingCommands } from './processing';
import { createStyles, registerStyleCommands } from './styles';
import { Formatter } from './format';
import { applyUiScale } from './commands';
import { createPreferences, createUiState, DraftingSettings, MessageLog } from './state';
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
export async function createApp(root: HTMLElement): Promise<AppContext> {
  const commands = new CommandRegistry();
  const keymap = new Keymap(commands);
  const ui = createUiState();
  const prefs = createPreferences();
  const doc = createSampleProject(prefs.defaultSrid.value);
  // The demo carries the whole system symbol library as a catalogue below the sheet.
  if (doc.homeView) {
    buildShowcase(doc, SYSTEM_LIBRARY.items, SYSTEM_LIBRARY.categories, { x: doc.homeView.minX, y: doc.homeView.minY - 80 });
    doc.dirty.set(false);
  }
  // Theme and type scale before any service reads CSS tokens (canvas palette).
  document.documentElement.dataset.theme = ui.theme.value;
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
    format: new Formatter(doc.settings),
    clipboard: new Clipboard(),
    // The visible area and the geometry store are read lazily: the viewport exists only after the context.
    processing: createProcessing(doc, selection, () => ctx.view.camera.visibleBounds(), { inBox: (r) => ctx.view.inBox(r), measures: (ids) => ctx.view.measures(ids) }),
    styles: createStyles(doc),
    server: new ServerStatus(),
  } as AppContext & { tools: ToolManager; view: ViewportController; files: DocumentFiles; cloud: CloudSession };
  ctx.tools = new ToolManager(ctx);
  ctx.view = new ViewportController(ctx);
  ctx.files = new DocumentFiles(ctx);
  ctx.cloud = new CloudSession(ctx);
  // Closing the tab with unsaved changes asks first. Not in development, where Vite reloads the page on every edit.
  if (import.meta.env.PROD) window.addEventListener('beforeunload', (e) => doc.dirty.value && e.preventDefault());
  TOOL_CATALOG.forEach((d) => ctx.tools.register(d));

  let shell: AppShell | null = null;
  registerCoreCommands(ctx, {
    openShortcuts: () => openShortcutsDialog(ctx),
    openAbout: () => openAboutDialog(ctx),
    openAppSettings: (section) => openAppSettings(ctx, section as AppSettingsSection | undefined),
    openProjectSettings: (section) => openProjectSettings(ctx, section as ProjectSettingsSection | undefined),
    openNewProject: () => openNewProjectDialog(ctx),
    focusCommandLine: () => shell?.bottom.commandLine.focus(),
    searchCommands: () => shell?.searchCommands(),
  });
  registerProcessingCommands(ctx, {
    open: (id, values) => openToolDialog(ctx, id, values),
    openModel: (id, values) => openModelDialog(ctx, id, values),
    // The designer is loaded when first opened: most sessions never need it.
    design: (id) => void import('../ui/processing/model/ModelDesigner').then((m) => m.openModelDesigner(ctx, id)),
    show: (tab) => shell?.showProcessing(tab),
  });
  registerStyleCommands(ctx);
  registerFileExchangeCommands(ctx);
  // The open cloud project as the rename and delete dialogs name it.
  const openTarget = () => {
    const p = ctx.cloud.project.value;
    return p && { tenantId: p.tenantId, tenantName: p.tenantName, projectId: p.projectId, name: p.name };
  };
  registerCloudCommands(ctx, {
    signIn: (then) => openLoginDialog(ctx, then),
    projects: (mode) => openProjectsDialog(ctx, mode),
    conflicts: () => openConflictDialog(ctx),
    rename: () => {
      const t = openTarget();
      if (t) openRenameDialog(ctx, t);
    },
    remove: () => {
      const t = openTarget();
      if (t) openDeleteDialog(ctx, t);
    },
  });
  registerDefaultKeybindings(ctx);
  commands.events.on('missing', ({ id }) => ctx.log.error(`Komut bulunamadı: ${id}`));
  // A trap in the geometry core is a bug in it; the drawing itself is safe (saving needs no core).
  onCoreFault(() => ctx.log.error('Geometri çekirdeği beklenmedik biçimde durdu. Çizimi kaydedip sayfayı yenileyin; hata sürerse bildirin.'));

  shell = new AppShell(ctx);
  root.replaceChildren(shell.el);
  keymap.attach(window);

  ctx.tools.activate('select');
  await ctx.view.mount(shell.viewportHost);

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
  return ctx;
}
