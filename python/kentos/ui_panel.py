# kentos.ui_panel: the panel components' builders (docs/adr/8057 §34). Generated from the panel's JSON Schema by
# `KENTOS_WRITE_SDK=1 cargo test -p kentos-extensions --lib -- python_builders`; do not edit.
"""Builders of ``kentos.ui.panel`` 1 components: each returns the component's JSON, for ``ctx.dialog``'s fields,
``ctx.panel.set(options=…)`` or a panel written from Python (:func:`panel`). Required keys come in order, the rest
by name; a choice's options may be values alone (``["a", "b"]``). Kapı 1 still judges what a schema cannot say
(a button's command, ids once, a rule's names)."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, Literal, TypedDict

FORMAT = "kentos.ui.panel"
SCHEMA_VERSION = 1


class Choice(TypedDict):
    """One of a choice's options."""

    value: str
    label: str


def _choices(options: Sequence[Choice | str]) -> list[dict[str, str]]:
    return [
        {"value": o, "label": o} if isinstance(o, str) else {"value": o["value"], "label": o.get("label", o["value"])}
        for o in options
    ]


def choice(value: str, label: str | None = None) -> Choice:
    """One option of a ``select`` or a ``multiSelect``, its label the value when none is given."""
    return {"value": str(value), "label": str(value if label is None else label)}


def file_filter(name: str, *extensions: str) -> dict[str, Any]:
    """A kind of file a ``filePicker`` offers: a name, extensions without dots (``file_filter("CSV", "csv")``)."""
    return {"name": str(name), "extensions": [str(e).lstrip(".").lower() for e in extensions]}


def panel(title: str, body: Sequence[dict[str, Any]]) -> dict[str, Any]:
    """A whole panel (``ui/<id>.panel.json``): its title and components."""
    return {"format": FORMAT, "schemaVersion": SCHEMA_VERSION, "title": str(title), "body": list(body)}


