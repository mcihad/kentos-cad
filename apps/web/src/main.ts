import './styles/fonts.css';
import './styles/tokens.css';
import './styles/accents.css';
import './styles/base.css';
import './styles/shell.css';
import './styles/controls.css';
import './styles/panels.css';
import './styles/settings.css';
import './styles/processing.css';
import './styles/model.css';
import './styles/style.css';
import './styles/svgfile.css';
import './styles/svgedit.css';
import './styles/cloud.css';
import { createApp } from './app/createApp';
import { takeInvitationLink } from './app/cloud/invitationLink';
import { loadStartContent } from './app/startContent';
import { initCore } from './wasm/core';

// An invitation link's one-time token leaves the address before anything else is fetched (docs/adr/0042).
takeInvitationLink();

const root = document.getElementById('app')!;

/** The geometry core did not load (network, old browser): say so and offer a retry. */
function coreFailed(err: Error): void {
  console.error(err);
  const retry = document.createElement('button');
  retry.className = 'btn';
  retry.textContent = 'Yeniden dene';
  retry.addEventListener('click', () => location.reload());
  const text = document.createElement('p');
  text.textContent = `Geometri çekirdeği yüklenemedi: ${err.message} Bağlantınızı denetleyip yeniden deneyin.`;
  root.replaceChildren(text, retry);
}

// The system symbol library and the demo drawing download while the core compiles.
const start = loadStartContent();
start.catch(() => undefined);

// Every drawing calculation runs in the Rust core (docs/adr/0008): it starts before the app.
initCore()
  .then(
    () => createApp(root, start),
    (err: Error) => {
      coreFailed(err);
      return null;
    },
  )
  .then((ctx) => {
    if (!ctx) return;
    // Dev-only debugging handle (stripped from production builds).
    if (import.meta.env.DEV) (window as unknown as { kentos: unknown }).kentos = ctx;
    // Start-up measurement (scripts/perf/startup.mjs, docs/adr/0005): the first frame is drawn
    // in the next animation frame; the one after it runs once that frame has been presented.
    requestAnimationFrame(() => requestAnimationFrame(() => performance.mark('kentos:interactive')));
  })
  .catch((err) => {
    console.error(err);
    root.textContent = `Uygulama başlatılamadı: ${(err as Error).message}`;
  });
