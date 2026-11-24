#!/usr/bin/env python3
"""Kaynak dosyalarının boyu (CLAUDE.md §8, ADR 8054 §7): hiçbir Rust dosyası 2 000 satırı geçmez.

Masaüstünün çekirdeği (apps/desktop/src) bu sınırı kendi sınamasıyla tutar (src/tests/source_size_tests.rs). Bu betik
aynı sınırı çalışma alanının öbür crate'lerine ve programlarına uygular (crates/, apps/; crates/vendor/ dış koddur).

- Yeni bir dosya sınırı geçemez.
- Modül bildirimleri (`mod x;`, `#[path = …]`) sayılmaz: içeriği alt dosyaya veren satırlardır.
- Bugün sınırı geçen dosyalar tabandadır (source_size.baseline.json) ve büyüyemez: böyle bir dosyaya ekleyen iş onu önce
  konularına göre alt dosyalara böler.
- Taban yalnız küçülür: küçülen dosyanın sayısı `--write` ile düşürülür, sınırın altına inen tabandan çıkar. Büyüyen ya
  da yeni bir aşan varken `--write` yazmaz.

    python3 scripts/dev/source_size.py           # denetler (çıkış kodu 1: sorun var)
    python3 scripts/dev/source_size.py --write   # tabanı küçültür
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BASELINE = Path(__file__).with_name('source_size.baseline.json')
MOST_LINES = 2_000


def sources():
    listed = subprocess.run(['git', 'ls-files', '*.rs'], cwd=ROOT, capture_output=True, text=True, check=True)
    for path in listed.stdout.splitlines():
        if path.startswith(('crates/', 'apps/')) and not path.startswith('crates/vendor/'):
            yield path


# A module's declaration (`mod x;`, its `#[path = …]`) is how a big file hands its content to a child file: it is not
# counted, so splitting a file never makes its parent grow.
DECLARATION = re.compile(r'^\s*(#\[path\s*=.*\]|(pub(\([a-z]+\))?\s+)?mod\s+\w+;)\s*$')


def lines_of(path):
    with open(ROOT / path, encoding='utf-8', errors='replace') as f:
        return sum(1 for line in f if not DECLARATION.match(line))


def main():
    write = '--write' in sys.argv[1:]
    now = {path: n for path in sources() if (n := lines_of(path)) > MOST_LINES}
    # The first run writes today's files as the baseline; after that it only shrinks.
    baseline = json.loads(BASELINE.read_text(encoding='utf-8'))['files'] if BASELINE.exists() else dict(now)
    problems = []
    for path, n in sorted(now.items()):
        if path not in baseline:
            problems.append(f'{path}: {n} satır, sınır {MOST_LINES}. Konularına göre alt dosyalara bölün.')
        elif n > baseline[path]:
            problems.append(f'{path}: {baseline[path]} → {n} satır büyüdü; sınırı aşan dosya büyüyemez. Eklemeden önce '
                            f'konularına göre bölün.')
    gone = sorted(path for path in baseline if path not in now)
    shrunk = sorted(path for path, n in now.items() if path in baseline and n < baseline[path])
    if write:
        if problems:
            print('\n'.join(problems))
            sys.exit('Taban yazılmadı: önce yukarıdakileri bölün (taban yalnız küçülür).')
        kept = {path: min(n, baseline[path]) for path, n in now.items()}
        BASELINE.write_text(json.dumps({
            'note': 'scripts/dev/source_size.py: 2 000 satırı bugün aşan Rust dosyaları ve satır sayıları; büyüyemezler, '
                    'taban yalnız küçülür.',
            'files': dict(sorted(kept.items())),
        }, ensure_ascii=False, indent=1) + '\n', encoding='utf-8')
        print(f'Taban yazıldı: {len(kept)} dosya.')
        return
    for path in gone:
        problems.append(f'{path}: artık sınırın altında ya da yok; tabandan çıkarın (--write).')
    for path in shrunk:
        problems.append(f'{path}: {baseline[path]} → {now[path]} satır küçüldü; tabanı düşürün (--write).')
    if problems:
        print('\n'.join(problems))
        sys.exit(1)
    print(f'Kaynak boyu temiz: {len(now)} dosya tabanda, hiçbiri büyümedi; yeni bir aşan yok.')


main()
