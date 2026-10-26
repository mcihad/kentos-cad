#!/usr/bin/env python3
"""ARCH-PERF-01: isolated, repeatable moving-wall plans and cold/warm memory.

Build the native module with maturin --release first. Copy the resulting module
before changing the implementation; --native loads that immutable binary in each
worker. SDK/JSON/FFI/native planning are timed, GPU and UI events are not.

.run/py/bin/python scripts/perf/architecture_preview.py \
  --native .run/architecture-preview/before/_native.abi3.so \
  --output .run/architecture-preview/before.json --profile release

No timing thresholds. Before/after comparisons require matching input and output
fingerprints, independent workers and the same machine/profile/SDK/harness.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json
import math
import os
from pathlib import Path
import platform
import resource
import subprocess
import sys
from time import perf_counter_ns
from datetime import datetime, timezone

from wall_rectangle import ROOT, git_value, summary


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                     ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def sdk(native):
    if native:
        spec = importlib.util.spec_from_file_location('kentos._native', native)
        module = importlib.util.module_from_spec(spec)
        sys.modules['kentos._native'] = module
        spec.loader.exec_module(module)
    from kentos import cad, _native
    return cad, Path(_native.__file__).resolve()


def setup(cad, case):
    """A validated legacy snapshot, not O(n²) command-based seed construction.

    Distant walls lie on a 20 m grid. Crowded adds twelve crossing short walls
    around the target, independently of background size: local join density and
    unrelated entity count are separate variables, not 100k coincident walls.
    """
    doc = copy.deepcopy(json.loads((ROOT / 'fixtures/commands/v1/cad.walls.rectangle.json').read_text())['setup'])
    ox, oy = case['origin']
    doc['origin'] = {'x': ox, 'y': oy}
    doc['name'] = 'Architectural preview performance fixture'
    doc['entities'] = []
    layer = doc['activeLayer']

    def wall(points, closed=False):
        return {'kind': 'wall', 'id': len(doc['entities']) + 1, 'layerId': layer, 'attrs': {},
                **cad.NewWall(pts=points, closed=closed, width=.2, justify='left').to_json()}

    for index in range(case['background']):
        x, y = ox + 1000 + index % 100 * 20, oy + 1000 + index // 100 * 20
        if case['kind'] == 'walls':
            entity = wall([(x, y), (x+4, y), (x+4, y+3), (x, y+3)], True)
        else:
            entity = {'kind': 'line', 'id': index+1, 'layerId': layer, 'attrs': {},
                      'a': {'x': x, 'y': y}, 'b': {'x': x+4, 'y': y}}
        doc['entities'].append(entity)
    if case['density'] == 'crowded':
        for i in range(6):
            x, y = ox + .3 + i*.6, oy + .2 + i*.45
            doc['entities'].append(wall([(x, oy-.5), (x, oy+3.5)]))
            doc['entities'].append(wall([(ox-.5, y), (ox+4.5, y)]))
    payload = json.dumps(doc, separators=(',', ':'), ensure_ascii=False).encode()
    return cad.Document.from_bytes(payload), digest(doc), len(doc['entities'])


def rss_bytes():
    """Current RSS from ps; no unsupported-platform zero masquerading as a measure."""
    try:
        value = subprocess.check_output(['ps', '-o', 'rss=', '-p', str(os.getpid())], text=True)
        return int(value.strip()) * 1024
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def peak_bytes():
    value = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    return int(value if sys.platform == 'darwin' else value * 1024)


def worker(args):
    cad, native = sdk(args.native)
    case = json.loads(args.child)
    opened = perf_counter_ns()
    doc, input_hash, count = setup(cad, case)
    setup_ms = (perf_counter_ns()-opened)/1e6
    assert len(doc) == count
    before = doc.to_bytes()
    revision = doc.revision
    ox, oy = case['origin']
    inputs = []
    for i in range(32):
        angle = i * math.tau / 32
        bx, by = ox+4+.2*math.cos(angle), oy+3+.2*math.sin(angle)
        frame = cad.CornersRectangleFrame(a=(ox, oy), b=(bx, by))
        wall = cad.NewWall(pts=[(ox, oy), (bx, oy), (bx, by), (ox, by)],
                           closed=True, width=.2, justify='left')
        inputs.append((cad.WallsRectangle(layer_id=doc.active_layer, frame=frame, reference='inner', width=.2),
                       cad.WallsCreate(layer_id=doc.active_layer, walls=[wall])))
    commands = [cad.walls.rectangle, cad.walls.create]

    def invoke(command, index, source=None):
        result = commands[command].run(doc if source is None else source, inputs[index % len(inputs)][command], op='plan')
        assert result.ok, result
        return {'output': result.output.to_json(),
                'warnings': [{'code': w.code, 'message': w.message, 'path': w.path} for w in result.warnings]}

    rss_before = rss_bytes()
    cold = perf_counter_ns()
    first = invoke(0, 0)
    cold_ns = perf_counter_ns()-cold
    cold_ms = cold_ns/1e6
    rss_after_cold = rss_bytes()
    fingerprints = []
    # Exact geometry AND warning comparison, outside the timed samples.
    for i in range(len(inputs)):
        a, b = invoke(0, i), invoke(1, i)
        assert a == b, f'Different rectangle/create plans at pointer sample {i}'
        fingerprints.append(digest(a))
    assert digest(first) == fingerprints[0]
    # Every extra cold sample has a freshly decoded document and no candidate
    # cache. Decode/teardown are excluded; the module itself is already loaded.
    # This measures per-document cold planning, not process-start latency.
    cold_samples = [cold_ns]
    for _ in range(args.cold_samples - 1):
        fresh = cad.Document.from_bytes(before)
        t = perf_counter_ns()
        value = invoke(0, 0, fresh)
        cold_samples.append(perf_counter_ns() - t)
        assert digest(value) == fingerprints[0], 'Cold reload changed its plan'
        del fresh
    for i in range(args.warmup):
        for c in range(2):
            invoke(c, i)
    samples = [[], []]
    for i in range(args.iterations):
        for c in ([0, 1] if i % 2 == 0 else [1, 0]):
            t = perf_counter_ns()
            value = invoke(c, i)
            samples[c].append(perf_counter_ns()-t)
            assert digest(value) == fingerprints[i % len(inputs)], 'A moving preview changed its result'
    assert doc.to_bytes() == before and doc.revision == revision, 'A preview wrote the document'
    if args.seed:
        doc.save(args.seed)
    return {'case': case, 'entities': count, 'setup_ms_excluded': setup_ms,
            'input_sha256': input_hash, 'moving_outputs_sha256': digest(fingerprints),
            'document_unchanged': True, 'identical_commands': True, 'pointer_positions': len(inputs),
            'cold_rectangle_ms': cold_ms, 'cold_rectangle': summary(cold_samples),
            'rectangle': summary(samples[0]), 'create': summary(samples[1]),
            'memory': {'rss_before_cold_bytes': rss_before, 'rss_after_cold_bytes': rss_after_cold,
                       'rss_after_warm_bytes': rss_bytes(), 'process_peak_bytes': peak_bytes(),
                       'scope': 'Process RSS/peak includes Python, SDK, seeding and checks; not a native allocation count.'},
            'native': {'path': str(native), 'sha256': hashlib.sha256(native.read_bytes()).hexdigest()}}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--native', type=Path)
    p.add_argument('--output', type=Path)
    p.add_argument('--profile', required=True, choices=['release', 'debug'])
    p.add_argument('--iterations', type=int, default=60)
    p.add_argument('--warmup', type=int, default=10)
    p.add_argument('--cold-samples', type=int, default=20)
    p.add_argument('--sizes', default='1000,10000,100000')
    p.add_argument('--child')
    p.add_argument('--seed', type=Path, help='Worker-only: save an isolated KCAD for the GUI probe.')
    p.add_argument('--compare', type=Path, help='Require identical seed and output fingerprints to this baseline.')
    args = p.parse_args()
    if args.iterations < 20 or args.warmup < 0 or args.cold_samples < 20:
        p.error('Use at least 20 warm/cold samples and nonnegative warmup.')
    if args.native:
        args.native = args.native.resolve(strict=True)
    if args.child:
        print(json.dumps(worker(args), ensure_ascii=False, allow_nan=False))
        return
    if not args.output:
        p.error('--output is required for the matrix')
    sizes = [int(n) for n in args.sizes.split(',')]
    if not sizes or any(n < 0 or n > 100000 for n in sizes):
        p.error('--sizes must be within 0..100000')
    rows = []
    for kind in ('walls', 'lines'):
        for n in sizes:
            for density in ('separate', 'crowded'):
                for origin in ([0., 0.], [500000., 4400000.]):
                    case = {'kind': kind, 'background': n, 'density': density, 'origin': origin}
                    command = [sys.executable, str(Path(__file__).resolve()), '--profile', args.profile,
                               '--iterations', str(args.iterations), '--warmup', str(args.warmup),
                               '--cold-samples', str(args.cold_samples), '--child', json.dumps(case)]
                    if args.native:
                        command += ['--native', str(args.native)]
                    result = subprocess.run(command, capture_output=True, text=True, check=True)
                    rows.append(json.loads(result.stdout))
                    print(f'{kind}/{n}/{density}/{origin[0]:.0f}: {rows[-1]["rectangle"]["median_ms"]:.3f} ms', flush=True)
    report = {'format': 'kentos.architecture-moving-preview', 'version': 1,
              'started_utc': datetime.now(timezone.utc).isoformat(),
              'environment': {'platform': platform.platform(), 'machine': platform.machine(),
                              'logical_cpus': os.cpu_count(), 'python': sys.version,
                              'git_head': git_value('rev-parse', 'HEAD'), 'profile_declared': args.profile,
                              'harness_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()},
              'method': {'warmup': args.warmup, 'samples': args.iterations, 'cold_samples': args.cold_samples,
                         'cold_scope': 'Fresh document per cold sample; decode/teardown excluded, module already loaded.',
                         'paired_order': 'AB/BA alternating',
                         'timed': 'prebuilt DTO, SDK JSON/FFI/native plan/decode, outcome serialization',
                         'not_timed': ['seed/load', 'DTO construction', 'output hashing', 'save', 'GPU', 'window events'],
                         'p95': 'nearest rank; no timing thresholds', 'worker_per_case': True}, 'rows': rows}
    if args.compare:
        old = json.loads(args.compare.read_text())
        assert old['environment']['harness_sha256'] == report['environment']['harness_sha256']
        for key in ('platform', 'machine', 'profile_declared', 'python'):
            assert old['environment'][key] == report['environment'][key], key
        assert old['method'] == report['method']
        assert len(old['rows']) == len(rows)
        for a, b in zip(old['rows'], rows):
            assert (a['case'], a['input_sha256'], a['moving_outputs_sha256']) == (b['case'], b['input_sha256'], b['moving_outputs_sha256']), b['case']
        report['baseline_identical'] = True
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False)+'\n')


if __name__ == '__main__':
    main()
