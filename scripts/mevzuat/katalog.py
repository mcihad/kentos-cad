#!/usr/bin/env python3
"""The 2026 digital project regulation's annexes as KentOS's rule catalog (plan/bim-plan.md BIM-01, plan/ifc-uyum.md
IFC-01, docs/adr/8062). Reads the annexes' transcription (mevzuat/bim/*.md: every PDF page marked, every table under
its own heading) and writes the base profile's rows, each with its annex, its table or clause, its row and its PDF
page:

- crates/shared/digital-project/data/temel-2026.json: EK-1…EK-4 and EK-8 (the project code, the files' names, the
  rooms' numbers and names, the CAD layers, blocks, fonts and marks, the development level);
- crates/shared/ifc/data/temel-2026.json: EK-5…EK-7 and EK-9 (the IFC classes and their codes, every class's
  attributes, properties and quantities, the project's classes, the model quality control forms).

    python3 scripts/mevzuat/katalog.py          # writes both
    python3 scripts/mevzuat/katalog.py --check  # fails when a written file is not what the transcription gives

It reads the transcription only and keeps its words as printed (the source's own misspellings too: a name the IFC 4.3
schema spells otherwise is kentos-ifc's schema comparison's to find and the decisions' to settle, kararlar-2026.json
in each crate). A table row or block it cannot read stops it: nothing is left out silently.
"""
from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "mevzuat" / "bim"
OUT_DIGITAL = ROOT / "crates" / "shared" / "digital-project" / "data" / "temel-2026.json"
OUT_IFC = ROOT / "crates" / "shared" / "ifc" / "data" / "temel-2026.json"

FILES = [
    "ek-1-proje-kodu.md",
    "ek-2-dosya-isimlendirme.md",
    "ek-3-mahal-isimlendirme.md",
    "ek-4-cad-cizim-hazirlama.md",
    "ek-5-ifc-siniflari-varlik-isimlendirme.md",
    "ek-6-1-zorunlu-a-f.md",
    "ek-6-2-zorunlu-f-z.md",
    "ek-6-3-istege-bagli-a-f.md",
    "ek-6-4-istege-bagli-f-z.md",
    "ek-7-proje-siniflari.md",
    "ek-8-gelisim-seviyesi.md",
    "ek-9-kalite-kontrol-formu.md",
]

PROFILE = {
    "id": "temel-2026",
    "name": "Temel profil",
    "version": "2026.1",
    "act": "Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik",
    "gazette": "5.8.2026/33331",
    "annexesPdf": {
        "pages": 241,
        "bytes": 22801668,
        "sha256": "7dc88971d3a7a8ae837ef148169fa5bf6c9ce7a01cc8fcd560ae3d342bb0ecb3",
    },
    "transcription": "mevzuat/bim",
}

PAGE = re.compile(r"^\*\[PDF sayfa (\d+)\]\*$")
TABLE = re.compile(r"^## Tablo (\d+)\.(\d+)\s*(.*)$")
CLAUSE = re.compile(r"^\*\*(\d+(?:\.\d+)*)\.?\s*-?\s*\*\*\s*(.*)$")
FIGURE = re.compile(r"^\*\*Şekil (\d+\.\d+)\*\*\s*(.*)$")
GROUP = re.compile(r"^\*\*([A-ZÇĞİÖŞÜ][A-ZÇĞİÖŞÜ ]+)\*\*$")
CONTINUED = "Tablo önceki sayfadan sürüyor"


class Unread(Exception):
    pass


def plain(s: str) -> str:
    """A cell's or a clause's words without the transcription's markdown: code marks, escapes, links, footnote marks."""
    s = s.replace("\\_", "_").replace("\\*", "*")
    s = re.sub(r"\[\[`?([^`\]]+)`?\]\]", r"\1", s)
    s = re.sub(r"\[`([^`\]]+)`\]", r"\1", s)
    s = s.replace("`", "")
    s = re.sub(r"\s+", " ", s)
    return s.strip()


def name_of(s: str) -> str:
    """An IFC name as a cell writes it: no code marks, no footnote stars, no spaces."""
    s = plain(s).rstrip("*").strip()
    return s