def text(
    text: str,
    *,
    id: str | None = None,
    visible_when: str | None = None,
    calculate: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A line of text."""
    out: dict[str, Any] = {"type": "text", "text": text}
    if id is not None:
        out["id"] = id
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if calculate is not None:
        out["calculate"] = calculate
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def section(
    label: str,
    *,
    id: str | None = None,
    collapsed: bool = False,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """A heading that gathers the components after it, up to the next section: the user folds it to hide them
    (`collapsed`: folded when the panel opens). A folded section's values still go to the commands, and one of its
    fields left wrong opens it."""
    out: dict[str, Any] = {"type": "section", "label": label}
    if id is not None:
        out["id"] = id
    if collapsed:
        out["collapsed"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def divider(*, id: str | None = None, visible_when: str | None = None, tab: str | None = None) -> dict[str, Any]:
    """A line across the panel between what it separates (docs/adr/8057 §59): no words, no value."""
    out: dict[str, Any] = {"type": "divider"}
    if id is not None:
        out["id"] = id
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def text_field(
    id: str,
    label: str,
    *,
    placeholder: str | None = None,
    value: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    calculate: str | None = None,
    default_expr: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A text the user types."""
    out: dict[str, Any] = {"type": "textField", "id": id, "label": label}
    if placeholder is not None:
        out["placeholder"] = placeholder
    if value is not None:
        out["value"] = value
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if calculate is not None:
        out["calculate"] = calculate
    if default_expr is not None:
        out["defaultExpr"] = default_expr
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def number_field(
    id: str,
    label: str,
    *,
    value: float | None = None,
    min: float | None = None,
    max: float | None = None,
    unit: str | None = None,
    step: float | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    calculate: str | None = None,
    default_expr: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A number the user types, in its bounds."""
    out: dict[str, Any] = {"type": "numberField", "id": id, "label": label}
    if value is not None:
        out["value"] = value
    if min is not None:
        out["min"] = min
    if max is not None:
        out["max"] = max
    if unit is not None:
        out["unit"] = unit
    if step is not None:
        out["step"] = step
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if calculate is not None:
        out["calculate"] = calculate
    if default_expr is not None:
        out["defaultExpr"] = default_expr
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def select(
    id: str,
    label: str,
    options: Sequence[Choice | str],
    *,
    value: str | None = None,
    on_change: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """One of its choices."""
    out: dict[str, Any] = {"type": "select", "id": id, "label": label, "options": _choices(options)}
    if value is not None:
        out["value"] = value
    if on_change is not None:
        out["onChange"] = on_change
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def layer_picker(
    id: str,
    label: str,
    *,
    geometry: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """One of the open drawing's layers, of `geometry`'s type (and the untyped ones) when given: its id."""
    out: dict[str, Any] = {"type": "layerPicker", "id": id, "label": label}
    if geometry is not None:
        out["geometry"] = geometry
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def field_picker(
    id: str,
    label: str,
    layer: str,
    *,
    kinds: Sequence[str] | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """One of the fields of the layer the panel's layer picker `layer` holds, of `kinds` only when given (the layer
    schema's words): its name, none when none is chosen."""
    out: dict[str, Any] = {"type": "fieldPicker", "id": id, "label": label, "layer": layer}
    if kinds is not None:
        out["kinds"] = list(kinds)
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def file_picker(
    id: str,
    label: str,
    *,
    filters: Sequence[dict[str, Any]] | None = None,
    placeholder: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    save: bool = False,
    default_name: str | None = None,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A file the user gives to be read: its path typed, or chosen in the computer's file window (Seç…), of
    `filters`' kinds when given. Its value is the file's absolute path, none until one is given; a path that is no
    file, or of another kind, is said by its label and nothing runs. Its extension asks for `files.userSelected`.
    With `save`, where a file is to be written (docs/adr/8057 §44): the file need not be there, its folder must; a
    name without an extension takes the first kind's; its extension asks for `files.userSelectedWrite`."""
    out: dict[str, Any] = {"type": "filePicker", "id": id, "label": label}
    if filters is not None:
        out["filters"] = list(filters)
    if placeholder is not None:
        out["placeholder"] = placeholder
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if save:
        out["save"] = True
    if default_name is not None:
        out["defaultName"] = default_name
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def crs_picker(
    id: str,
    label: str,
    *,
    value: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A coordinate system of KentOS's list (docs/adr/8057 §45): its value `EPSG:<code>`, `value` when given, else
    the open drawing's own (none for a drawing without one). The code reads it as it is (`pyproj.CRS(value)` too);
    KentOS's `cad.crs.transform` takes its code (`CrsSide(srid=…)`)."""
    out: dict[str, Any] = {"type": "crsPicker", "id": id, "label": label}
    if value is not None:
        out["value"] = value
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def point_picker(
    id: str,
    label: str,
    *,
    value: Sequence[float] | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A point of the drawing (docs/adr/8057 §57): typed as `x, y` or picked on the map with "Haritadan seç" (snaps
    apply); its value `[x, y]` in the drawing's plane, metres, none when empty."""
    out: dict[str, Any] = {"type": "pointPicker", "id": id, "label": label}
    if value is not None:
        out["value"] = list(value)
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def feature_picker(
    id: str,
    label: str,
    *,
    layer: str | None = None,
    geometry: str | None = None,
    multiple: bool = True,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """The objects selected in the drawing, taken when the user asks: of the layer the panel's layer picker `layer`
    holds and of `geometry`'s type, when given. Its value is their persistent ids (`multiple`, the default), else one
    id or none."""
    out: dict[str, Any] = {"type": "featurePicker", "id": id, "label": label}
    if layer is not None:
        out["layer"] = layer
    if geometry is not None:
        out["geometry"] = geometry
    if not multiple:
        out["multiple"] = False
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    return out


def multi_select(
    id: str,
    label: str,
    options: Sequence[Choice | str],
    *,
    value: Sequence[str] | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """Several of its options the user picks, each a chip that toggles: its value is the chosen options' values in
    the options' order, an empty list when none is chosen."""
    out: dict[str, Any] = {"type": "multiSelect", "id": id, "label": label, "options": _choices(options)}
    if value is not None:
        out["value"] = list(value)
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def date_field(
    id: str,
    label: str,
    *,
    value: str | None = None,
    min: str | None = None,
    max: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    calculate: str | None = None,
    default_expr: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A date the user types (GG.AA.YYYY or YYYY-AA-GG) or picks from KentOS's calendar; its value is the ISO date
    (`2026-11-15`), none when empty. `value`, `min` and `max` are ISO dates."""
    out: dict[str, Any] = {"type": "dateField", "id": id, "label": label}
    if value is not None:
        out["value"] = value
    if min is not None:
        out["min"] = min
    if max is not None:
        out["max"] = max
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if calculate is not None:
        out["calculate"] = calculate
    if default_expr is not None:
        out["defaultExpr"] = default_expr
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def time_field(
    id: str,
    label: str,
    *,
    value: str | None = None,
    min: str | None = None,
    max: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    calculate: str | None = None,
    default_expr: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A time of day the user types (14:30, 14.30, 9:05) or picks from KentOS's time picker; its value is `HH:MM` (24
    hours), none when empty. `value`, `min` and `max` are `HH:MM`."""
    out: dict[str, Any] = {"type": "timeField", "id": id, "label": label}
    if value is not None:
        out["value"] = value
    if min is not None:
        out["min"] = min
    if max is not None:
        out["max"] = max
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if calculate is not None:
        out["calculate"] = calculate
    if default_expr is not None:
        out["defaultExpr"] = default_expr
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def date_time_field(
    id: str,
    label: str,
    *,
    value: str | None = None,
    min: str | None = None,
    max: str | None = None,
    on_change: str | None = None,
    required: bool = False,
    visible_when: str | None = None,
    required_when: str | None = None,
    enabled_when: str | None = None,
    calculate: str | None = None,
    default_expr: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """A day and a time the user types (`15.11.2026 14:30`, `2026-11-15 14:30`) or picks from KentOS's calendar and
    clock; its value is the local ISO date and time, `2026-11-15T14:30` (no zone: the user's own clock), none when
    empty. `value`, `min` and `max` are written so."""
    out: dict[str, Any] = {"type": "dateTimeField", "id": id, "label": label}
    if value is not None:
        out["value"] = value
    if min is not None:
        out["min"] = min
    if max is not None:
        out["max"] = max
    if on_change is not None:
        out["onChange"] = on_change
    if required:
        out["required"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if required_when is not None:
        out["requiredWhen"] = required_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if calculate is not None:
        out["calculate"] = calculate
    if default_expr is not None:
        out["defaultExpr"] = default_expr
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def checkbox(
    id: str,
    label: str,
    *,
    value: bool = False,
    on_change: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """Yes or no."""
    out: dict[str, Any] = {"type": "checkbox", "id": id, "label": label}
    if value:
        out["value"] = True
    if on_change is not None:
        out["onChange"] = on_change
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def button(
    id: str,
    label: str,
    command: str,
    *,
    table: str | None = None,
    primary: bool = False,
    visible_when: str | None = None,
    enabled_when: str | None = None,
    tab: str | None = None,
    row: str | None = None,
) -> dict[str, Any]:
    """Runs `command` with the panel's values; its rows fill `table` (none: the panel's first table)."""
    out: dict[str, Any] = {"type": "button", "id": id, "label": label, "command": command}
    if table is not None:
        out["table"] = table
    if primary:
        out["primary"] = True
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if enabled_when is not None:
        out["enabledWhen"] = enabled_when
    if tab is not None:
        out["tab"] = tab
    if row is not None:
        out["row"] = row
    return out


def table(
    id: str,
    *,
    label: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """The rows a button's command answered."""
    out: dict[str, Any] = {"type": "table", "id": id}
    if label is not None:
        out["label"] = label
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def tree(
    id: str,
    group: Sequence[str],
    *,
    label: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """The rows a button's command answered, grouped by `group`'s columns (one to three) into nodes that open and
    close, each with its count; a row with a `uid` is an object."""
    out: dict[str, Any] = {"type": "tree", "id": id, "group": list(group)}
    if label is not None:
        out["label"] = label
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def stats(
    id: str,
    *,
    label: str | None = None,
    title: str | None = None,
    value: str | None = None,
    unit: str | None = None,
    note: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """The rows a button's command answered, a figure each (KentOS's stat card): `title`'s column names it (none: the
    first column), `value`'s is its figure (none: the second), `unit`'s and `note`'s beside and under it; the cards
    side by side as the panel's width allows. A row with a `uid` is an object."""
    out: dict[str, Any] = {"type": "stats", "id": id}
    if label is not None:
        out["label"] = label
    if title is not None:
        out["title"] = title
    if value is not None:
        out["value"] = value
    if unit is not None:
        out["unit"] = unit
    if note is not None:
        out["note"] = note
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def chart(
    id: str,
    *,
    label: str | None = None,
    kind: Literal["bar", "pie", "histogram", "line"] | None = None,
    of: str | None = None,
    category: str | None = None,
    value: str | None = None,
    aggregate: Literal["count", "sum", "mean", "min", "max", "median"] | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """A chart of rows, drawn by KentOS's charts (the Grafik tab's engine): bars or a pie of `category`'s values,
    counted or with `value`'s column summed, averaged … (`aggregate`); a histogram of `value`'s numbers; a line of
    `value` over `category`. Its rows are the ones a button's `table` sends it or, with `of`, another component's (a
    table's, a tree's, a card's, a figure's): the same rows drawn twice, each its own way."""
    out: dict[str, Any] = {"type": "chart", "id": id}
    if label is not None:
        out["label"] = label
    if kind is not None:
        out["kind"] = kind
    if of is not None:
        out["of"] = of
    if category is not None:
        out["category"] = category
    if value is not None:
        out["value"] = value
    if aggregate is not None:
        out["aggregate"] = aggregate
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def image(
    id: str,
    *,
    label: str | None = None,
    alt: str | None = None,
    height: float | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """A picture its code gives (`ctx.panel.set(id, image=…)`, a PNG): a chart drawn in Python, a preview. `alt` says
    what it shows and stands in its place until the code gives one; `height` in pixels (80–600, none: 200), the
    picture fitted inside it."""
    out: dict[str, Any] = {"type": "image", "id": id}
    if label is not None:
        out["label"] = label
    if alt is not None:
        out["alt"] = alt
    if height is not None:
        out["height"] = height
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


def card(
    id: str,
    *,
    label: str | None = None,
    title: str | None = None,
    tag: str | None = None,
    visible_when: str | None = None,
    tab: str | None = None,
) -> dict[str, Any]:
    """The rows a button's command answered, a card each: `title`'s column heads it (none: the first column), `tag`'s
    beside it, the other columns under it by name; a card with a `uid` is an object. Where a table's columns do not
    fit (a narrow dock), a card writes them one under the other."""
    out: dict[str, Any] = {"type": "card", "id": id}
    if label is not None:
        out["label"] = label
    if title is not None:
        out["title"] = title
    if tag is not None:
        out["tag"] = tag
    if visible_when is not None:
        out["visibleWhen"] = visible_when
    if tab is not None:
        out["tab"] = tab
    return out


__all__ = [
    "Choice", "choice", "file_filter", "panel", "text", "section", "divider", "text_field", "number_field", "select",
    "layer_picker", "field_picker", "file_picker", "crs_picker", "point_picker", "feature_picker", "multi_select",
    "date_field", "time_field", "date_time_field", "checkbox", "button", "table", "tree", "stats", "chart", "image",
    "card",
]
