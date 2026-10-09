"""kentos.temporal (docs/adr/0210 §11): the core's time rules against the independent reference's cases
(fixtures/temporal/v1/cases.json, scripts/fixtures/temporal_cases.py), the way Python reads them; then the traces'
scene (fixtures/interaction/v1/temporal.kcad) as Python asks it: the temporal layers, their objects' times, what a
moment and a period show, the slider as the apps lay it out, and the two commands with their undo steps."""

from __future__ import annotations

import json
import math
import unittest
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from kentos import _native, cad, temporal
from kentos.cad.types import LayerTime
from kentos.temporal import Step, Time, TimeError

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/temporal/v1/cases.json").read_text(encoding="utf-8"))
SCENE = ROOT / "fixtures/interaction/v1/temporal.kcad"
EPOCH = datetime(1970, 1, 1)


def at(ms: float) -> datetime:
    return EPOCH + timedelta(milliseconds=int(ms))


def bound(v: Any) -> float:
    """A case's start or end: a number, or ±∞ written as text."""
    return {"-inf": -math.inf, "inf": math.inf}.get(v, v) if isinstance(v, str) else float(v)


class Cases(unittest.TestCase):
    def test_values_read_as_the_reference_reads_them(self) -> None:
        for c in CASES["reads"]:
            with self.subTest(text=c["text"]):
                kind, t = _native.time_read(c["text"])
                if c["expect"] == "empty":
                    self.assertEqual(kind, 0)
                    self.assertIsNone(temporal.read(c["text"]))
                elif c["expect"] == "unreadable":
                    self.assertEqual(kind, 2)
                    with self.assertRaises(TimeError):
                        temporal.read(c["text"])
                else:
                    self.assertEqual((kind, t), (1, c["expect"]))
                    self.assertEqual(temporal.read(c["text"]), at(c["expect"]))

    def test_moments_write_and_show(self) -> None:
        for c in CASES["writes"]:
            with self.subTest(write=c):
                self.assertEqual(temporal.write(at(c["ms"]), date_only=c["dateOnly"]), c["expect"])
        for c in CASES["shows"]:
            with self.subTest(show=c):
                self.assertEqual(temporal.show(at(c["ms"]), c["unit"]), c["expect"])
        for c in CASES["floors"]:
            with self.subTest(floor=c):
                self.assertEqual(_native.time_floor(c["ms"], c["unit"]), c["expect"])

    def test_steps_positions_and_the_opening_step(self) -> None:
        for c in CASES["steps"]:
            with self.subTest(step=c):
                self.assertEqual(_native.time_position(c["anchor"], c["n"], c["unit"], c["k"]), c["expect"])
        for c in CASES["positions"]:
            with self.subTest(positions=c):
                got = _native.time_positions(c["extent"][0], c["extent"][1], c["n"], c["unit"])
                want = c["expect"]
                self.assertEqual(got, None if want is None else (want["anchor"], want["k"]))
        for c in CASES["autoSteps"]:
            with self.subTest(auto=c):
                self.assertEqual(_native.time_auto_step(*c["extent"]), (c["expect"]["n"], c["expect"]["unit"]))

    def test_objects_times_and_their_windows(self) -> None:
        for c in CASES["times"]:
            with self.subTest(start=c["start"], end=c.get("end"), rule=c["rule"]):
                rule = c["rule"]
                got = _native.time_of(rule["ranged"], rule["cumulative"], c["start"], c.get("end"))
                want = c["expect"]
                if want is None:
                    self.assertIsNone(got)
                    continue
                self.assertEqual(got, (bound(want["s"]), bound(want["e"]), want["mode"]))
                t = Time._of(got)
                assert t is not None
                for w in c["windows"]:
                    win = w["window"]
                    if "instant" in win:
                        self.assertEqual(t.shows(at(win["instant"])), w["expect"], win)
                    else:
                        a, b = win["range"]
                        self.assertEqual(t.shows(between=(at(a), at(b))), w["expect"], win)

    def test_a_layers_values_and_what_they_give(self) -> None:
        for c in CASES["summaries"]:
            with self.subTest(rule=c["rule"]):
                values = [(a, b) for a, b in c["values"]]
                _, ((timed, timeless, unreadable), span) = _native.time_values(c["rule"]["ranged"], c["rule"]["cumulative"], values)
                want = c["expect"]
                self.assertEqual((timed, timeless, unreadable), (want["timed"], want["timeless"], want["unreadable"]))
                self.assertEqual(span, None if want["extent"] is None else tuple(want["extent"]))

    def test_moments_given_as_python_gives_them(self) -> None:
        self.assertEqual(temporal.write(date(2018, 6, 1)), "2018-06-01")
        self.assertEqual(temporal.write("01.06.2018 14:30"), "2018-06-01T14:30:00")
        aware = datetime(2020, 9, 30, 14, 30, tzinfo=timezone(timedelta(hours=3)))
        self.assertEqual(temporal.write(aware), "2020-09-30T11:30:00")
        self.assertEqual(temporal.read("2020-09-30T14:30:00+03:00"), datetime(2020, 9, 30, 11, 30))
        self.assertEqual(temporal.show(datetime(2020, 9, 30, 14, 30, 5), "second"), "30.09.2020 14:30:05")
        self.assertEqual(temporal.write(datetime(2020, 1, 2, 3, 4, 5, 678_901)), "2020-01-02T03:04:05.678")
        with self.assertRaises(TimeError) as e:
            temporal.write("yarın")
        self.assertIn("okunamadı", str(e.exception))
        with self.assertRaises(TimeError):
            temporal.show(date(2020, 1, 1), "fortnight")  # type: ignore[arg-type]
        with self.assertRaises(TimeError):
            Time(None, None, "range").shows(date(2020, 1, 1), between=(date(2020, 1, 1), date(2021, 1, 1)))