def cells_of(line: str) -> list[str]:
    inner = line.strip()
    assert inner.startswith("|") and inner.endswith("|"), line
    return [c.strip() for c in inner[1:-1].split("|")]


def is_separator(line: str) -> bool:
    return re.fullmatch(r"\|(\s*:?-+:?\s*\|)+", line.strip()) is not None


# ---------------------------------------------------------------- reading the transcription into pieces


class Piece:
    """A clause, a figure's caption, a bold group line, a paragraph or a table block, with its PDF page."""

    def __init__(self, kind, page, **kw):
        self.kind = kind
        self.page = page
        self.__dict__.update(kw)


def pieces(path: Path):
    """The file's pieces in order. A table block is its header and rows, each row with its page; a clause carries its
    pages (a clause cut by a page break goes on after the page mark)."""
    page = None
    lines = path.read_text(encoding="utf-8").splitlines()
    out = []
    i = 0
    after_page = False
    while i < len(lines):
        line = lines[i].rstrip()
        if not line or line.startswith("<!--"):
            i += 1
            continue
        m = PAGE.match(line)
        if m:
            page = int(m.group(1))
            after_page = True
            i += 1
            continue
        if line.startswith("|"):
            block = []
            while i < len(lines) and lines[i].startswith("|"):
                block.append((lines[i].rstrip(), page))
                i += 1
            if len(block) < 2 or not is_separator(block[1][0]):
                raise Unread(f"{path.name}: a table without its header line near page {page}: {block[0][0][:80]}")
            header = cells_of(block[0][0])
            rows = [(cells_of(l), p) for l, p in block[2:]]
            out.append(Piece("table", block[0][1], header=header, rows=rows))
            after_page = False
            continue
        m = TABLE.match(line)
        if m:
            out.append(Piece("heading", page, annex=int(m.group(1)), no=f"{m.group(1)}.{m.group(2)}", title=plain(m.group(3))))
            after_page = False
            i += 1
            continue
        m = CLAUSE.match(line)
        if m:
            out.append(Piece("clause", page, no=m.group(1), text=plain(m.group(2)), pages=[page]))
            after_page = False
            i += 1
            continue
        m = FIGURE.match(line)
        if m:
            out.append(Piece("figure", page, no=m.group(1), text=plain(m.group(2))))
            after_page = False
            i += 1
            continue
        if line.startswith("# "):
            out.append(Piece("title", page, text=plain(line[2:])))
            after_page = False
            i += 1
            continue
        m = GROUP.match(line)
        if m:
            out.append(Piece("group", page, text=plain(m.group(1))))
            after_page = False
            i += 1
            continue
        if CONTINUED in line:
            out.append(Piece("continued", page))
            i += 1
            continue
        if line.startswith("!["):
            i += 1
            continue
        # A paragraph: a clause's words going on over a page break, a table's note, a spilled description.
        text = plain(line)
        if after_page and out and out[-1].kind == "clause":
            out[-1].text = f"{out[-1].text} {text}"
            out[-1].pages.append(page)
        else:
            out.append(Piece("paragraph", page, text=text))
        after_page = False
        i += 1
    return out


# ---------------------------------------------------------------- the annexes' typed rows


def annex_of(no: str) -> int:
    return int(no.split(".")[0])


def clause_row(p: Piece) -> dict:
    return {"id": f"M{p.no}", "annex": annex_of(p.no), "clause": p.no, "pages": sorted(set(p.pages)), "text": p.text}


def required_of(title: str) -> bool | None:
    if "(Zorunlu)" in title:
        return True
    if "(İsteğe Bağlı)" in title:
        return False
    return None


