#!/usr/bin/env python3
"""ARCH-PERF-01: RSS during repeated index invalidation in one large document.

This is separate from the cold-reload matrix: no reload, no timing claim.
Use the same saved release modules as architecture_preview.py, one process each.
"""
import argparse
import hashlib
import json
from pathlib import Path

import architecture_preview as probe


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    cad, native = probe.sdk(args.native.resolve(strict=True))
    case = {'kind': 'walls', 'background': 100000, 'density': 'separate',
            'origin': [500000., 4400000.]}
    doc, seed_hash, _ = probe.setup(cad, case)
    uid = cad.walls.rectangle(
        doc, layer_id=doc.active_layer, width=.2,
        frame=cad.CornersRectangleFrame(a=(500000, 4400000), b=(500006, 4400004))).uids[0]
    before = doc.entity(uid).entity
    request = cad.WallsRectangle(
        layer_id=doc.active_layer, width=.2,
        frame=cad.CornersRectangleFrame(a=(500007, 4400000), b=(500011, 4400003)))
    samples = []
    for i in range(101):
        if i:
            cad.walls.edit(doc, uids=[uid], operation=cad.SetWallsOperation(width=.21))
            assert cad.walls.rectangle.run(doc, request, op='plan').ok
            doc.undo()
        assert cad.walls.rectangle.run(doc, request, op='plan').ok
        if i % 10 == 0:
            samples.append({'cycles': i, 'rss_bytes': probe.rss_bytes(),
                            'peak_bytes': probe.peak_bytes()})
    assert len(doc) == 100001 and doc.entity(uid).entity == before
    report = {
        'format': 'kentos.architecture-preview-memory-cycle', 'version': 1,
        'native': {'path': str(native), 'sha256': hashlib.sha256(native.read_bytes()).hexdigest()},
        'input_sha256': seed_hash, 'entities': len(doc), 'edited_wall_restored': True,
        'method': '100 width edits, plans, undo, plans; no document reload. RSS includes SDK, seed, history and allocator; not a native allocation count. No timing claim.',
        'samples': samples,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
