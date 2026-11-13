#!/usr/bin/env node
// Selective fixture check (docs/adr/8054 §11): the shared command cases (fixtures/commands/v1) of what changed since
// BASE, instead of all of them.
//
// - A changed case file runs itself; a changed command handler (crates/native/application/src/…) runs the cases of
//   the commands whose registry arm names it.
// - A change the map cannot place on commands (a shared check, the registry, the runner, another crate) runs every
//   case: guessing there would let a broken case through.
// - When every changed handler is one discipline's, the application is built with that module only
//   (`--no-default-features --features mod-…`, the tests' own CBS with it), else whole.
//
//   node scripts/dev/changed-fixtures.mjs [--dry-run]     BASE=origin/main by default; FILES=a,b names the changed
//                                                         files instead of git
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
const dry = process.argv.includes('--dry-run');
const base = process.env.BASE || 'origin/main';
const APP = 'crates/native/application/src';
const DISCIPLINES = ['arch', 'map', 'gis', 'road'];

const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' });
const read = (p) => readFileSync(join(root, p), 'utf8');

// What changed: committed since the merge base, staged, unstaged and untracked (or FILES, as given).
const since = process.env.FILES ? 'FILES' : git('merge-base', base, 'HEAD').trim();
const listed = process.env.FILES
  ? process.env.FILES.split(',')
  : [...git('diff', '--name-only', since).split('\n'), ...git('ls-files', '--others', '--exclude-standard').split('\n')];
const changed = [...new Set(listed.map((s) => s.trim()).filter(Boolean))];

// The application's modules by file: lib.rs's `#[path = "…"] mod name;`.
const lib = read(`${APP}/lib.rs`);
const fileOf = new Map(); // module name → its file, relative to src
for (const m of lib.matchAll(/#\[path = "([^"]+)"\]\n(?:#\[[^\]]*\]\n)*(?:pub(?:\([a-z]+\))? )?mod ([a-z_0-9]+);/g)) {
  fileOf.set(m[2], m[1]);
}
// The names the dispatchers' arms use for a module: itself, the root's re-exports of its items and the registry's
// aliases.
const owner = new Map(); // name in an arm → module name
for (const name of fileOf.keys()) owner.set(name, name);
const items = (body) =>
  body
    .replace(/^\{|\}$/g, '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
for (const m of lib.matchAll(/(?:pub(?:\([a-z]+\))? )?use ([a-z_0-9]+)::(\{[^}]*\}|[A-Za-z_0-9]+(?: as [A-Za-z_0-9]+)?);/g)) {
  if (!fileOf.has(m[1])) continue;
  for (const it of items(m[2])) owner.set(it.split(' as ').pop().trim(), m[1]);
}
const registry = read(`${APP}/core/registry.rs`);
for (const m of registry.matchAll(/use crate::(\{[^}]*\}|[^;{]+);/g)) {
  for (const it of m[1].startsWith('{') ? items(m[1]) : [m[1]]) {
    const [path, alias] = it.split(' as ').map((s) => s.trim());
    const top = path.split('::')[0];
    if (alias && fileOf.has(top)) owner.set(alias, top);
  }
}
// Command ids by their constant (kentos_commands).
const ids = new Map();
const walk = (dir) =>
  readdirSync(join(root, dir), { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? walk(join(dir, e.name)) : e.name.endsWith('.rs') ? [join(dir, e.name)] : [],
  );
for (const f of walk('crates/shared/commands/src')) {
  for (const m of read(f).matchAll(/pub const ([A-Z_0-9]+): &str = "([a-z0-9_.]+)";/g)) ids.set(m[1], m[2]);
}
// The commands each module answers: the arms of the five dispatchers.
const commandsOf = new Map(); // module name → command ids
for (const d of ['core', ...DISCIPLINES]) {
  const text = read(`${APP}/${d}/dispatch.rs`);
  const arms = [...text.matchAll(/^\s+(kentos_commands::[A-Z_0-9]+(?:\s*\|\s*kentos_commands::[A-Z_0-9]+)*)\s*=>/gm)];
  arms.forEach((arm, i) => {
    const body = text.slice(arm.index, i + 1 < arms.length ? arms[i + 1].index : text.length);
    const used = body.match(/_arm!\(\$command, \$then, ([a-z_0-9]+),/) || body.match(/\$then!\(([a-z_0-9]+),/);
    if (!used) return;
    const module = owner.get(used[1]);
    if (!module) return;
    for (const c of arm[1].matchAll(/kentos_commands::([A-Z_0-9]+)/g)) {
      const id = ids.get(c[1]);
      if (id) commandsOf.set(module, [...(commandsOf.get(module) || []), id]);
    }
  });
}

// Each changed file placed: a case file, a handler's commands, or a change that asks for every case.
const words = new Set();
const disciplines = new Set();
const broad = [];
for (const f of changed) {
  if (/^fixtures\/commands\/v1\/(?:desktop\/)?[^/]+\.json$/.test(f)) {
    words.add(f.split('/').pop());
    continue;
  }
  if (f.startsWith(`${APP}/`)) {
    const rel = relative(APP, f);
    // A module's own file, or a file of a module with a folder of its own (`core/corner/mod.rs`, its children).
    const module = [...fileOf].find(([, p]) => p === rel || (p.endsWith('/mod.rs') && rel.startsWith(p.slice(0, -6))))?.[0];
    const commands = module && commandsOf.get(module);
    if (commands) {
      commands.forEach((c) => words.add(c));
      disciplines.add(rel.split('/')[0]);
    } else {
      broad.push(f);
    }
    continue;
  }
  if (f.startsWith('crates/') || f.startsWith('fixtures/') || f === 'Cargo.toml' || f === 'Cargo.lock') broad.push(f);
}

const one = [...disciplines].filter((d) => d !== 'core');
const features = broad.length === 0 && one.length === 1 ? ['--no-default-features', '--features', `mod-${one[0]}`] : [];
console.log(`• Temel: ${base} (${since.slice(0, 9)}), değişen dosya: ${changed.length}`);
if (broad.length) {
  console.log(`• Komutlara eşlenemeyen değişiklik var; bütün durumlar koşulacak:`);
  broad.slice(0, 12).forEach((f) => console.log(`    ${f}`));
  if (broad.length > 12) console.log(`    … ve ${broad.length - 12} dosya daha`);
} else if (words.size === 0) {
  console.log('• Değişen komut ya da durum dosyası yok; fixture koşusu gerekmiyor.');
  process.exit(0);
} else {
  console.log(`• Koşulacak komut ve durum dosyaları (${words.size}): ${[...words].sort().join(', ')}`);
}
console.log(`• Derleme: ${features.length ? `yalnız ${features[2]} (testler CBS'yi kendileri açar)` : 'bütün modüller'}`);
const env = { ...process.env };
if (broad.length === 0) env.KENTOS_CASES_ONLY = [...words].sort().join(',');
const args = ['test', '-p', 'kentos-native-application', ...features, '--test', 'all', 'fixtures::'];
console.log(`• ${broad.length === 0 ? `KENTOS_CASES_ONLY=${env.KENTOS_CASES_ONLY} ` : ''}cargo ${args.join(' ')}`);
if (dry) process.exit(0);
const run = spawnSync('cargo', args, { cwd: root, env, stdio: 'inherit' });
process.exit(run.status ?? 1);