class Scene(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.open(SCENE)
        self.ids = {}
        for r in self.doc.entities():
            attrs = r.entity.attrs or {}
            name = attrs.get("parsel_no") or attrs.get("yapim_tarihi") or attrs.get("tarih") or r.entity.layer_id
            self.ids[r.uid] = name

    def names(self, uids: list[str]) -> list[str]:
        return [self.ids[u] for u in uids]

    def test_the_temporal_layers_and_their_times(self) -> None:
        self.assertEqual([n.id for n in temporal.temporal_layers(self.doc)], ["parsel", "bina", "ariza"])
        parcels = temporal.times(self.doc, "Parsel")
        self.assertEqual((parcels.timed, parcels.timeless, parcels.unreadable), (6, 0, 0))
        self.assertEqual(parcels.extent, (datetime(2010, 3, 5), datetime(2022, 4, 1)))
        self.assertEqual(parcels.rule.key, "parsel_no")
        t = [v for u, v in parcels.times.items() if self.ids[u] == "101"][0]
        self.assertEqual(t, Time(datetime(2010, 3, 5), datetime(2018, 6, 1), "range"))
        open_ = [v for u, v in parcels.times.items() if self.ids[u] == "102"][0]
        self.assertEqual(open_, Time(datetime(2012, 1, 1), None, "range"))
        buildings = temporal.times(self.doc, "bina")
        self.assertEqual({v.mode for v in buildings.times.values() if v}, {"cumulative"})
        self.assertEqual(temporal.extent(self.doc), (datetime(2010, 3, 5), datetime(2023, 2, 15)))
        with self.assertRaises(TimeError) as e:
            temporal.times(self.doc, "yol")
        self.assertIn("zamansal değil", str(e.exception))

    def test_what_a_moment_and_a_period_show(self) -> None:
        self.assertEqual(self.names(temporal.shown(self.doc, date(2015, 1, 1), layers=["parsel"])), ["101", "102"])
        self.assertEqual(
            self.names(temporal.shown(self.doc, date(2024, 1, 1), layers=["parsel"])), ["101/1", "101/2", "102", "104"]
        )
        # Every temporal layer: the buildings built by then, a breakdown only on its day.
        self.assertEqual(
            self.names(temporal.shown(self.doc, date(2016, 4, 12))), ["101", "102", "103", "2013-05-20", "2016-04-12"]
        )
        within = temporal.shown(self.doc, between=(date(2018, 1, 1), date(2019, 1, 1)), layers=["parsel"])
        self.assertEqual(self.names(within), ["101", "101/1", "101/2", "102", "103"])

    def test_the_slider_as_the_apps_open_it(self) -> None:
        s = temporal.slider(self.doc)
        self.assertEqual(s.step, Step(1, "year"))
        self.assertEqual(str(s.step), "1 yıl")
        self.assertEqual(s.positions[0], datetime(2010, 1, 1))
        self.assertEqual(s.positions[-1], datetime(2024, 1, 1))
        self.assertEqual(s.last, 14)
        self.assertFalse(s.ranged)  # Parsel is ranged: moments.
        self.assertEqual(s.label(s.last), "01.01.2024")
        k = s.positions.index(datetime(2015, 1, 1))
        self.assertEqual(self.names(s.shown(self.doc, k)), ["101", "102", "2013-05-20"])
        # From the first breakdown's month on, half a year each.
        months = temporal.slider(self.doc, step=(6, "month"), ranged=True, layers=["ariza"])
        self.assertEqual(months.label(0), "01.04.2016 – 01.10.2016")
        self.assertEqual(months.last, 9)
        self.assertEqual(months.layers, ["ariza"])
        self.assertEqual(self.names(months.shown(self.doc, 0)), ["2016-04-12"])
        with self.assertRaises(TimeError) as e:
            temporal.slider(self.doc, step=(1, "second"))
        self.assertIn("100 000", str(e.exception))
        with self.assertRaises(TimeError):
            s.window(15)

    def test_the_commands_write_one_step_each(self) -> None:
        done = temporal.set_time(self.doc, "yol", "acilis_tarihi", cumulative=True)
        self.assertTrue(done.changed)
        self.assertEqual(self.names(temporal.shown(self.doc, date(2012, 1, 1), layers=["yol"])), ["yol"])
        self.assertEqual(temporal.shown(self.doc, date(2011, 1, 1), layers=["yol"]), [])
        self.assertEqual(self.doc.undo(), "Zaman ayarları")
        self.assertNotIsInstance(self.doc.layer("yol").time, LayerTime)
        temporal.clear_time(self.doc, "ariza")
        self.assertEqual([n.id for n in temporal.temporal_layers(self.doc)], ["parsel", "bina"])
        self.doc.undo()
        with self.assertRaises(cad.CommandError) as e:
            temporal.set_time(self.doc, "parsel", "gecerlilik_baslangic", "gecerlilik_baslangic")
        self.assertIn("aynı alan", str(e.exception))

        made = temporal.create_scenario(self.doc, "Öneri B", ["parsel"], note="Parsel düzeni")
        group = self.doc.layer(made.scenario)
        self.assertEqual(group.name, "Öneri B")
        self.assertEqual(made.objects, 6)
        pair = made.layers[0]
        self.assertEqual(pair.base, "parsel")
        self.assertEqual(self.doc.layer(pair.layer).replaces, "parsel")
        applied = temporal.apply_scenario(self.doc, made.scenario)
        self.assertEqual((applied.objects, applied.removed), (6, 6))
        self.assertEqual(self.doc.undo(), "Senaryoyu uygula")
        self.assertEqual(self.doc.undo(), "Senaryo oluştur")


if __name__ == "__main__":
    unittest.main()