def template_from(table_no: str, title: str, page: int, header: list[str], rows) -> dict:
    """A name template (Tablo 2.1, 3.1, 4.1–4.6, 5.3): its fields in order, each with its length, its kind and whether
    it is required."""
    names = [plain(h) for h in header[1:]]
    length = kind = None
    for cells, _ in rows:
        key = plain(cells[0])
        if key.startswith("Karakter Sayısı"):
            length = [plain(c) for c in cells[1:]]
        elif key.startswith("Veri Türü"):
            kind = [plain(c) for c in cells[1:]]
        else:
            raise Unread(f"Tablo {table_no}: a template row that is no length or kind: {cells}")
    fields = []
    for k, n in enumerate(names):
        req = required_of(n)
        base = re.sub(r"\s*\((Zorunlu|İsteğe Bağlı)\)\s*", "", n).rstrip("*").strip()
        fields.append(
            {
                "no": k + 1,
                "name": base,
                "length": length[k] if length else None,
                "kind": kind[k] if kind else None,
                "required": req,
            }
        )
    return {"id": f"T{table_no}", "table": table_no, "title": title, "page": page, "fields": fields}


def code_cell(s: str) -> tuple[str, str | None]:
    """A code cell's code and its note: “KS (Tip kesitler de dahil)” is KS and its note."""
    s = plain(s)
    m = re.fullmatch(r"([A-Z0-9]+)\s*\((.*)\)", s)
    if m:
        return m.group(1), m.group(2).strip()
    return s, None


LISTS = {
    "2.2": "discipline",
    "2.3": "item:MM",
    "2.4": "item:IM",
    "2.5": "subdiscipline:ST",
    "2.6": "subdiscipline:MK",
    "2.7": "subdiscipline:EE",
    "2.8": "item:HR",
    "2.9": "subdiscipline:AL",
    "2.10": "subdiscipline:PY",
    "2.11": "item:ZE",
    "2.12": "fileKind",
    "2.13": "fileType",
    "2.14": "storey",
    "3.2": "room",
}


def code_rows(table_no: str, title: str, block_pieces, groups_before) -> list[dict]:
    """A code list's rows (Tablo 2.2–2.14, 3.2): the name, its code, its note; Tablo 2.13's rows their discipline's
    group. A row of dots stands for the series around it (Tablo 2.14: B03, B02 …) and is no code."""
    out = []
    row_no = 0
    group = groups_before
    for kind, value in block_pieces:
        if kind == "group":
            group = value
            continue
        header, rows = value
        for cells, page in rows:
            if len(cells) != 2:
                raise Unread(f"Tablo {table_no}: a code row of {len(cells)} cells: {cells}")
            a, b = plain(cells[0]), cells[1]
            if a.startswith("**") or (cells[0].startswith("**") and not b.strip()):
                group = plain(cells[0].strip("*"))
                continue
            if [a, plain(b)] == [plain(h) for h in header]:
                continue  # the header written again inside the block
            if set(a) <= {"."} and set(plain(b)) <= {"."}:
                out.append({"series": True, "table": table_no, "page": page, "after": out[-1]["code"] if out else None})
                continue
            code, note = code_cell(b)
            row_no += 1
            out.append(
                {
                    "id": f"T{table_no}/{code}",
                    "list": LISTS[table_no],
                    "table": table_no,
                    "row": row_no,
                    "page": page,
                    "name": a,
                    "code": code,
                    "note": note,
                    "group": group,
                }
            )
    return out


