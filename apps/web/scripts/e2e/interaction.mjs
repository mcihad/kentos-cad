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
//   pnpm e2e:interaction [trace-id…] [--variant=us|tr-q|hidpi]   (CHROME_BIN overrides the browser binary)
import { launch, sleep } from './cdp.mjs';
import { attach, loadTraces, play, setLayout, startApp, VARIANTS, waitReady } from './tracePlayer.mjs';

const args = process.argv.slice(2);
const only = args.filter((a) => !a.startsWith('--'));
const wanted = args.filter((a) => a.startsWith('--variant=')).map((a) => a.slice('--variant='.length));
const variants = VARIANTS.filter((v) => !wanted.length || wanted.includes(v.id));
if (!variants.length) throw new Error(`no variant ${wanted.join(', ')}; there are ${VARIANTS.map((v) => v.id).join(', ')}`);
const traces = loadTraces(only);
if (!traces.length) throw new Error(`no trace matches ${only.join(', ')}`);

const app = await startApp();
const b = await launch(app.url);
attach(b);

let failed = 0;
try {
  await waitReady(b);
  for (const v of variants) {
    setLayout(v.layout);
    await b.send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 900, deviceScaleFactor: v.dpr, mobile: false });
    await sleep(300);
    for (const t of traces) {
      const problems = await play(t);
      // The end state of each trace, to look at (scripts/e2e/out, not committed).
      if (v.id === 'us') await b.shot(`interaction-${t.id}`);
      console.log(`${problems.length ? '✗' : '✓'} [${v.id}] ${t.id}: ${t.title}`);
      for (const p of problems) console.log(p);
      if (problems.length) failed++;
    }
  }
  const errors = b.consoleLog.filter((l) => l.startsWith('EXCEPTION') || l.startsWith('error'));
  if (errors.length) {
    console.log(`Sayfada hata:\n  ${errors.join('\n  ')}`);
    failed++;
  }
} finally {
  b.close();
  await app.close();
}
console.log(failed ? `${failed} iz geçmedi.` : `${traces.length} iz × ${variants.length} varyant geçti.`);
process.exit(failed ? 1 : 0);
