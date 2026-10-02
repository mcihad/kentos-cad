// Plays the interaction traces (fixtures/interaction/v1, docs/adr/0018) in the
// real app: its own Vite server, headless Chrome, real mouse and keyboard
// events. After each step the expectations are read through the dev-only
// `window.kentos` handle. The desktop plays the same files natively once its
// drawing area and tool session exist; a trace is the behaviour both keep.
// The step engine is tracePlayer.mjs, which the usage scenarios (use.mjs) play with too.
//
// Every trace runs once per variant (the §5 acceptance variants): the key
// events a US and a Turkish Q keyboard send, and a 2× (HiDPI) screen.
//
// The plays share out among several browsers on the one Vite server, each
// driven from a worker thread of its own: the player's state (the browser it
// drives, the keyboard it types with) is its module's, and a worker has modules
// of its own. A free browser takes the next play, so a long trace holds up no
// other; every trace starts from its own drawing and settings (the player's
// `setUp`), whichever browser plays it. The results print as the plays end.
//
//   pnpm e2e:interaction [trace-id…] [--variant=us|tr-q|hidpi] [--browsers=N]
//   (BROWSERS when not given, 1 plays them in turn; CHROME_BIN overrides the browser binary)
import { once } from 'node:events';
import { availableParallelism } from 'node:os';
import { isMainThread, parentPort, Worker, workerData } from 'node:worker_threads';
import { launch, sleep } from './cdp.mjs';
import { attach, loadTraces, play, setLayout, startApp, VARIANTS, waitReady } from './tracePlayer.mjs';

/**
 * The browser composites the page on the processor: the traces read the app's state, not its pixels, and
 * compositing through SwiftShader's GPU (cdp.mjs) costs nearly three times the processor time. WebGL still draws
 * through it.
 */
const BROWSER_ARGS = ['--disable-gpu-compositing'];

/**
 * Browsers side by side when `--browsers` is not given. Each keeps nearly a core busy and takes nearly a gigabyte;
 * two cores are left to the rest of the machine. On 8 cores six played the 240 plays in 127 s and four in 159 s; one
 * browser compositing through SwiftShader had taken 563 s (2 October 2026).
 */
const BROWSERS = Math.min(6, Math.max(1, availableParallelism() - 2));

if (isMainThread) await main();
else await browse(workerData.url);

/** Shares the plays out among the browsers and reports them; exits with 1 when one did not pass. */
async function main() {
  const args = process.argv.slice(2);
  const only = args.filter((a) => !a.startsWith('--'));
  const wanted = args.filter((a) => a.startsWith('--variant=')).map((a) => a.slice('--variant='.length));
  const variants = VARIANTS.filter((v) => !wanted.length || wanted.includes(v.id));
  if (!variants.length) throw new Error(`no variant ${wanted.join(', ')}; there are ${VARIANTS.map((v) => v.id).join(', ')}`);
  const traces = loadTraces(only);
  if (!traces.length) throw new Error(`no trace matches ${only.join(', ')}`);
  const given = args.find((a) => a.startsWith('--browsers='));
  const browsers = given ? Number(given.slice('--browsers='.length)) : BROWSERS;
  if (!Number.isInteger(browsers) || browsers < 1) throw new Error(`--browsers takes a whole number above 0, not “${given}”`);

  // Handed out in the order one browser would play them: a variant's traces, then the next variant's.
  const plays = variants.flatMap((v) => traces.map((t) => ({ variant: v.id, trace: t.id, title: t.title })));
  const failed = [];
  const pageErrors = [];
  let next = 0;
  const app = await startApp();
  try {
    const runs = Array.from({ length: Math.min(browsers, plays.length) }, (_, n) =>
      new Promise((resolve) => {
        const worker = new Worker(new URL(import.meta.url), { workerData: { url: app.url } });
        /** The play this browser has in hand. */
        let playing = null;
        const give = () => {
          playing = next < plays.length ? plays[next++] : null;
          worker.postMessage(playing);
        };
        const report = (p, problems) => {
          console.log([`${problems.length ? '✗' : '✓'} [${p.variant}] ${p.trace}: ${p.title}`, ...problems].join('\n'));
          if (problems.length) failed.push(p);
        };
        worker.on('message', (m) => {
          if (m.problems) report(playing, m.problems);
          if (m.errors) pageErrors.push(...m.errors.map((e) => `tarayıcı ${n + 1}: ${e}`));
          else give();
        });
        // A browser that stops (it would not start, or its page went) leaves the rest to the others.
        worker.on('error', (e) => {
          const why = `tarayıcı ${n + 1} durdu: ${e instanceof Error ? e.message : e}`;
          if (playing) report(playing, [`  ${why}`]);
          else pageErrors.push(why);
          playing = null;
        });
        worker.on('exit', resolve);
      }),
    );
    await Promise.all(runs);
  } finally {
    await app.close();
  }
  const unplayed = plays.length - next;
  if (unplayed) console.log(`${unplayed} iz oynatılamadı: bütün tarayıcılar durdu.`);
  if (pageErrors.length) console.log(`Sayfada hata:\n  ${pageErrors.join('\n  ')}`);
  if (failed.length) console.log(`${failed.length} iz geçmedi:\n${failed.map((p) => `  [${p.variant}] ${p.trace}`).join('\n')}`);
  const ok = !failed.length && !unplayed && !pageErrors.length;
  if (ok) console.log(`${traces.length} iz × ${variants.length} varyant geçti.`);
  process.exit(ok ? 0 : 1);
}

/**
 * One browser's worker: opens the app, then plays what it is handed until it is handed nothing, and tells what the
 * page logged as errors last.
 */
async function browse(url) {
  const traces = new Map(loadTraces().map((t) => [t.id, t]));
  const b = await launch(url, { args: BROWSER_ARGS });
  attach(b);
  try {
    await waitReady(b);
    let variant = null;
    parentPort.postMessage({ ready: true });
    for (;;) {
      const [given] = await once(parentPort, 'message');
      if (!given) break;
      const v = VARIANTS.find((x) => x.id === given.variant);
      if (v !== variant) {
        setLayout(v.layout);
        await b.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: v.dpr, mobile: false });
        await sleep(300);
        variant = v;
      }
      const t = traces.get(given.trace);
      const problems = await play(t);
      // The end state of each trace, to look at (scripts/e2e/out, not committed).
      if (v.id === 'us') await b.shot(`interaction-${t.id}`);
      parentPort.postMessage({ problems });
    }
    parentPort.postMessage({ errors: b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error')) });
  } finally {
    b.close();
  }
}