def digital_project(all_pieces) -> dict:
    """EK-1…EK-4 and EK-8."""
    clauses, templates, codes, fonts, marks, figures, series = [], [], [], [], [], [], []
    project_fields = None
    current = None  # (annex, table no, title, page)
    block_pieces = []
    group = None

    def flush():
        nonlocal block_pieces, group
        if current is None or not block_pieces:
            block_pieces = []
            return
        no, title, page = current[1], current[2], current[3]
        if no in LISTS:
            for r in code_rows(no, title, block_pieces, None):
                (series if r.get("series") else codes).append(r)
        block_pieces = []
        group = None

    for p in all_pieces:
        annex = getattr(p, "annex", None)
        if p.kind == "clause":
            if annex_of(p.no) in (1, 2, 3, 4, 8):
                clauses.append(clause_row(p))
            continue
        if p.kind == "heading":
            flush()
            current = (p.annex, p.no, p.title, p.page) if p.annex in (1, 2, 3, 4) else None
            continue
        if current is None:
            continue
        no = current[1]
        if p.kind == "figure":
            figures.append({"id": f"S{p.no}", "figure": p.no, "page": p.page, "title": p.text})
            continue
        if p.kind == "group":
            block_pieces.append(("group", p.text))
            continue
        if p.kind != "table":
            continue
        first = plain(p.header[0])
        if no == "1.1":
            project_fields = {
                "id": "T1.1",
                "table": "1.1",
                "title": current[2],
                "page": p.page,
                "fields": [plain(h) for h in p.header],
            }
        elif first.startswith("Bilgi Alan"):
            templates.append(template_from(no, current[2], p.page, p.header, p.rows))
        elif no in LISTS:
            block_pieces.append(("table", (p.header, p.rows)))
        elif first == "YAZI TİPİ":
            for k, (cells, page) in enumerate(p.rows):
                fonts.append({"id": f"T4.7/{k + 1}", "table": "4.7", "row": k + 1, "page": page, "font": plain(cells[0]), "points": plain(cells[1])})
        elif first == "İşaretin Adı":
            name = None
            for k, (cells, page) in enumerate(p.rows):
                if plain(cells[0]):
                    name = plain(cells[0])
                scales = [plain(h).replace("Çizim Ölçeği ", "") for h in p.header[3:]]
                marks.append(
                    {
                        "id": f"S4.7/{k + 1}",
                        "figure": "4.7",
                        "row": k + 1,
                        "page": page,
                        "mark": name,
                        "measure": plain(cells[2]),
                        "sizes": {s: plain(c) for s, c in zip(scales, cells[3:])},
                    }
                )
        else:
            raise Unread(f"Tablo {no}: a block KentOS does not read: {p.header}")
    flush()
    return {
        "format": 1,
        "profile": PROFILE,
        "clauses": clauses,
        "projectInformation": project_fields,
        "templates": templates,
        "codes": codes,
        "series": series,
        "fonts": fonts,
        "marks": marks,
        "figures": figures,
    }


# ---------------------------------------------------------------- EK-5…EK-7, EK-9

CLASS_IN_TITLE = re.compile(r"`(Ifc[A-Za-z0-9]+)`")
ATTRIBUTE = re.compile(r"^Attribute\s*(?:\((.*?)\))?\s*:\s*(.+)$", re.I)
SETS = re.compile(r"^(Pset_|Qto_|TREpys_|Ifc)")


def requirement_kind(set_name: str) -> str:
    if set_name.startswith("Qto_"):
        return "quantity"
    if set_name.startswith("Pset_") or set_name.startswith("TREpys_"):
        return "property"
    return "relation"


def predefined_list(cell: str) -> list[str]:
    s = plain(cell)
    if s in ("", "-"):
        return []
    return [v.strip() for v in s.split(",") if v.strip()]


