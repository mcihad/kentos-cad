#!/usr/bin/env python3
"""Paired Python/FFI architectural preview comparison; no timing thresholds.

Run only after the current branch's native Python extension is ready:
    .run/py/bin/python scripts/perf/wall_rectangle.py --large --build-profile debug

The comparison is one physically identical closed wall, through two public SDK
commands. This does not measure GPU frame time or the full interactive UI loop.
"""
from __future__ import annotations

import argparse
import copy
import gc
import json
import math
import os
import platform
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from time import perf_counter_ns


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT = ROOT / ".run/architecture-preview-perf.json"


def git_value(*args: str) -> str | None:
    try:
        return subprocess.check_output(
            ["git", *args], cwd=ROOT, text=True, stderr=subprocess.DEVNULL
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def summary(samples_ns: list[int]) -> dict:
    ordered = sorted(samples_ns)
    return {
        "count": len(samples_ns),
        "median_ms": statistics.median(samples_ns) / 1_000_000,
        "p95_ms": ordered[math.ceil(0.95 * len(ordered)) - 1] / 1_000_000,
        "min_ms": ordered[0] / 1_000_000,
        "max_ms": ordered[-1] / 1_000_000,
        "samples_ms": [sample / 1_000_000 for sample in samples_ns],
    }


def seed_document(cad, kind: str, count: int):
    """SDK NewWall fields in a single legacy snapshot load, outside timings.

    This avoids making seed construction itself a benchmark of batch wall-join
    warnings or undo journals. The production document reader validates seeds.
    Every seed is distant from both the new room and every other seed.
    """
    fixture = json.loads(
        (ROOT / "fixtures/commands/v1/cad.walls.rectangle.json").read_text()
    )
    setup = copy.deepcopy(fixture["setup"])
    setup["name"] = f"Preview benchmark: {count} unrelated {kind}"
    setup["entities"] = []
    layer = setup["activeLayer"]
    for index in range(count):
        x = 1_000.0 + (index % 100) * 20.0
        y = 1_000.0 + (index // 100) * 20.0
        base = {"id": index + 1, "layerId": layer, "attrs": {}}
        if kind == "walls":
            wall = cad.NewWall(
                pts=[(x, y), (x + 4, y), (x + 4, y + 3), (x, y + 3)],
                closed=True, width=0.2, justify="left",
            )
            entity = {"kind": "wall", **base, **wall.to_json()}
        elif kind == "lines":
            entity = {"kind": "line", **base,
                      "a": {"x": x, "y": y}, "b": {"x": x + 4, "y": y}}
        else:
            raise ValueError(f"Unsupported seed kind: {kind}")
        setup["entities"].append(entity)
    doc = cad.Document.from_bytes(json.dumps(setup).encode())
    if len(doc) != count:
        raise RuntimeError("Seed document count differs from requested count")
    return doc


def compare(cad, kind: str, count: int, warmup: int, iterations: int) -> dict:
    seed_started = perf_counter_ns()
    doc = seed_document(cad, kind, count)
    seed_ms = (perf_counter_ns() - seed_started) / 1_000_000
    layer = doc.active_layer
    frame = cad.CornersRectangleFrame(a=(0, 0), b=(4, 3))
    wall = cad.NewWall(
        pts=[(0, 0), (4, 0), (4, 3), (0, 3)], closed=True,
        width=0.2, justify="left",
    )
    # Construct input geometry once; each public wrapper still creates its DTO,
    # serializes it, crosses FFI, runs native planning and decodes the response.
    calls = {
        "rectangle": lambda: cad.walls.rectangle.plan(
            doc, layer_id=layer, frame=frame, reference="inner", width=0.2
        ),
        "create": lambda: cad.walls.create.plan(
            doc, layer_id=layer, walls=[wall]
        ),
    }
    revision = doc.revision
    # Semantic checks are deliberately outside the measured samples.
    rectangle_plan = calls["rectangle"]()
    create_plan = calls["create"]()
    if rectangle_plan.to_json() != create_plan.to_json():
        raise RuntimeError("The two SDK plans are not the same closed wall")
    if len(rectangle_plan.walls) != 1:
        raise RuntimeError("A single-wall plan was expected")
    for _ in range(warmup):
        for invoke in calls.values():
            invoke()
    samples = {"rectangle": [], "create": []}
    for index in range(iterations):
        # Alternating paired order limits a one-command-first thermal/cache bias.
        order = ("rectangle", "create") if index % 2 == 0 else ("create", "rectangle")
        for command in order:
            start = perf_counter_ns()
            calls[command]()
            samples[command].append(perf_counter_ns() - start)
    if len(doc) != count or doc.revision != revision:
        raise RuntimeError("A plan call changed the seed document")
    result = {
        "background_kind": kind,
        "background_count": count,
        "seed_setup_ms_excluded": seed_ms,
        "identical_plans": True,
        "document_unchanged": True,
        "rectangle": summary(samples["rectangle"]),
        "create": summary(samples["create"]),
    }
    result["rectangle_to_create_median_ratio"] = (
        result["rectangle"]["median_ms"] / result["create"]["median_ms"]
    )
    return result


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--warmup", type=int, default=10)
    parser.add_argument("--iterations", type=int, default=30)
    parser.add_argument("--large", action="store_true",
                        help="Also measure separate 100,000-wall and 100,000-line documents")
    parser.add_argument("--build-profile", default="debug",
                        help="Declared extension profile; not inferred from Python flags")
    args = parser.parse_args()
    if args.warmup < 0 or args.iterations < 1:
        parser.error("warmup must be nonnegative and iterations positive")
    # Deliberately imported only on execution, once maturin readiness is confirmed.
    import kentos
    from kentos import cad, _native

    native_path = Path(_native.__file__).resolve()
    native_stat = native_path.stat()
    result = {
        "format": "kentos.architecture-preview-perf",
        "version": 1,
        "started_utc": datetime.now(timezone.utc).isoformat(),
        "environment": {
            "platform": platform.platform(), "machine": platform.machine(),
            "processor": platform.processor(), "logical_cpus": os.cpu_count(),
            "python": sys.version, "python_executable": sys.executable,
            "sdk_path": str(Path(kentos.__file__).resolve()),
            "native_path": str(native_path), "native_size_bytes": native_stat.st_size,
            "native_modified_ns": native_stat.st_mtime_ns,
            "kentos_version": kentos.__version__,
            "declared_build_profile": args.build_profile,
            "build_profile_source": "runner argument; not introspected from native binary",
            "python_gc_enabled": gc.isenabled(),
            "git_head": git_value("rev-parse", "HEAD"),
            "git_branch": git_value("branch", "--show-current"),
            "tracked_working_tree_changes": git_value("diff", "--stat"),
        },
        "method": {
            "warmup_each_command": args.warmup,
            "timed_each_command": args.iterations,
            "timer": "time.perf_counter_ns",
            "p95": "nearest rank ceil(0.95*n), no interpolation",
            "order": "paired AB then BA, alternating each iteration",
            "timing_thresholds": None,
            "includes": ["SDK DTO construction", "JSON serialization", "Python/native FFI",
                         "native validation and wall planning", "response decoding"],
            "excludes": ["seed generation/load", "initial geometry construction",
                         "GPU rendering", "event dispatch", "full UI preview loop"],
            "scope_note": "Current shared wall neighbourhood scans all document entities; this paired plan probe is not FPS certification.",
            "seed_method": "NewWall.to_json for walls; validated legacy snapshot Document.from_bytes once",
            "seed_layout_m": "20 m grid from (1000,1000), separated 4x3 m closed walls or 4 m lines",
            "target": {"path_m": [[0, 0], [4, 0], [4, 3], [0, 3]],
                       "closed": True, "width_m": 0.2, "justify": "left",
                       "rectangle_reference": "inner", "clear_area_m2": 12},
        },
        "scenarios": [],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    scenarios = [("walls", 0), ("walls", 1_000), ("walls", 10_000),
                 ("lines", 1_000), ("lines", 10_000)]
    if args.large:
        scenarios.extend([("walls", 100_000), ("lines", 100_000)])
    for kind, count in scenarios:
        measured = compare(cad, kind, count, args.warmup, args.iterations)
        result["scenarios"].append(measured)
        result["updated_utc"] = datetime.now(timezone.utc).isoformat()
        args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
        print(f"{count:>5} {kind}: rectangle median "
              f"{measured['rectangle']['median_ms']:.3f} ms, p95 "
              f"{measured['rectangle']['p95_ms']:.3f} ms; create median "
              f"{measured['create']['median_ms']:.3f} ms, p95 "
              f"{measured['create']['p95_ms']:.3f} ms", flush=True)
    print(args.output, flush=True)


if __name__ == "__main__":
    main()
