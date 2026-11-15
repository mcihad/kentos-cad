"""``kentos.ui_panel`` (docs/adr/8057 §34): the panel components' builders, made from the panel's JSON Schema. Each
writes its component's JSON with only what was given (a default is left out); a choice's options may be values alone;
every component of the sample panels is written back the same by its builder; a builder's parameters are its
component's keys, the required ones first."""

from __future__ import annotations

import inspect
import json
import pathlib
import re
import unittest
from typing import Any

from kentos import ui_panel as ui

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / "crates/shared/extensions/sdk/kentos-ui-panel.schema.json").read_text(encoding="utf-8"))
KINDS = {k["properties"]["type"]["const"]: k for k in SCHEMA["$defs"]["Component"]["oneOf"]}
PANELS = sorted((ROOT / "samples").rglob("*.panel.json"))


def snake(key: str) -> str:
    return re.sub(r"[A-Z]", lambda m: "_" + m.group(0).lower(), key)


def rebuilt(component: dict[str, Any]) -> dict[str, Any]:
    """``component`` written again by its builder from its own keys, each by name."""
    build = getattr(ui, snake(component["type"]))
    return build(**{snake(k): v for k, v in component.items() if k != "type"})


def without_defaults(component: dict[str, Any]) -> dict[str, Any]:
    """``component`` without the yes-or-no keys written as their default (a builder leaves those out)."""
    props = KINDS[component["type"]]["properties"]
    return {
        k: v
        for k, v in component.items()
        if not (isinstance(v, bool) and props.get(k, {}).get("default", False) is v)
    }


class BuilderTests(unittest.TestCase):
    def test_a_builder_writes_only_what_was_given(self) -> None:
        self.assertEqual(
            ui.text_field("onek", "Önek", value="Y-"),
            {"type": "textField", "id": "onek", "label": "Önek", "value": "Y-"},
        )
        self.assertEqual(
            ui.number_field("ilk", "İlk numara", value=1, min=1, required=True),
            {"type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1, "min": 1, "required": True},
        )
        self.assertEqual(ui.text("Bir satır."), {"type": "text", "text": "Bir satır."})
        # A yes-or-no key is written only when it is not its default: multiple is true unless said.
        self.assertNotIn("multiple", ui.feature_picker("nesneler", "Nesneler"))
        self.assertIs(ui.feature_picker("nesneler", "Nesneler", multiple=False)["multiple"], False)
        self.assertEqual(ui.checkbox("tamam", "Tamam", value=True)["value"], True)
        self.assertNotIn("value", ui.checkbox("tamam", "Tamam"))
        # A rule under its JSON key.
        self.assertEqual(
            ui.date_field("gun", "Gün", visible_when="içerir(bilgiler, 'tarih')")["visibleWhen"],
            "içerir(bilgiler, 'tarih')",
        )
        self.assertEqual(ui.button("ara", "Ara", "yapi-ara", primary=True)["command"], "yapi-ara")
        self.assertEqual(ui.chart("grafik", kind="pie", aggregate="sum")["aggregate"], "sum")
        self.assertEqual(
            ui.panel("Yapı ara", [ui.table("sonuc")]),
            {"format": "kentos.ui.panel", "schemaVersion": 1, "title": "Yapı ara",
             "body": [{"type": "table", "id": "sonuc"}]},
        )

    def test_a_choices_options_may_be_values_alone(self) -> None:
        options = ui.select("tur", "Tür", ["Okul", ui.choice("lise", "Lise"), {"value": "kres"}])["options"]
        self.assertEqual(
            options,
            [{"value": "Okul", "label": "Okul"}, {"value": "lise", "label": "Lise"}, {"value": "kres", "label": "kres"}],
        )
        self.assertEqual(ui.choice(3), {"value": "3", "label": "3"})
        self.assertEqual(ui.multi_select("bilgiler", "Bilgiler", ("alan", "tarih"), value=["alan"])["value"], ["alan"])

    def test_a_key_the_component_does_not_take_is_refused_by_python(self) -> None:
        with self.assertRaises(TypeError):
            ui.text_field("a", "b", colour="kırmızı")  # type: ignore[call-arg]
        with self.assertRaises(TypeError):
            ui.select("tur", "Tür")  # type: ignore[call-arg]

    def test_every_component_of_the_sample_panels_is_written_back_the_same(self) -> None:
        self.assertTrue(PANELS, "the samples have panels")
        seen: set[str] = set()
        for path in PANELS:
            panel = json.loads(path.read_text(encoding="utf-8"))
            for component in panel["body"]:
                seen.add(component["type"])
                with self.subTest(panel=path.name, component=component.get("id") or component["type"]):
                    self.assertEqual(rebuilt(component), without_defaults(component))
            self.assertEqual(
                ui.panel(panel["title"], [rebuilt(c) for c in panel["body"]]),
                {**panel, "body": [without_defaults(c) for c in panel["body"]]},
            )
        self.assertGreaterEqual(len(seen), 8, seen)

    def test_a_builders_parameters_are_its_components_keys(self) -> None:
        for kind, schema in KINDS.items():
            with self.subTest(kind=kind):
                build = getattr(ui, snake(kind))
                params = inspect.signature(build).parameters
                keys = {snake(k) for k in schema["properties"] if k != "type"}
                self.assertEqual(set(params), keys)
                positional = [n for n, p in params.items() if p.kind is p.POSITIONAL_OR_KEYWORD]
                required = {snake(k) for k in schema.get("required", []) if k != "type"}
                self.assertEqual(set(positional), required)
                self.assertIn(snake(kind), ui.__all__)


if __name__ == "__main__":
    unittest.main()