def attribute_row(cells, page, table, row_no, level, cls):
    """An attribute row: Türkçe tanım, öznitelik, ilişki, veri tipi, açıklama."""
    tr, expr = plain(cells[0]), plain(cells[1])
    relation = plain(cells[2]) if len(cells) > 2 else ""
    datatype = plain(cells[3]) if len(cells) > 3 else ""
    description = plain(cells[4]) if len(cells) > 4 else ""
    expr = expr.rstrip("*").strip()
    tr = tr.strip('"').strip()
    path = None
    if re.fullmatch(r"Ifc[A-Za-z0-9]+", expr) and ">" in relation:
        # A relation the element takes part in (`IfcRelAssignsToGroup>IfcSystem`: the system it belongs to).
        rel = relation.replace(" ", "")
        return {
            "id": f"T{table}/{rel}",
            "table": table,
            "row": row_no,
            "page": page,
            "class": cls,
            "level": level,
            "kind": "relation",
            "set": None,
            "path": rel,
            "name": expr,
            "tr": tr,
            "datatype": datatype,
            "relation": rel,
            "description": description or None,
        }
    via = ATTRIBUTE.match(relation.rstrip("*").strip()) if relation not in ("", "-") else None
    if via and via.group(1):
        # The attribute is the material layer's, its path in the relation cell (Tablo 6.20: the covering's layer's
        # Material and LayerThickness through IfcMaterialLayerSetUsage).
        path = via.group(1).replace(" ", "")
        name = via.group(2).strip().rstrip("*").strip()
        return {
            "id": f"T{table}/{path.split('>')[-1]}.{name}",
            "table": table,
            "row": row_no,
            "page": page,
            "class": cls,
            "level": level,
            "kind": "material",
            "set": None,
            "path": path,
            "name": name,
            "tr": tr,
            "datatype": datatype,
            "relation": None,
            "description": description or None,
        }
    if expr == "PredefinedType" or expr.endswith(": PredefinedType"):
        name = "PredefinedType"
    else:
        m = ATTRIBUTE.match(expr)
        if not m:
            raise Unread(f"Tablo {table}: an attribute cell KentOS does not read: {cells[1]!r}")
        path = m.group(1)
        name = m.group(2).strip().rstrip("*").strip()
        if ">" in name:
            # “Roles>ifcActorRole”: the attribute and the classes it takes (Tablo 7.4); the attribute is the name.
            name, takes = (part.strip() for part in name.split(">", 1))
            relation = takes
    if path:
        # Attribute(IfcRelAssociatesMaterial>IfcMaterial): Name — the material's name, through its relation.
        path = path.replace(" ", "")
        kind = "material" if "IfcMaterial" in path else "relation"
        rid = f"T{table}/{path.split('>')[-1]}.{name}"
    else:
        kind = "attribute"
        rid = f"T{table}/{name}"
    return {
        "id": rid,
        "table": table,
        "row": row_no,
        "page": page,
        "class": cls,
        "level": level,
        "kind": kind,
        "set": None,
        "path": path,
        "name": name,
        "tr": tr,
        "datatype": datatype,
        "relation": None if relation in ("", "-") else relation,
        "description": description or None,
    }


def property_row(cells, page, table, row_no, level, cls):
    """A property or quantity row: Türkçe tanım, özellik, set, veri tipi, açıklama."""
    if len(cells) < 4:
        raise Unread(f"Tablo {table}: a property row of {len(cells)} cells: {cells}")
    tr, name, set_name, datatype = plain(cells[0]), name_of(cells[1]), name_of(cells[2]), plain(cells[3])
    description = plain(cells[4]) if len(cells) > 4 else ""
    if not SETS.match(set_name):
        raise Unread(f"Tablo {table}: a property's set KentOS does not read: {cells}")
    return {
        "id": f"T{table}/{set_name}.{name}",
        "table": table,
        "row": row_no,
        "page": page,
        "class": cls,
        "level": level,
        "kind": requirement_kind(set_name),
        "set": set_name,
        "path": None,
        "name": name,
        "tr": tr,
        "datatype": datatype,
        "relation": None,
        "description": description or None,
    }


