#!/usr/bin/env python3
"""The tables of a large mixed drawing's measurement (CAD-ADV-19, docs/adr/1062), from the JSON that
`perf::large::large` writes (`docs/perf/large-desktop-<label>-<date>.json`), with no number typed by hand:

    python3 scripts/perf/large_report.py docs/perf/large-desktop-sonra-2026-11-06.json [before.json] > out.md

With a second file, the frames' table shows both (önce → sonra). Only the standard library."""

import json
import sys


def ms(v):
    return "–" if v is None else f"{v:.1f}".replace(".", ",")


def rows_by(report):
    return {r["objects"]: r for r in report["rows"]}


def frames(row):
    return {(f["run"], f["view"], f["trace"]): f["stats"] for f in row["measured"]["frames"]}


def main():
    after = json.load(open(sys.argv[1]))
    before = json.load(open(sys.argv[2])) if len(sys.argv) > 2 else None
    env, gpu = after["environment"], after["gpu"]
    out = []
    p = out.append
    p(f"Ortam: {env['machineText']}; GPU {gpu.get('name')} ({gpu.get('backend')}, {gpu.get('driverInfo') or gpu.get('driver')}); "
      f"commit `{env['commit']}`{' (kaydedilmemiş değişiklikle)' if env['dirtyTree'] else ''}; {after['setup']['profile']}; "
      f"pencere {after['setup']['window'][0]:.0f}×{after['setup']['window'][1]:.0f}; tohum {after['setup']['seed']}.")
    p("")
    p("| Nesne | Dosya | Parmak izi | İş parçacığı (oku, çöz, belge) | İlk kare MİB / GPU | Açılıştan ilk kareye | Açılışta karelerin MİB p95 |")
    p("|---:|---:|---|---:|---:|---:|---:|")
    for n, r in sorted(rows_by(after).items()):
        m, d = r["measured"]["open"], r["drawing"]
        p(f"| {n} | {d['bytes'] / 1e6:.1f} MB | `{d['fingerprint']}` | {ms(m['threadMs'])} | {ms(m['firstFrame']['uiMs'])} / {ms(m['firstFrame']['gpuMs'])} "
          f"| {ms(m['toFirstFrameMs'])} | {ms(m['framesWhileOpening']['uiP95'])} |")
    p("")
    p("İlk etkileşim (açılıştan sonraki ilk girdi), MİB / GPU ms:")
    p("")
    p("| Nesne | İmleç | Tekerlek | Kaydırma |")
    p("|---:|---:|---:|---:|")
    for n, r in sorted(rows_by(after).items()):
        f = r["measured"]["firstInteraction"]
        p(f"| {n} | {ms(f['pointer']['uiMs'])} / {ms(f['pointer']['gpuMs'])} | {ms(f['wheel']['uiMs'])} / {ms(f['wheel']['gpuMs'])} "
          f"| {ms(f['pan']['uiMs'])} / {ms(f['pan']['gpuMs'])} |")
    p("")
    p("Kareler (MİB: arayüz iş parçacığı; GPU: çizim ve geri okuma). Bütçe 16,7 ms (60 Hz).")
    p("")
    head = "| Nesne | Koşu | Görünüm | İz | Kare | MİB p50 | MİB p95 | MİB p99 | Bütçeyi aşan | GPU p50 | GPU p95 | GPU p99 |"
    if before:
        head = "| Nesne | Koşu | Görünüm | İz | Kare | MİB p95 önce → sonra | MİB p99 sonra | Bütçeyi aşan önce → sonra | GPU p50 önce → sonra | GPU p95 sonra |"
    p(head)
    p("|" + "---|" * (head.count("|") - 1))
    prior = rows_by(before) if before else {}
    for n, r in sorted(rows_by(after).items()):
        old = frames(prior[n]) if n in prior else {}
        for key, s in frames(r).items():
            run, view, trace = key
            if before:
                o = old.get(key, {})
                p(f"| {n} | {run} | {view} | {trace} | {s['frames']} | {ms(o.get('uiP95'))} → {ms(s['uiP95'])} | {ms(s['uiP99'])} "
                  f"| {o.get('uiOverBudget', '–')} → {s['uiOverBudget']} | {ms(o.get('gpuP50'))} → {ms(s['gpuP50'])} | {ms(s['gpuP95'])} |")
            else:
                p(f"| {n} | {run} | {view} | {trace} | {s['frames']} | {ms(s['uiP50'])} | {ms(s['uiP95'])} | {ms(s['uiP99'])} | {s['uiOverBudget']} "
                  f"| {ms(s['gpuP50'])} | {ms(s['gpuP95'])} | {ms(s['gpuP99'])} |")
    p("")
    p("Geri okuma tabanı (boş pencerenin karesi, aynı yolla), GPU p50: "
      + ", ".join(f"{n}: {ms(r['measured']['readbackFloor']['gpuP50'])} ms" for n, r in sorted(rows_by(after).items())) + ".")
    p("")
    p("Yazı girişi (bir tuşun karesi; yerel yankı = MİB + GPU), ms:")
    p("")
    p("| Nesne | Komut satırı MİB p95 / GPU p95 | Değer alanı MİB p95 / GPU p95 | En kötü yankı (MİB en çok + GPU en çok) |")
    p("|---:|---:|---:|---:|")
    for n, r in sorted(rows_by(after).items()):
        k = r["measured"]["keys"]
        c, v = k["commandLine"], k["valueField"]
        worst = max(c["uiMax"] + c["gpuMax"], v["uiMax"] + v["gpuMax"])
        p(f"| {n} | {ms(c['uiP95'])} / {ms(c['gpuP95'])} | {ms(v['uiP95'])} / {ms(v['gpuP95'])} | {ms(worst)} |")
    p("")
    p("Komutlar (bir kare; parçalar: yeniden kurulan stilli katman parçası / bütün parçalar), ms:")
    p("")
    p("| Nesne | Komut | MİB | GPU | Kurulan parça | Kurma |")
    p("|---:|---|---:|---:|---:|---:|")
    for n, r in sorted(rows_by(after).items()):
        for c in r["measured"]["commands"]:
            parts = f"{c['partsBuilt']} / {c['partsTotal']}" if "partsTotal" in c else "–"
            p(f"| {n} | {c['command']} | {ms(c['uiMs'])} | {ms(c['gpuMs'])} | {parts} | {ms(c.get('buildMs'))} |")
    p("")
    p("İptal (ikinci açılış, okurken Esc), ms:")
    p("")
    p("| Nesne | Esc'nin karesi MİB + GPU | Pencere o karede kapandı | İş parçacığının durması (Esc'den sonra) | Hiçbir şey uygulanmadı |")
    p("|---:|---:|---|---:|---|")
    for n, r in sorted(rows_by(after).items()):
        c = r["measured"]["cancel"]
        if not c.get("stopped"):
            p(f"| {n} | – | – | – | {c.get('note')} |")
            continue
        p(f"| {n} | {ms(c['escUiPlusGpuMs'])} | {'evet' if c['windowGoneOnEscFrame'] else 'hayır'} | {ms(c['threadStoppedAfterMs'])} "
          f"| {'evet' if c['threadGaveNothing'] else 'hayır'} |")
    p("")
    p("Bellek, MB (`footprint`: fiziksel ayak izi ve sürecin tepesi; GPU: IOAccelerator; görünümün GPU tamponları `FrameStats`):")
    p("")
    p("| Nesne | Açılışta yerleşik | Açılışta ayak izi / tepe | Sonda ayak izi / tepe | GPU (IOAccelerator) açılış / son | Görünümün tamponları |")
    p("|---:|---:|---:|---:|---:|---:|")
    for n, r in sorted(rows_by(after).items()):
        m = r["measured"]["memory"]
        fo, fe = m["footprintOpen"], m["footprintEnd"]
        v = r["measured"]["viewFar"]
        p(f"| {n} | {m['residentOpenMb']:.0f} | {fo['physFootprintMb']:.0f} / {fo['physFootprintPeakMb']:.0f} | {fe['physFootprintMb']:.0f} / {fe['physFootprintPeakMb']:.0f} "
          f"| {fo['gpuMemoryMb']:.0f} / {fe['gpuMemoryMb']:.0f} | {(v['residentBytes'] + v['targetBytes']) / 1e6:.0f} |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