def class_table(heading: Piece, body) -> tuple[dict, list[dict]]:
    """An EK-6 or EK-7 table: the class, its definition and predefined types, its rows in order. A block whose header
    the page break lost is read by its cells: a set's name in the third cell makes it a property row."""
    no = heading.no
    major, minor = no.split(".")
    minor = int(minor)
    if major == "6":
        level = "mandatory" if minor <= 68 else "optional"
    else:
        level = "mandatory" if minor <= 9 else "optional"
    m = CLASS_IN_TITLE.search(heading.title) or CLASS_IN_TITLE.search(heading.title.replace(" ", ""))
    cls = m.group(1) if m else re.search(r"(Ifc[A-Za-z0-9]+)", heading.title).group(1)
    definition, predefined, tr_name, pages = None, None, None, set()
    rows = []
    mode = None
    row_no = 0
    seen_ids = {}
    for p in body:
        if p.kind == "paragraph" and predefined is None and p.text.startswith("Ön Tanımlı Tip Listesi"):
            mode = "spilled-predefined"
            continue
        if p.kind == "paragraph" and mode == "spilled-predefined":
            predefined = predefined_list(p.text.replace(" Açıklama", ""))
            mode = None
            continue
        if p.kind != "table":
            continue
        head = [plain(h) for h in p.header]
        pages.add(p.page)
        if len(head) == 2 and ("Ön Tanımlı" in head[1] or "Ön Tammlı" in head[1]):
            mt = re.match(r"^(.*?)\s*\(", head[0])
            tr_name = mt.group(1).strip() if mt else head[0]
            for cells, page in p.rows:
                definition = plain(cells[0]).removeprefix("Tanım:").strip()
                predefined = predefined_list(cells[1]) if len(cells) > 1 else []
            continue
        if len(head) == 2 and not any(head):
            # A table's note the page break put in a block of its own (Tablo 6.113's material note).
            for cells, _ in p.rows:
                if not any("giriliken" in plain(c) or "Attribute" in plain(c) for c in cells):
                    raise Unread(f"Tablo {no}: a two-cell block KentOS does not read: {cells}")
            continue
        joined = " ".join(head)
        if "Öznitelik" in joined:
            mode = "attribute"
        elif "Özellik" in joined:
            mode = "property"
        elif not any(head):
            mode = None  # a headerless block: read by its cells
        else:
            raise Unread(f"Tablo {no}: a block KentOS does not read: {head}")
        for cells, page in p.rows:
            if not any(plain(c) for c in cells):
                continue
            pages.add(page)
            words = " ".join(plain(c) for c in cells)
            if "Türkçe Tanım" in words and ("Özellik" in words or "Öznitelik" in words):
                # A header inside the block: the rows under it are of its kind.
                mode = "attribute" if "Öznitelik" in words else "property"
                continue
            if plain(cells[0]) and not any(plain(c) for c in cells[1:]) and rows:
                # The Turkish name of the row before goes on (Tablo 6.134's “Akustik / Performans Sınıfı”).
                rows[-1]["tr"] = f"{rows[-1]['tr']} {plain(cells[0])}".strip()
                continue
            if not plain(cells[0]) and not plain(cells[1]) and rows:
                # The row goes on after a page break: its words join the row before.
                tail = " ".join(plain(c) for c in cells if plain(c))
                prev = rows[-1]
                prev["description"] = f"{prev['description'] or ''} {tail}".strip()
                continue
            row_mode = mode
            if row_mode is None:
                row_mode = "property" if len(cells) > 2 and SETS.match(name_of(cells[2])) and not name_of(cells[2]).startswith("Ifc") else "attribute"
            if row_mode == "property" and len(cells) > 2 and name_of(cells[2]) in ("-", ""):
                row_mode = "attribute"
            if row_mode == "property" and len(cells) > 2 and name_of(cells[2]).startswith("Attribute"):
                # An attribute in the properties' block, its set cell saying so (Tablo 6.93's OverallWidth).
                cells = [cells[0], f"Attribute: {name_of(cells[1])}", "-", *cells[3:]]
                row_mode = "attribute"
            row_no += 1
            r = (property_row if row_mode == "property" else attribute_row)(cells, page, no, row_no, level, cls)
            if r["id"] in seen_ids:
                # The same name twice in one table (the source repeats a row): the second keeps its row number.
                r["id"] = f"{r['id']}#{row_no}"
            seen_ids[r["id"]] = True
            rows.append(r)
    table = {
        "id": f"T{no}",
        "table": no,
        "class": cls,
        "tr": tr_name,
        "level": level,
        "pages": sorted(pages | {heading.page}),
        "title": heading.title,
        "definition": definition,
        "predefined": predefined or [],
    }
    return table, rows


def classes_rows(table_no: str, header, rows, level) -> list[dict]:
    out = []
    for cells, page in rows:
        if len(cells) != 4:
            raise Unread(f"Tablo {table_no}: a class row of {len(cells)} cells: {cells}")
        no, cls, tr, code = plain(cells[0]), name_of(cells[1]), plain(cells[2]), plain(cells[3])
        out.append({"id": f"T{table_no}/{cls}", "table": table_no, "row": int(no), "page": page, "class": cls, "tr": tr, "code": code, "level": level})
    return out


def form_rows(table_no: str, header, rows, state) -> list[dict]:
    """A model quality control form's rows (Tablo 9.1, 9.2): bold rows are its groups; a row whose number is empty
    goes on the row before over a page break."""
    out = []
    for cells, page in rows:
        first = cells[0].strip()
        if first.startswith("**"):
            state["group"] = plain(first.strip("*"))
            continue
        if not plain(first):
            if not state["last"]:
                raise Unread(f"Tablo {table_no}: a form row's words without its row: {cells}")
            tail = plain(cells[3]) if len(cells) > 3 else ""
            state["last"]["question"] = f"{state['last']['question']} {tail}".strip()
            if page not in state["last"]["pages"]:
                state["last"]["pages"].append(page)
            continue
        r = {
            "id": f"T{table_no}/{int(plain(first))}",
            "form": table_no,
            "no": int(plain(first)),
            "pages": [page],
            "group": state["group"],
            "discipline": plain(cells[1]),
            "title": plain(cells[2]),
            "question": plain(cells[3]),
        }
        out.append(r)
        state["last"] = r
    return out


def ifc(all_pieces) -> dict:
    """EK-5…EK-7 and EK-9."""
    clauses, classes, tables, requirements, questions = [], [], [], [], []
    template = None
    i = 0
    n = len(all_pieces)
    while i < n:
        p = all_pieces[i]
        if p.kind == "clause":
            if annex_of(p.no) in (5, 6, 7, 9):
                clauses.append(clause_row(p))
            i += 1
            continue
        if p.kind != "heading" or p.annex not in (5, 6, 7, 9):
            i += 1
            continue
        heading = p
        body = []
        i += 1
        while i < n and all_pieces[i].kind != "heading":
            if all_pieces[i].kind == "clause":
                break
            body.append(all_pieces[i])
            i += 1
        if heading.annex in (6, 7):
            table, rows = class_table(heading, body)
            tables.append(table)
            requirements.extend(rows)
        elif heading.no in ("5.1", "5.2"):
            for b in body:
                if b.kind == "table":
                    classes.extend(classes_rows(heading.no, b.header, b.rows, "mandatory" if heading.no == "5.1" else "optional"))
        elif heading.no == "5.3":
            for b in body:
                if b.kind == "table":
                    template = template_from("5.3", heading.title, b.page, b.header, b.rows)
        elif heading.annex == 9:
            state = {"group": None, "last": None}
            for b in body:
                if b.kind == "table":
                    questions.extend(form_rows(heading.no, b.header, b.rows, state))
        else:
            raise Unread(f"Tablo {heading.no}: a table KentOS does not read")
    return {
        "format": 1,
        "profile": PROFILE,
        "clauses": clauses,
        "template": template,
        "classes": classes,
        "tables": tables,
        "requirements": requirements,
        "questions": questions,
    }


def build() -> tuple[dict, dict]:
    all_pieces = []
    for f in FILES:
        all_pieces.extend(pieces(SRC / f))
    return digital_project(all_pieces), ifc(all_pieces)


def text_of(data: dict) -> str:
    return json.dumps(data, ensure_ascii=False, indent=1) + "\n"


def main(argv: list[str]) -> int:
    check = "--check" in argv
    try:
        digital, ifc_data = build()
    except Unread as e:
        print(f"katalog: {e}", file=sys.stderr)
        return 1
    stats = (
        f"EK-1…4, 8: {len(digital['clauses'])} madde, {len(digital['templates'])} şablon, {len(digital['codes'])} kod, "
        f"{len(digital['fonts'])} yazı tipi, {len(digital['marks'])} işaret ölçüsü · EK-5…7, 9: {len(ifc_data['clauses'])} madde, "
        f"{len(ifc_data['classes'])} sınıf, {len(ifc_data['tables'])} tablo, {len(ifc_data['requirements'])} satır, "
        f"{len(ifc_data['questions'])} soru"
    )
    failed = False
    for path, data in ((OUT_DIGITAL, digital), (OUT_IFC, ifc_data)):
        text = text_of(data)
        if check:
            have = path.read_text(encoding="utf-8") if path.exists() else None
            if have != text:
                print(f"katalog: {path.relative_to(ROOT)} is not what the transcription gives; run scripts/mevzuat/katalog.py", file=sys.stderr)
                failed = True
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
            digest = hashlib.sha256(text.encode()).hexdigest()[:12]
            print(f"{path.relative_to(ROOT)}  {len(text.encode())} bayt  {digest}")
    print(stats)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
