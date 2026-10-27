#!/usr/bin/env python3
"""Independent reader of KCAD v2, the KentOS project file (`.kcad`).

Written from docs/specs/kcad-v2.md with Python's standard library only, never
from KentOS code, so the file format is held to an outside implementation
(TODOS.md FILE-23, docs/adr/0025): the Rust codec (crates/shared/kcad), its
browser build (crates/wasm/formats-wasm) and this reader read the same
fixtures (fixtures/kcad/v2, expected.json) and must agree on every one.

    python3 tools/kcad/kcad.py inspect FILE       # header and a summary of the drawing
    python3 tools/kcad/kcad.py validate FILE...   # valid, or the error code and its message
    python3 tools/kcad/kcad.py dump FILE          # the drawing as JSON, in the contract's form
    python3 tools/kcad/kcad.py sniff FILE...      # what a file is by its content

Exit status: 0 when every file is valid, 1 when one is not, 2 on a usage error.

The reader checks the container (signature, versions, lengths, SHA-256),
decodes the payload by the KentOS CBOR profile (definite lengths, shortest
integers, binary64 floats only, text keys in RFC 8949 §4.2.1 order, no
duplicates, no tags, size and depth limits) and then checks the document
schema, the block rules with it (§6.9). The document rules the apps add on
top (layer references, known CRS, vertex counts; spec §6.11) are not checked
here.
"""

import base64
import hashlib
import json
import math
import re
import struct
import sys
import unicodedata
from collections import Counter

# ── Container (spec §2, §3) ─────────────────────────────────────────────

MAGIC = b"\x89KCAD\r\n\x1a\n"
MAGIC_PREFIX = MAGIC[:5]
FIXED_HEADER = 36
MAJOR = 2
MINOR = 0
ENCODING_CBOR_PROFILE_1 = 1
CODEC_NONE = 0
HASH_SIZE = 32
MAX_PAYLOAD = 1 << 30
MAX_EXTENSIONS = 64
MAX_EXTENSION_NAME = 64
EXTENSION_CHARS = set(b"abcdefghijklmnopqrstuvwxyz0123456789._-")

# ── CBOR profile 1 (spec §5) ────────────────────────────────────────────

MAX_DEPTH = 64
MAX_ITEMS = 1 << 24
MAX_STRING = 1 << 24

DOCUMENT_FORMAT = "kentos.document"
DOCUMENT_VERSION = 2
# Schema 3: schema 2 and an object's own line weight (`lineWeight`, mm; docs/adr/0139).
SCHEMA_WITH_LINE_WEIGHTS = 3
MAX_LINE_WEIGHT = 100.0
# Schema 4: schema 3 and vertex elevations (`za`, `zb` of a line, `zs` of a polyline, a polygon and a hole; docs/adr/0142).
SCHEMA_WITH_ELEVATIONS = 4
# Schema 5: schema 4 and multi-part areas (`parts` of a polygon; docs/adr/0143).
SCHEMA_WITH_PARTS = 5
# Schema 6: schema 5 and blocks (the document's `blocks`, the `insert` kind; docs/adr/0144).
SCHEMA_WITH_BLOCKS = 6
# Schema 7: schema 6 and a text's `align`, `widthFactor` and `mask`, an attribute definition's `align` and
# `widthFactor` (docs/adr/0145).
SCHEMA_WITH_TEXT_EXTRAS = 7
# Schema 8: schema 7 and the `leader` kind (docs/adr/0146).
SCHEMA_WITH_LEADERS = 8
# Schema 9: schema 8 and the dimension's new kinds, its `mask`, a slope's `za` and `zb` (docs/adr/0147).
SCHEMA_WITH_DIMENSIONS = 9
# Schema 10: schema 9 and a layer's own snapping, a layer node's `snap` (docs/adr/0163 §4).
SCHEMA_WITH_LAYER_SNAP = 10
# Schema 11: schema 10 and a local project's drawing unit, the settings' `drawingUnit` (docs/adr/0165 §2).
SCHEMA_WITH_DRAWING_UNIT = 11
# Schema 12: schema 11 and the project's second coordinate system, the settings' `secondSrid` (docs/adr/0167 §1).
SCHEMA_WITH_SECOND_SRID = 12
# Schema 13: schema 12 and the project's own coordinate systems and datum choices, the settings' `customCrs`,
# `secondCustomCrs` and `datumTransforms` (docs/adr/0168).
SCHEMA_WITH_CUSTOM_CRS = 13
# Schema 14: schema 13 and the project's survey constants and tolerances, the settings' `survey` (docs/adr/0169 §3).
SCHEMA_WITH_SURVEY = 14
# Schema 15: schema 14 and the survey settings' traverse tolerances, `twoWay`, `traverseAngle`, `traverseCoord` (docs/adr/0169 §3).
SCHEMA_WITH_TRAVERSE_TOLERANCES = 15
# Schema 16: schema 15 and the survey settings' ground, `groundHeight` and `reduceToGrid` (docs/adr/0171 §2, §4).
SCHEMA_WITH_GROUND = 16
# Schema 17: schema 16 and multi-part polylines and points, a polyline's and a point's `parts` (docs/adr/0174).
SCHEMA_WITH_LINE_PARTS = 17
# Schema 18: schema 17 and the text that writes an object's label, a text's `labelOf` and `labelScale` (docs/adr/0175 §4).
SCHEMA_WITH_LINKED_TEXTS = 18
# Schema 19: schema 18 and the project's named layer states, the settings' `layerStates` (docs/adr/0177 §4).
SCHEMA_WITH_LAYER_STATES = 19
# Schema 20: schema 19 and a multi-line text's `boxWidth`, `lineSpacing` and `runs` (docs/adr/0182 §1).
SCHEMA_WITH_PARAGRAPHS = 20
# Schema 21: schema 20 and the named text and dimension styles, the settings' `textStyles` and `dimensionStyles`, a text's
# face (`textStyle`, `font`, `bold`, `italic`, `oblique`) and a dimension's look (`dimStyle`, `arrow`, `arrowSize`,
# `extOffset`, `extBeyond`, `textGap`, `textPlace`, `decimals`, `unit`, `prefix`, `suffix`, `font`; docs/adr/0183).
SCHEMA_WITH_STYLES = 21
# Schema 22: schema 21 and the `table` kind, only in the drawing (docs/adr/0184 §1).
SCHEMA_WITH_TABLES = 22
# Schema 23: schema 22 and the hatch's patterns, gradients and ties: `pattern`'s types `pattern` and `gradient`, its
# `name`, `scale`, `lines` and `gradient`; a hatch's `assoc`, only in the drawing (docs/adr/0186).
SCHEMA_WITH_HATCH_PATTERNS = 23
# Schema 24: schema 23 and the `image` kind, only in the drawing (docs/adr/0192 §1).
SCHEMA_WITH_IMAGES = 24
# Schema 25: schema 24 and a text's curve, `path`, in the drawing and in block definitions (docs/adr/0196 §1).
SCHEMA_WITH_TEXT_PATHS = 25
# Schema 26: schema 25 and a layer's fields, a layer node's `fields` (docs/adr/0199 §1).
SCHEMA_WITH_LAYER_FIELDS = 26
# Schema 27: schema 26 and the project's topology rules, tolerance and exceptions, the settings' `topology` (docs/adr/0202 §7).
SCHEMA_WITH_TOPOLOGY = 27
# Schema 28: schema 27 and the survey settings' a priori standard deviations, `sigmaDirection`, `sigmaDistance`,
# `sigmaPpm`, `sigmaCentering`, `sigmaZenith`, `sigmaLevelling` (docs/adr/0203 §1).
SCHEMA_WITH_SURVEY_SIGMAS = 28
# Schema 29: schema 28 and the `raster` kind, only in the drawing (docs/adr/0204 §2).
SCHEMA_WITH_RASTERS = 29
# Schema 30: schema 29 and the project's annotation heights, the settings' `annotation`; a dimension's and a dimension
# style's lines (`dimLineColor`, `dimLineWeight`, `dimLineType`, `extColor`, `extWeight`, `extLineType`, `textColor`); a
# leader's AutoCAD arrowheads and `arrowSize` (docs/adr/0205).
SCHEMA_WITH_ANNOTATION = 30
# Schema 31: schema 30 and the `pointcloud` kind, only in the drawing, and a raster's `url` (docs/adr/0207 §1, §3).
SCHEMA_WITH_POINT_CLOUDS = 31
# Schema 32: schema 31 and map services (docs/adr/0208 §2): a layer node's `service` and `feed`, the settings'
# `connections` (without their secrets).
SCHEMA_WITH_SERVICES = 32
# Schema 33: schema 32 and the settings' `networks` (docs/adr/0209 §2).
SCHEMA_WITH_NETWORKS = 33
# Schema 34: schema 33 and a layer node's `time`, a group's `scenario`, a scenario layer's `replaces` (docs/adr/0210 §2).
SCHEMA_WITH_TEMPORAL = 34
# Schema 35: schema 34 and a layer node's `filter` (docs/adr/0211 §2).
SCHEMA_WITH_FILTERS = 35
# Schema 36: schema 35 and the label engine (docs/adr/0212 §2): a label style's engine fields, a layer style's
# `labels`, an object's `labelPins`.
SCHEMA_WITH_LABELS = 36
# Schema 37: schema 36 and the settings' `variables`, the project's `@` values (docs/adr/0214 §2.3).
SCHEMA_WITH_VARIABLES = 37
# A project's variables' bounds (kentos_contracts::variables).
VARIABLES_MAX = 200
VARIABLE_NAME_MAX = 64
VARIABLE_TEXT_MAX = 4000
VARIABLE_KINDS = ("text", "number", "bool", "date")
BUILTIN_VARIABLES = ("proje_adi", "koordinat_sistemi", "epsg", "olcek", "tarih", "simdi", "katman_adi", "katman", "kullanici")
# Schema 38: schema 37 and multidimensional rasters (docs/adr/0243 §6): a raster's `dataset` (a NetCDF variable and its
# slice) and its look's `edges` (a mesh's lines).
SCHEMA_WITH_MULTIDIM = 38
# Schema 39: schema 38 and vertex measures (M), a line's `ma`, `mb`, a polyline's and its parts' `ms` (docs/adr/0224 §1).
SCHEMA_WITH_MEASURES = 39
# Schema 40: schema 39 and an object's `attachments`, a layer's `actions` and `card` (docs/adr/0225).
SCHEMA_WITH_ATTACHMENTS = 40
# Schema 41: schema 40 and a layer's `compose`: its opacity, blending mode and mask (docs/adr/0226 §3, §4).
SCHEMA_WITH_COMPOSE = 41
# Schema 42: schema 41 and the settings' `addressLocators`, the project's address data (docs/adr/0229 §2).
SCHEMA_WITH_ADDRESSES = 42
# Schema 43: schema 42 and a dimension's `refs`, the places on objects its points follow; only in the drawing
# (docs/adr/0249 §4).
SCHEMA_WITH_DIMENSION_REFS = 43
# Schema 44: schema 43 and the document's `sheets`, the project's sheet book and its pictures' bytes (docs/adr/0251).
SCHEMA_WITH_SHEETS = 44
# Schema 45: schema 44 and the document's `requests`, the requests the drawing executed (docs/adr/0253 §6).
SCHEMA_WITH_REQUESTS = 45
# Schema 46: schema 45 and a polyline's `closed` (a closed line, not an area) and `widths` (each vertex's edge's start
# and end width in the drawing's length), in the drawing and in a block definition (docs/adr/0258).
SCHEMA_WITH_POLYLINE_EXTRAS = 46
# Schema 47: schema 46 and the `nurbs` kind, a general NURBS curve as given, in the drawing and in a block definition
# (docs/adr/0259).
SCHEMA_WITH_NURBS = 47
# Schema 48: schema 47 and an object's `lineType`, `lineTypeScale`, `transparency`, `order` (the drawing's objects) and
# `byLayer` (a block's objects), a layer's `plot` (docs/adr/0260).
SCHEMA_WITH_LOOK = 48
# Schema 49: schema 48 and the settings' `ucs` and `namedUcs`, the drawing's user coordinate systems (docs/adr/0261 §1).
SCHEMA_WITH_UCS = 49
# Schema 50: schema 49, the drawing units `in` and `ft` and the settings' `lengthStyle` (docs/adr/0262).
SCHEMA_WITH_IMPERIAL = 50
# Schema 51: schema 50, a reference's place `along` (part, ring, index, t) and its block `path` (docs/adr/0263).
SCHEMA_WITH_SUBREF_EXTRAS = 51
# Schema 52: schema 51 and the settings' `namedViews`, the project's named views (docs/adr/0285 §1).
SCHEMA_WITH_VIEWS = 52
# Schema 53: schema 52 and the document's `arrays`, its associative arrays (docs/adr/0300).
SCHEMA_WITH_ARRAYS = 53
# Schema 54: schema 53 and an area's mask, `wipeout` (`{ frame }`; docs/adr/0310).
SCHEMA_WITH_WIPEOUTS = 54
# Schema 55: schema 54 and a layer's `frozen` (only true), a layer state node's `frozen` and `plot` (docs/adr/0311).
SCHEMA_WITH_FROZEN = 55
# Schema 56: schema 55 and the settings' line types with patterns (`lineTypes`) and line type scale (`lineTypeScale`),
# a layer style's and an object's `linePattern` (docs/adr/0313 §1).
SCHEMA_WITH_PATTERNS = 56
# Schema 57: schema 56 and an insert's own y factor (`yScale`, only when it is not `scale`) and a block definition's
# `units` (docs/adr/0315 §3, §4).
SCHEMA_WITH_BLOCK_EXTRAS = 57
# Schema 58: schema 57 and an attribute definition's options (`invisible`, `constant`, `verify`, `preset`, `movable`,
# `multiline`, `boxWidth`, `field`), an insert's `attributePlaces` and a block definition's `card` (docs/adr/0317).
SCHEMA_WITH_BLOCK_CARDS = 58
# Schema 59: a block definition's `dynamic` part (parameters and actions, as their JSON) and an insert's `parameters`
# (docs/adr/0318).
SCHEMA_WITH_DYNAMIC_BLOCKS = 59
# Schema 60: schema 59 and a dimension's references' `rule` (`curve` or `edges`): how its points no place names follow
# (docs/adr/0320 §1).
SCHEMA_WITH_DIMENSION_RULES = 60
# Schema 61: schema 60, a dimension's and a dimension style's `options` and a dimension's `textShift` and `breaks`
# (docs/adr/0321).
SCHEMA_WITH_DIMENSION_OPTIONS = 61
# Schema 62: schema 61, a dimension's kinds `centerMark`, `centerLine`, `elevation` and its references' rule `pair`
# (docs/adr/0322).
SCHEMA_WITH_DIMENSION_MARKS = 62
# Schema 63: schema 62, a text's and a dimension's `overrides`: the fields set apart from their style (docs/adr/0322 §5).
SCHEMA_WITH_OVERRIDES = 63
# Schema 64: schema 63, a text's, a dimension's and a leader's `annotative`: its scales (docs/adr/0323).
SCHEMA_WITH_ANNOTATIVE = 64
# Schema 65: schema 64, a multi-line text's `layout` (indents, tab stops, columns) and a run's `stack` script (docs/adr/0324).
SCHEMA_WITH_PARAGRAPH_LAYOUT = 65
# Schema 66: schema 65, a text's `fields`: ranges of its letters bound to a source (docs/adr/0325).
SCHEMA_WITH_TEXT_FIELDS = 66
FIELD_OBJECT_VALUES = ("area", "perimeter", "length", "layer", "label", "attribute", "text", "x", "y")
FIELD_PROJECT_VALUES = ("name", "scale", "unit", "crs", "variable", "blockCount")
FIELD_UNITS = ("mm", "cm", "m", "km", "m2", "donum", "ha")
FIELD_BROKEN = ("removed", "noValue", "notNamed", "cycle")
MAX_FIELDS = 256
# Schema 67: schema 66, a leader's `arms`, `tip`, `blocks`, `leaderStyle` and the settings' `leaderStyles` (docs/adr/0326).
SCHEMA_WITH_MULTILEADER = 67
MAX_LEADER_ARMS = 64
# Schema 68: schema 67, a table's `styles`, `formulas`, `fields`, its linked file's `link` and a block schedule's
# `schedule` (docs/adr/0327).
SCHEMA_WITH_TABLE_CELLS = 68
# Schema 69: schema 68, a hatch's exact `boundary` (loops of lines, arcs, ellipses' arcs and NURBS curves) and a tie's
# `bounds` (the line work a face follows; `outer` then absent) (docs/adr/0328).
SCHEMA_WITH_HATCH_BOUNDARIES = 69
# Schema 70: schema 69 and a face's tie's `gap` (docs/adr/0329 §3).
SCHEMA_WITH_HATCH_GAPS = 70
# Schema 71: schema 70, the settings' `hatchPatterns` (a project's own patterns), a pattern's `origin` and a hatch's
# `annotative` (docs/adr/0331).
SCHEMA_WITH_HATCH_EXTRAS = 71
MAX_HATCH_GAP = 1000.0
MAX_BOUNDARY_LOOPS = 100_000
MAX_LOOP_EDGES = 1_000_000
MAX_CURVE_POINTS = 100_000
MAX_CURVE_DEGREE = 10
MAX_CELL_STYLES = 1000
MAX_FORMULAS = 10_000
MAX_CELL_FIELDS = 1000
MAX_FORMULA_LETTERS = 1000
MAX_CELL_DECIMALS = 10
COLOUR = re.compile(r"#[0-9A-Fa-f]{6}")
SHA256_HEX = re.compile(r"[0-9A-Fa-f]{64}")
UUID_TEXT = re.compile(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
# A layout's numbers in the text's height, at most this much; at most this many tab stops (docs/adr/0324 §1).
MAX_PARAGRAPH_HEIGHTS = 1000.0
MAX_TABS = 32
MAX_ANNOTATIVE_SCALES = 32
MAX_ANNOTATIVE_SCALE = 1.0e6
TEXT_OVERRIDES = ("font", "bold", "italic", "oblique", "widthFactor", "height")
DIMENSION_OVERRIDES = ("arrow", "arrowSize", "extOffset", "extBeyond", "textGap", "textPlace", "decimals", "unit", "prefix", "suffix",
                       "font", "dimLineColor", "dimLineWeight", "dimLineType", "extColor", "extWeight", "extLineType", "textColor",
                       "options", "height")
# Schema 72: schema 71, the settings' `forms` (the project's forms) and a layer field's stable `id` (docs/adr/0401).
SCHEMA_WITH_FORMS = 72
# Schema 73: schema 72 and the document's `constraints` (the geometric and dimensional constraints and the parameter
# table, docs/adr/0334).
SCHEMA_WITH_CONSTRAINTS = 73
# The most constraints and parameters a drawing keeps, the longest expression and parameter name (docs/adr/0334 §1, §3).
CONSTRAINTS_MAX = 10_000
PARAMETERS_MAX = 10_000
EXPRESSION_MAX = 1_000
PARAMETER_NAME_MAX = 64
PARAMETER_FUNCTIONS = {"sin": 1, "cos": 1, "tan": 1, "asin": 1, "acos": 1, "atan": 1, "sqrt": 1, "abs": 1, "round": 1,
                       "floor": 1, "ceil": 1, "min": 2, "max": 2}
LITERAL_UNITS = ("mm", "cm", "km", "m", "ft", "in", "deg", "grad", "rad")
# Schema 74: schema 73 and the settings' `sources`: layers bound to a table of a file (docs/adr/0503).
SCHEMA_WITH_SOURCES = 74
# Schema 75: schema 74, the `wall` kind (its path, width, justification, offset, style, components, ends, fill, height
# and architectural information) and the settings' `wallStyles` and `arch` (docs/adr/0339).
SCHEMA_WITH_WALLS = 75
# Schema 76: schema 75, a wall's `joins` (its start's and end's join when not automatic) and `cleanup` (only when
# false), and a wall component's `priority` (1 to 9) on a wall and in a wall style (docs/adr/0340 §1).
SCHEMA_WITH_WALL_JOINS = 76
WALL_JOINS = ("auto", "none", "miter", "butt", "through")
# Schema 77: schema 76 and the `opening` kind: a door, a window or a bare opening in a wall (its wall, its place
# along it, its width, its jambs and the rest of its fields as one map; docs/adr/0341 §1).
SCHEMA_WITH_OPENINGS = 77
OPENING_MAX_WIDTH = 100.0
OPENING_FORMS = ("door", "window", "opening")
# Schema 78: schema 77, the `space` kind (its boundary's parts and the rest of its fields as one map) and the settings'
# `arch.spaceMethod` (docs/adr/0342 §1).
SCHEMA_WITH_SPACES = 78
# Schema 79: schema 78, the `grid`, `column`, `stair` and `ramp` kinds, each's fields as one map under its kind's name
# (docs/adr/0343 §1).
SCHEMA_WITH_ARCH_ELEMENTS = 79
ARCH_ELEMENTS = ("grid", "column", "stair", "ramp")
ARCH_NAMES = {"grid": "aks", "column": "kolon", "stair": "merdiven", "ramp": "rampa"}
# Schema 80: schema 79, the `tag` kind (its fields as one map under `tag`) and a table's schedule of the drawing's
# openings, spaces, columns or walls under `schedule` (docs/adr/0344 §1, §3).
SCHEMA_WITH_TAGS = 80
TAG_PLACES = ("number", "type", "form", "width", "height", "sill", "head", "name", "usage", "area", "perimeter", "level",
              "phase", "tag", "place", "label", "style", "risers", "slope", "floorElevation", "ceilingElevation", "netArea")
SCHEDULE_FIELDS = ("number", "type", "form", "width", "height", "sill", "head", "wall", "level", "phase", "count", "area",
                   "perimeter", "volume", "name", "usage", "tag", "place", "length", "thickness", "attr")
# Schema 81: schema 80, the `viewmark` kind (its fields as one map under `viewmark`) and a named view's `id`
# (docs/adr/0345 §1, §2).
SCHEMA_WITH_VIEWMARKS = 81
# Schema 82: schema 81 and the architectural settings' `phaseView` (docs/adr/0346 §2).
SCHEMA_WITH_PHASES = 82
# Schema 83: schema 82 and a block definition's external reference source `xref` (docs/adr/0348 §1).
SCHEMA_WITH_XREFS = 83
# Schema 84: schema 83 and the settings' `xrefPaths`, the folders external references are looked for in (docs/adr/0349 §1).
SCHEMA_WITH_XREF_PATHS = 84
# Schema 85: schema 84, an insert's `clipping` and an external reference's `page` and `underlay` (a PDF underlay;
# docs/adr/0351 §1, §2).
SCHEMA_WITH_CLIPPING = 85
MAX_CLIPPING_CORNERS = 10_000
MAX_UNDERLAY_FADE = 80
# Schema 86: schema 85 and the settings' `heights` and `coordinateEpoch` (docs/adr/0519 §1).
SCHEMA_WITH_HEIGHTS = 86
# Schema 87: schema 86 and a network's `turns` and `levels` and a speed cost's `periods` (docs/adr/0521).
SCHEMA_WITH_NETWORK_TURNS = 87
# The vertical systems a project may name (kentos_contracts::VERTICAL_SYSTEMS, EPSG's registry).
VERTICAL_SRIDS = (5775, 3855, 5773, 5714, 8050, 5735)
# Schema 88: schema 87 and a raster's georeference record `georef` (docs/adr/0518 §1).
SCHEMA_WITH_GEOREF = 88
# Schema 89: schema 88 and the settings' join views `joinViews` (docs/adr/0527).
SCHEMA_WITH_JOIN_VIEWS = 89
# Schema 90: schema 89 and a layer time's `recordedStart` and `recordedEnd`, its versions' system time (docs/adr/0537).
SCHEMA_WITH_RECORDED_TIME = 90
# Schema 91: schema 90 and the settings' `sourceBases`, each bound layer's objects' hashes when the layer and its table
# last matched (docs/adr/0538 §1).
SCHEMA_WITH_SOURCE_BASES = 91
# Schema 92: schema 91 and a source base's `fields`, what each object held besides its hashes (docs/adr/0553 §1).
SCHEMA_WITH_SOURCE_FIELD_BASES = 92
# Schema 93: schema 92 and a network's `terminals` and `subnetworks` (docs/adr/0557).
SCHEMA_WITH_NETWORK_TERMINALS = 93
# Schema 94: schema 93 and a network device's `by` and `configs` and the subnetworks' `keepCurrent` (docs/adr/0562).
SCHEMA_WITH_NETWORK_CONFIGS = 94
# Schema 95: schema 94 and a network's vehicle `profiles` and dated `closures` (docs/adr/0567).
SCHEMA_WITH_NETWORK_PROFILES = 95
# Schema 96: schema 95 and a closure's `days` and `hours` (docs/adr/0568).
SCHEMA_WITH_TIMED_CLOSURES = 96
# Schema 97: schema 96 and the settings' links of event layers to their routes `linearEvents` (docs/adr/0569).
SCHEMA_WITH_LINEAR_EVENTS = 97
# Schema 98 (docs/adr/0583): a layer's `sourceCrs` and the settings' `mapSrid`.
SCHEMA_WITH_SOURCE_CRS = 98
# Schema 99 (docs/adr/0589 §3): a topology exception's `digest`, its objects' geometry when marked.
SCHEMA_WITH_EXCEPTION_DIGESTS = 99
# Schema 100 (docs/adr/0594 §1): a georeference record's `crs`, the targets' system as a definition.
SCHEMA_WITH_GEOREF_CRS = 100
# Schema 101 (docs/adr/0598 §1): the settings' `lineage`, what the split and merged objects came from.
SCHEMA_WITH_LINEAGE = 101
# Schema 102 (docs/adr/0622 §5): a layer's `geometry` (what it holds) and the settings' `autoSelect` (Otomatik seçim).
SCHEMA_WITH_LAYER_GEOMETRY = 102
# Schema 103 (docs/adr/0627 §3): a raster's `tiles`, the GeoPackage tile table it shows from its file.
SCHEMA_WITH_RASTER_TILES = 103
# Schema 104 (docs/adr/0629 §5): a layer field of the kinds bigint, instant, uuid, json, binary.
SCHEMA_WITH_FIELD_KINDS = 104
# Schema 105 (docs/adr/0632 §4): a join view's `source` (a source's table) and `aggregates` (its summing columns).
SCHEMA_WITH_JOIN_SOURCES = 105
# Schema 106 (docs/adr/0633 §3): a layer's `dataset` (its description as a dataset) and `derivation` (what a tool made
# it from).
SCHEMA_WITH_DATASETS = 106
# Schema 107 (docs/adr/0638 §5): a layer field's `withheld` (its values do not leave the drawing).
SCHEMA_WITH_WITHHELD_FIELDS = 107
MAX_TILE_TABLE_NAME = 256
# Schema 108 (docs/adr/0672): persistent column deductions on associative spaces.
SCHEMA_WITH_SPACE_COLUMNS = 108
# Schema 109 (docs/adr/0674): an associative space's unassigned source scope.
SCHEMA_WITH_SPACE_SCOPE = 109
# Schema 110 (docs/adr/0678): persistent explicit architectural source UUIDs.
SCHEMA_WITH_SPACE_SOURCES = 110
# Schema 111 (docs/adr/0679): explicit virtual room separators.
SCHEMA_WITH_SPACE_SEPARATORS = 111
# Schema 112 (docs/adr/0690): the project's room names, usages and level prefixes.
SCHEMA_WITH_SPACE_CHOICES = 112
# Schema 113: persistent architectural dimension recipes, JSON-shaped maps.
SCHEMA_WITH_DIMENSION_SETS = 113
# Schema 114 (docs/adr/0700): a dimension recipe's `edge`, the straight wall edge it measures.
SCHEMA_WITH_DIMENSION_SET_EDGES = 114
# Schema 115 (docs/adr/0705): a stair's and a ramp's `railings` (left, right or both).
SCHEMA_WITH_RAILINGS = 115
MAX_GEOREF_CRS = 16 * 1024
# The most bytes a base's field records take decoded, and the most vertex hashes a record keeps.
MAX_FIELD_BASE_BYTES = 8 * 1024 * 1024
MAX_BASE_VERTICES = 128
# A base's rows: 16 bytes of lasting id, the layer's 8-byte hash and the table's, at most this many.
BASE_ROW = 32
MAX_BASE_ROWS = 200_000
GEOREF_METHODS = ("helmert", "affine", "projective", "poly2", "poly3", "thinPlate")
# The fewest control points each method is fitted to.
GEOREF_NEED = {"helmert": 2, "affine": 3, "projective": 4, "poly2": 6, "poly3": 10, "thinPlate": 3}
MAX_GEOREF_POINTS = 1000
MAX_GEOREF_NAME = 64
MAX_XREF_PATHS = 64
MAX_XREF_PATH = 1024
SPACE_METHODS = ("net", "centerline", "gross")
SPACE_MAX_POINTS = 10_000
# A wall's bounds (kentos_contracts::arch).
WALL_MAX_POINTS = 10_000
WALL_MAX_COMPONENTS = 16
WALL_MAX_WIDTH = 100.0
WALL_DETAILS = ("schematic", "standard", "detailed")
ARCH_PHASES = ("existing", "demolish", "new", "temporary")
ARCH_STATUSES = ("proposed", "approved", "underConstruction", "built")
ARCH_DISCIPLINES = ("architecture", "structure", "mechanical", "electrical", "plumbing", "landscape", "interior")
# A definition opens into at most a million pieces (kentos_contracts::blocks::MAX_BLOCK_PIECES, docs/adr/0316 §4).
MAX_BLOCK_PIECES = 1_000_000
# A pattern has at most 12 elements (AutoCAD's LIN, kentos_contracts::MAX_PATTERN_ELEMENTS).
MOST_PATTERN_ELEMENTS = 12
# An associative array holds 2 to 10 000 places (kentos_contracts::ARRAY_PLACES_MAX).
ARRAY_PLACES_MAX = 10_000
# A named view's bounds (kentos_contracts::cad_views): a middle within 10 000 km, a width and a height above 0 and at
# most 20 000 km, a twist in [0, 2π), at most 100 named, a name trimmed, 1 to 64 characters, no control characters.
MOST_VIEW_SIZE = 2.0e7
MOST_NAMED_VIEWS = 100
# A UCS's bounds (kentos_contracts::cad_ucs): an origin within 10 000 km, an angle in [0, 2π), at most 100 named, a name
# trimmed, 1 to 64 characters, no control characters.
UCS_REACH = 1.0e7
MOST_NAMED_UCS = 100
MOST_UCS_NAME = 64
LOOK_PARTS = ("color", "lineType", "lineWeight", "transparency")
# A NURBS curve's bounds (kentos_contracts::nurbs).
NURBS_MAX_DEGREE = 11
NURBS_MAX_POINTS = 100_000
# The request ledger's bounds (kentos_contracts::command_call).
REQUEST_LEDGER = 256
REQUEST_ANSWER_LIMIT = 64 * 1024
# A reference's places and how many numbers (part, ring, index) each has; the codes of a broken one
# (kentos_contracts::subref).
SUBREF_PLACES = {"node": 1, "start": 0, "end": 0, "center": 0, "vertex": 3, "mid": 3, "along": 3}
SUBREF_INDEX_LIMIT = 10_000_000
SUBREF_BROKEN = ("object_gone", "kind_changed", "no_such_place", "vertex_gone", "ambiguous", "piece_gone")
# A block path's most levels (the block core's MAX_DEPTH, docs/adr/0263 §3).
SUBREF_PATH_DEPTH = 16
# Address data's bounds and levels (kentos_contracts::address).
ADDRESS_LOCATOR_LIMIT = 16
ADDRESS_MIN_SCORES = (50, 100)
ADDRESS_REACH_LIMIT = 10_000.0
AREA_LEVELS = ("neighbourhood", "district", "province")
# A composition's bounds and modes (kentos_contracts::layer_compose); `normal` is never written.
MAX_MASK_SIZE = 10.0
MAX_MASK_TARGETS = 64
BLEND_MODES = ("multiply", "screen", "overlay", "darken", "lighten", "colorDodge", "colorBurn", "hardLight", "softLight", "difference", "exclusion", "addition")
# Attachments' and actions' bounds (kentos_contracts::attachment, ::layer_info).
MAX_ATTACHMENTS = 64
MAX_ATTACHMENT_NAME = 256
MAX_ATTACHMENT_NOTE = 1000
MAX_ATTACHMENT_LINK = 4096
MAX_ACTIONS = 32
MAX_ACTION_NAME = 100
MAX_ACTION_TARGET = 4096
MAX_CARD_TITLE = 1000
MAX_CARD_TEXT = 10000
MAX_CARD_FIELDS = 200
# A dataset's bounds (kentos_contracts::raster).
MAX_DATASET_NAME = 256
MAX_DATASET_DIMS = 8
MAX_DIM_VALUES = 100_000
# The label engine's bounds (kentos_contracts::labels).
LABEL_CLASSES_MAX = 64
LABEL_EXPRESSION_MAX = 10_000
LABEL_NAME_MAX = 100
LABEL_WORDS_MAX = 500
LABEL_PINS_MAX = 64
# A filter's bounds (kentos_contracts::layer_filter).
FILTER_EXPRESSION_MAX = 10_000
FILTER_OBJECTS_MAX = 100_000
# A time setting's field names and a scenario's note (kentos_contracts::temporal).
TIME_FIELD_MAX = 64
SCENARIO_NOTE_MAX = 500
# Networks' bounds and names (kentos_contracts::network).
NETWORK_KINDS = ("road", "utility")
NETWORK_CONNECTS = ("ends", "vertices")
JUNCTION_ROLES = ("junction", "source", "valve")
NETWORK_COST_KINDS = ("speed", "field")
DIRECTION_KINDS = ("both", "digitized", "field")
U_TURNS = ("allowed", "deadEnds", "never")
SUBNETWORK_REACHES = ("downstream", "connected")
# Map services' bounds and names (kentos_contracts::service).
SERVICE_KINDS = ("xyz", "wms", "wmts", "ogcTiles", "arcgis", "google", "vector")
FEED_KINDS = ("wfs", "ogcFeatures", "arcgis", "geojson")
AUTH_KINDS = ("none", "query", "header", "basic", "bearer", "arcgis", "oauth2", "google")
GOOGLE_MAP_TYPES = ("roadmap", "satellite", "terrain", "hybrid")
WMS_VERSIONS = ("1.1.1", "1.3.0")
WFS_VERSIONS = ("1.0.0", "1.1.0", "2.0.0")
MAX_SERVICE_URL = 4096
MAX_SERVICE_LAYERS = 64
MAX_TILE_MATRICES = 40
MAX_SERVICE_PARAMS = 32
MAX_SUBDOMAINS = 16
MAX_ZOOM = 30
MAX_FEED_LIMIT = 500_000
MAX_CONNECTIONS = 256
MAX_AUTH_NAMES = 8
# A point cloud's bounds (kentos_contracts::pointcloud): its files, a path's or an address's letters, a point's size,
# its opacity; its files' formats, looks, units (px is the field's absence) and shapes (round is the field's absence).
MAX_CLOUD_SOURCES = 4096
MAX_CLOUD_PATH = 4096
MIN_POINT_SIZE = 0.5
MAX_POINT_SIZE = 32.0
MIN_CLOUD_OPACITY = 0.1
MAX_CLOUD_OPACITY = 1.0
CLOUD_FORMATS = ("las", "laz", "copc", "xyz")
CLOUD_RENDERS = ("rgb", "classification", "elevation", "intensity", "returns", "single")
# The project's annotation heights by kind (docs/adr/0205 §1): each on paper, over 0 and at most 100 mm.
ANNOTATION_KINDS = ("text", "leader", "dimension", "table", "coordinate", "station", "measure")
MAX_ANNOTATION_MM = 100.0
# A leader's arrowhead's size, times its note's height (docs/adr/0205 §7).
MIN_LEADER_ARROW = 0.1
MAX_LEADER_ARROW = 10.0
LINE_TYPES = ("continuous", "dashed", "dashdot", "dotted")
# The topology rules' kinds (docs/adr/0202 §1): whether each is between two layers and what value it takes.
TOPOLOGY_KINDS = {
    "mustNotOverlap": (False, None),
    "mustNotHaveGaps": (False, None),
    "mustNotHaveSlivers": (False, "length"),
    "mustNotHaveDuplicates": (False, None),
    "mustNotHaveDangles": (False, None),
    "mustNotHaveShortEdges": (False, "length"),
    "mustNotHaveSmallAngles": (False, "angle"),
    "mustBeValid": (False, None),
    "mustNotHaveMissingVertices": (False, None),
    "mustNotOverlapWith": (True, None),
    "mustBeCoveredBy": (True, None),
    "boundaryMustBeCoveredBy": (True, None),
    "mustBeOnEndOf": (True, None),
}
# A picture's bounds (kentos_contracts::image): its opacity, its clip's corners, a side (m), a linked file's letters.
MIN_IMAGE_OPACITY = 0.1
MAX_IMAGE_OPACITY = 1.0
MAX_CLIP_CORNERS = 10_000
MAX_IMAGE_SIZE = 1e7
MAX_IMAGE_PATH = 4096
# A raster's bounds (kentos_contracts::raster): a side (pixels), its bands, a linked file's letters, its opacity; its
# samples, looks, stretches (none is the field's absence), resamplings (bilinear is the field's absence) and ramps.
MAX_RASTER_SIDE = 4_000_000
MAX_RASTER_BANDS = 255
MAX_RASTER_PATH = 4096
MIN_RASTER_OPACITY = 0.1
MAX_RASTER_OPACITY = 1.0
RASTER_SAMPLES = ("u8", "i8", "u16", "i16", "u32", "i32", "f32", "f64")
RASTER_RENDERS = ("rgb", "gray", "palette", "ramp", "hillshade", "rampShade")
RASTER_STRETCHES = ("minMax", "percent", "manual")
RASTER_RESAMPLINGS = ("nearest",)
RASTER_RAMPS = ("Gri", "Arazi", "Spektral", "Viridis", "Mavi-kırmızı", "Sıcaklık")
HATCH_TYPES = ("solid", "lines", "cross", "pattern", "gradient")
GRADIENT_SHAPES = ("linear", "cylinder", "spherical")
# A pattern's bounds (kentos_contracts::hatch): families, dashes a family, its name's letters, a number; a tie's objects.
MAX_PATTERN_LINES = 64
MAX_PATTERN_DASHES = 16
MAX_PATTERN_NAME = 64
MAX_PATTERN_SIZE = 1e6
MAX_ASSOC_OBJECTS = 100_000
DRAWING_FONTS = ("barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono")
DIMENSION_ARROWS = ("closed", "open", "dot", "none")
MAX_OBLIQUE = 85.0
MAX_DIMENSION_RATIO = 100.0
MAX_DIMENSION_DECIMALS = 8
MAX_AFFIX = 32
MAX_STYLE_NAME = 64
MAX_STYLE_MM = 1000.0
# A multi-line text's line spacing's bounds (AutoCAD's own).
MIN_LINE_SPACING = 0.25
MAX_LINE_SPACING = 4.0
REGISTRY_DATUMS = ("TUREF", "ED50", "WGS84")
# The kinds a layer may keep to; `endpoint` brings the quadrants with it, so `quadrant` is none of them.
LAYER_SNAP_KINDS = ("endpoint", "midpoint", "center", "node", "intersection", "perpendicular", "tangent", "nearest", "centroid", "extension", "parallel", "grid")
# A dimension's kinds; schema 9 added the last five.
DIMENSION_STYLES = ("aligned", "linear", "angular", "radius", "diameter", "ordinate", "arcLength", "jogged", "azimuth", "slope",
                    "centerMark", "centerLine", "elevation")
# A leader's arrowheads (spec §6.6); the filled arrow is the field's absence, no value. Schema 30 added AutoCAD's others
# (docs/adr/0205 §7).
LEADER_ARROWS = ("open", "dot", "none")
LEADER_ARROWS_30 = LEADER_ARROWS + ("closed", "open30", "open90", "dotSmall", "dotBlank", "oblique", "archTick", "boxFilled", "boxBlank", "datumFilled")
# A table's bounds (kentos_contracts::table): rows, columns, cells, a cell's letters, a size (m), a source's objects.
MAX_TABLE_ROWS = 10_000
MAX_TABLE_COLUMNS = 100
MAX_TABLE_CELLS = 100_000
MAX_CELL_LETTERS = 1000
MAX_TABLE_SIZE = 1e6
MAX_SOURCE_OBJECTS = 100_000
# A table's columns' alignments and its lines (spec §6.6); all lines is the field's absence, no value.
TABLE_ALIGNS = ("left", "center", "right")
TABLE_GRIDS = ("outer", "rows", "none")
# A text's alignments (spec §6.6); the left of the baseline is the field's absence, no value.
TEXT_ALIGNS = ("baselineCenter", "baselineRight", "bottomLeft", "bottomCenter", "bottomRight", "middleLeft", "middleCenter", "middleRight", "topLeft", "topCenter", "topRight")
# The widest a text's letters may be drawn, times their width.
MAX_WIDTH_FACTOR = 100.0
MAX_BLOCK_DEPTH = 16
def field_records_ok(raw, count):
    """Whether `raw` is `count` whole field records (docs/adr/0553 §1): two 8-byte shape hashes, a flag byte (1:
    attributes, 2: vertices), a 2-byte attribute count and that many (4-byte name hash, 8-byte value hash) in strictly
    rising name hash order, a 2-byte vertex count (at most 128) and that many 4-byte hashes in strictly rising order;
    what a flag does not name is empty."""
    at = 0

    def take(n):
        nonlocal at
        if at + n > len(raw):
            raise ValueError
        part = raw[at:at + n]
        at += n
        return part

    try:
        for _ in range(count):
            take(16)
            flags = take(1)[0]
            if flags & ~3:
                return False
            na = int.from_bytes(take(2), "big")
            names = [int.from_bytes(take(12)[:4], "big") for _ in range(na)]
            nv = int.from_bytes(take(2), "big")
            if nv > MAX_BASE_VERTICES:
                return False
            vertices = [int.from_bytes(take(4), "big") for _ in range(nv)]
            if any(a >= b for a, b in zip(names, names[1:])) or any(a >= b for a, b in zip(vertices, vertices[1:])):
                return False
            if (not flags & 1 and na) or (not flags & 2 and nv):
                return False
    except ValueError:
        return False
    return at == len(raw)


SCHEMAS = (DOCUMENT_VERSION, SCHEMA_WITH_LINE_WEIGHTS, SCHEMA_WITH_ELEVATIONS, SCHEMA_WITH_PARTS, SCHEMA_WITH_BLOCKS, SCHEMA_WITH_TEXT_EXTRAS, SCHEMA_WITH_LEADERS, SCHEMA_WITH_DIMENSIONS, SCHEMA_WITH_LAYER_SNAP, SCHEMA_WITH_DRAWING_UNIT, SCHEMA_WITH_SECOND_SRID, SCHEMA_WITH_CUSTOM_CRS, SCHEMA_WITH_SURVEY, SCHEMA_WITH_TRAVERSE_TOLERANCES, SCHEMA_WITH_GROUND, SCHEMA_WITH_LINE_PARTS, SCHEMA_WITH_LINKED_TEXTS, SCHEMA_WITH_LAYER_STATES, SCHEMA_WITH_PARAGRAPHS, SCHEMA_WITH_STYLES, SCHEMA_WITH_TABLES, SCHEMA_WITH_HATCH_PATTERNS, SCHEMA_WITH_IMAGES, SCHEMA_WITH_TEXT_PATHS, SCHEMA_WITH_LAYER_FIELDS, SCHEMA_WITH_TOPOLOGY, SCHEMA_WITH_SURVEY_SIGMAS, SCHEMA_WITH_RASTERS, SCHEMA_WITH_ANNOTATION, SCHEMA_WITH_POINT_CLOUDS, SCHEMA_WITH_SERVICES, SCHEMA_WITH_NETWORKS, SCHEMA_WITH_TEMPORAL, SCHEMA_WITH_FILTERS, SCHEMA_WITH_LABELS, SCHEMA_WITH_VARIABLES, SCHEMA_WITH_MULTIDIM, SCHEMA_WITH_MEASURES, SCHEMA_WITH_ATTACHMENTS, SCHEMA_WITH_COMPOSE, SCHEMA_WITH_ADDRESSES, SCHEMA_WITH_DIMENSION_REFS, SCHEMA_WITH_SHEETS, SCHEMA_WITH_REQUESTS, SCHEMA_WITH_POLYLINE_EXTRAS, SCHEMA_WITH_NURBS, SCHEMA_WITH_LOOK, SCHEMA_WITH_UCS, SCHEMA_WITH_IMPERIAL, SCHEMA_WITH_SUBREF_EXTRAS, SCHEMA_WITH_VIEWS, SCHEMA_WITH_ARRAYS, SCHEMA_WITH_WIPEOUTS, SCHEMA_WITH_FROZEN, SCHEMA_WITH_PATTERNS, SCHEMA_WITH_BLOCK_EXTRAS, SCHEMA_WITH_BLOCK_CARDS, SCHEMA_WITH_DYNAMIC_BLOCKS, SCHEMA_WITH_DIMENSION_RULES, SCHEMA_WITH_DIMENSION_OPTIONS, SCHEMA_WITH_DIMENSION_MARKS, SCHEMA_WITH_OVERRIDES, SCHEMA_WITH_ANNOTATIVE, SCHEMA_WITH_PARAGRAPH_LAYOUT, SCHEMA_WITH_TEXT_FIELDS, SCHEMA_WITH_MULTILEADER, SCHEMA_WITH_TABLE_CELLS, SCHEMA_WITH_HATCH_BOUNDARIES, SCHEMA_WITH_HATCH_GAPS, SCHEMA_WITH_HATCH_EXTRAS, SCHEMA_WITH_FORMS, SCHEMA_WITH_CONSTRAINTS, SCHEMA_WITH_SOURCES, SCHEMA_WITH_WALLS, SCHEMA_WITH_WALL_JOINS, SCHEMA_WITH_OPENINGS, SCHEMA_WITH_SPACES, SCHEMA_WITH_ARCH_ELEMENTS, SCHEMA_WITH_TAGS, SCHEMA_WITH_VIEWMARKS, SCHEMA_WITH_PHASES, SCHEMA_WITH_XREFS, SCHEMA_WITH_XREF_PATHS, SCHEMA_WITH_CLIPPING, SCHEMA_WITH_HEIGHTS, SCHEMA_WITH_NETWORK_TURNS, SCHEMA_WITH_GEOREF, SCHEMA_WITH_JOIN_VIEWS, SCHEMA_WITH_RECORDED_TIME, SCHEMA_WITH_SOURCE_BASES, SCHEMA_WITH_SOURCE_FIELD_BASES, SCHEMA_WITH_NETWORK_TERMINALS, SCHEMA_WITH_NETWORK_CONFIGS, SCHEMA_WITH_NETWORK_PROFILES, SCHEMA_WITH_TIMED_CLOSURES, SCHEMA_WITH_LINEAR_EVENTS, SCHEMA_WITH_SOURCE_CRS, SCHEMA_WITH_EXCEPTION_DIGESTS, SCHEMA_WITH_GEOREF_CRS, SCHEMA_WITH_LINEAGE, SCHEMA_WITH_LAYER_GEOMETRY, SCHEMA_WITH_RASTER_TILES, SCHEMA_WITH_FIELD_KINDS, SCHEMA_WITH_JOIN_SOURCES, SCHEMA_WITH_DATASETS, SCHEMA_WITH_WITHHELD_FIELDS, SCHEMA_WITH_SPACE_COLUMNS, SCHEMA_WITH_SPACE_SCOPE, SCHEMA_WITH_SPACE_SOURCES, SCHEMA_WITH_SPACE_SEPARATORS, SCHEMA_WITH_SPACE_CHOICES, SCHEMA_WITH_DIMENSION_SETS, SCHEMA_WITH_DIMENSION_SET_EDGES, SCHEMA_WITH_RAILINGS)
# Unicode's White_Space characters: a block's name is not made of these alone.
WHITE_SPACE = set("\t\n\x0b\x0c\r \x85\xa0\u1680\u2028\u2029\u202f\u205f\u3000") | {chr(c) for c in range(0x2000, 0x200B)}


# ── The project's coordinate systems' rules (spec §6.4.1; kentos_contracts::crs) ────────────────────────────────


def finite_all(values):
    return all(math.isfinite(x) for x in values)


def helmert_problem(h):
    if not finite_all(h["translation"] + h["rotation"] + [h["scale"]]):
        return "yedi parametre sonlu sayılar olmalı"
    if "accuracy" in h and not (math.isfinite(h["accuracy"]) and h["accuracy"] >= 0):
        return "doğruluk 0 ya da büyük bir sayı olmalı"
    return None


def datum_problem(s):
    """A definition's datum: exactly one of `datum` and `customDatum`, the project's by its rules."""
    if ("datum" in s) == ("customDatum" in s):
        return "datum ya kayıttaki bir ad ya projenin datumu olur, yalnız biri"
    if "datum" in s:
        return None
    d = s["customDatum"]
    e = d["ellipsoid"]
    if not d["name"].strip() or not e["name"].strip():
        return "datumun ve elipsoidin adı boş olamaz"
    ok = math.isfinite(e["semiMajor"]) and e["semiMajor"] > 0 and math.isfinite(e["inverseFlattening"]) and e["inverseFlattening"] > 1
    if not ok:
        return "elipsoidin büyük yarı ekseni 0'dan, ters basıklığı 1'den büyük olmalı"
    return helmert_problem(d["toWgs84"]) if "toWgs84" in d else None


def plane_problem(p):
    if p["kind"] == "similarity":
        if not finite_all([p["east"], p["north"], p["rotation"], p["scale"]]) or p["scale"] <= 0:
            return "benzerliğin değerleri sonlu, ölçeği 0'dan büyük olmalı"
        return None
    if not finite_all([p[k] for k in "abcdef"]) or p["a"] * p["e"] - p["b"] * p["d"] == 0:
        return "afinin katsayıları sonlu olmalı, düzlemi katlamamalı"
    return None


def crs_problem(d, base=False):
    """What is wrong with a definition (kentos_contracts::CrsDefinition::problem), or None."""
    if not d["name"].strip():
        return "tanımın adı boş olamaz"
    s = d["system"]
    if s["kind"] == "tm":
        lat0 = s.get("latitudeOfOrigin", 0.0)
        if not finite_all([lat0, s["centralMeridian"], s["scaleFactor"], s["falseEasting"], s["falseNorthing"]]):
            return "izdüşümün değerleri sonlu sayılar olmalı"
        if s["scaleFactor"] <= 0:
            return "ölçek 0'dan büyük olmalı"
        if abs(s["centralMeridian"]) > 180 or abs(lat0) > 90:
            return "orta meridyen ya da başlangıç enlemi aralık dışında"
        return datum_problem(s)
    if base:
        return "yerel sistemin tabanı bir TM izdüşümü olmalı"
    if s["kind"] == "geographic":
        return datum_problem(s)
    b = s["base"]
    if ("srid" in b) == ("definition" in b):
        return "yerel sistemin tabanı ya bir EPSG kodu ya bir tanımdır, yalnız biri"
    if "srid" in b:
        return "yerel sistemin tabanı kayıttaki bir sistem olmalı" if b["srid"] == 0 else plane_problem(s["plane"])
    return crs_problem(b["definition"], True) or plane_problem(s["plane"])


# A layer's fields (docs/adr/0199 §1): kinds, and the rules a field list keeps.
FIELD_KINDS = ("text", "integer", "decimal", "date", "boolean")
# docs/adr/0629 §2: schema 104's kinds.
WIDE_KINDS = ("bigint", "instant", "uuid", "json", "binary")
JS_SPACE = " \t\n\v\f\r\u00a0\u1680\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u2028\u2029\u202f\u205f\u3000\ufeff"


def _fold(text):
    out = []
    for c in text:
        low = "ı" if c == "I" else "i" if c == "İ" else c.lower()
        out.append(low if len(low) == 1 else c)
    return "".join(out)


def _canonical(kind, scale, v):
    """The canonical text of `v` in a field of `kind` (None: it is no value of the kind)."""
    s = v.strip(JS_SPACE)
    if kind in ("integer", "bigint"):
        m = re.fullmatch(r"([+-]?)([0-9]+)", s)
        if not m:
            return None
        n = int(m.group(2)) * (-1 if m.group(1) == "-" else 1)
        # docs/adr/0629 §2: a big integer is a 64-bit one.
        if (kind == "integer" and abs(n) > 9007199254740991) or not -(1 << 63) <= n < (1 << 63):
            return None
        return str(n)
    if kind == "decimal":
        m = re.fullmatch(r"([+-]?)([0-9]*)(?:[.,]([0-9]*))?", s)
        if not m or not (m.group(2) or m.group(3)) or len(m.group(2)) + len(m.group(3) or "") > 30:
            return None
        frac = m.group(3) or ""
        if scale is not None and len(frac) > scale:
            return None
        whole = m.group(2).lstrip("0") or "0"
        zero = set(whole + frac) <= {"0"}
        return ("-" if m.group(1) == "-" and not zero else "") + whole + ("." + frac if frac else "")
    return s


def _instant_moment(t):
    """A time stamp's canonical text (docs/adr/0629 §2) as its moment in UTC: seconds and nanoseconds; None if it is
    not one."""
    import datetime

    m = re.fullmatch(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{0,8}[1-9]))?(Z|[+-]\d{2}:\d{2})", t)
    if not m:
        return None
    try:
        day = datetime.date(int(m.group(1)), int(m.group(2)), int(m.group(3)))
    except ValueError:
        return None
    h, mi, sec = int(m.group(4)), int(m.group(5)), int(m.group(6))
    if h > 23 or mi > 59 or sec > 59:
        return None
    z = m.group(8)
    offset = 0
    if z != "Z":
        oh, om = int(z[1:3]), int(z[4:6])
        if om > 59 or oh * 60 + om > 18 * 60 or (oh == 0 and om == 0):
            return None
        offset = (oh * 3600 + om * 60) * (1 if z[0] == "+" else -1)
    days = (day - datetime.date(1970, 1, 1)).days
    return (days * 86400 + h * 3600 + mi * 60 + sec - offset, int((m.group(7) or "").ljust(9, "0")))


def _number(text):
    from fractions import Fraction

    return Fraction(text)


def _control(text):
    return any(unicodedata.category(c) == "Cc" for c in text)


def http_problem(what, text):
    """Why `text` is no HTTP or HTTPS address of at most MAX_SERVICE_URL letters without control characters
    (kentos_contracts::service::http_problem)."""
    lower = text.strip().lower()
    rest = lower[8:] if lower.startswith("https://") else lower[7:] if lower.startswith("http://") else None
    if rest is None or not rest or rest.startswith("/"):
        return f"{what} bir HTTP ya da HTTPS adresi olmalı (http:// ya da https:// ile başlamalı); “{text}” değil."
    if len(text) > MAX_SERVICE_URL or _control(text):
        return f"{what} en çok {MAX_SERVICE_URL} harf olmalı ve denetim karakteri içermemeli."
    return None


def _token(name):
    return 0 < len(name) <= 64 and all(c.isascii() and (c.isalnum() or c in "-_.") for c in name)


def _plain(text, most):
    return len(text) <= most and not _control(text)


def grid_problem(g):
    if g["srid"] == 0:
        return "Karo ızgarasının sistemi bir EPSG kodu olmalı."
    ms = g["matrices"]
    if not 0 < len(ms) <= MAX_TILE_MATRICES:
        return f"Karo ızgarasının 1 ile {MAX_TILE_MATRICES} arasında matrisi olmalı."
    for i, m in enumerate(ms, 1):
        if not m["id"] or not _plain(m["id"], 128):
            return f"{i}. matrisin adı 1 ile 128 harf arasında olmalı."
        if any(o["id"] == m["id"] for o in ms[: i - 1]):
            return f"“{m['id']}” matrisi iki kez var."
        if not (math.isfinite(m["resolution"]) and m["resolution"] > 0):
            return f"{i}. matrisin pikseli sıfırdan büyük, sonlu olmalı."
        if not all(1 <= m[k] <= 4096 for k in ("tileWidth", "tileHeight")):
            return f"{i}. matrisin karosu 1 ile 4096 piksel arasında olmalı."
        if not all(1 <= m[k] <= 1 << 40 for k in ("matrixWidth", "matrixHeight")):
            return f"{i}. matrisin karo sayısı 1 ile 2⁴⁰ arasında olmalı."
    return None


def service_problem(s):
    """A layer's map service by the contract's rules (kentos_contracts::service::ServiceLayer::problem), in their order."""
    kind = s["kind"]
    if kind == "google":
        if s["url"]:
            return "Google katmanının adresi boş olmalı; karoların adresi oturumdan gelir."
        if s.get("style") not in GOOGLE_MAP_TYPES:
            return "Google katmanının harita türü bir Google harita türü olmalı."
        if "connection" not in s:
            return "Google katmanının API anahtarı bir bağlantıda olmalı."
    else:
        p = http_problem("Servisin adresi", s["url"])
        if p:
            return p
    u = s["url"]
    layers = s.get("layers", [])
    if kind == "xyz":
        if not ("{quadkey}" in u or ("{z}" in u and "{x}" in u and ("{y}" in u or "{-y}" in u))):
            return "XYZ şablonunda {z}, {x} ve {y} (ya da {-y}) ya da {quadkey} olmalı."
        if "{s}" in u and not s.get("subdomains"):
            return "Şablonda {s} var; alt alanları verin."
    elif kind == "wms":
        if not layers:
            return "WMS katmanı en az bir katman adı istemeli."
        if "srid" not in s:
            return "WMS katmanının istenen sistemi (srid) olmalı."
        if "version" in s and s["version"] not in WMS_VERSIONS:
            return "WMS sürümü 1.1.1 ya da 1.3.0 olmalı."
    elif kind == "wmts":
        if len(layers) != 1:
            return "WMTS katmanı tek bir katman göstermeli."
        if "grid" not in s or "matrixSet" not in s:
            return "WMTS katmanının karo ızgarası ve matris kümesinin adı olmalı."
    elif kind == "ogcTiles":
        if "grid" not in s or "template" not in s:
            return "OGC API Tiles katmanının ızgarası ve karo şablonu olmalı."
    elif kind == "arcgis":
        if "srid" not in s and "grid" not in s:
            return "ArcGIS katmanının ızgarası ya da istenen sistemi olmalı."
    if len(layers) > MAX_SERVICE_LAYERS or any(not l or not _plain(l, 256) for l in layers):
        return "Katman adları en çok 64 tane ve 1 ile 256 harf arasında olmalı."
    for key, most in (("style", 256), ("format", 128), ("attribution", 512), ("version", 16), ("preset", 64), ("connection", 64), ("matrixSet", 256)):
        if key in s and (not s[key] or not _plain(s[key], most)):
            return f"{key} 1 ile {most} harf arasında olmalı ve denetim karakteri içermemeli."
    if "template" in s:
        p = http_problem("Karo şablonu", s["template"])
        if p:
            return p
    if s.get("srid") == 0:
        return "İstenen sistem bir EPSG kodu olmalı."
    if "grid" in s:
        p = grid_problem(s["grid"])
        if p:
            return p
    if "tileSize" in s and not 64 <= s["tileSize"] <= 4096:
        return "Karonun boyu 64 ile 4096 piksel arasında olmalı."
    lo, hi = s.get("minZoom", 0), s.get("maxZoom", MAX_ZOOM)
    if lo > hi or hi > MAX_ZOOM:
        return "Katlar 0 ile 30 arasında olmalı, en küçüğü en büyüğünden büyük olmamalı."
    subs = s.get("subdomains", [])
    if len(subs) > MAX_SUBDOMAINS or not all(_token(x) for x in subs):
        return "Alt alanlar en çok 16 tane; her biri harf, rakam ve tire olmalı."
    params = s.get("params", [])
    if len(params) > MAX_SERVICE_PARAMS:
        return "En çok 32 ek parametre olabilir."
    for p in params:
        if not _token(p["name"]) or not _plain(p["value"], 1024):
            return f"Ek parametre “{p['name']}”: adı ve değeri uygun değil."
    o = s.get("opacity")
    if o is not None and not (math.isfinite(o) and 0.1 <= o <= 1.0):
        return f"Servis katmanının donukluğu 0.1 ile 1 arasında olmalı; {o} verildi."
    b = s.get("bbox")
    if b is not None:
        w, so, e, n = b
        if not (all(math.isfinite(x) for x in b) and -180 <= w <= e <= 180 and -90 <= so <= n <= 90):
            return "Servisin kapsamı WGS 84 derecesinde batı, güney, doğu ve kuzey olmalı."
    return None


def feed_problem(f):
    """A layer's feed by the contract's rules (kentos_contracts::service::FeatureFeed::problem), in their order."""
    p = http_problem("Verinin adresi", f["url"])
    if p:
        return p
    if "name" not in f and f["kind"] in ("wfs", "ogcFeatures", "arcgis"):
        return "Kaynağın tür, koleksiyon ya da katman adı (name) olmalı."
    if "name" in f and (not f["name"] or not _plain(f["name"], 256)):
        return "Tür adı 1 ile 256 harf arasında olmalı."
    if "version" in f:
        if f["kind"] != "wfs":
            return "Sürüm yalnız WFS kaynağının olur."
        if f["version"] not in WFS_VERSIONS:
            return "WFS sürümü 1.0.0, 1.1.0 ya da 2.0.0 olmalı."
    if f.get("srid") == 0:
        return "İstenen sistem bir EPSG kodu olmalı."
    b = f.get("bbox")
    if b is not None and not (all(math.isfinite(x) for x in b) and b[0] <= b[2] and b[1] <= b[3]):
        return "İstenen alan sonlu dört sayı olmalı, en küçükler en büyüklerden büyük olmamalı."
    if "limit" in f and not 1 <= f["limit"] <= MAX_FEED_LIMIT:
        return "En çok nesne 1 ile 500000 arasında olmalı."
    for key, most in (("filter", 8192), ("key", 128), ("connection", 64), ("fetched", 40)):
        if key in f and (not f[key] or not _plain(f[key], most)):
            return f"{key} 1 ile {most} harf arasında olmalı ve denetim karakteri içermemeli."
    return None


def origin_problem(origin):
    """Why `origin` is no lower-case `http(s)://host[:port]` without a path (kentos_contracts::service::origin_problem)."""
    rest = origin[8:] if origin.startswith("https://") else origin[7:] if origin.startswith("http://") else None
    bad = f"Bağlantının kökeni şema ve makine olmalı, yolu olmamalı; “{origin}” değil."
    if rest is None or any(c in rest for c in "/?#@"):
        return bad
    if rest.startswith("["):
        close = rest.find("]")
        if close < 0:
            return bad
        host, after = rest[: close + 1], rest[close + 1:]
        if after and not after.startswith(":"):
            return bad
        port = after[1:] if after else None
        ok = len(host) > 2 and all(c in "0123456789abcdef:." for c in host[1:-1])
    else:
        host, _, port = rest.rpartition(":") if ":" in rest else (rest, "", None)
        port = port if ":" in rest else None
        ok = 0 < len(host) <= 253 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-." for c in host)
    if port is not None and not (port.isdigit() and 0 < int(port) <= 65535):
        ok = False
    return None if ok else bad


def connections_problem(list_):
    """The project's connections by the contract's rules (kentos_contracts::service::connections_problem)."""
    if len(list_) > MAX_CONNECTIONS:
        return "Projenin en çok 256 bağlantısı olabilir."
    for i, c in enumerate(list_):
        cid = c["id"]
        if not (0 < len(cid) <= 64 and all(ch.isascii() and (ch.isalnum() or ch in "-_") for ch in cid)):
            return "Bağlantının kimliği 1 ile 64 harf, rakam, tire ya da alt çizgi olmalı."
        if not c["name"].strip() or not _plain(c["name"], 128):
            return "Bağlantının adı 1 ile 128 harf arasında olmalı."
        p = origin_problem(c["origin"])
        if p:
            return p
        names = c.get("names", [])
        named = c["auth"] in ("query", "header")
        if named and not 0 < len(names) <= MAX_AUTH_NAMES:
            return "Parametre ya da başlık bağlantısının 1 ile 8 arasında adı olmalı."
        if not named and names:
            return "Bu doğrulama türü ad taşımaz."
        if not all(_token(n) for n in names):
            return "Parametre ve başlık adları harf, rakam, tire, nokta ve alt çizgiden olmalı."
        token = c.get("tokenUrl")
        if token is None and c["auth"] == "oauth2":
            return "OAuth 2 bağlantısının belirteç adresi olmalı."
        if token is not None:
            if c["auth"] not in ("arcgis", "oauth2"):
                return "Belirteç adresi yalnız ArcGIS ve OAuth 2 bağlantısının olur."
            p = http_problem("Belirteç adresi", token)
            if p:
                return p
        if "scope" in c:
            if c["auth"] != "oauth2":
                return "Kapsam yalnız OAuth 2 bağlantısının olur."
            if not c["scope"] or not _plain(c["scope"], 256):
                return "Kapsam 1 ile 256 harf arasında olmalı."
        if any(o["id"] == cid for o in list_[:i]):
            return f"“{cid}” kimlikli bağlantı iki kez var."
    return None


def caseless(s):
    """A text as the networks' rules compare it: trimmed, upper and lower case the same in Turkish (I is ı's, İ is i's)."""
    out = []
    for ch in s.strip():
        if ch == "I":
            out.append("ı")
        elif ch == "İ":
            out.append("i")
        else:
            low = ch.lower()
            out.append(low if len(low) == 1 else ch)
    return "".join(out)


def network_problem(n):
    """A network by the rules of docs/adr/0209 §2 (kentos_contracts::network::NetworkDef::problem)."""
    nid = n["id"]
    if not (1 <= len(nid) <= 40 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in nid)):
        return "Ağın kimliği 1–40 küçük harf, rakam ya da tire olmalı."
    name = n["name"]
    if not name.strip() or name.strip() != name or len(name) > 80:
        return "Ağın adı boş, kırpılmamış ya da 80 karakterden uzun."

    def bad_expression(e):
        return e is not None and (not e.strip() or len(e) > 1000)

    edges, junctions = n["edges"], n.get("junctions", [])
    if not edges:
        return "Ağın kenar katmanı yok."
    if len(edges) > 16 or len(junctions) > 16:
        return "Ağın en çok 16 kenar ve 16 düğüm katmanı olur."
    seen = []
    for e in edges:
        key = (e["layer"], e.get("filter"))
        if not e["layer"] or bad_expression(e.get("filter")) or key in seen:
            return "Kenar katmanı boş, süzgeci boş ya da uzun, ya da aynı süzgeçle iki kez var."
        seen.append(key)
    seen = []
    for j in junctions:
        key = (j["layer"], j.get("filter"))
        if not j["layer"] or bad_expression(j.get("filter")) or bad_expression(j.get("closed")) or key in seen:
            return "Düğüm katmanı boş, ifadesi boş ya da uzun, ya da aynı süzgeçle iki kez var."
        seen.append(key)
    tol = n["tolerance"]
    if not (math.isfinite(tol) and 0.0001 <= tol <= 10.0):
        return "Ağın toleransı 0,0001 ile 10 m arasında olmalı."
    d = n["direction"]
    if d["kind"] == "field":
        if not d["field"].strip() or len(d["field"]) > 64:
            return "Yön alanı boş ya da çok uzun."
        lists = [d.get("forward", []), d.get("backward", []), d.get("closed", [])]
        if not any(lists):
            return "Yön alanının en az bir değeri olmalı."
        values = []
        for lst in lists:
            if len(lst) > 16:
                return "Yön listesinde en çok 16 değer olur."
            for v in lst:
                if not v.strip() or len(v) > 40:
                    return "Yön değeri boş ya da 40 karakterden uzun."
                if caseless(v) in values:
                    return "Yön değeri iki kez var."
                values.append(caseless(v))
    costs = n.get("costs", [])
    if len(costs) > 8:
        return "Ağın en çok 8 ek maliyeti olur."
    names = []
    for c in costs:
        cname = c["name"]
        if not cname.strip() or cname.strip() != cname or len(cname) > 40:
            return "Maliyetin adı boş, kırpılmamış ya da uzun."
        if caseless(cname) == caseless("Uzunluk") or caseless(cname) in names:
            return "Maliyetin adı Uzunluk ya da iki kez var."
        names.append(caseless(cname))
        if not c["field"].strip() or len(c["field"]) > 64:
            return "Maliyetin alanı boş ya da çok uzun."
        unit, speed = c.get("unit", ""), c.get("speed")
        if c["kind"] == "speed":
            if unit or speed is None or not (math.isfinite(speed) and 0.0 < speed <= 1000.0):
                return "Süre maliyeti birim almaz; varsayılan hızı 0 ile 1000 km/sa arasında olmalı."
        elif speed is not None or len(unit) > 12 or unit.strip() != unit:
            return "Alan maliyeti hız almaz; birimi kırpılmış ve en çok 12 karakter olmalı."
        if c.get("periods"):
            if c["kind"] != "speed":
                return "Zaman profili yalnız hızla maliyette olur."
            p = periods_problem(c["periods"])
            if p:
                return f"Maliyetin zaman profili: {p}"
    if bad_expression(n.get("closed")):
        return "Kapalı kenarlar ifadesi boş ya da uzun."

    def bad_field(f):
        return not f.strip() or f.strip() != f or len(f) > 64

    t = n.get("turns")
    if t is not None:
        for key in ("straight", "right", "left", "back"):
            d = t.get(key, 0.0)
            if not (math.isfinite(d) and 0.0 <= d <= 3600.0):
                return "Dönüş gecikmesi 0 ile 3600 sn arasında olmalı."
        layers = t.get("layers", [])
        if len(layers) > 4:
            return "En çok 4 dönüş katmanı olur."
        seen = []
        for l_ in layers:
            key = (l_["layer"], l_.get("filter"))
            if not l_["layer"] or bad_expression(l_.get("filter")) or ("field" in l_ and bad_field(l_["field"])) or key in seen:
                return "Dönüş katmanı boş, süzgeci ya da alanı bozuk, ya da aynı süzgeçle iki kez var."
            seen.append(key)
    lv = n.get("levels")
    if lv is not None and (bad_field(lv["start"]) or bad_field(lv["end"]) or ("inside" in lv and bad_field(lv["inside"]))):
        return "Seviye alanı boş, kırpılmamış ya da çok uzun."
    tm = n.get("terminals")
    if tm is not None:
        p = terminals_problem(n, tm, bad_field)
        if p:
            return p
    p = profiles_problem(n.get("profiles", []), bad_field)
    if p:
        return p
    cl = n.get("closures")
    if cl is not None:
        if bad_field(cl["from"]) or bad_field(cl["to"]):
            return "Kapanmanın tarih alanı boş, kırpılmamış ya da çok uzun."
        if cl["from"] == cl["to"]:
            return "Kapanmanın ilk ve son gününün alanı aynı."
        for key, what in (("days", "haftanın günleri"), ("hours", "saat")):
            if key in cl and bad_field(cl[key]):
                return f"Kapanmanın {what} alanı boş, kırpılmamış ya da çok uzun."
        names = [cl[key] for key in ("from", "to", "days", "hours") if key in cl]
        if len(set(names)) != len(names):
            return "Kapanmanın iki alanı aynı."
    sn = n.get("subnetworks")
    if sn is not None:
        p = subnetworks_problem(n, sn, bad_field)
        if p:
            return p
    return None


def profiles_problem(profiles, bad_field):
    """A network's vehicle profiles (docs/adr/0567 §1; kentos_contracts::network::NetworkDef::profiles_problem): at most
    8, each named once (Turkish case aside), at most 8 limits and 8 forbidding fields, a limit's value above zero, a
    forbidding field's 1–16 trimmed values, a top speed in (0, 300] km/h."""
    if len(profiles) > 8:
        return "En çok 8 araç profili olur."
    seen = []
    for p in profiles:
        name = p["name"]
        if not name.strip() or name.strip() != name or len(name) > 40:
            return "Araç profilinin adı boş, kırpılmamış ya da çok uzun."
        key = network_caseless(name)
        if key in seen:
            return "Araç profili iki kez var."
        seen.append(key)
        limits, forbidden = p.get("limits", []), p.get("forbidden", [])
        if len(limits) > 8 or len(forbidden) > 8:
            return "Araç profilinin sınırı ya da yasağı çok."
        for l_ in limits:
            if bad_field(l_["field"]):
                return "Araç profilinin sınır alanı bozuk."
            if not (math.isfinite(l_["value"]) and l_["value"] > 0):
                return "Araç profilinin sınırı sıfırdan büyük olmalı."
        for f in forbidden:
            if bad_field(f["field"]):
                return "Araç profilinin yasak alanı bozuk."
            vs = f["values"]
            if not 1 <= len(vs) <= 16 or any(not v.strip() or v.strip() != v or len(v) > 40 for v in vs):
                return "Araç profilinin yasak değerleri bozuk."
        t = p.get("topSpeed")
        if t is not None and not (math.isfinite(t) and 0 < t <= 300):
            return "Araç profilinin en yüksek hızı 0 ile 300 km/sa arasında olmalı."
    return None


def network_caseless(text):
    """Trimmed, case folded with Turkish I and İ (kentos_contracts::network::network_caseless)."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower()) for c in text.strip())


def network_id_holds(i):
    return 1 <= len(i) <= 40 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in i)


def terminals_problem(n, t, bad_field):
    """A network's terminals (docs/adr/0557 §1; kentos_contracts::network::NetworkDef::terminals_problem)."""
    if bad_field(t["start"]) or bad_field(t["end"]):
        return "Terminal alanı boş, kırpılmamış ya da çok uzun."
    if t["start"] == t["end"]:
        return "Başlangıç ve bitiş terminal alanları aynı."
    devices = t["devices"]
    if not devices or len(devices) > 16:
        return "Terminalli cihaz yok ya da çok."
    layers = [j["layer"] for j in n.get("junctions", [])]
    for i, d in enumerate(devices):
        if d["layer"] not in layers:
            return "Cihazın katmanı ağın düğüm katmanlarından biri değil."
        if any(o["layer"] == d["layer"] for o in devices[:i]):
            return "Cihazın katmanı iki kez var."
        p = terminal_set_problem(d["terminals"], d["paths"])
        if p:
            return p
        p = configs_problem(d, bad_field)
        if p:
            return p
    return None


def terminal_set_problem(terms, paths):
    """A device's (or a configuration's) terminals and paths (kentos_contracts::network::terminal_set_problem)."""
    if not 2 <= len(terms) <= 8:
        return "Cihazın terminalleri 2–8 olmalı."
    keys = []
    for tm in terms:
        if not network_id_holds(tm["id"]):
            return "Terminal kimliği 1–40 küçük harf, rakam ya da tire olmalı."
        name = tm["name"]
        if not name.strip() or name.strip() != name or len(name) > 40:
            return "Terminal adı boş, kırpılmamış ya da 40 karakterden uzun."
        mine = [network_caseless(tm["id"])]
        if network_caseless(name) != mine[0]:
            mine.append(network_caseless(name))
        for k in mine:
            if k in keys:
                return "Cihazın terminalleri karışıyor."
            keys.append(k)
    if not paths or len(paths) > 16:
        return "Cihazın yolu yok ya da çok."
    ids = [tm["id"] for tm in terms]
    for path in paths:
        if len(path) < 2:
            return "Yol en az iki terminal içermeli."
        for m, x in enumerate(path):
            if x not in ids:
                return "Yoldaki terminal cihazın terminali değil."
            if x in path[:m]:
                return "Yolda terminal iki kez var."
    return None


def configs_problem(d, bad_field):
    """A device's configurations (docs/adr/0562 §2; kentos_contracts::network::NetworkDef::configs_problem)."""
    configs = d.get("configs", [])
    if "by" not in d:
        return "Cihazın yapılandırmaları var ama hangi öznitelikle seçildiği yok." if configs else None
    if not configs:
        return "Cihazın yapılandırma özniteliği var ama yapılandırması yok."
    if bad_field(d["by"]):
        return "Cihazın yapılandırma özniteliği boş, kırpılmamış ya da çok uzun."
    if len(configs) > 16:
        return "Cihazın yapılandırması çok."
    seen = []
    for c in configs:
        v = c["value"]
        if not v.strip() or v.strip() != v or len(v) > 40:
            return "Cihazın yapılandırma değeri boş, kırpılmamış ya da 40 karakterden uzun."
        key = network_caseless(v)
        if key in seen:
            return "Cihazın yapılandırması iki kez var."
        seen.append(key)
        p = terminal_set_problem(c["terminals"], c["paths"])
        if p:
            return p
    return None


def subnetworks_problem(n, s, bad_field):
    """A network's subnetworks (docs/adr/0557 §2; kentos_contracts::network::NetworkDef::subnetworks_problem)."""
    if not any(j["role"] == "source" for j in n.get("junctions", [])):
        return "Alt ağların denetleyicisi yok."
    if bad_field(s["attribute"]):
        return "Alt ağ özniteliği boş, kırpılmamış ya da çok uzun."
    if "name" in s and bad_field(s["name"]):
        return "Alt ağ adı alanı boş, kırpılmamış ya da çok uzun."
    reads = [s["name"]] if "name" in s else []
    if "terminals" in n:
        reads += [n["terminals"]["start"], n["terminals"]["end"]]
        reads += [d["by"] for d in n["terminals"]["devices"] if "by" in d]
    d = n["direction"]
    if d["kind"] == "field":
        reads.append(d["field"])
    for p in n.get("profiles", []):
        reads += [l_["field"] for l_ in p.get("limits", [])] + [f["field"] for f in p.get("forbidden", [])]
    if "closures" in n:
        reads += [n["closures"][key] for key in ("from", "to", "days", "hours") if key in n["closures"]]
    if s["attribute"] in reads:
        return "Alt ağ özniteliği ağın okuduğu bir alanın üstüne yazar."
    return None


def periods_problem(periods):
    """A speed cost's time profile (kentos_contracts::network::periods_problem): at most 24 periods within the day, in
    the day's order without overlapping, each a factor (0–10) or a field, not both; not the whole day closed."""
    if len(periods) > 24:
        return "en çok 24 dönem"
    last = 0
    for i, p in enumerate(periods):
        if p["from"] >= p["to"] or p["to"] > 1440:
            return f"{i + 1}. dönem günün içinde değil"
        if i > 0 and p["from"] < last:
            return f"{i + 1}. dönem bir öncekiyle örtüşüyor"
        last = p["to"]
        factor, field = p.get("factor"), p.get("field")
        if (factor is None) == (field is None):
            return f"{i + 1}. dönemin ya çarpanı ya hız alanı olur"
        if factor is not None and not (math.isfinite(factor) and 0.0 <= factor <= 10.0):
            return f"{i + 1}. dönemin çarpanı 0 ile 10 arasında olmalı"
        if field is not None and (not field.strip() or field.strip() != field or len(field) > 64):
            return f"{i + 1}. dönemin hız alanı bozuk"
    if sum(p["to"] - p["from"] for p in periods if p.get("factor") == 0.0) == 1440:
        return "bütün gün kapalı"
    return None


def variable_key(name):
    """A variable's name as names are compared: case and the Turkish letters' marks aside (ı and i alike)."""
    marks = {"ı": "I", "i": "I", "İ": "I", "ğ": "G", "Ğ": "G", "ü": "U", "Ü": "U", "ş": "S", "Ş": "S", "ö": "O", "Ö": "O", "ç": "C", "Ç": "C"}
    return "".join(marks.get(c, c.upper()) for c in name)


def is_iso_date(t):
    m = re.fullmatch(r"(\d{4})-(\d{2})-(\d{2})", t)
    if not m:
        return False
    y, mo, d = map(int, m.groups())
    if not (1 <= y <= 9999 and 1 <= mo <= 12):
        return False
    leap = y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)
    days = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][mo - 1]
    return 1 <= d <= days


def variables_problem(list_):
    """A project's variables (kentos_contracts::variables::variables_problem): at most 200; a name a letter or _ then
    letters, digits and _, at most 64 characters, no built-in one's, each once; a value of its kind."""
    if len(list_) > VARIABLES_MAX:
        return f"projede {len(list_)} değişken var; en çok {VARIABLES_MAX}"
    seen = set()
    builtins = {variable_key(b) for b in BUILTIN_VARIABLES}
    for v in list_:
        name = v["name"]
        if not name:
            return "değişkenin adı boş"
        if len(name) > VARIABLE_NAME_MAX:
            return f"“{name}” değişken adı {VARIABLE_NAME_MAX} karakterden uzun"
        if not (name[0].isalpha() or name[0] == "_") or not all(c.isalnum() or c == "_" for c in name):
            return f"“{name}” değişken adı olamaz"
        if variable_key(name) in builtins:
            return f"@{name} yerleşik bir değişkendir"
        value, kind = v.get("value"), v["kind"]
        if value is not None:
            ok = {"number": type(value) is float, "bool": type(value) is bool,
                  "text": type(value) is str and len(value) <= VARIABLE_TEXT_MAX,
                  "date": type(value) is str and is_iso_date(value)}[kind]
            if not ok:
                return f"@{name} değeri türüne uymuyor"
        if variable_key(name) in seen:
            return f"@{name} adında iki değişken var"
        seen.add(variable_key(name))
    return None


def address_field_problem(name):
    """A field's name as the address data's rules take one: not blank, trimmed, at most 64 characters, no control ones."""
    if not name.strip():
        return "alan boş"
    if name.strip() != name or len(name) > 64 or any(unicodedata.category(c) == "Cc" for c in name):
        return "alan kırpılmamış, 64 karakterden uzun ya da denetim karakterli"
    return None


def address_locator_problem(l_):
    """A locator by the rules of docs/adr/0229 §2 (kentos_contracts::address::AddressLocator::problem)."""
    lid = l_["id"]
    if not (1 <= len(lid) <= 40 and all(c in "abcdefghijklmnopqrstuvwxyz0123456789-" for c in lid)):
        return "Adres verisinin kimliği 1–40 küçük harf, rakam ya da tire olmalı."
    name = l_["name"]
    if not name.strip() or name.strip() != name or len(name) > 80:
        return "Adres verisinin adı boş, kırpılmamış ya da 80 karakterden uzun."
    doors, streets, areas = l_.get("doors"), l_.get("streets"), l_.get("areas", [])
    if doors is None and streets is None and not areas:
        return "Adres verisinin kaynağı yok."
    if doors is not None:
        if not doors["layer"]:
            return "Kapı katmanının kimliği boş."
        for key in ("street", "door", "neighbourhood", "district", "province", "postcode", "building", "code"):
            if key in doors and address_field_problem(doors[key]):
                return f"Kapıların {key} {address_field_problem(doors[key])}."
    if streets is not None:
        if not streets["layer"]:
            return "Yol katmanının kimliği boş."
        for key in ("name", "neighbourhood", "district"):
            if key in streets and address_field_problem(streets[key]):
                return f"Yolların {key} {address_field_problem(streets[key])}."
    if len(areas) > 3:
        return "Bölge katmanları en çok üç."
    levels = []
    for a in areas:
        if not a["layer"]:
            return "Bölge katmanının kimliği boş."
        if address_field_problem(a["name"]):
            return f"Bölgenin adı {address_field_problem(a['name'])}."
        if "district" in a:
            if a["level"] != "neighbourhood":
                return "Yalnız mahalle alanlarının ilçesi yazılır."
            if address_field_problem(a["district"]):
                return f"Mahalle alanlarının ilçesi {address_field_problem(a['district'])}."
        if a["level"] in levels:
            return "Bir bölge düzeyi iki kez var."
        levels.append(a["level"])
    if not (ADDRESS_MIN_SCORES[0] <= l_["minScore"] <= ADDRESS_MIN_SCORES[1]):
        return "En düşük puan 50 ile 100 arasında olmalı."
    reach = l_["reach"]
    if not (math.isfinite(reach) and 0.0 < reach <= ADDRESS_REACH_LIMIT):
        return "Uzaklık 0'dan büyük, en çok 10 000 m olmalı."
    return None


def address_locators_problem(list_):
    """A project's address data (kentos_contracts::address::address_locators_problem): at most 16, each holding, ids
    and names once."""
    if len(list_) > ADDRESS_LOCATOR_LIMIT:
        return "Projenin en çok 16 adres verisi olur."
    for i, l_ in enumerate(list_):
        p = address_locator_problem(l_)
        if p:
            return p
        if any(o["id"] == l_["id"] for o in list_[:i]):
            return f"“{l_['id']}” kimlikli adres verisi iki kez var."
        if any(caseless(o["name"]) == caseless(l_["name"]) for o in list_[:i]):
            return f"“{l_['name']}” adlı adres verisi iki kez var."
    return None


def networks_problem(list_):
    """A project's networks (kentos_contracts::network::networks_problem): at most 32, each holding, ids and names once."""
    if len(list_) > 32:
        return "Projenin en çok 32 ağı olur."
    for i, n in enumerate(list_):
        p = network_problem(n)
        if p:
            return p
        if any(o["id"] == n["id"] for o in list_[:i]):
            return f"“{n['id']}” kimlikli ağ iki kez var."
        if any(caseless(o["name"]) == caseless(n["name"]) for o in list_[:i]):
            return f"“{n['name']}” adlı ağ iki kez var."
    return None


def time_field_problem(what, name):
    if name.strip() != name:
        return f"{what} alanının adının başında ya da sonunda boşluk var"
    if not name:
        return f"{what} alanının adı boş"
    if len(name) > TIME_FIELD_MAX:
        return f"{what} alanının adı {TIME_FIELD_MAX} karakterden uzun"
    return None


def layer_time_problem(t):
    """A layer's time setting (kentos_contracts::temporal::LayerTime::problem): its fields' names trimmed, 1–64 characters;
    the end another field than the start."""
    for what, key in (("başlangıç", "start"), ("bitiş", "end"), ("kimlik", "key")):
        if key in t:
            p = time_field_problem(what, t[key])
            if p:
                return p
    if t.get("end") == t["start"]:
        return "başlangıç ve bitiş aynı alan olamaz"
    for what, key in (("kayıt başlangıcı", "recordedStart"), ("kayıt bitişi", "recordedEnd")):
        if key in t:
            p = time_field_problem(what, t[key])
            if p:
                return p
    if "recordedEnd" in t and "recordedStart" not in t:
        return "kayıt bitişi kayıt başlangıcı ister"
    valid = {t["start"], t.get("end"), t.get("key")} - {None}
    for key in ("recordedStart", "recordedEnd"):
        if t.get(key) in valid:
            return f"kayıt zamanının “{t[key]}” alanı geçerlilik zamanının ya da kimliğin alanı olamaz"
    if "recordedStart" in t and t.get("recordedStart") == t.get("recordedEnd"):
        return "kayıt başlangıcı ve bitişi aynı alan olamaz"
    return None


def scenario_problem(s):
    """A scenario's note (kentos_contracts::temporal::ScenarioInfo::problem): trimmed, 1–500 characters."""
    note = s.get("note")
    if note is None:
        return None
    if note.strip() != note or not note:
        return "senaryonun notu boş ya da başında veya sonunda boşluk var"
    if len(note) > SCENARIO_NOTE_MAX:
        return f"senaryonun notu {SCENARIO_NOTE_MAX} karakterden uzun"
    return None


def layer_filter_problem(f):
    """A layer's filter (kentos_contracts::layer_filter::LayerFilter::problem, docs/adr/0211 §2): a condition or a list
    of objects or both; the condition trimmed, 1–10 000 characters; the list at most 100 000 ids, each once (the reader
    refuses an empty one: an empty list is not written)."""
    if "expression" not in f and not f.get("objects"):
        return "süzgeçte ne ifade ne nesne listesi var"
    e = f.get("expression")
    if e is not None:
        if e.strip() != e or not e:
            return "süzgecin ifadesi boş ya da başında veya sonunda boşluk var"
        if len(e) > FILTER_EXPRESSION_MAX:
            return f"süzgecin ifadesi {FILTER_EXPRESSION_MAX} karakterden uzun"
    objects = f.get("objects")
    if objects is not None:
        if not objects:
            return "süzgecin nesne listesi boş; boş liste yazılmaz"
        if len(objects) > FILTER_OBJECTS_MAX:
            return f"süzgecin listesinde {len(objects)} nesne var; en çok {FILTER_OBJECTS_MAX}"
        seen = set()
        for x in objects:
            if x in seen:
                return f"süzgecin listesinde {x} iki kez var"
            seen.add(x)
    return None


def label_color_ok(c):
    """A colour a label may name (kentos_contracts::labels::label_color_ok): `#rrggbb`, `#rrggbbaa` or a theme name."""
    if c in ("fg", "fg-dim", "label", "ink", "paper"):
        return True
    if not c.startswith("#"):
        return False
    h = c[1:]
    return len(h) in (6, 8) and all(ch in "0123456789abcdefABCDEF" for ch in h)


def label_style_problem(s):
    """A label style with the label engine's fields (kentos_contracts::labels::style_problem, docs/adr/0212 §2): its
    numbers in their ranges, its colours and expressions as the rules say."""

    def out(x, lo, hi):
        return not (lo <= x <= hi)

    def bad_expr(e):
        return e is not None and (e.strip() != e or not e or len(e) > LABEL_EXPRESSION_MAX)

    def bad_color(c):
        return c is not None and not label_color_ok(c)

    for k, lo, hi in (("size", 1, 200), ("grow", 0, 1000), ("maxSize", 1, 200), ("minFeaturePx", 0, 100_000), ("minScale", 0, 1e9),
                      ("maxScale", 0, 1e9), ("distance", 0, 500), ("repeat", 20, 100_000), ("maxAngle", 5, 90), ("shrink", 0.5, 1),
                      ("duplicates", 1, 10_000)):
        if k in s and out(s[k], lo, hi):
            return f"{k} {lo}–{hi} arasında olmalı"
    if "weight" in s and out(s["weight"], 100, 900):
        return "kalınlık 100–900 olmalı"
    if "priority" in s and s["priority"] > 10:
        return "öncelik 0–10 olmalı"
    if bad_expr(s.get("text")):
        return "metnin ifadesi boş, uzun ya da başında veya sonunda boşluk var"
    if bad_color(s.get("color")):
        return "yazının rengi renk değil"
    h = s.get("halo")
    if h is not None and (out(h["width"], 0, 10) or bad_color(h.get("color"))):
        return "hale geçersiz"
    b = s.get("background")
    if b is not None and (bad_color(b.get("fill")) or bad_color(b.get("stroke")) or ("padding" in b and out(b["padding"], 0, 50))):
        return "zemin geçersiz"
    h = s.get("shadow")
    if h is not None and (out(h["dx"], -50, 50) or out(h["dy"], -50, 50) or bad_color(h.get("color")) or ("opacity" in h and out(h["opacity"], 0, 1))):
        return "gölge geçersiz"
    c = s.get("callout")
    if c is not None and (bad_color(c.get("color")) or ("width" in c and out(c["width"], 0.1, 10)) or ("minLength" in c and out(c["minLength"], 0, 1000))):
        return "çağrı çizgisi geçersiz"
    k = s.get("stack")
    if k is not None:
        if out(k["chars"], 2, 500):
            return "yığmanın satırı 2–500 harf olmalı"
        if "at" in k and (not k["at"] or len(k["at"]) > 20):
            return "yığmanın bölme karakterleri 1–20 harf olmalı"
    a = s.get("abbreviate")
    if a is not None:
        words = a["words"]
        if not words or len(words) > LABEL_WORDS_MAX:
            return f"kısaltma sözlüğü 1–{LABEL_WORDS_MAX} sözcük olmalı"
        seen = set()
        for w in words:
            for t in (w["word"], w["short"]):
                if not t or len(t) > 100 or any(ch.isspace() for ch in t):
                    return "kısaltma sözlüğünün sözcüğü ve kısası 1–100 harf, boşluksuz olmalı"
            if w["word"] in seen:
                return f"kısaltma sözlüğünde “{w['word']}” iki kez var"
            seen.add(w["word"])
    return None


def layer_labels_problem(l):
    """A layer's labelling (kentos_contracts::labels::layer_labels_problem): 1–64 named classes in rules, none
    otherwise; each class's name trimmed, 1–100 characters, once; its condition and style by their rules; an
    obstacle's weight 1–10."""
    classes = l.get("classes", [])
    if l["mode"] == "rules":
        if not classes or len(classes) > LABEL_CLASSES_MAX:
            return f"kurallı etiketlemede 1–{LABEL_CLASSES_MAX} sınıf olmalı"
    elif classes:
        return "sınıflar yalnız kurallı etiketlemede olur"
    names = set()
    for c in classes:
        n = c["name"]
        if not n or n.strip() != n or len(n) > LABEL_NAME_MAX:
            return "sınıfın adı geçersiz"
        if n in names:
            return f"“{n}” adlı sınıf iki kez var"
        names.add(n)
        w = c.get("when")
        if w is not None and (w.strip() != w or not w or len(w) > LABEL_EXPRESSION_MAX):
            return f"“{n}” sınıfının koşulu geçersiz"
        p = label_style_problem(c["style"])
        if p:
            return f"“{n}” sınıfı: {p}"
    o = l.get("obstacle")
    if o is not None and not 1 <= o["weight"] <= 10:
        return "engelin ağırlığı 1–10 olmalı"
    return None


def pins_problem(pins):
    """An object's pins (kentos_contracts::labels::pins_problem): at most 64, one a class, a place or hidden (true
    only), a turn only with a place, the numbers finite and in their ranges."""
    if len(pins) > LABEL_PINS_MAX:
        return f"en çok {LABEL_PINS_MAX} etiket iğnesi"
    classes = []
    for p in pins:
        c = p.get("class")
        if c is not None and (c.strip() != c or not c or len(c) > LABEL_NAME_MAX):
            return "etiket iğnesinin sınıfı geçerli bir ad değil"
        if c in classes:
            return "aynı sınıfın iki etiket iğnesi var"
        classes.append(c)
        if "at" not in p and "rotation" in p:
            return "etiket iğnesinin açısı yerle birlikte verilir"
        if "at" not in p and p.get("hidden") is not True:
            return "etiket iğnesi ya bir yer ya da gizli olur"
        if p.get("hidden") is False:
            return "etiket iğnesinin hidden'ı yalnız true yazılır"
        if "at" in p and any(not (abs(p["at"][k]) <= 1e7) for k in ("x", "y")):
            return "etiket iğnesinin yeri sonlu ve 10 000 km'den yakın olmalı"
        if "rotation" in p and not abs(p["rotation"]) <= 360:
            return "etiket iğnesinin açısı −360–360 derece olmalı"
    return None


def labels_problem(layers):
    """The tree's labelling (kentos_contracts::labels::labels_problem): no group labels."""
    for n in layers:
        if n["type"] == "group" and "labels" in n["style"]:
            return f"“{n['name']}” bir grup; grubun etiketlemesi olmaz, etiketleme katmanındır"
        p = labels_problem(n["children"])
        if p:
            return p
    return None


def _control(text, allowed=""):
    """A control character (Unicode Cc) other than those allowed."""
    return any(unicodedata.category(c) == "Cc" and c not in allowed for c in text)


def attachments_problem(xs):
    """An object's attachments (docs/adr/0225 §1): at most 64; each a name (not blank, at most 256 characters, no control
    character), exactly one source (an asset id, a file's path or an http or https address without white space; at most
    4 096 characters, no control character) and a note (at most 1 000 characters, line breaks only)."""
    if len(xs) > MAX_ATTACHMENTS:
        return f"nesnenin {len(xs)} eki var; en çok {MAX_ATTACHMENTS}"
    for a in xs:
        name = a["name"]
        if not name.strip():
            return "ekin adı boş"
        if len(name) > MAX_ATTACHMENT_NAME or _control(name):
            return f"“{name.strip()}” ekinin adı uzun ya da denetim karakteri içeriyor"
        sources = [k for k in ("asset", "file", "url") if k in a]
        if len(sources) != 1:
            return f"“{name}” ekinin kaynağı tek olmalı"
        if "asset" in a and (not a["asset"].strip() or len(a["asset"]) > MAX_ATTACHMENT_NAME or _control(a["asset"])):
            return f"“{name}” ekinin varlık kimliği geçersiz"
        for k in ("file", "url"):
            if k in a and (not a[k].strip() or len(a[k]) > MAX_ATTACHMENT_LINK or _control(a[k])):
                return f"“{name}” ekinin yolu ya da adresi geçersiz"
        if "url" in a:
            u, lower = a["url"], a["url"].lower()
            web = (lower.startswith("http://") and len(u.encode()) > 7) or (lower.startswith("https://") and len(u.encode()) > 8)
            if not web or any(c.isspace() for c in u):
                return f"“{name}” ekinin adresi http:// ya da https:// ile başlamalı ve boşluk içermemeli"
        if "note" in a and (len(a["note"]) > MAX_ATTACHMENT_NOTE or _control(a["note"], "\n")):
            return f"“{name}” ekinin notu uzun ya da denetim karakteri içeriyor"
    return None


def actions_problem(xs):
    """A layer's actions (docs/adr/0225 §4): at most 32; each a trimmed name (at most 100 characters, no control
    character, once whatever its case) and a target (not blank, at most 4 096 characters, no control character)."""
    if len(xs) > MAX_ACTIONS:
        return f"katmanın {len(xs)} eylemi var; en çok {MAX_ACTIONS}"
    seen = set()
    for a in xs:
        name = a["name"]
        if not name.strip() or name.strip() != name:
            return "eylemin adı boş ya da başında veya sonunda boşluk var"
        if len(name) > MAX_ACTION_NAME or _control(name):
            return f"“{name}” eyleminin adı uzun ya da denetim karakteri içeriyor"
        if name.lower() in seen:
            return f"“{name}” adlı iki eylem var"
        seen.add(name.lower())
        t = a["target"]
        if not t.strip():
            return f"“{name}” eyleminin hedefi boş"
        if len(t) > MAX_ACTION_TARGET or _control(t):
            return f"“{name}” eyleminin hedefi uzun ya da denetim karakteri içeriyor"
    return None


def card_problem(c):
    """A layer's information card (docs/adr/0225 §5): a title not blank, at most 1 000 characters, one line; a text not
    blank, at most 10 000 characters, line breaks only; at most 200 fields, none blank, each once."""
    if "title" in c and (not c["title"].strip() or len(c["title"]) > MAX_CARD_TITLE or _control(c["title"])):
        return "kartın başlığı boş, uzun ya da satır sonu içeriyor"
    if "text" in c and (not c["text"].strip() or len(c["text"]) > MAX_CARD_TEXT or _control(c["text"], "\n")):
        return "kartın metni boş, uzun ya da denetim karakteri içeriyor"
    fs = c.get("fields", [])
    if len(fs) > MAX_CARD_FIELDS:
        return f"kart {len(fs)} alan gösteriyor; en çok {MAX_CARD_FIELDS}"
    if any(not f.strip() for f in fs):
        return "kartın alanlarında boş ad var"
    if len(set(fs)) != len(fs):
        return "kart bir alanı iki kez gösteriyor"
    return None


def infos_problem(layers):
    """The tree's actions and cards (kentos_contracts::layer_info::infos_problem, docs/adr/0225 §4, §5): only on a layer
    that holds objects (not a group, not a layer drawn from a service), each by its rule."""
    for n in layers:
        if ("actions" in n or "card" in n) and (n["type"] != "layer" or "service" in n):
            return f"“{n['name']}”: eylemler ve bilgi kartı yalnız nesneleri olan katmanda olur"
        p = actions_problem(n.get("actions", [])) or (card_problem(n["card"]) if "card" in n else None)
        if p:
            return f"“{n['name']}”: {p}"
        p = infos_problem(n["children"])
        if p:
            return p
    return None


def compose_problem(c):
    """A layer's composition alone (docs/adr/0226 §3, §4): an opacity in 0–1; a mask with a source, a growth in 0–10 mm
    and 1–64 targets, none blank, each once; something said."""
    if "opacity" in c and not (math.isfinite(c["opacity"]) and 0.0 <= c["opacity"] <= 1.0):
        return "saydamlık 0 ile 1 arasında olmalı"
    m = c.get("mask")
    if m is not None:
        if not m.get("labels") and not m.get("markers"):
            return "maskenin kaynağı yok: etiketler ya da işaretler seçilmeli"
        if not (math.isfinite(m["size"]) and 0.0 <= m["size"] <= MAX_MASK_SIZE):
            return "maskenin boyu 0 ile 10 mm arasında olmalı"
        ts = m["targets"]
        if not ts:
            return "maskelenecek katman yok"
        if len(ts) > MAX_MASK_TARGETS:
            return f"maske en çok {MAX_MASK_TARGETS} katmanı maskeler"
        seen = set()
        for t in ts:
            if not t.strip():
                return "maskelenen katmanlar arasında boş kimlik var"
            if t in seen:
                return f"“{t}” katmanı iki kez maskeleniyor"
            seen.add(t)
    if not ({"opacity", "blend", "mask"} & set(c)):
        return "katman çizimi boş"
    return None


def composes_problem(layers):
    """The tree's compositions (kentos_contracts::layer_compose::composes_problem, docs/adr/0226 §3, §4): only on a
    layer, each by its rule. A mask's target the tree does not have as another layer is skipped when drawn, not refused."""
    for n in layers:
        c = n.get("compose")
        if c is not None:
            if n["type"] != "layer":
                return f"“{n['name']}” bir grup; katman çizimi yalnız katmanın olur"
            p = compose_problem(c)
            if p:
                return f"“{n['name']}”: {p}"
        p = composes_problem(n["children"])
        if p:
            return p
    return None


def filters_problem(layers):
    """The tree's filters (kentos_contracts::layer_filter::filters_problem, docs/adr/0211 §2): a filter only on a layer
    that holds objects (not a group, not a layer drawn from a service), and every filter by its rule."""
    for n in layers:
        if "filter" in n:
            if n["type"] == "group":
                return f"“{n['name']}” bir grup; grubun süzgeci olmaz, süzgeç katmanındır"
            if "service" in n:
                return f"“{n['name']}” servisten çizilir; nesnesi olmayan katmanın süzgeci olmaz"
            p = layer_filter_problem(n["filter"])
            if p:
                return f"“{n['name']}” katmanının süzgeci: {p}"
        p = filters_problem(n["children"])
        if p:
            return p
    return None


def scenarios_problem(layers):
    """The tree's time settings and scenarios (kentos_contracts::temporal::scenarios_problem, docs/adr/0210 §2): a time
    only on a layer that holds objects; a scenario only a group, never inside another; `replaces` only on a layer inside
    a scenario, not empty, not itself, naming a base layer (a layer outside every scenario) when the node is in the tree,
    each base layer replaced at most once in a scenario. A `replaces` naming no node is left out."""
    where = {}

    def index(nodes, inside):
        for n in nodes:
            where[n["id"]] = (n, inside)
            index(n["children"], inside or "scenario" in n)

    index(layers, False)
    replaced = set()

    def walk(nodes, scenario):
        for n in nodes:
            if "time" in n:
                if n["type"] == "group":
                    return f"“{n['name']}” bir grup; grubun zamanı olmaz, zaman katmanındır"
                if "service" in n:
                    return f"“{n['name']}” servisten çizilir; nesnesi olmayan katmanın zamanı olmaz"
                p = layer_time_problem(n["time"])
                if p:
                    return f"“{n['name']}” katmanının zamanı: {p}"
            if "scenario" in n:
                if n["type"] == "layer":
                    return f"“{n['name']}” bir katman; yalnız grup senaryo olur"
                if scenario is not None:
                    return f"“{n['name']}” senaryosu “{scenario['name']}” senaryosunun içinde; senaryo iç içe olmaz"
                p = scenario_problem(n["scenario"])
                if p:
                    return f"“{n['name']}” senaryosu: {p}"
            if "replaces" in n:
                target = n["replaces"]
                if scenario is None:
                    return f"“{n['name']}” bir senaryonun içinde değil; yalnız senaryo katmanı bir katmanın yerine geçer"
                if n["type"] == "group":
                    return f"“{n['name']}” bir grup; yalnız katman bir katmanın yerine geçer"
                if not target:
                    return f"“{n['name']}” katmanının yerine geçtiği katman boş"
                if target == n["id"]:
                    return f"“{n['name']}” katmanı kendi yerine geçemez"
                if target in where:
                    base, inside = where[target]
                    if base["type"] != "layer" or inside or "scenario" in base:
                        return (f"“{n['name']}” katmanı “{base['name']}” düğümünün yerine geçiyor; yalnız bir ana katmanın "
                                "(senaryo dışındaki katmanın) yerine geçilir")
                    if (scenario["id"], target) in replaced:
                        return f"“{scenario['name']}” senaryosunda “{base['name']}” katmanının yerine iki katman geçiyor"
                    replaced.add((scenario["id"], target))
            p = walk(n["children"], n if "scenario" in n else scenario)
            if p:
                return p
        return None

    return walk(layers, None)


def service_links_problem(layers, connections, entities):
    """The drawing's services' links (kentos_contracts::service::service_links): a connection named is the project's;
    an object is on no layer drawn from a service. Returns (place, words) or None."""
    known = {c["id"] for c in connections}
    served = set()

    def walk(nodes):
        for n in nodes:
            yield n
            yield from walk(n["children"])

    for n in walk(layers):
        for owner in ("service", "feed"):
            c = n.get(owner, {}).get("connection")
            if c is not None and c not in known:
                return ("layers", f"“{n['name']}” katmanının bağlantısı “{c}” projenin bağlantıları arasında yok")
        if "service" in n:
            served.add(n["id"])
    for i, e in enumerate(entities):
        if e["layerId"] in served:
            return (f"entities/{i}", "nesne bir servis katmanında; servis katmanı nesne tutmaz")
    return None


def contingents_problem(fields, subtypes, groups):
    """The contract's `contingents_problem` (docs/adr/0423 §1): at most 50 groups, each named once, 2–8 single-valued
    listed fields each once, 1–10000 rows of a cell per field, codes of the fields' lists, subtypes of the layer's, no
    row twice."""
    def key(f):
        return f.get("id") or f["name"]

    if len(groups) > 50:
        return "bir katmanın en çok 50 değer grubu olur"
    names = set()
    for g in groups:
        if not g["name"] or len(g["name"]) > 64 or g["name"].strip() != g["name"] or g["name"] in names:
            return f"“{g['name']}” bir grup adı olamaz ya da iki kez var"
        names.add(g["name"])
        if not 2 <= len(g["fields"]) <= 8 or len(set(g["fields"])) != len(g["fields"]):
            return f"“{g['name']}” grubunun alanları olamaz"
        listed = []
        for k in g["fields"]:
            f = next((x for x in fields if key(x) == k), None)
            if f is None or not f.get("values") or (f.get("multiple") and f["kind"] == "text"):
                return f"“{k}” alanı “{g['name']}” grubuna giremez"
            listed.append([c["code"] for c in f["values"]])
        if not 1 <= len(g["rows"]) <= 10000:
            return f"“{g['name']}” grubunun bileşimleri olamaz"
        seen = []
        codes = [t["code"] for t in (subtypes or {}).get("types", [])]
        for r in g["rows"]:
            if len(r["cells"]) != len(g["fields"]):
                return f"“{g['name']}” grubunda hücre sayısı alanlarınkinden farklı"
            if any(c["kind"] == "code" and c["code"] not in ok for c, ok in zip(r["cells"], listed)):
                return f"“{g['name']}” grubunda listede olmayan bir kod var"
            if "subtype" in r and r["subtype"] not in codes:
                return f"“{g['name']}” grubunda olmayan bir alt tür var"
            if r in seen:
                return f"“{g['name']}” grubunda bir bileşim iki kez var"
            seen.append(r)
    return None


def subtypes_problem(fields, st):
    """The contract's `subtypes_problem` (docs/adr/0422 §1): the subtype field a single-valued listed field, the
    subtypes its codes, each once, labels within 64 characters; each rule a field of the layer, not the subtype field,
    once; values of the field's list, each once, not none; a default not empty and offered by its subtype."""
    def key(f):
        return f.get("id") or f["name"]

    def many(f):
        return bool(f.get("multiple")) and f["kind"] == "text"

    sorter = next((f for f in fields if key(f) == st["field"]), None)
    if sorter is None or not sorter.get("values") or many(sorter):
        return f"“{st['field']}” alt tür alanı olamaz: katmanın tek değerli, listeli bir alanı olmalı"
    if not st["types"] or len(st["types"]) > 256:
        return "bir katmanın 1–256 alt türü olur"
    codes = [c["code"] for c in sorter["values"]]
    seen = set()
    for t in st["types"]:
        if t["code"] not in codes or t["code"] in seen:
            return f"“{t['code']}” alt türü listede yok ya da iki kez var"
        seen.add(t["code"])
        lab = t.get("label", "")
        if len(lab) > 64 or lab.strip() != lab:
            return f"“{lab}” bir alt tür adı olamaz"
        named = set()
        for r in t.get("fields", []):
            f = next((x for x in fields if key(x) == r["field"]), None)
            if f is None or key(f) == key(sorter) or r["field"] in named:
                return f"“{r['field']}” alanının “{t['code']}” alt türündeki kuralı olamaz"
            named.add(r["field"])
            if "values" in r:
                listed = [c["code"] for c in f.get("values") or []]
                if not f.get("values") or not r["values"] or len(set(r["values"])) != len(r["values"]) or any(
                        c not in listed for c in r["values"]):
                    return f"“{r['field']}” alanının “{t['code']}” alt türündeki değerleri olamaz"
            if "default" in r:
                d = r["default"]
                parts = [p.strip() for p in d.split(";") if p.strip()] if many(f) else [d]
                if not d or (f.get("values") and "values" in r and any(p not in r["values"] for p in parts)):
                    return f"“{r['field']}” alanının “{t['code']}” alt türündeki varsayılanı olamaz"
    return None


def _is_date(s):
    """A real calendar day written `YYYY-AA-GG`."""
    import datetime
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", s):
        return False
    try:
        datetime.date(int(s[:4]), int(s[5:7]), int(s[8:]))
    except ValueError:
        return False
    return True


def layer_rules_problem(fields, rules):
    """The contract's `layer_rules_problem` (docs/adr/0438 §1): at most 64 rules, each its id once, a name, an
    expression and a message within their lengths, its fields the layer's, each once (the expressions are the forms
    core's to compile)."""
    keys = [f.get("id", f["name"]) for f in fields]
    if len(rules) > 64:
        return "bir katmanın en çok 64 kayıt kuralı olur"
    seen = set()
    for r in rules:
        for k, most in (("id", 64), ("name", 64)):
            t = r[k]
            if not t.strip(JS_SPACE) or t.strip(JS_SPACE) != t or len(t) > most or any(unicodedata.category(c) == "Cc" for c in t):
                return f"kuralın {k} geçersiz"
        if r["id"] in seen:
            return f"“{r['id']}” kuralı iki kez var"
        seen.add(r["id"])
        if not r["rule"].strip(JS_SPACE) or len(r["rule"]) > 2000:
            return f"“{r['name']}” kuralının ifadesi geçersiz"
        if not r["message"].strip(JS_SPACE) or len(r["message"]) > 500:
            return f"“{r['name']}” kuralının iletisi geçersiz"
        named = r.get("fields", [])
        if any(k not in keys for k in named) or len(set(named)) != len(named):
            return f"“{r['name']}” kuralının alanları geçersiz"
    return None


def fields_problem(fields):
    """The contract's `layer_fields_problem` (kentos_contracts::fields): a list that is not empty, each field's own
    rules, no name twice (folded)."""
    if not fields:
        return "katmanın alan listesi boş olamaz"
    seen = set()
    keys = set()
    for f in fields:
        name, kind = f["name"], f["kind"]
        if not name.strip(JS_SPACE) or name.strip(JS_SPACE) != name or len(name) > 64:
            return f"“{name}” alanının adı geçersiz"
        if any(ord(c) < 32 or 127 <= ord(c) < 160 for c in name):
            return f"“{name}” alanının adında denetim karakteri var"
        if "alias" in f and (not f["alias"].strip(JS_SPACE) or len(f["alias"]) > 64):
            return f"“{name}” alanının takma adı geçersiz"
        if "length" in f and (kind != "text" or not 1 <= f["length"] <= 10000):
            return f"“{name}” alanının uzunluğu geçersiz"
        if "scale" in f and (kind != "decimal" or f["scale"] > 15):
            return f"“{name}” alanının ondalık basamağı geçersiz"
        ends = [f[k] for k in ("min", "max") if k in f]
        if ends:
            # docs/adr/0436: a date, a date and time or a time of day too, its text sorting as written.
            temporal = {"date": r"\d{4}-\d{2}-\d{2}", "datetime": r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}",
                        "time": r"\d{2}:\d{2}:\d{2}"}
            if kind == "instant":
                moments = [_instant_moment(v) for v in ends]
                if None in moments:
                    return f"“{name}” alanının aralığı geçersiz"
                if len(ends) == 2 and moments[0] > moments[1]:
                    return f"“{name}” alanının en azı en çoğundan büyük"
            elif kind in temporal:
                if any(not re.fullmatch(temporal[kind], v) for v in ends):
                    return f"“{name}” alanının aralığı geçersiz"
                if len(ends) == 2 and ends[0] > ends[1]:
                    return f"“{name}” alanının en azı en çoğundan büyük"
            elif kind not in ("integer", "decimal", "bigint") or any(_canonical(kind, f.get("scale"), v) != v for v in ends):
                return f"“{name}” alanının aralığı geçersiz"
            elif len(ends) == 2 and _number(ends[0]) > _number(ends[1]):
                return f"“{name}” alanının en azı en çoğundan büyük"
        if "values" in f:
            vs = f["values"]
            if kind not in ("text", "integer", "decimal", "bigint") or not vs:
                return f"“{name}” alanının değer listesi geçersiz"
            codes = [c["code"] for c in vs]
            labels = [_fold(c["label"].strip(JS_SPACE)) for c in vs]
            if any(c == "" or (kind != "text" and _canonical(kind, f.get("scale"), c) != c) for c in codes):
                return f"“{name}” alanının değer listesinde geçersiz kod var"
            if any(not lb for lb in labels) or len(set(codes)) != len(codes) or len(set(labels)) != len(labels):
                return f"“{name}” alanının değer listesinde boş ya da iki kez yazılmış değer var"
            if f.get("multiple") and any(";" in c["code"] or ";" in c["label"] for c in vs):
                return f"“{name}” çok değerli; değer listesinde “;” olamaz"
            if all(c.get("retired") for c in vs):
                return f"“{name}” alanının bütün değerleri kullanımdan kalkmış"
            # Schema 60 (docs/adr/0438 §5): a value's description and days.
            for c in vs:
                if "description" in c and (not c["description"].strip(JS_SPACE) or len(c["description"]) > 500):
                    return f"“{name}” alanının “{c['code']}” değerinin açıklaması geçersiz"
                for k in ("from", "until"):
                    if k in c and not _is_date(c[k]):
                        return f"“{name}” alanının “{c['code']}” değerinin geçerlilik tarihi geçersiz"
                if "from" in c and "until" in c and c["from"] > c["until"]:
                    return f"“{name}” alanının “{c['code']}” değerinin geçerliliği bitişinden sonra başlıyor"
        # Schema 60 (docs/adr/0438 §3, §4): the range's left-out ends, the unit, unique.
        if f.get("minExclusive") and "min" not in f:
            return f"“{name}” alanının alt ucu en azı olmadan hariç tutulamaz"
        if f.get("maxExclusive") and "max" not in f:
            return f"“{name}” alanının üst ucu en çoğu olmadan hariç tutulamaz"
        if (f.get("minExclusive") or f.get("maxExclusive")) and "min" in f and "max" in f and (
                f["min"] == f["max"] if kind in ("date", "datetime", "time")
                else _instant_moment(f["min"]) == _instant_moment(f["max"]) if kind == "instant"
                else _number(f["min"]) == _number(f["max"])):
            return f"“{name}” alanının aralığı boş"
        if "unit" in f:
            u = f["unit"]
            if kind not in ("integer", "decimal", "bigint") or not u.strip(JS_SPACE) or u.strip(JS_SPACE) != u or len(u) > 16 \
                    or any(unicodedata.category(c) == "Cc" for c in u):
                return f"“{name}” alanının birimi geçersiz"
        if f.get("unique") and (kind == "boolean" or f.get("multiple")):
            return f"“{name}” alanı tekil olamaz"
        units = kind in ("integer", "bigint") or (kind == "decimal" and "scale" in f)
        if f.get("split") == "ratio" and not units:
            return f"“{name}” alanı oranla paylaştırılamaz"
        if f.get("merge") == "sum" and not units:
            return f"“{name}” alanı toplanamaz"
        if f.get("merge") in ("min", "max") and kind not in ("integer", "decimal"):
            return f"“{name}” alanında en küçüğü ve en büyüğü yalnız sayı alanlarında olur"
        if f.get("multiple") and kind != "text":
            return f"“{name}” alanı çok değerli olamaz"
        if "default" in f and not _default_fits(f, f["default"]):
            return f"“{name}” alanının varsayılanı alanın kurallarına uymuyor"
        retired = [c["code"] for c in f.get("values", []) if c.get("retired")]
        parts = f["default"].split("; ") if f.get("multiple") and "default" in f else [f.get("default")]
        if "default" in f and any(p in retired for p in parts):
            return f"“{name}” alanının varsayılanı kullanımdan kalkmış bir değer"
        key = _fold(name)
        if key in seen:
            return f"“{name}” adlı iki alan var"
        seen.add(key)
        # Schema 60 (docs/adr/0401 §2): the stable id, and no field's key (its id, else its name) twice.
        if "id" in f:
            fid = f["id"]
            if not fid.strip(JS_SPACE) or fid.strip(JS_SPACE) != fid or len(fid) > 64 or any(ord(c) < 32 or 127 <= ord(c) < 160 for c in fid):
                return f"“{name}” alanının kimliği geçersiz"
        # Schema 60 (docs/adr/0430): the value list it links, by its id.
        if "domain" in f and (not f["domain"] or len(f["domain"]) > 64):
            return f"“{name}” alanının değer listesi kimliği geçersiz"
        stable = f.get("id", name)
        if stable in keys:
            return f"“{stable}” kimliği iki alanda var"
        keys.add(stable)
    return None


MOST_DOMAINS = 200


def domains_problem(domains):
    """The contract's `domains_problem` (docs/adr/0430): at most 200 lists, ids and names that hold (names unique,
    folded to lower case), a description within 500 characters, a coded list or a range, never both, not every code
    retired, the values and bounds a field of the list's kind could hold."""
    if len(domains) > MOST_DOMAINS:
        return f"projede en çok {MOST_DOMAINS} değer listesi olur"
    ids, names = set(), set()
    for d in domains:
        for k in ("id", "name"):
            t = d[k]
            if not t.strip() or t.strip() != t or len(t) > 64 or any(unicodedata.category(c) == "Cc" for c in t):
                return f"değer listesinin {k} geçersiz"
        if "description" in d and (not d["description"].strip() or len(d["description"]) > 500):
            return f"“{d['name']}” listesinin açıklaması geçersiz"
        ranged = "min" in d or "max" in d
        if ("values" in d) == ranged:
            return f"“{d['name']}” listesi ya kod listesi ya aralık olmalı"
        if "values" in d and all(c.get("retired") for c in d["values"]):
            return f"“{d['name']}” listesinin bütün kodları kullanımdan kalkmış"
        probe = {"name": d["name"], "kind": d["kind"],
                 **{k: d[k] for k in ("values", "min", "max", "minExclusive", "maxExclusive", "unit") if k in d}}
        problem = fields_problem([probe])
        if problem:
            return problem
        if d["id"] in ids:
            return f"“{d['id']}” kimlikli iki değer listesi var"
        if d["name"].lower() in names:
            return f"“{d['name']}” adlı iki değer listesi var"
        ids.add(d["id"])
        names.add(d["name"].lower())
    return None


# The forms' bounds (kentos_contracts::forms).
MOST_FORMS = 100
MOST_FORM_ITEMS = 500
MOST_FORM_RELEASES = 50
MOST_FORM_NAME = 64
MOST_FORM_TEXT = 1000
MOST_FORM_RULE = 2000
MOST_FORM_CHECKS = 20
MOST_FORM_DEPTH = 4


def canonical_json(v):
    """The canonical JSON of docs/adr/0253 §2 for the values a form holds (texts, whole numbers, true, lists, maps): no
    white space, keys by their UTF-8 bytes (code points), only the escapes JSON requires."""
    if v is True:
        return "true"
    if v is False:
        return "false"
    if v is None:
        return "null"
    if type(v) is int:
        return str(v)
    if type(v) is str:
        out = ['"']
        for c in v:
            o = ord(c)
            if c == '"':
                out.append('\\"')
            elif c == "\\":
                out.append("\\\\")
            elif c in "\b\f\n\r\t":
                out.append({"\b": "\\b", "\f": "\\f", "\n": "\\n", "\r": "\\r", "\t": "\\t"}[c])
            elif o < 0x20:
                out.append("\\u%04x" % o)
            else:
                out.append(c)
        out.append('"')
        return "".join(out)
    if type(v) is list:
        return "[" + ",".join(canonical_json(x) for x in v) + "]"
    if type(v) is dict:
        return "{" + ",".join(canonical_json(k) + ":" + canonical_json(v[k]) for k in sorted(v, key=lambda k: k.encode("utf-8"))) + "}"
    raise ValueError(f"formda beklenmeyen değer: {v!r}")


def _form_id_ok(i):
    return 1 <= len(i) <= MOST_FORM_NAME and all(c.isalnum() or c in "-_." for c in i)


def _form_name_ok(n):
    t = n.strip()
    return bool(t) and t == n and len(n) <= MOST_FORM_NAME and not any(unicodedata.category(c) == "Cc" for c in n)


def _form_text_ok(t, most):
    return len(t) <= most and not any(unicodedata.category(c) == "Cc" and c != "\n" for c in t)


def _rule_ok(r):
    return bool(r.strip()) and _form_text_ok(r, MOST_FORM_RULE)


def body_problem(body):
    """The contract's `body_problem`: unique ids that hold, tabs only at the top and alone there, four levels, 500 items,
    names, texts and rules within their lengths, one or two columns, checks with a rule and a message."""
    walked = []

    def walk(items, depth):
        for i in items:
            walked.append((depth, i))
            walk(i.get("items", []), depth + 1)

    walk(body["items"], 0)
    if len(walked) > MOST_FORM_ITEMS:
        return "formda çok öğe var"
    tops = [i for i in body["items"] if i["type"] == "tab"]
    if tops and len(tops) != len(body["items"]):
        return "formun üstünde sekmelerle başka öğeler var"
    # docs/adr/0437: a form of steps is one of two or more tabs.
    if body.get("steps") and len(tops) < 2:
        return "adım adım form en az iki sekmeden olur"
    seen = set()
    for depth, i in walked:
        iid = i["id"]
        if not _form_id_ok(iid):
            return f"“{iid}” bir öğe kimliği olamaz"
        if iid in seen:
            return f"“{iid}” kimliği iki kez kullanılmış"
        seen.add(iid)
        if depth >= MOST_FORM_DEPTH:
            return f"“{iid}” çok derinde"
        t = i["type"]
        if t == "tab" and (depth > 0 or not _form_name_ok(i["label"])):
            return f"“{iid}” sekmesi yalnız üstte ve adlı olur"
        if t == "tab" and "visibleWhen" in i and not _rule_ok(i["visibleWhen"]):
            return f"“{iid}” sekmesinin kuralı geçersiz"
        if t == "group":
            if "label" in i and not _form_name_ok(i["label"]):
                return f"“{iid}” grubunun adı geçersiz"
            if "columns" in i and i["columns"] not in (1, 2):
                return f"“{iid}” grubu bir ya da iki sütun olur"
            if "visibleWhen" in i and not _rule_ok(i["visibleWhen"]):
                return f"“{iid}” grubunun kuralı geçersiz"
        if t == "note" and (not i["text"].strip() or not _form_text_ok(i["text"], MOST_FORM_TEXT)):
            return f"“{iid}” notu geçersiz"
        if t == "related":
            # docs/adr/0404 §4: the objects of another layer whose field holds this record's id.
            if "label" in i and not _form_name_ok(i["label"]):
                return f"“{iid}” öğesinin adı geçersiz"
            if any(not i[k].strip() or len(i[k]) > MOST_FORM_NAME for k in ("layer", "field")):
                return f"“{iid}” öğesinin katmanı ve alanı yazılmalı"
            if "visibleWhen" in i and not _rule_ok(i["visibleWhen"]):
                return f"“{iid}” öğesinin kuralı geçersiz"
            cols = i.get("columns", [])
            if "columns" in i and not cols:
                return f"“{iid}” öğesinin boş sütun listesi yazılmaz"
            if len(cols) > 8 or len(set(cols)) != len(cols) or any(not c.strip() for c in cols):
                return f"“{iid}” öğesinin sütunları geçersiz"
        if t == "attachments":
            if "label" in i and not _form_name_ok(i["label"]):
                return f"“{iid}” öğesinin adı geçersiz"
            lo, hi = i.get("min"), i.get("max")
            if any(n is not None and n > 100 for n in (lo, hi)) or (lo is not None and hi is not None and lo > hi):
                return f"“{iid}” öğesinin sayıları geçersiz"
            if "visibleWhen" in i and not _rule_ok(i["visibleWhen"]):
                return f"“{iid}” öğesinin kuralı geçersiz"
        if t in ("location", "divider") and "label" in i and not _form_name_ok(i["label"]):
            return f"“{iid}” öğesinin adı geçersiz"
        if t in ("location", "divider", "action") and "visibleWhen" in i and not _rule_ok(i["visibleWhen"]):
            return f"“{iid}” öğesinin kuralı geçersiz"
        if t == "action":
            # docs/adr/0441 §6, 0442 §5: a tool by its id, a step by its label, the others no target.
            kind = i.get("kind", "tool")
            if not _form_name_ok(i["label"]):
                return f"“{iid}” eylemi geçersiz"
            if kind == "tool":
                tool = i.get("tool")
                parts = tool.split(".") if tool is not None else []
                if tool is None or "step" in i or len(tool) > MOST_FORM_NAME or len(parts) < 2 or any(
                        not p or not p[0].isascii() or not p[0].isalpha()
                        or not all(c.isascii() and c.isalnum() for c in p) for p in parts):
                    return f"“{iid}” eylemi geçersiz"
            elif kind == "step":
                if "tool" in i or "step" not in i or not _form_name_ok(i["step"]) or "runs" in i:
                    return f"“{iid}” eylemi geçersiz"
            elif "tool" in i or "step" in i or "runs" in i:
                return f"“{iid}” eylemi geçersiz"
        if t == "field":
            if not i["field"].strip() or len(i["field"]) > MOST_FORM_NAME:
                return f"“{iid}” öğesinin alanı yazılmamış"
            if "label" in i and not _form_name_ok(i["label"]):
                return f"“{iid}” alanının etiketi geçersiz"
            if any(k in i and not _form_text_ok(i[k], MOST_FORM_TEXT) for k in ("help", "placeholder", "default")):
                return f"“{iid}” alanının bir metni çok uzun"
            if any(k in i and not _rule_ok(i[k]) for k in ("visibleWhen", "requiredWhen", "readOnlyWhen", "defaultExpr",
                                                         "calculate", "choiceFilter")):
                return f"“{iid}” öğesinin kuralı geçersiz"
            lk = i.get("lookup")
            if lk is not None:
                # docs/adr/0404 §1: a field's list from another layer; docs/adr/0439: its order,
                # columns, page, copies, stamp and tolerance.
                names = [lk["layer"], lk["key"]] + ([lk["label"]] if "label" in lk else []) + \
                    [n for m in lk.get("matches", []) for n in (m["field"], m["record"])] + \
                    ([lk["sort"]["field"]] if "sort" in lk else []) + lk.get("columns", []) + \
                    [n for c in lk.get("copy", []) for n in (c["from"], c["to"])] + \
                    ([lk["stamp"]] if "stamp" in lk else [])
                if any(not n.strip() or len(n) > MOST_FORM_NAME for n in names) \
                        or len(lk.get("matches", [])) > MOST_FORM_CHECKS \
                        or len(lk.get("copy", [])) > MOST_FORM_CHECKS \
                        or ("filter" in lk and not _rule_ok(lk["filter"])):
                    return f"“{iid}” alanının listesi geçersiz"
                tol = lk.get("spatial", {}).get("tolerance")
                if len(lk.get("columns", [])) > 4 or not 0 < lk.get("limit", 1) <= 1000 \
                        or (tol is not None and (tol != tol or tol in (float("inf"), float("-inf")) or tol < 0)):
                    return f"“{iid}” alanının listesinin sınırları geçersiz"
            if "minSearch" in i and not 0 < i["minSearch"] <= 10:
                return f"“{iid}” alanının aramaya başlama harf sayısı geçersiz"
            checks = i.get("checks", [])
            if len(checks) > MOST_FORM_CHECKS or any(not _rule_ok(c["rule"]) or not c["message"].strip() or not _form_text_ok(c["message"], MOST_FORM_TEXT) for c in checks):
                return f"“{iid}” alanının denetimi geçersiz"
    # docs/adr/0420: at most 50 scenarios, each named once, 1–64 characters, no white space at its ends.
    tests = body.get("tests", [])
    if len(tests) > 50:
        return f"Bir formda en çok 50 senaryo olur; {len(tests)} var."
    names = set()
    for t in tests:
        n = t["name"]
        if not n.strip() or n.strip() != n or len(n) > 64:
            return f"“{n}” bir senaryo adı olamaz"
        if n in names:
            return f"“{n}” senaryosu formda iki kez var."
        names.add(n)
    return None


def relations_problem(relations):
    """The contract's `relations_problem` (docs/adr/0405 §1): at most 100, ids and names that hold, ids unique, the
    layers and the field written."""
    if len(relations) > 100:
        return "projede çok ilişki var"
    ids = set()
    for r in relations:
        if not _form_id_ok(r["id"]) or r["id"] in ids:
            return f"“{r['id']}” bir ilişki kimliği olamaz ya da iki kez kullanılmış"
        ids.add(r["id"])
        if not _form_name_ok(r["name"]):
            return f"“{r['id']}” ilişkisinin adı geçersiz"
        if any(not r[k].strip() or len(r[k]) > MOST_FORM_NAME for k in ("parent", "child", "field")):
            return f"“{r['id']}” ilişkisinin katmanları ve alanı yazılmalı"
    return None


def join_views_problem(views):
    """The contract's `join_views_problem` (docs/adr/0527): at most 50, each view's id and name holding, its layers
    and keys written, at most 200 fields none empty or twice, ids unique."""
    if len(views) > 50:
        return "projede çok birleşik görünüm var"
    ids = set()
    for v in views:
        if not _form_id_ok(v["id"]):
            return f"“{v['id']}” bir görünüm kimliği olamaz"
        if not _form_name_ok(v["name"]):
            return f"“{v['id']}” görünümünün adı geçersiz"
        if any(not v[k] for k in ("layer", "key", "withKey")):
            return f"“{v['name']}” görünümünün katmanları ve anahtarları yazılmalı"
        # docs/adr/0632 §1: the joined records are a layer's (`with`) or a source's table's (`source`), never both.
        source = v.get("source")
        if (source is None) == (not v["with"]):
            return f"“{v['name']}” görünümü ya bir katmanla ya bir kaynak tablosuyla birleşir"
        if source is not None:
            if not source["path"].strip() or not source["table"].strip():
                return f"“{v['name']}” görünümünün kaynak tablosunun yolu ya da adı yazılmamış"
            low = source["path"].lower()
            for scheme in ("postgresql://", "postgres://"):
                if low.startswith(scheme) and ":" in low[len(scheme):].split("@", 1)[0] and "@" in low[len(scheme):]:
                    return f"“{v['name']}” görünümünün kaynak adresinde parola var"
        # docs/adr/0632 §2: at most 20 summing columns, names holding and unique, a field for all but a count.
        aggregates = v.get("aggregates", [])
        if len(aggregates) > 20:
            return f"“{v['name']}” görünümünde çok toplama sütunu var"
        names = set()
        for a in aggregates:
            if not _form_name_ok(a["name"]) or a["name"] in names:
                return f"“{v['name']}” görünümünün toplama sütunlarının adları geçersiz"
            names.add(a["name"])
            if "field" not in a and a["how"] != "count":
                return f"“{a['name']}” toplama sütunu bir alan ister"
            if "field" in a and not a["field"]:
                return f"“{a['name']}” toplama sütununun alanı boş"
        fields = v.get("fields", [])
        if len(fields) > 200 or any(not f for f in fields) or len(set(fields)) != len(fields):
            return f"“{v['name']}” görünümünün alanları geçersiz"
        if v["id"] in ids:
            return f"“{v['id']}” kimlikli iki birleşik görünüm var"
        ids.add(v["id"])
    return None


def event_links_problem(links):
    """The contract's `event_links_problem` (docs/adr/0569): at most 50 links, each link's id and name holding, an
    event layer, 1-16 distinct route layers none the event layer, the field names written, a point link's measure or
    a line link's from and to (never the other's), a tolerance of zero or more; ids unique, an event layer of one
    link only and never another link's route layer."""
    if len(links) > 50:
        return "projede çok olay bağı var"
    ids, layers = set(), set()
    for l in links:
        if not _form_id_ok(l["id"]):
            return f"“{l['id']}” bir bağ kimliği olamaz"
        if not _form_name_ok(l["name"]):
            return f"“{l['id']}” bağının adı geçersiz"
        routes = l["routes"]
        if not l["events"] or not 1 <= len(routes) <= 16 or any(not r for r in routes) or len(set(routes)) != len(routes) \
                or l["events"] in routes:
            return f"“{l['name']}” bağının katmanları geçersiz"
        names = [l["routeField"], l["eventRoute"]] + [l[k] for k in ("measure", "from", "to", "offset") if k in l]
        if any(not n for n in names):
            return f"“{l['name']}” bağının bir alanı boş"
        if l["kind"] == "point":
            if "measure" not in l or "from" in l or "to" in l:
                return f"“{l['name']}” nokta bağının ölçü alanı yanlış"
        elif "from" not in l or "to" not in l or "measure" in l:
            return f"“{l['name']}” çizgi bağının ölçü alanları yanlış"
        if "tolerance" in l and not l["tolerance"] >= 0:
            return f"“{l['name']}” bağının toleransı geçersiz"
        if l["id"] in ids:
            return f"“{l['id']}” kimlikli iki olay bağı var"
        if l["events"] in layers:
            return f"“{l['events']}” katmanının olayları iki bağda"
        ids.add(l["id"])
        layers.add(l["events"])
    for l in links:
        if any(l["events"] in m["routes"] for m in links):
            return f"“{l['events']}” katmanı hem olay hem rota katmanı"
    return None


def forms_problem(forms):
    """The contract's `forms_problem`: at most 100 forms with ids and names that hold, ids unique, a layer, bodies that
    hold, releases numbered on from the first with their body's digest (the first 32 hex digits of the SHA-256 of its
    canonical JSON), at most 50, `enforce` only with a release."""
    if len(forms) > MOST_FORMS:
        return "projede çok form var"
    ids = set()
    for f in forms:
        if not _form_id_ok(f["id"]) or f["id"] in ids:
            return f"“{f['id']}” bir form kimliği olamaz ya da iki kez kullanılmış"
        ids.add(f["id"])
        if not _form_name_ok(f["name"]):
            return f"“{f['id']}” formunun adı geçersiz"
        if not f["layer"].strip():
            return f"“{f['name']}” formunun katmanı yazılmamış"
        problem = body_problem(f["draft"])
        if problem:
            return problem
        releases = f.get("releases", [])
        if len(releases) > MOST_FORM_RELEASES:
            return f"“{f['name']}” formunun çok yayını var"
        for k, r in enumerate(releases):
            if r["version"] == 0 or r["version"] != releases[0]["version"] + k:
                return f"“{f['name']}” formunun yayınları sırayla numaralanmamış"
            problem = body_problem(r["body"])
            if problem:
                return problem
            if r["digest"] != hashlib.sha256(canonical_json(r["body"]).encode("utf-8")).hexdigest()[:32]:
                return f"“{f['name']}” formunun {r['version']}. yayınının özeti içeriğine uymuyor"
            if "note" in r and not _form_text_ok(r["note"], MOST_FORM_TEXT):
                return f"“{f['name']}” formunun yayın notu çok uzun"
        if f.get("enforce") and not releases:
            return f"“{f['name']}” formu yayınlanmadan kuralları uygulanamaz"
        # docs/adr/0441 §5: at most 200 comments, numbered once, with a text.
        comments = f.get("comments", [])
        if len(comments) > 200 or len({c["id"] for c in comments}) != len(comments) or any(
                c["id"] == 0 or not c["text"].strip() or not _form_text_ok(c["text"], MOST_FORM_TEXT) for c in comments):
            return f"“{f['name']}” formunun yorumları geçersiz"
    return None


def _default_fits(f, d):
    """Whether a default is a value of its field in its canonical text."""
    kind = f["kind"]
    s = d.strip(JS_SPACE)
    if not s:
        return False
    if kind == "text":
        ok = "length" not in f or len(d) <= f["length"]
        codes = [c["code"] for c in f.get("values", [])]
        if f.get("multiple"):
            # docs/adr/0408 §1, 0409 §1: listed codes in the list's order, free values in theirs, once each,
            # joined with "; ", each trimmed and not empty.
            parts = d.split("; ")
            ok = ok and all(p and p.strip(JS_SPACE) == p and ";" not in p for p in parts) and len(set(parts)) == len(parts)
            if not codes:
                return ok
            return ok and all(p in codes for p in parts) and [c for c in codes if c in parts] == parts
        return ok and ("values" not in f or d in codes)
    if kind == "date":
        m = re.fullmatch(r"([0-9]{4})-([0-9]{2})-([0-9]{2})", d)
        if not m:
            return False
        y, mo, da = (int(x) for x in m.groups())
        import calendar

        return 1 <= y <= 9999 and 1 <= mo <= 12 and 1 <= da <= calendar.monthrange(y, mo)[1]
    if kind == "boolean":
        return d in ("true", "false")
    if _canonical(kind, f.get("scale"), d) != d:
        return False
    if "values" in f and d not in [c["code"] for c in f["values"]]:
        return False
    if "min" in f and _number(d) < _number(f["min"]):
        return False
    return not ("max" in f and _number(d) > _number(f["max"]))


def choices_problem(choices):
    """What is wrong with a project's datum choices together (kentos_contracts::crs::choices_problem), or None."""
    pairs = []
    for t in choices:
        if t["from"] == t["to"]:
            return "datum seçimi iki ayrı datum arasında olur"
        if not t["name"].strip():
            return "datum seçiminin adı boş olamaz"
        if ("helmert" in t) == ("grid" in t):
            return "datum seçimi ya yedi parametre ya ızgaradır, yalnız biri"
        if "helmert" in t:
            problem = helmert_problem(t["helmert"])
            if problem:
                return problem
        else:
            g = t["grid"]
            if not re.fullmatch(r"[0-9a-f]{64}", g["id"]):
                return "ızgaranın kimliği küçük harfli 64 onaltılık rakamla SHA-256 olmalı"
            if g["size"] == 0:
                return "ızgaranın boyu 0 olamaz"
            if "accuracy" in g and not (math.isfinite(g["accuracy"]) and g["accuracy"] >= 0):
                return "doğruluk 0 ya da büyük bir sayı olmalı"
        pair = frozenset((t["from"], t["to"]))
        if pair in pairs:
            return "bir datum çifti için en çok bir seçim olur"
        pairs.append(pair)
    return None



def parameter_name_ok(name):
    """A parameter's name (docs/adr/0334 §3): a letter or `_` first, then letters, digits and `_`, at most 64
    characters, not a function's name nor `pi`."""
    return (
        bool(name)
        and (name[0].isalpha() or name[0] == "_")
        and all(c.isalnum() or c == "_" for c in name)
        and len(name) <= PARAMETER_NAME_MAX
        and name != "pi"
        and name not in PARAMETER_FUNCTIONS
    )


def expression_names(text):
    """The parameter names an expression reads (docs/adr/0334 §3), parsed by its grammar; ValueError when it is not
    one: numbers (with a unit right after), names, + - * / ^, parentheses, the functions with their argument counts."""
    if len(text) > EXPRESSION_MAX:
        raise ValueError("ifade çok uzun")
    at, names = 0, set()

    def space():
        nonlocal at
        while at < len(text) and text[at].isspace():
            at += 1

    def peek():
        space()
        return text[at] if at < len(text) else None

    def summ(depth):
        nonlocal at
        product(depth)
        while peek() in ("+", "-"):
            at += 1
            product(depth)

    def product(depth):
        nonlocal at
        power(depth)
        while peek() in ("*", "/"):
            at += 1
            power(depth)

    def power(depth):
        nonlocal at
        if depth > 64:
            raise ValueError("ifade çok derin")
        if peek() in ("+", "-"):
            at += 1
            return power(depth + 1)
        atom(depth + 1)
        if peek() == "^":
            at += 1
            power(depth + 1)

    def atom(depth):
        nonlocal at
        c = peek()
        if c is None:
            raise ValueError("ifade yarım kaldı")
        if c == "(":
            at += 1
            summ(depth + 1)
            if peek() != ")":
                raise ValueError("“)” bekleniyordu")
            at += 1
            return
        m = re.match(r"(\d+\.?\d*|\.\d+)", text[at:])
        if m:
            at += m.end()
            e = re.match(r"[eE][+-]?\d+", text[at:])
            if e:
                at += e.end()
            u = re.match(r"\w+", text[at:])
            if u:
                if u.group(0) not in LITERAL_UNITS:
                    raise ValueError(f"“{u.group(0)}” bilinen bir birim değil")
                at += u.end()
            return
        m = re.match(r"[^\W\d]\w*", text[at:])
        if not m:
            raise ValueError(f"“{c}” beklenmiyordu")
        word = m.group(0)
        at += m.end()
        if peek() == "(":
            if word not in PARAMETER_FUNCTIONS:
                raise ValueError(f"“{word}” bilinen bir işlev değil")
            at += 1
            count = 0
            if peek() != ")":
                while True:
                    summ(depth + 1)
                    count += 1
                    if peek() in (",", ";"):
                        at += 1
                        continue
                    break
            if peek() != ")":
                raise ValueError("“)” bekleniyordu")
            at += 1
            if count != PARAMETER_FUNCTIONS[word]:
                raise ValueError(f"“{word}” {PARAMETER_FUNCTIONS[word]} değer alır")
            return
        if word in PARAMETER_FUNCTIONS:
            raise ValueError(f"“{word}” bir işlevdir")
        if word != "pi":
            names.add(word)

    if peek() is None:
        raise ValueError("ifade boş")
    summ(0)
    if peek() is not None:
        raise ValueError(f"“{peek()}” beklenmiyordu")
    return names


class KcadError(Exception):
    """A file the reader refuses: `code` is from the specification's table (§9)."""

    def __init__(self, code, message):
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def sniff(data):
    """What a file is by content (spec §8): kcad, kcad-damaged, json, empty or foreign."""
    if data[: len(MAGIC)] == MAGIC:
        return "kcad"
    if data[: len(MAGIC_PREFIX)] == MAGIC_PREFIX:
        return "kcad-damaged"
    rest = data[3:] if data[:3] == b"\xef\xbb\xbf" else data
    rest = rest.lstrip(b" \t\r\n")
    if not rest:
        return "empty"
    if rest[:1] == b"{":
        return "json"
    return "foreign"


def _u16(data, at):
    return int.from_bytes(data[at : at + 2], "little")


def _u64(data, at):
    return int.from_bytes(data[at : at + 8], "little")


def read_header(data):
    """Checks the container in the specification's order (§4); returns the header and the payload."""
    size = len(data)
    if size == 0:
        raise KcadError("empty", "Dosya boş; içinde çizim yok.")
    if data[: len(MAGIC)] != MAGIC:
        if data[: len(MAGIC_PREFIX)] == MAGIC_PREFIX:
            if size < len(MAGIC) and MAGIC.startswith(bytes(data)):
                raise KcadError("truncated", f"Dosya eksik: {size} bayt, imza bile tamamlanmıyor.")
            raise KcadError("damaged_signature", "KCAD imzası bozuk: dosya metin olarak aktarılmış (satır sonları değişmiş) olabilir.")
        raise KcadError("not_kcad", "KCAD (v2) çizim dosyası değil: imza yok.")
    if size < FIXED_HEADER:
        raise KcadError("truncated", f"Dosya eksik: başlık {FIXED_HEADER} bayt olmalı, dosya {size} bayt.")
    major, minor, min_reader = data[9], data[10], data[11]
    if major != MAJOR:
        raise KcadError("unsupported_version", f"KCAD ana sürümü {major}; bu okuyucu yalnız {MAJOR}'yi okur.")
    if min_reader > MINOR:
        raise KcadError("newer_version", f"Dosya en az KCAD {MAJOR}.{min_reader} okuyucusu istiyor; bu okuyucu {MAJOR}.{MINOR}.")
    header_length = _u16(data, 12)
    encoding, codec = data[14], data[15]
    payload_length = _u64(data, 16)
    decoded_length = _u64(data, 24)
    flags = _u16(data, 32)
    count = _u16(data, 34)
    if header_length < FIXED_HEADER:
        raise KcadError("bad_header", f"Başlık uzunluğu {header_length}; en az {FIXED_HEADER} olmalı.")
    if payload_length > MAX_PAYLOAD or decoded_length > MAX_PAYLOAD:
        raise KcadError("too_large", f"Yük {max(payload_length, decoded_length)} bayt; sınır {MAX_PAYLOAD}.")
    total = header_length + payload_length + HASH_SIZE
    if size < total:
        raise KcadError("truncated", f"Dosya eksik: {total} bayt olmalı, {size} bayt var.")
    if size > total:
        raise KcadError("trailing_data", f"Dosyanın sonunda {size - total} fazla bayt var.")
    if hashlib.sha256(data[: size - HASH_SIZE]).digest() != bytes(data[size - HASH_SIZE :]):
        raise KcadError("hash_mismatch", "SHA-256 özeti tutmuyor: dosya bozulmuş.")
    if minor < min_reader:
        raise KcadError("bad_header", f"Küçük sürüm {minor}, en az okuyucu sürümünden ({min_reader}) küçük.")
    if flags != 0:
        raise KcadError("bad_header", f"Bilinmeyen başlık bayrakları: {flags:#06x}.")
    if count > MAX_EXTENSIONS:
        raise KcadError("bad_header", f"{count} zorunlu uzantı; en çok {MAX_EXTENSIONS}.")
    extensions = []
    at = FIXED_HEADER
    for _ in range(count):
        if at >= header_length:
            raise KcadError("bad_header", "Uzantı listesi başlığın dışına taşıyor.")
        length = data[at]
        name = bytes(data[at + 1 : at + 1 + length])
        if not 1 <= length <= MAX_EXTENSION_NAME or at + 1 + length > header_length or not set(name) <= EXTENSION_CHARS:
            raise KcadError("bad_header", "Uzantı adı geçersiz.")
        extensions.append(name.decode("ascii"))
        at += 1 + length
    if at != header_length:
        raise KcadError("bad_header", "Başlık uzunluğu uzantı listesiyle bitmiyor.")
    if encoding != ENCODING_CBOR_PROFILE_1:
        raise KcadError("unknown_encoding", f"Yük kodlaması {encoding} tanımsız.")
    if codec != CODEC_NONE:
        raise KcadError("unknown_codec", f"Sıkıştırma kodeki {codec} tanımsız.")
    if decoded_length != payload_length:
        raise KcadError("bad_header", "Sıkıştırmasız yükte çözülmüş uzunluk yük uzunluğuna eşit olmalı.")
    if extensions:
        raise KcadError("unknown_extension", f"Tanınmayan zorunlu uzantı: {', '.join(extensions)}.")
    header = {
        "major": major,
        "minor": minor,
        "minReader": f"{MAJOR}.{min_reader}",
        "headerLength": header_length,
        "encoding": encoding,
        "codec": codec,
        "payloadLength": payload_length,
        "sha256": bytes(data[size - HASH_SIZE :]).hex(),
    }
    return header, bytes(data[header_length : header_length + payload_length])


class _Cbor:
    """A decoder of the KentOS CBOR profile: every rule of spec §5 is checked while reading."""

    def __init__(self, data):
        self.data = data
        self.pos = 0

    def fail(self, code, message):
        raise KcadError(code, f"{message} (yükün {self.pos}. baytı)")

    def need(self, n):
        if self.pos + n > len(self.data):
            self.fail("cbor_truncated", "Yük bir öğenin ortasında bitiyor")

    def head(self):
        self.need(1)
        initial = self.data[self.pos]
        self.pos += 1
        major, info = initial >> 5, initial & 31
        if info < 24:
            return major, info, info
        if info in (28, 29, 30):
            self.fail("malformed", f"Ayrılmış ek bilgi {info}")
        if info == 31:
            if major in (2, 3, 4, 5):
                self.fail("indefinite_length", "Belirsiz uzunluk kullanılamaz")
            self.fail("malformed", "Yerinde olmayan belirsiz uzunluk ya da kırma baytı")
        if major == 7:
            return major, info, None
        size = {24: 1, 25: 2, 26: 4, 27: 8}[info]
        self.need(size)
        arg = int.from_bytes(self.data[self.pos : self.pos + size], "big")
        self.pos += size
        if arg < {24: 24, 25: 256, 26: 65536, 27: 1 << 32}[info]:
            self.fail("non_shortest", f"{arg} en kısa biçimde yazılmamış")
        return major, info, arg

    def item(self, depth):
        major, info, arg = self.head()
        if major == 0:
            return arg
        if major == 1:
            return -1 - arg
        if major in (2, 3):
            if arg > MAX_STRING:
                self.fail("too_long", f"{arg} baytlık dizgi; sınır {MAX_STRING}")
            self.need(arg)
            raw = self.data[self.pos : self.pos + arg]
            self.pos += arg
            if major == 2:
                return bytes(raw)
            try:
                return bytes(raw).decode("utf-8")
            except UnicodeDecodeError:
                self.fail("invalid_utf8", "Metin geçerli UTF-8 değil")
        if major in (4, 5):
            if depth > MAX_DEPTH:
                self.fail("too_deep", f"İç içe derinlik {MAX_DEPTH}'ı aşıyor")
            if arg > MAX_ITEMS:
                self.fail("too_long", f"{arg} öğe; sınır {MAX_ITEMS}")
            if (arg if major == 4 else 2 * arg) > len(self.data) - self.pos:
                self.fail("cbor_truncated", f"{arg} öğe yükte kalan baytlara sığmıyor")
            if major == 4:
                return [self.item(depth + 1) for _ in range(arg)]
            result = {}
            previous = None
            for _ in range(arg):
                self.need(1)
                if self.data[self.pos] >> 5 != 3:
                    self.fail("non_text_key", "Harita anahtarı metin değil")
                start = self.pos
                key = self.item(depth + 1)
                raw = bytes(self.data[start : self.pos])
                if previous is not None:
                    if raw == previous:
                        self.fail("duplicate_key", f"“{key}” anahtarı iki kez yazılmış")
                    if raw < previous:
                        self.fail("unsorted_keys", f"“{key}” anahtarı sırasında değil")
                previous = raw
                result[key] = self.item(depth + 1)
            return result
        if major == 6:
            self.fail("tag", "CBOR etiketi kullanılamaz")
        if info == 20:
            return False
        if info == 21:
            return True
        if info == 22:
            return None
        if info in (25, 26):
            self.fail("narrow_float", "Kayan noktalı sayı binary64 (8 bayt) yazılmalı")
        if info == 27:
            self.need(8)
            (value,) = struct.unpack(">d", self.data[self.pos : self.pos + 8])
            self.pos += 8
            if math.isnan(value) or math.isinf(value):
                self.fail("non_finite", "NaN ya da sonsuz sayı yazılamaz")
            return value
        self.fail("simple_value", "Bu basit değer kullanılamaz (yalnız true, false, null)")


def decode_payload(payload):
    """The payload as Python values: dict, list, str, bytes, int, float, bool, None."""
    decoder = _Cbor(payload)
    value = decoder.item(1)
    if decoder.pos != len(payload):
        decoder.fail("cbor_trailing", "Yükte belgeden sonra fazladan bayt var")
    return value


# ── Document schemas 2 to 10 (spec §6) ──────────────────────────────────


def view_id_of_name(name):
    """A named view's id from its name (docs/adr/0345 §2): `v-` and the name's FNV-1a, 32 bits, 8 lowercase hex digits."""
    h = 0x811C9DC5
    for b in name.encode():
        h = ((h ^ b) * 0x01000193) & 0xFFFFFFFF
    return f"v-{h:08x}"


class _Schema:
    """Checks the decoded payload and turns it into the contract's JSON form (`DocumentSnapshotV2`)."""

    def __init__(self):
        self.path = []
        self.uids = []
        self.seen = set()
        # What the payload's schema lets an object hold: its own line weight (schema 3 and up), vertex elevations
        # (schema 4 and up), an area's parts (schema 5 and up), blocks (schema 6 and up), a text's extras (schema 7).
        self.weights = False
        self.elevations = False
        self.parts = False
        self.line_parts = False
        self.linked_texts = False
        self.paragraphs = False
        self.styles = False
        self.blocks = False
        self.texts = False
        # While a block definition's objects are read: how many so far (they have no persistent ids).
        self.inside = None
        # The definitions' ids and places, once read; where each insert inside them is.
        self.index = {}
        self.movable = {}
        self.dynamics = {}

    def where(self):
        return "/".join(self.path) or "(kök)"

    def fail(self, code, message):
        raise KcadError(code, f"{self.where()}: {message}")

    def at(self, segment, check, value):
        self.path.append(str(segment))
        try:
            return check(value)
        finally:
            self.path.pop()

    # Scalars.

    def text(self, v):
        if type(v) is not str:
            self.fail("wrong_type", "metin olmalı")
        return v

    def float(self, v):
        if type(v) is not float:
            self.fail("wrong_type", "float64 olmalı")
        return v

    def uint(self, bits):
        def check(v):
            if type(v) is not int:
                self.fail("wrong_type", "tam sayı olmalı")
            if not 0 <= v < (1 << bits):
                self.fail("bad_value", f"{v} aralık dışında (0…{(1 << bits) - 1})")
            return v

        return check

    def bool(self, v):
        if type(v) is not bool:
            self.fail("wrong_type", "true ya da false olmalı")
        return v

    def enum(self, values):
        def check(v):
            self.text(v)
            if v not in values:
                self.fail("bad_value", f"“{v}” bilinmiyor ({', '.join(values)})")
            return v

        return check

    def id16(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "16 baytlık bayt dizgisi olmalı")
        if len(v) != 16:
            self.fail("bad_value", f"{len(v)} bayt; kimlik 16 bayt olmalı")
        if v == bytes(16):
            self.fail("bad_value", "kimlik boş (sıfır) olamaz")
        return uuid_text(v)

    def sha256(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "32 baytlık bayt dizgisi olmalı")
        if len(v) != 32:
            self.fail("bad_value", f"{len(v)} bayt; SHA-256 32 bayt olmalı")
        return v.hex()

    # Compounds.

    def array(self, item):
        def check(v):
            if type(v) is not list:
                self.fail("wrong_type", "dizi olmalı")
            return [self.at(i, item, x) for i, x in enumerate(v)]

        return check

    def point(self, v):
        if type(v) is not list:
            self.fail("wrong_type", "nokta [x, y] dizisi olmalı")
        if len(v) != 2:
            self.fail("bad_value", f"nokta 2 sayı olmalı, {len(v)} var")
        return {"x": self.at(0, self.float, v[0]), "y": self.at(1, self.float, v[1])}

    def bounds(self, v):
        if type(v) is not list:
            self.fail("wrong_type", "sınırlar [minX, minY, maxX, maxY] dizisi olmalı")
        if len(v) != 4:
            self.fail("bad_value", f"sınırlar 4 sayı olmalı, {len(v)} var")
        names = ("minX", "minY", "maxX", "maxY")
        return {n: self.at(i, self.float, x) for i, (n, x) in enumerate(zip(names, v))}

    def keyed(self, of):
        """A map of text keys to values `of` reads."""
        def read(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            return {k: self.at(k, of, x) for k, x in v.items()}
        return read

    def attrs(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "harita olmalı")
        return {k: self.at(k, self.text, x) for k, x in v.items()}

    def any(self, v):
        """An opaque part: JSON-compatible values, integers within JSON's 64-bit range."""
        if v is None or type(v) in (bool, str, float):
            return v
        if type(v) is int:
            if not -(1 << 63) <= v < (1 << 64):
                self.fail("bad_value", f"{v} 64 bit tam sayı aralığının dışında")
            return v
        if type(v) is list:
            return [self.at(i, self.any, x) for i, x in enumerate(v)]
        if type(v) is dict:
            return {k: self.at(k, self.any, x) for k, x in v.items()}
        self.fail("wrong_type", "JSON'la gösterilebilen bir değer olmalı (bayt dizgisi olamaz)")

    def not_null(self, v):
        if v is None:
            self.fail("bad_value", "null yazılmaz; değer yoksa alan hiç yazılmaz")
        return self.any(v)

    def fields(self, table, rename=None):
        """A map with the given fields: {key: (check, required)}; unknown keys and missing required ones are refused."""

        def check(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            for key in v:
                if key not in table:
                    self.path.append(key)
                    self.fail("unknown_field", "bilinmeyen alan")
            for key, (_, required) in table.items():
                if required and key not in v:
                    self.path.append(key)
                    self.fail("missing_field", "zorunlu alan yok")
            return {(rename or {}).get(k, k): self.at(k, table[k][0], x) for k, x in v.items()}

        return check

    # The document.

    def root(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "yük bir harita olmalı")
        if v.get("format") != DOCUMENT_FORMAT:
            self.fail("schema_format", f"KentOS çizimi değil (format ≠ {DOCUMENT_FORMAT})")
        version = v.get("version")
        if type(version) is not int or version not in SCHEMAS:
            self.fail("schema_version", f"belge şeması sürümü {version!r} okunamıyor (desteklenen: {', '.join(map(str, SCHEMAS))})")
        # In an older schema these fields are unknown ones.
        self.weights = version >= SCHEMA_WITH_LINE_WEIGHTS
        self.elevations = version >= SCHEMA_WITH_ELEVATIONS
        self.parts = version >= SCHEMA_WITH_PARTS
        self.line_parts = version >= SCHEMA_WITH_LINE_PARTS
        self.linked_texts = version >= SCHEMA_WITH_LINKED_TEXTS
        self.paragraphs = version >= SCHEMA_WITH_PARAGRAPHS
        self.styles = version >= SCHEMA_WITH_STYLES
        self.tables = version >= SCHEMA_WITH_TABLES
        self.hatches = version >= SCHEMA_WITH_HATCH_PATTERNS
        self.images = version >= SCHEMA_WITH_IMAGES
        self.rasters = version >= SCHEMA_WITH_RASTERS
        self.point_clouds = version >= SCHEMA_WITH_POINT_CLOUDS
        self.multidim = version >= SCHEMA_WITH_MULTIDIM
        self.measures = version >= SCHEMA_WITH_MEASURES
        self.attachments = version >= SCHEMA_WITH_ATTACHMENTS
        self.compose = version >= SCHEMA_WITH_COMPOSE
        self.addresses = version >= SCHEMA_WITH_ADDRESSES
        self.dim_refs = version >= SCHEMA_WITH_DIMENSION_REFS
        self.sheets = version >= SCHEMA_WITH_SHEETS
        self.requests = version >= SCHEMA_WITH_REQUESTS
        self.polyline_extras = version >= SCHEMA_WITH_POLYLINE_EXTRAS
        self.nurbs = version >= SCHEMA_WITH_NURBS
        self.look = version >= SCHEMA_WITH_LOOK
        self.ucs = version >= SCHEMA_WITH_UCS
        self.imperial = version >= SCHEMA_WITH_IMPERIAL
        self.subref_extras = version >= SCHEMA_WITH_SUBREF_EXTRAS
        self.views = version >= SCHEMA_WITH_VIEWS
        self.arrays = version >= SCHEMA_WITH_ARRAYS
        self.wipeouts = version >= SCHEMA_WITH_WIPEOUTS
        self.frozen = version >= SCHEMA_WITH_FROZEN
        self.patterns = version >= SCHEMA_WITH_PATTERNS
        self.block_extras = version >= SCHEMA_WITH_BLOCK_EXTRAS
        self.block_cards = version >= SCHEMA_WITH_BLOCK_CARDS
        self.dynamic_blocks = version >= SCHEMA_WITH_DYNAMIC_BLOCKS
        self.dim_rules = version >= SCHEMA_WITH_DIMENSION_RULES
        self.dim_options = version >= SCHEMA_WITH_DIMENSION_OPTIONS
        self.dim_marks = version >= SCHEMA_WITH_DIMENSION_MARKS
        self.overrides = version >= SCHEMA_WITH_OVERRIDES
        self.annotative = version >= SCHEMA_WITH_ANNOTATIVE
        self.paragraph_layout = version >= SCHEMA_WITH_PARAGRAPH_LAYOUT
        self.text_fields = version >= SCHEMA_WITH_TEXT_FIELDS
        self.multileader = version >= SCHEMA_WITH_MULTILEADER
        self.table_cells = version >= SCHEMA_WITH_TABLE_CELLS
        self.hatch_boundaries = version >= SCHEMA_WITH_HATCH_BOUNDARIES
        self.hatch_gaps = version >= SCHEMA_WITH_HATCH_GAPS
        self.hatch_extras = version >= SCHEMA_WITH_HATCH_EXTRAS
        self.forms = version >= SCHEMA_WITH_FORMS
        self.constraints = version >= SCHEMA_WITH_CONSTRAINTS
        self.sources = version >= SCHEMA_WITH_SOURCES
        self.walls = version >= SCHEMA_WITH_WALLS
        self.wall_joins = version >= SCHEMA_WITH_WALL_JOINS
        self.openings = version >= SCHEMA_WITH_OPENINGS
        self.spaces = version >= SCHEMA_WITH_SPACES
        self.space_columns = version >= SCHEMA_WITH_SPACE_COLUMNS
        self.space_scope = version >= SCHEMA_WITH_SPACE_SCOPE
        self.space_sources = version >= SCHEMA_WITH_SPACE_SOURCES
        self.space_separators = version >= SCHEMA_WITH_SPACE_SEPARATORS
        self.space_choices = version >= SCHEMA_WITH_SPACE_CHOICES
        self.dimension_sets = version >= SCHEMA_WITH_DIMENSION_SETS
        self.dimension_set_edges = version >= SCHEMA_WITH_DIMENSION_SET_EDGES
        self.railings = version >= SCHEMA_WITH_RAILINGS
        self.arch_elements = version >= SCHEMA_WITH_ARCH_ELEMENTS
        self.tags = version >= SCHEMA_WITH_TAGS
        self.viewmarks = version >= SCHEMA_WITH_VIEWMARKS
        self.phases = version >= SCHEMA_WITH_PHASES
        self.xrefs = version >= SCHEMA_WITH_XREFS
        self.xref_paths = version >= SCHEMA_WITH_XREF_PATHS
        self.clipping = version >= SCHEMA_WITH_CLIPPING
        self.heights = version >= SCHEMA_WITH_HEIGHTS
        self.network_turns = version >= SCHEMA_WITH_NETWORK_TURNS
        self.georef = version >= SCHEMA_WITH_GEOREF
        self.join_views = version >= SCHEMA_WITH_JOIN_VIEWS
        self.recorded_time = version >= SCHEMA_WITH_RECORDED_TIME
        self.source_bases = version >= SCHEMA_WITH_SOURCE_BASES
        self.source_field_bases = version >= SCHEMA_WITH_SOURCE_FIELD_BASES
        self.network_terminals = version >= SCHEMA_WITH_NETWORK_TERMINALS
        self.network_configs = version >= SCHEMA_WITH_NETWORK_CONFIGS
        self.network_profiles = version >= SCHEMA_WITH_NETWORK_PROFILES
        self.timed_closures = version >= SCHEMA_WITH_TIMED_CLOSURES
        self.linear_events = version >= SCHEMA_WITH_LINEAR_EVENTS
        self.source_crs = version >= SCHEMA_WITH_SOURCE_CRS
        self.exception_digests = version >= SCHEMA_WITH_EXCEPTION_DIGESTS
        self.georef_crs = version >= SCHEMA_WITH_GEOREF_CRS
        self.lineage = version >= SCHEMA_WITH_LINEAGE
        self.layer_geometry = version >= SCHEMA_WITH_LAYER_GEOMETRY
        self.tile_tables = version >= SCHEMA_WITH_RASTER_TILES
        self.field_kinds = version >= SCHEMA_WITH_FIELD_KINDS
        self.join_sources = version >= SCHEMA_WITH_JOIN_SOURCES
        self.datasets = version >= SCHEMA_WITH_DATASETS
        self.withheld_fields = version >= SCHEMA_WITH_WITHHELD_FIELDS
        self.text_paths = version >= SCHEMA_WITH_TEXT_PATHS
        self.layer_fields = version >= SCHEMA_WITH_LAYER_FIELDS
        self.blocks = version >= SCHEMA_WITH_BLOCKS
        self.texts = version >= SCHEMA_WITH_TEXT_EXTRAS
        self.leaders = version >= SCHEMA_WITH_LEADERS
        self.dimensions = version >= SCHEMA_WITH_DIMENSIONS
        self.layer_snap = version >= SCHEMA_WITH_LAYER_SNAP
        self.drawing_unit = version >= SCHEMA_WITH_DRAWING_UNIT
        self.second_srid = version >= SCHEMA_WITH_SECOND_SRID
        self.custom_crs = version >= SCHEMA_WITH_CUSTOM_CRS
        self.survey_fields = version >= SCHEMA_WITH_SURVEY
        self.traverse_tolerances = version >= SCHEMA_WITH_TRAVERSE_TOLERANCES
        self.ground = version >= SCHEMA_WITH_GROUND
        self.layer_states = version >= SCHEMA_WITH_LAYER_STATES
        self.topology = version >= SCHEMA_WITH_TOPOLOGY
        self.survey_sigmas = version >= SCHEMA_WITH_SURVEY_SIGMAS
        self.annotation = version >= SCHEMA_WITH_ANNOTATION
        self.services = version >= SCHEMA_WITH_SERVICES
        self.networks = version >= SCHEMA_WITH_NETWORKS
        self.temporal = version >= SCHEMA_WITH_TEMPORAL
        self.filters = version >= SCHEMA_WITH_FILTERS
        self.labels = version >= SCHEMA_WITH_LABELS
        self.variables = version >= SCHEMA_WITH_VARIABLES
        checked = self.fields({"format": (self.text, True), "version": (self.uint(32), True), "document": (self.document, True)})(v)
        return checked["document"]

    def document(self, v):
        d = self.fields(
            {
                "name": (self.text, True),
                **({"blocks": (self.definitions, False)} if self.blocks else {}),
                "layers": (self.array(self.layer), True),
                "origin": (self.point, True),
                "styles": (self.fields({"items": (self.array(self.any), True), "categories": (self.array(self.any), True)}), True),
                "entities": (self.array(self.entity), True),
                "homeView": (self.bounds, False),
                "settings": (self.settings, True),
                "projectId": (self.id16, False),
                "activeLayer": (self.text, True),
                "migratedFrom": (self.source, False),
                **({"sheets": (self.sheets_, False)} if self.sheets else {}),
                **({"requests": (self.requests_, False)} if self.requests else {}),
                **({"arrays": (self.arrays_, False)} if self.arrays else {}),
                **({"constraints": (self.constraints_, False)} if self.constraints else {}),
                **({"dimensionSets": (self.dimension_sets_, False)} if self.dimension_sets else {}),
            }
        )(v)
        # Schema 53's rules (docs/adr/0300 §1), once the blocks and the objects are known.
        if "arrays" in d:
            self.check_arrays(d["arrays"], [b["id"] for b in d.get("blocks", [])], set(self.uids))
        # Schema 73's rules (docs/adr/0334 §1), once the objects are known.
        if "constraints" in d:
            self.check_constraints(d["constraints"], set(self.uids))
        # Schema 32's links (docs/adr/0208 §2).
        links = service_links_problem(d["layers"], d["settings"].get("connections", []), d["entities"])
        if links:
            place, words = links
            self.path.extend(place.split("/"))
            self.fail("bad_value", words)
        # Schema 34's tree rules (docs/adr/0210 §2).
        tree = scenarios_problem(d["layers"])
        if tree:
            self.path.append("layers")
            self.fail("bad_value", tree)
        # Schema 35's tree rule (docs/adr/0211 §2).
        tree = filters_problem(d["layers"])
        if tree:
            self.path.append("layers")
            self.fail("bad_value", tree)
        # Schema 36's tree rule (docs/adr/0212 §2).
        tree = labels_problem(d["layers"])
        if tree:
            self.path.append("layers")
            self.fail("bad_value", tree)
        # Schema 40's tree rule (docs/adr/0225 §4, §5).
        tree = infos_problem(d["layers"])
        if tree:
            self.path.append("layers")
            self.fail("bad_value", tree)
        # Schema 41's tree rule (docs/adr/0226 §3, §4).
        tree = composes_problem(d["layers"])
        if tree:
            self.path.append("layers")
            self.fail("bad_value", tree)
        out = {
            "format": DOCUMENT_FORMAT,
            "version": DOCUMENT_VERSION,
            "name": d["name"],
            "settings": d["settings"],
            "origin": d["origin"],
        }
        if "homeView" in d:
            out["homeView"] = d["homeView"]
        out["layers"] = d["layers"]
        out["activeLayer"] = d["activeLayer"]
        out["entities"] = d["entities"]
        out["uids"] = list(self.uids)
        out["styles"] = d["styles"]
        if "blocks" in d:
            out["blocks"] = d["blocks"]
        if "projectId" in d:
            out["projectId"] = d["projectId"]
        if "migratedFrom" in d:
            out["migratedFrom"] = d["migratedFrom"]
        if "sheets" in d:
            out["sheets"] = d["sheets"]
        if "arrays" in d:
            out["arrays"] = d["arrays"]
        if "constraints" in d:
            out["constraints"] = d["constraints"]
        if "dimensionSets" in d:
            out["dimensionSets"] = d["dimensionSets"]
        if "requests" in d:
            out["requests"] = d["requests"]
        return out

    def dimension_sets_(self, v):
        """Schema 113: JSON-shaped ids/points, recipes and stable ordinary member ids.

        A recipe is preserved when a host disappears; shape validation deliberately
        does not prune unresolved sources. Object/layer integrity is the application's.
        """
        try:
            return self.dimension_sets_checked(v)
        except KcadError as error:
            # This JSON-shaped root field has one shape-error code, like the Rust reader.
            raise KcadError("bad_value", error.message) from error

    def dimension_sets_checked(self, v):
        def uid(value):
            value = self.text(value)
            if not UUID_TEXT.fullmatch(value) or value == "00000000-0000-0000-0000-000000000000":
                self.fail("bad_value", "ölçü dizisinin kimliği dolu, küçük harfli UUID olmalı")
            return value
        row = self.enum(("openings", "overall"))
        def source(value):
            def place(value):
                if type(value) is not dict:
                    self.fail("bad_value", "kaynak yeri harita olmalı")
                kind = value.get("kind")
                table = {"kind": (self.enum(("vertex", "center")), True)}
                if kind == "vertex":
                    table.update({key:(self.uint(32), True) for key in ("part", "ring", "index")})
                result = self.fields(table)(value)
                if any(result.get(key, 0) > SUBREF_INDEX_LIMIT for key in ("part", "ring", "index")):
                    self.fail("bad_value", "kaynak yerinin sırası çok büyük")
                return result
            return self.fields({"uid": (uid, True), "place": (place, True)})(value)
        def look(value):
            table = {
                "dimStyle": (self.style_id, False), "arrow": (self.enum(DIMENSION_ARROWS), False),
                "arrowSize": (self.size(True), False), "extOffset": (self.size(False), False),
                "extBeyond": (self.size(False), False), "textGap": (self.size(False), False),
                "textPlace": (self.enum(("centre",)), False), "decimals": (self.decimals, False),
                "unit": (self.enum(("mm", "cm", "m", "in", "ft")), False),
                "prefix": (self.affix("prefix"), False), "suffix": (self.affix("suffix"), False),
                "font": (self.enum(DRAWING_FONTS), False), "dimLineColor": (self.dimension_colour("dimLineColor"), False),
                "dimLineWeight": (self.dimension_weight("dimLineWeight"), False), "dimLineType": (self.enum(LINE_TYPES), False),
                "extColor": (self.dimension_colour("extColor"), False), "extWeight": (self.dimension_weight("extWeight"), False),
                "extLineType": (self.enum(LINE_TYPES), False), "textColor": (self.dimension_colour("textColor"), False),
                "options": (self.dimension_options, False), "overrides": (self.overrides_of(DIMENSION_OVERRIDES), False),
            }
            return self.fields(table)(value)
        segment = {"row": (row, True), "a": (source, True), "b": (source, True)}
        def nonempty_array(check):
            def read(value):
                result = self.array(check)(value)
                if not result:
                    self.fail("bad_value", "boş isteğe bağlı liste yazılmaz")
                return result
            return read
        def edge(value):
            # Schema 114 (docs/adr/0700): the straight edge of a closed or many-edged wall.
            if not self.dimension_set_edges:
                self.fail("bad_value", "ölçü dizisinin kenarı (edge) şema 114 ister")
            value = self.uint(32)(value)
            if value >= 100000:
                self.fail("bad_value", "ölçü dizisinin kenar numarası çok büyük")
            return value
        sets = nonempty_array(self.fields({
            "id": (uid, True), "wall": (uid, True), "edge": (edge, False),
            "face": (self.enum(("left", "right")), True),
            "rows": (self.array(row), True), "openings": (self.enum(("ends", "centers")), True),
            "layerId": (self.text, True), "at": (self.json_point, True), "gapMm": (self.float, True),
            "height": (self.float, True), "look": (look, True),
            "rowPlaces": (nonempty_array(self.fields({"row": (row, True), "at": (self.json_point, True)})), False),
            "members": (nonempty_array(self.fields({**segment, "uid": (uid, True)})), False),
            "suppressed": (nonempty_array(self.fields(segment)), False),
            "broken": (self.enum(("hostGone", "unsupportedWall", "openingUnplaced")), False),
        }))(v)
        if len(sets) > 10000:
            self.fail("bad_value", "çok fazla ölçü dizisi")
        ids, member_ids = set(), set()
        for item in sets:
            if item["id"] in ids or not item["layerId"] or not 1 <= len(item["rows"]) <= 2 or len(set(item["rows"])) != len(item["rows"]):
                self.fail("bad_value", "ölçü dizisinin kimliği, katmanı veya sıraları geçersiz")
            ids.add(item["id"])
            if item["gapMm"] <= 0 or item["height"] <= 0:
                self.fail("bad_value", "aralık ve yazı yüksekliği pozitif olmalı")
            places = [p["row"] for p in item.get("rowPlaces", [])]
            if len(set(places)) != len(places):
                self.fail("bad_value", "elle sıra yeri yineleniyor")
            members = item.get("members", [])
            segments = members + item.get("suppressed", [])
            if len(segments) > 10000:
                self.fail("bad_value", "çok fazla ölçü segmenti")
            keys = set()
            for k, part in enumerate(segments):
                key = json.dumps({n:part[n] for n in ("row", "a", "b")}, sort_keys=True)
                if part["a"] == part["b"] or key in keys or (k < len(members) and part["row"] not in item["rows"]):
                    self.fail("bad_value", "ölçü segmenti geçersiz veya yineleniyor")
                keys.add(key)
            for part in members:
                if part["uid"] in member_ids:
                    self.fail("bad_value", "ölçü üyesi yineleniyor")
                member_ids.add(part["uid"])
        return sets

    # Schema 53's associative arrays (docs/adr/0300 §1): read field by field (true flags only, a layout's own fields
    # only), then checked against the blocks and the objects.

    def arrays_(self, v):
        def true(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "yalnız doğruyken yazılır")
            return True

        def count(v):
            n = self.uint(64)(v)
            if n > ARRAY_PLACES_MAX:
                self.fail("bad_value", f"{n} aralık dışında (0…{ARRAY_PLACES_MAX})")
            return n

        kinds = {
            "grid": {"rows": (count, True), "cols": (count, True), "dx": (self.float, True), "dy": (self.float, True)},
            "polar": {"center": (self.point, True), "count": (count, True), "fill": (self.float, True), "rotate": (self.bool, True)},
            "line": {"count": (count, True), "dx": (self.float, True), "dy": (self.float, True), "fit": (true, False)},
            "path": {"path": (self.id16, True), "count": (count, True), "spacing": (self.float, False), "align": (self.bool, True)},
        }

        def layout(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            kind = self.at("kind", self.enum(tuple(kinds)), v.get("kind"))
            return self.fields({"kind": (self.text, True), **kinds[kind]})(v)

        item = self.fields({"uid": (self.id16, False), "kept": (true, False)})
        array = self.fields(
            {
                "p": (self.point, True),
                "id": (self.id16, True),
                "block": (self.id16, True),
                "items": (self.array(item), True),
                "scale": (self.float, True),
                "layout": (layout, True),
                "mirror": (true, False),
                "layerId": (self.text, True),
                "rotation": (self.float, True),
            }
        )
        if type(v) is list and not v:
            self.fail("bad_value", "boş dizi listesi yazılmaz")
        return self.array(array)(v)

    # Schema 73's constraints and parameters (docs/adr/0334 §1): read field by field (flags only when true, an axis only
    # when not aligned, no empty lists), then checked against the rules and the objects.

    def constraints_(self, v):
        def true(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "yalnız doğruyken yazılır")
            return True

        def uuid(v):
            self.text(v)
            if not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", v) or v == "00000000-0000-0000-0000-000000000000":
                self.fail("bad_value", "küçük harfli, tireli, boş olmayan bir UUID olmalı")
            return v

        index = self.uint(32)
        places = {
            "start": {}, "end": {}, "center": {},
            "node": {"part": (index, True)},
            "vertex": {"part": (index, True), "ring": (index, True), "index": (index, True)},
        }

        def place(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            kind = self.at("kind", self.enum(tuple(places)), v.get("kind"))
            return self.fields({"kind": (self.text, True), **places[kind]})(v)

        elements = {
            "point": {"place": (place, True)},
            "segment": {"part": (index, True), "ring": (index, True), "index": (index, True)},
            "circle": {},
        }

        def element(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            kind = self.at("kind", self.enum(tuple(elements)), v.get("kind"))
            return self.fields({"kind": (self.text, True), **elements[kind]})(v)

        ref = self.fields({"uid": (uuid, True), "element": (element, True)})
        pair = {"a": (ref, True), "b": (ref, True)}
        rules = {
            "coincident": pair,
            "horizontal": {"a": (ref, True), "b": (ref, False)},
            "vertical": {"a": (ref, True), "b": (ref, False)},
            "parallel": pair,
            "perpendicular": pair,
            "tangent": {**pair, "inside": (true, False), "left": (true, False)},
            "concentric": pair,
            "collinear": pair,
            "equal": pair,
            "symmetric": {**pair, "axis": (ref, True)},
            "fix": {"a": (ref, True)},
            "distance": {**pair, "axis": (self.enum(("horizontal", "vertical")), False), "left": (true, False), "parameter": (self.text, True)},
            "angle": {**pair, "reverseA": (true, False), "reverseB": (true, False), "parameter": (self.text, True)},
            "radius": {"a": (ref, True), "parameter": (self.text, True)},
            "diameter": {"a": (ref, True), "parameter": (self.text, True)},
        }

        def rule(v):
            if type(v) is not dict:
                self.fail("wrong_type", "harita olmalı")
            kind = self.at("kind", self.enum(tuple(rules)), v.get("kind"))
            return self.fields({"kind": (self.text, True), **rules[kind]})(v)

        def bound(v):
            self.float(v)
            if not math.isfinite(v):
                self.fail("bad_value", "sınır sonlu bir sayı olmalı")
            return v

        def nonempty(v):
            self.text(v)
            if not v:
                self.fail("bad_value", "boş metin yazılmaz; yoksa alan hiç yazılmaz")
            return v

        def nonempty_list(item):
            def check(v):
                if type(v) is list and not v:
                    self.fail("bad_value", "boş liste yazılmaz; yoksa alan hiç yazılmaz")
                return self.array(item)(v)
            return check

        constraint = self.fields({"id": (uuid, True), "rule": (rule, True), "reference": (true, False)})
        parameter = self.fields({
            "name": (self.text, True),
            "kind": (self.enum(("length", "angle", "number")), True),
            "expression": (nonempty, False),
            "min": (bound, False),
            "max": (bound, False),
            "description": (nonempty, False),
        })
        out = self.fields({"parameters": (nonempty_list(parameter), False), "constraints": (nonempty_list(constraint), False)})(v)
        if not out:
            self.fail("bad_value", "boş kısıt kümesi yazılmaz; kısıt yoksa alan hiç yazılmaz")
        return out

    def check_constraints(self, cs, uids):
        """The contract's rules (`constraints_problem`, docs/adr/0334 §1) as the ADR says them, and every object an
        object of the drawing."""
        params = cs.get("parameters", [])
        cons = cs.get("constraints", [])
        self.path.append("constraints")
        if len(cons) > CONSTRAINTS_MAX or len(params) > PARAMETERS_MAX:
            self.fail("bad_value", "çok fazla kısıt ya da parametre")
        names = {}
        for i, p in enumerate(params):
            self.path.extend(["parameters", str(i)])
            if not parameter_name_ok(p["name"]):
                self.fail("bad_value", f"“{p['name']}” bir parametre adı değil")
            if p["name"] in names:
                self.fail("bad_value", "ad iki kez verilmiş")
            names[p["name"]] = i
            if "min" in p and "max" in p and p["min"] > p["max"]:
                self.fail("bad_value", "en az değer en çok değerden büyük")
            del self.path[-2:]
        shape = lambda r: r["element"]["kind"]  # noqa: E731
        owner, ids = {}, set()
        for i, c in enumerate(cons):
            self.path.extend(["constraints", str(i)])
            r = c["rule"]
            if c["id"] in ids:
                self.fail("bad_value", "kısıtın kimliği iki kez verilmiş")
            ids.add(c["id"])
            refs = [r[k] for k in ("a", "b", "axis") if k in r and type(r[k]) is dict]
            for e in refs:
                if e["element"]["kind"] == "point" and e["element"]["place"]["kind"] not in ("start", "end", "center", "node", "vertex"):
                    self.fail("bad_value", "nokta öğesi bir uç, köşe, merkez ya da düğümdür")
                if e["uid"] not in uids:
                    self.fail("bad_value", "kısıtın nesnesi çizimde yok")
            for k, e in enumerate(refs):
                if e in refs[k + 1:]:
                    self.fail("bad_value", "aynı öğe iki kez anılmış")
            kinds = [shape(e) for e in refs]
            kind = r["kind"]
            fits = {
                "coincident": kinds[0] == "point",
                "horizontal": kinds == ["segment"] or kinds == ["point", "point"],
                "vertical": kinds == ["segment"] or kinds == ["point", "point"],
                "parallel": kinds == ["segment", "segment"],
                "perpendicular": kinds == ["segment", "segment"],
                "collinear": kinds == ["segment", "segment"],
                "tangent": (kinds == ["segment", "circle"] and "inside" not in r) or (kinds == ["circle", "circle"] and "left" not in r),
                "concentric": kinds == ["circle", "circle"],
                "equal": kinds in (["segment", "segment"], ["circle", "circle"]),
                "symmetric": kinds == ["point", "point", "segment"],
                "fix": True,
                "distance": (kinds == ["point", "point"] and "left" not in r) or (kinds == ["point", "segment"] and "axis" not in r),
                "angle": kinds == ["segment", "segment"],
                "radius": kinds == ["circle"],
                "diameter": kinds == ["circle"],
            }[kind]
            if not fits:
                self.fail("bad_value", f"{kind} kısıtı bu öğeleri almaz")
            reference = c.get("reference", False)
            if "parameter" not in r:
                if reference:
                    self.fail("bad_value", "yalnız ölçü kısıtı başvuru olur")
            else:
                name = r["parameter"]
                if name not in names:
                    self.fail("bad_value", f"“{name}” diye bir parametre yok")
                if name in owner:
                    self.fail("bad_value", "parametre iki kısıtın")
                owner[name] = i
                p = params[names[name]]
                if p["kind"] != ("angle" if kind == "angle" else "length"):
                    self.fail("bad_value", "parametrenin türü kısıta uymuyor")
                if reference != ("expression" not in p):
                    self.fail("bad_value", "başvuru kısıtının parametresi ölçülür; sürücününkinin ifadesi olur")
            del self.path[-2:]
        reads = {}
        for i, p in enumerate(params):
            self.path.extend(["parameters", str(i)])
            if "expression" not in p:
                if p["name"] not in owner:
                    self.fail("bad_value", "kullanıcı parametresinin bir ifadesi olur")
                reads[p["name"]] = set()
            else:
                try:
                    used = expression_names(p["expression"])
                except ValueError as e:
                    self.fail("bad_value", f"ifade okunamadı: {e}")
                missing = sorted(used - set(names))
                if missing:
                    self.fail("bad_value", f"“{missing[0]}” diye bir parametre yok")
                reads[p["name"]] = used
            del self.path[-2:]
        # Names reading each other round: a depth-first search for a circle.
        state = {}

        def visit(n):
            state[n] = 1
            for m in reads[n]:
                if state.get(m) == 1 or (m not in state and visit(m)):
                    return True
            state[n] = 2
            return False

        for n in sorted(reads):
            if n not in state and visit(n):
                self.fail("bad_value", "parametreler birbirini okuyor")
        self.path.pop()

    def check_arrays(self, arrays, blocks, uids):
        """The contract's rules (`arrays_problem`), and every item's insert an object of the drawing."""
        ids, used = set(), set()
        for i, a in enumerate(arrays):
            self.path.extend(["arrays", str(i)])
            if a["id"] in ids:
                self.fail("bad_value", "dizinin kimliği iki kez verilmiş")
            ids.add(a["id"])
            if a["block"] not in blocks:
                self.fail("bad_value", "dizinin bloğu çizimde tanımlı değil")
            if not a["layerId"]:
                self.fail("bad_value", "dizinin katmanı boş")
            if not (math.isfinite(a["p"]["x"]) and math.isfinite(a["p"]["y"]) and math.isfinite(a["rotation"])):
                self.fail("bad_value", "dizinin yeri ya da dönüşü sonlu değil")
            if not (math.isfinite(a["scale"]) and a["scale"] > 0):
                self.fail("bad_value", "dizinin ölçeği sıfırdan büyük sonlu bir sayı olmalı")
            lay = a["layout"]
            numbers = [lay[k] for k in ("dx", "dy", "fill", "spacing") if k in lay]
            if "center" in lay:
                numbers += [lay["center"]["x"], lay["center"]["y"]]
            if not all(math.isfinite(x) for x in numbers):
                self.fail("bad_value", "dizinin yerleşiminde sonlu olmayan sayı var")
            if "spacing" in lay and lay["spacing"] <= 0:
                self.fail("bad_value", "yol dizisinin aralığı sıfırdan büyük olmalı")
            places = lay["rows"] * lay["cols"] if lay["kind"] == "grid" else lay["count"]
            if not 2 <= places <= ARRAY_PLACES_MAX:
                self.fail("bad_value", "dizinin yeri 2 ile 10 000 arasında olmalı")
            if len(a["items"]) != places:
                self.fail("bad_value", "dizinin her yerinin bir öğesi olmalı")
            for k, item in enumerate(a["items"]):
                if "uid" in item:
                    self.path.extend(["items", str(k)])
                    if item["uid"] in used:
                        self.fail("bad_value", "bir yerleştirme iki öğede")
                    used.add(item["uid"])
                    if item["uid"] not in uids:
                        self.path.append("uid")
                        self.fail("bad_value", "öğenin yerleştirmesi çizimde yok")
                    del self.path[-2:]
            del self.path[-2:]

    def requests_(self, v):
        """The request ledger (schema 45, docs/adr/0253 §6): not empty, at most 256; each request's id 1–128 of letters,
        digits and `. _ : -`, none twice; its fingerprint 16 bytes (32 hex digits in the JSON form); a command named,
        a version from 1, the revision decimal digits, its status done or undone, an answer of at most 64 KiB."""
        seen = set()

        def request(r):
            q = self.fields(
                {
                    "answer": (self.text, False),
                    "status": (self.enum(("done", "undone")), True),
                    "command": (self.text, True),
                    "version": (self.uint(32), True),
                    "revision": (self.text, True),
                    "requestId": (self.text, True),
                    "fingerprint": (self.fingerprint_, True),
                }
            )(r)
            rid = q["requestId"]
            if not (1 <= len(rid) <= 128 and all(c.isascii() and (c.isalnum() or c in "._:-") for c in rid)):
                self.path.append("requestId")
                self.fail("bad_value", "istek kimliği 1–128 karakter, yalnız harf, rakam ve . _ : - olmalı")
            if rid in seen:
                self.path.append("requestId")
                self.fail("bad_value", "aynı istek iki kez yazılmış")
            seen.add(rid)
            if not q["command"]:
                self.path.append("command")
                self.fail("bad_value", "komut adı boş")
            if q["version"] == 0:
                self.path.append("version")
                self.fail("bad_value", "sürüm 1'den başlar")
            if not q["revision"] or not all("0" <= c <= "9" for c in q["revision"]):
                self.path.append("revision")
                self.fail("bad_value", "sürüm ondalık bir sayı metni olmalı")
            if "answer" in q and len(q["answer"].encode("utf-8")) > REQUEST_ANSWER_LIMIT:
                self.path.append("answer")
                self.fail("bad_value", "cevap en çok 65536 bayt olabilir")
            out = {
                "requestId": rid,
                "fingerprint": q["fingerprint"],
                "command": q["command"],
                "version": q["version"],
                "revision": q["revision"],
                "status": q["status"],
            }
            if "answer" in q:
                out["answer"] = q["answer"]
            return out

        if type(v) is list and not v:
            self.fail("bad_value", "boş istek listesi yazılmaz")
        if type(v) is list and len(v) > REQUEST_LEDGER:
            self.fail("bad_value", "en çok 256 istek yazılır")
        return self.array(request)(v)

    def fingerprint_(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "16 baytlık bayt dizgisi olmalı")
        if len(v) != 16:
            self.fail("bad_value", f"{len(v)} bayt; parmak izi 16 bayt olmalı")
        return v.hex()

    def sheets_(self, v):
        """The project's sheet book (schema 44, docs/adr/0251): the book an opaque map; each picture's bytes held to
        its 32-byte SHA-256, none given twice; an empty picture list refused. The JSON form writes the bytes as base64."""
        seen = set()

        def book(b):
            if type(b) is not dict:
                self.fail("wrong_type", "pafta kitabı bir harita olmalı")
            return self.any(b)

        def picture(p):
            q = self.fields({"bytes": (self.blob_, True), "sha256": (self.sha256, True)})(p)
            if hashlib.sha256(q["bytes"]).hexdigest() != q["sha256"]:
                self.fail("bad_value", "resmin baytları SHA-256'sına uymuyor")
            if q["sha256"] in seen:
                self.fail("bad_value", "aynı resim iki kez verilmiş")
            seen.add(q["sha256"])
            return {"sha256": q["sha256"], "bytes": base64.b64encode(q["bytes"]).decode("ascii")}

        def pictures(ps):
            if type(ps) is list and not ps:
                self.fail("bad_value", "boş resim listesi yazılmaz")
            return self.array(picture)(ps)

        return self.fields({"book": (book, True), "pictures": (pictures, False)})(v)

    def blob_(self, v):
        if type(v) is not bytes:
            self.fail("wrong_type", "bayt dizgisi olmalı")
        return v

    def settings(self, v):
        s = self.fields(
            {
                "srid": (self.uint(32), True),
                "areaUnit": (self.enum(("m2", "donum", "ha")), True),
                "angleUnit": (self.enum(("grad", "deg")), True),
                "plotScale": (self.float, True),
                "workspace": (self.enum(("hybrid", "cad", "gis", "plan3d", "disaster")), False),
                **({"secondSrid": (self.uint(32), False)} if self.second_srid else {}),
                "drawingFont": (self.enum(("barlow", "arimo", "overpass", "quicksand", "architects-daughter", "courier-prime", "plex-mono")), False),
                **({"drawingUnit": (self.enum(("mm", "cm", "m", "in", "ft") if self.imperial else ("mm", "cm", "m")), False)} if self.drawing_unit else {}),
                "areaDecimals": (self.uint(32), True),
                "lengthDecimals": (self.uint(32), True),
                **(
                    {
                        "customCrs": (self.crs_definition, False),
                        "secondCustomCrs": (self.crs_definition, False),
                        "datumTransforms": (self.datum_transforms, False),
                    }
                    if self.custom_crs
                    else {}
                ),
                **({"survey": (self.survey, False)} if self.survey_fields else {}),
                **({"layerStates": (self.layer_states_, False)} if self.layer_states else {}),
                **({"textStyles": (self.text_styles, False), "dimensionStyles": (self.dimension_styles, False)} if self.styles else {}),
                **({"leaderStyles": (self.leader_styles, False)} if self.multileader else {}),
                **({"topology": (self.topology_, False)} if self.topology else {}),
                **({"annotation": (self.annotation_, False)} if self.annotation else {}),
                **({"connections": (self.connections_, False)} if self.services else {}),
                **({"networks": (self.networks_, False)} if self.networks else {}),
                **({"variables": (self.variables_, False)} if self.variables else {}),
                **({"addressLocators": (self.address_locators_, False)} if self.addresses else {}),
                **({"ucs": (self.ucs_, False), "namedUcs": (self.named_ucs_, False)} if self.ucs else {}),
                **({"lengthStyle": (self.enum(("architectural", "engineering", "fractional")), False)} if self.imperial else {}),
                **({"namedViews": (self.named_views_, False)} if self.views else {}),
                **({"lineTypes": (self.line_types_, False), "lineTypeScale": (self.line_type_scale_, False)} if self.patterns else {}),
                **({"sources": (self.sources_, False)} if self.sources else {}),
                **({"sourceBases": (self.source_bases_, False)} if self.source_bases else {}),
                **({"wallStyles": (self.wall_styles_, False), "arch": (self.arch_, False)} if self.walls else {}),
                **({"forms": (self.forms_, False), "relations": (self.relations_, False),
                    "domains": (self.domains_, False)} if self.forms else {}),
                **({"hatchPatterns": (self.hatch_patterns_, False)} if self.hatch_extras else {}),
                **({"xrefPaths": (self.xref_paths_, False)} if self.xref_paths else {}),
                **({"joinViews": (self.join_views_, False)} if self.join_views else {}),
                **({"linearEvents": (self.linear_events_, False)} if self.linear_events else {}),
                **({"lineage": (self.lineage_, False)} if self.lineage else {}),
                **({"mapSrid": (self.uint(32), False)} if self.source_crs else {}),
                **({"autoSelect": (self.bool, False)} if self.layer_geometry else {}),
                **({"heights": (self.heights_, False), "coordinateEpoch": (self.epoch_, False)} if self.heights else {}),
            }
        )(v)
        # A second coordinate system is another system than the project's own; a project without one has none (a
        # definition of its own is one, docs/adr/0168 §1).
        has_system = s["srid"] != 0 or "customCrs" in s
        second = s.get("secondSrid")
        if second is not None and (second == 0 or not has_system or second == s["srid"]):
            self.path.append("secondSrid")
            self.fail("bad_value", "ikinci koordinat sistemi projeninkinden başka bir sistem olmalı; yerel projenin ikinci sistemi olmaz")
        # A base is a link's, one a link (docs/adr/0538 §1).
        bases = s.get("sourceBases", [])
        links = {x["id"] for x in s.get("sources", [])}
        seen = set()
        for i, b in enumerate(bases):
            if b["source"] not in links or b["source"] in seen:
                self.path.extend(["sourceBases", str(i)])
                self.fail("bad_value", f"“{b['source']}” bağı yok ya da ikinci tabanı")
            seen.add(b["source"])
        # The map's system is another EPSG system than the project's; a project without one has none (docs/adr/0583 §3).
        if "mapSrid" in s and (s["mapSrid"] == 0 or s["srid"] == 0 or s["mapSrid"] == s["srid"]):
            self.path.append("mapSrid")
            self.fail("bad_value", "harita sistemi projeninkinden başka bir EPSG sistemi olmalı; yerel projenin harita sistemi olmaz")
        # A feet-and-inches style is an inch or foot drawing's (docs/adr/0262 §2).
        if "lengthStyle" in s and s.get("drawingUnit") not in ("in", "ft"):
            self.path.append("lengthStyle")
            self.fail("bad_value", "ayak ve inç biçimi yalnız inç ya da ayak birimli projede olur")
        if "customCrs" in s and s["srid"] != 0:
            self.path.append("customCrs")
            self.fail("bad_value", "projenin kendi tanımı yalnız EPSG kodu olmayan (srid 0) projede olur")
        if "secondCustomCrs" in s and ("secondSrid" in s or not has_system):
            self.path.append("secondCustomCrs")
            self.fail("bad_value", "ikinci sistem ya EPSG kodu ya tanımdır; koordinat sistemi olmayan projenin ikinci sistemi olmaz")
        # Ellipsoidal heights and an epoch rest on a datum (docs/adr/0519 §1).
        if not has_system and s.get("heights", {}).get("kind") == "ellipsoidal":
            self.path.append("heights")
            self.fail("bad_value", "elipsoit yüksekliği bir koordinat sistemi ister")
        if not has_system and "coordinateEpoch" in s:
            self.path.append("coordinateEpoch")
            self.fail("bad_value", "koordinat epoku bir koordinat sistemi ister")
        return s

    def heights_(self, v):
        # What the elevations are (schema 85, docs/adr/0519 §1): ellipsoidal or orthometric; only an orthometric height
        # names a vertical system (one of VERTICAL_SRIDS) and a geoid model (a trimmed name of at most 128 letters, a
        # lower-case 64-digit SHA-256).
        h = self.fields({"kind": (self.text, True), "verticalSrid": (self.uint(32), False),
                         "geoid": (self.fields({"name": (self.text, True), "sha256": (self.text, False)}), False)})(v)
        if h["kind"] not in ("ellipsoidal", "orthometric"):
            self.fail("bad_value", f"“{h['kind']}” bir yükseklik türü değil")
        if h["kind"] != "orthometric" and ("verticalSrid" in h or "geoid" in h):
            self.fail("bad_value", "düşey sistem ve geoit yalnız ortometrik yüksekliğin olur")
        if "verticalSrid" in h and h["verticalSrid"] not in VERTICAL_SRIDS:
            self.fail("bad_value", f"EPSG:{h['verticalSrid']} bilinen düşey sistemlerden değil")
        g = h.get("geoid")
        if g is not None:
            name = g["name"]
            if not name or name != name.strip() or len(name) > 128:
                self.fail("bad_value", "geoit modelinin adı kırpılmış, boş olmayan ve en çok 128 harf olmalı")
            sha = g.get("sha256")
            if sha is not None and not (len(sha) == 64 and all(c in "0123456789abcdef" for c in sha)):
                self.fail("bad_value", "geoit dosyasının özeti küçük harfli 64 onaltılık rakam olmalı")
        return h

    def epoch_(self, v):
        # The coordinates' epoch (schema 85): a decimal year from 1900 to 2100.
        e = self.float(v)
        if not (1900.0 <= e <= 2100.0):
            self.fail("bad_value", "koordinat epoku 1900 ile 2100 arasında olmalı")
        return e

    def xref_paths_(self, v):
        # The external references' search paths (schema 84, docs/adr/0349 §1): not empty, each trimmed, not blank, at most
        # 1024 letters, none twice, at most 64.
        paths = self.array(self.text)(v)
        if not paths:
            self.fail("bad_value", "boş arama yolu listesi yazılmaz")
        if len(paths) > MAX_XREF_PATHS:
            self.fail("bad_value", f"{len(paths)} arama yolu; en çok {MAX_XREF_PATHS}")
        for k, p in enumerate(paths):
            if p != p.strip() or not p or len(p) > MAX_XREF_PATH or p in paths[:k]:
                self.path.append(str(k))
                self.fail("bad_value", "arama yolu kırpılmış, boş olmayan, en çok 1024 harf ve bir kez olmalı")
        return paths

    def annotation_(self, v):
        # The project's annotation heights (schema 30, spec §6.4.6, docs/adr/0205 §1): a kind's height on paper, finite,
        # over 0 and at most 100 mm; not empty.
        h = self.fields({k: (self.float, False) for k in ANNOTATION_KINDS})(v)
        if not h:
            self.fail("bad_value", "yazı yükseklikleri boş; yüksekliği olmayan proje alanı yazmaz")
        for kind in ANNOTATION_KINDS:
            mm = h.get(kind)
            if mm is not None and not (0.0 < mm <= MAX_ANNOTATION_MM):
                self.path.append(kind)
                self.fail("bad_value", f"{kind} yüksekliği {mm:g} mm; sıfırdan büyük, en çok 100 olmalı")
        return h

    def topology_(self, v):
        # The project's topology settings (schema 27, spec §6.4.5, docs/adr/0202 §7): a tolerance within [1e-6, 1] m; rules
        # with an id neither empty nor twice, a known kind, a layer, another layer only between two layers (neither empty
        # nor the rule's own), a value only for a kind that takes one (finite, above zero, an angle below a right
        # angle); exceptions of a rule there is, with objects; not all empty.
        rule = self.fields({"id": (self.text, True), "kind": (self.enum(tuple(TOPOLOGY_KINDS)), True), "layer": (self.text, True),
                            "other": (self.text, False), "value": (self.float, False)})
        def digest(x):
            # Schema 99 (docs/adr/0589 §3): the objects' geometry when marked, 16 lowercase hexadecimal digits.
            d = self.text(x)
            if len(d) != 16 or any(c not in "0123456789abcdef" for c in d):
                self.fail("bad_value", "istisnanın geometri özeti 16 küçük onaltılık rakam değil")
            return d

        exception = self.fields({"at": (self.point, True), "rule": (self.text, True), "objects": (self.array(self.id16), True),
                                 **({"digest": (digest, False)} if self.exception_digests else {})})
        t = self.fields({"rules": (self.array(rule), False), "tolerance": (self.float, False), "exceptions": (self.array(exception), False)})(v)
        rules, exceptions = t.get("rules", []), t.get("exceptions", [])
        if not rules and not exceptions and "tolerance" not in t:
            self.fail("bad_value", "topoloji ayarı boş; ayarı olmayan proje alanı yazmaz")
        if "tolerance" in t and not (1e-6 <= t["tolerance"] <= 1):
            self.fail("bad_value", f"topoloji toleransı {t['tolerance']} m; 0,000001 ile 1 arasında olmalı")
        ids = set()
        for r in rules:
            between, value = TOPOLOGY_KINDS[r["kind"]]
            if not r["id"]:
                self.fail("bad_value", "topoloji kuralının kimliği boş")
            if r["id"] in ids:
                self.fail("bad_value", f"“{r['id']}” kimlikli topoloji kuralı iki kez var")
            ids.add(r["id"])
            if not r["layer"]:
                self.fail("bad_value", f"“{r['id']}” kuralının katmanı boş")
            if between and "other" not in r:
                self.fail("bad_value", f"“{r['id']}” kuralının öbür katmanı yok")
            if between and (not r["other"] or r["other"] == r["layer"]):
                self.fail("bad_value", f"“{r['id']}” kuralının öbür katmanı boş ya da kendi katmanı")
            if not between and "other" in r:
                self.fail("bad_value", f"“{r['id']}” kuralı tek katmanlıdır; öbür katmanı olmaz")
            if "value" in r:
                if value is None:
                    self.fail("bad_value", f"“{r['id']}” kuralı değer almaz")
                if not (r["value"] > 0 and (value != "angle" or r["value"] < math.pi / 2)):
                    self.fail("bad_value", f"“{r['id']}” kuralının değeri {r['value']}; sıfırdan büyük olmalı (açı dik açıdan küçük)")
        for x in exceptions:
            if x["rule"] not in ids:
                self.fail("bad_value", f"istisnanın kuralı “{x['rule']}” yok")
            if not x["objects"]:
                self.fail("bad_value", "istisnanın nesnesi yok")
        return t

    def layer_states_(self, v):
        # The project's named layer states (schema 19, spec §6.4.3, docs/adr/0177 §4): an id and a name, neither empty
        # nor twice (a name as written, the spaces at its ends aside); in each, the nodes, none empty or twice, each
        # with its visibility and, when kept, its lock and a layer's style.
        node = self.fields({"node": (self.text, True), "style": (self.layer_style, False), "locked": (self.bool, False), "visible": (self.bool, True),
                            **({"frozen": (self.bool, False), "plot": (self.bool, False)} if self.frozen else {})})
        states = self.array(self.fields({"id": (self.text, True), "name": (self.text, True), "nodes": (self.array(node), True)}))(v)
        ids, names = set(), set()
        for s in states:
            name = s["name"].strip()
            if not s["id"]:
                self.fail("bad_value", "katman durumunun kimliği boş")
            if s["id"] in ids:
                self.fail("bad_value", f"“{s['id']}” kimlikli katman durumu iki kez var")
            if not name:
                self.fail("bad_value", "katman durumunun adı boş")
            if name in names:
                self.fail("bad_value", f"“{name}” adlı katman durumu iki kez var")
            ids.add(s["id"])
            names.add(name)
            seen = set()
            for n in s["nodes"]:
                if not n["node"]:
                    self.fail("bad_value", f"“{name}” durumunda düğüm kimliği boş")
                if n["node"] in seen:
                    self.fail("bad_value", f"“{name}” durumunda “{n['node']}” düğümü iki kez var")
                seen.add(n["node"])
        return states

    # The named text and dimension styles (schema 21, spec §6.4.4, docs/adr/0183): the tables' rule (an id neither empty
    # nor twice; a name not empty, without spaces at its ends, at most 64 letters, without a line break, not
    # “Standart” and not twice, whatever its letters' case), then each style's values.

    def style_names(self, kind, styles):
        ids, names = set(), set()
        for st in styles:
            sid, name = st["id"], st["name"]
            if not sid:
                self.fail("bad_value", f"{kind} kimliği boş")
            if sid in ids:
                self.fail("bad_value", f"“{sid}” kimlikli {kind} iki kez var")
            trimmed = name.strip()
            if not trimmed:
                self.fail("bad_value", f"{kind} adı boş")
            if trimmed != name:
                self.fail("bad_value", f"“{name}” {kind} adının başında ya da sonunda boşluk var")
            if len(trimmed) > MAX_STYLE_NAME:
                self.fail("bad_value", f"“{trimmed}” {kind} adı {MAX_STYLE_NAME} harften uzun")
            if any(ord(c) < 32 or 0x7F <= ord(c) <= 0x9F for c in trimmed):
                self.fail("bad_value", f"“{trimmed}” {kind} adında satır sonu ya da denetim karakteri var")
            folded = trimmed.lower()
            if folded == "standart":
                self.fail("bad_value", f"“{trimmed}” adı Standart'ındır; {kind} başka bir ad almalı")
            if folded in names:
                self.fail("bad_value", f"“{trimmed}” adlı {kind} iki kez var")
            ids.add(sid)
            names.add(folded)

    def style_flag(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "false yazılmaz; alan yoksa stil öyle değildir")
        return True

    def text_styles(self, v):
        styles = self.array(
            self.fields(
                {
                    "id": (self.text, True),
                    "name": (self.text, True),
                    "font": (self.enum(DRAWING_FONTS), True),
                    "bold": (self.style_flag, False),
                    "italic": (self.style_flag, False),
                    "oblique": (self.float, False),
                    "height": (self.float, False),
                    "widthFactor": (self.float, False),
                    "fontFile": (self.text, False),
                }
            )
        )(v)
        self.style_names("yazı stili", styles)
        for st in styles:
            name = st["name"]
            o = st.get("oblique")
            if o is not None and not (o != 0.0 and abs(o) < MAX_OBLIQUE):
                self.fail("bad_value", f"“{name}” yazı stilinin eğikliği {o:g}; −85 ile 85 arasında ve sıfırdan farklı olmalı")
            h = st.get("height")
            if h is not None and not 0.0 < h <= MAX_STYLE_MM:
                self.fail("bad_value", f"“{name}” yazı stilinin yüksekliği {h:g} mm; sıfırdan büyük, en çok 1000 olmalı")
            w = st.get("widthFactor")
            if w is not None and not (0.0 < w <= MAX_WIDTH_FACTOR and w != 1.0):
                self.fail("bad_value", f"“{name}” yazı stilinin genişlik çarpanı {w:g}; sıfırdan büyük, en çok 100 ve 1'den farklı olmalı (1 yazılmaz)")
            if st.get("fontFile") == "":
                self.fail("bad_value", f"“{name}” yazı stilinin yazı tipi dosyası boş")
        return styles

    def dimension_styles(self, v):
        styles = self.array(
            self.fields(
                {
                    "id": (self.text, True),
                    "name": (self.text, True),
                    "height": (self.float, True),
                    "arrow": (self.enum(DIMENSION_ARROWS), False),
                    "arrowSize": (self.float, False),
                    "extOffset": (self.float, False),
                    "extBeyond": (self.float, False),
                    "textGap": (self.float, False),
                    "textPlace": (self.enum(("centre",)), False),
                    "decimals": (self.uint(32), False),
                    "unit": (self.enum(("mm", "cm", "m")), False),
                    "prefix": (self.text, False),
                    "suffix": (self.text, False),
                    "font": (self.enum(DRAWING_FONTS), False),
                    # Schema 61 (docs/adr/0321): its options.
                    **({"options": (self.dimension_options, False)} if self.dim_options else {}),
                    # Schema 30 (docs/adr/0205 §6): its lines.
                    **(
                        {
                            "dimLineColor": (self.text, False),
                            "dimLineWeight": (self.float, False),
                            "dimLineType": (self.enum(LINE_TYPES), False),
                            "extColor": (self.text, False),
                            "extWeight": (self.float, False),
                            "extLineType": (self.enum(LINE_TYPES), False),
                            "textColor": (self.text, False),
                        }
                        if self.annotation
                        else {}
                    ),
                }
            )
        )(v)
        self.style_names("ölçü stili", styles)
        for st in styles:
            name, h = st["name"], st["height"]
            if not 0.0 < h <= MAX_STYLE_MM:
                self.fail("bad_value", f"“{name}” ölçü stilinin değer yüksekliği {h:g} mm; sıfırdan büyük, en çok 1000 olmalı")
            for key, positive in (("arrowSize", True), ("extOffset", False), ("extBeyond", False), ("textGap", False)):
                x = st.get(key)
                if x is not None and not ((x > 0.0 if positive else x >= 0.0) and x <= MAX_STYLE_MM and x / h <= MAX_DIMENSION_RATIO):
                    self.fail("bad_value", f"“{name}” ölçü stilinin {key} değeri {x:g} mm; sınırların dışında")
            if st.get("decimals", 0) > MAX_DIMENSION_DECIMALS:
                self.fail("bad_value", f"“{name}” ölçü stilinin basamak sayısı {st['decimals']}; en çok 8 olmalı")
            for key in ("prefix", "suffix"):
                if key in st:
                    self.affix_rule(st[key], key)
            for key in ("dimLineColor", "extColor", "textColor"):
                if key in st:
                    self.colour_rule(st[key], key)
            for key in ("dimLineWeight", "extWeight"):
                if key in st:
                    self.weight_rule(st[key], key)
        return styles

    # A dimension's lines (schema 30, docs/adr/0205 §6): colours #RRGGBB, weights 0 to 100 paper mm.

    def colour_rule(self, c, key):
        if not (len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:])):
            self.fail("bad_value", f"ölçünün {key} değeri “{c}”; #RRGGBB olmalı")

    def weight_rule(self, w, key):
        if not 0.0 <= w <= MAX_LINE_WEIGHT:
            self.fail("bad_value", f"ölçünün {key} değeri {w:g}; 0 ile 100 mm arasında olmalı")

    def dimension_colour(self, key):
        def read(v):
            c = self.text(v)
            self.colour_rule(c, key)
            return c

        return read

    def dimension_weight(self, key):
        def read(v):
            w = self.float(v)
            self.weight_rule(w, key)
            return w

        return read

    def leader_arrow_size(self, v):
        x = self.float(v)
        if not MIN_LEADER_ARROW <= x <= MAX_LEADER_ARROW:
            self.fail("bad_value", f"kılavuzun ok boyu {x:g}; notun yüksekliğinin 0,1 ile 10 katı olmalı")
        return x

    def affix_rule(self, text, key):
        if not text or len(text) > MAX_AFFIX or any(ord(c) < 32 or 0x7F <= ord(c) <= 0x9F for c in text):
            self.fail("bad_value", f"ölçünün {key} değeri yazılamaz: boş olmamalı, en çok 32 harf, satır sonu ya da denetim karakteri yok")

    # A text's face and a dimension's look (schema 21, spec §6.6, docs/adr/0183).

    def style_id(self, v):
        t = self.text(v)
        if not t:
            self.fail("bad_value", "stil kimliği boş")
        return t

    def oblique(self, v):
        x = self.float(v)
        if not (x != 0.0 and abs(x) < MAX_OBLIQUE):
            self.fail("bad_value", f"yazının eğikliği {x:g}; −85 ile 85 arasında ve sıfırdan farklı olmalı")
        return x

    def face_flag(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "false yazılmaz; alan yoksa yazı öyle değildir")
        return True

    def size(self, positive):
        def read(v):
            x = self.float(v)
            if not ((x > 0.0 if positive else x >= 0.0) and x <= MAX_DIMENSION_RATIO):
                self.fail("bad_value", f"ölçünün boyu {x:g}; {'sıfırdan büyük' if positive else '0 ya da büyük'} ve en çok 100 olmalı")
            return x

        return read

    def decimals(self, v):
        n = self.uint(32)(v)
        if n > MAX_DIMENSION_DECIMALS:
            self.fail("bad_value", f"ölçünün basamak sayısı {n}; en çok 8 olmalı")
        return n

    def affix(self, key):
        def read(v):
            t = self.text(v)
            self.affix_rule(t, key)
            return t

        return read

    def survey(self, v):
        # The project's survey constants and tolerances (schema 14, spec §6.4.2, docs/adr/0169 §3): at least one, k
        # within [−1, 1], tolerances above zero; schema 16's ground height within [−500, 9000] m, the reduction to the
        # grid only with one (docs/adr/0171); schema 28's a priori standard deviations above zero, the parts per million
        # and the centering not below (docs/adr/0203 §1).
        table = {"index": (self.float, False), "faceHz": (self.float, False), "faceSlope": (self.float, False), "refraction": (self.float, False)}
        if self.traverse_tolerances:
            # Schema 15: the traverse tolerances.
            table.update({"twoWay": (self.float, False), "traverseAngle": (self.float, False), "traverseCoord": (self.float, False)})
        if self.ground:
            # Schema 16: the mean ellipsoidal height and the reduction to the grid (docs/adr/0171).
            table.update({"groundHeight": (self.float, False), "reduceToGrid": (self.bool, False)})
        if self.survey_sigmas:
            # Schema 28: the a priori standard deviations of a network adjustment (docs/adr/0203 §1).
            table.update({k: (self.float, False) for k in ("sigmaDirection", "sigmaDistance", "sigmaPpm", "sigmaCentering", "sigmaZenith", "sigmaLevelling")})
        s = self.fields(table)(v)
        if not s:
            self.fail("bad_value", "ölçme ayarları boş; ayarı olmayan proje alanı yazmaz")
        k = s.get("refraction")
        if k is not None and not -1.0 <= k <= 1.0:
            self.fail("bad_value", f"kırılma katsayısı k {k}; −1 ile 1 arasında olmalı")
        h = s.get("groundHeight")
        if h is not None and not -500.0 <= h <= 9000.0:
            self.fail("bad_value", f"ortalama elipsoit yüksekliği {h} m; −500 ile 9000 arasında olmalı")
        if s.get("reduceToGrid") is True and h is None:
            self.fail("bad_value", "uzunlukları projeksiyona indirmek ortalama elipsoit yüksekliği ister")
        for key in ("faceHz", "index", "faceSlope", "twoWay", "traverseAngle", "traverseCoord"):
            if key in s and not s[key] > 0.0:
                self.fail("bad_value", f"{key} toleransı {s[key]}; sıfırdan büyük olmalı")
        for key in ("sigmaDirection", "sigmaDistance", "sigmaZenith", "sigmaLevelling"):
            if key in s and not s[key] > 0.0:
                self.fail("bad_value", f"{key} önsel doğruluğu {s[key]}; sıfırdan büyük olmalı")
        for key in ("sigmaPpm", "sigmaCentering"):
            if key in s and not s[key] >= 0.0:
                self.fail("bad_value", f"{key} {s[key]}; sıfırdan küçük olamaz")
        return s

    def source(self, v):
        s = self.fields({"format": (self.text, True), "version": (self.uint(32), True), "sourceSha256": (self.sha256, True)})(v)
        if s["format"] != DOCUMENT_FORMAT or s["version"] != 1:
            self.fail("bad_value", "göç kaynağı yalnız kentos.document sürüm 1 olabilir")
        return s

    def layer(self, v):
        n = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                **({"snap": (self.layer_snap_, False)} if self.layer_snap else {}),
                **({"fields": (self.layer_fields_, False)} if self.layer_fields else {}),
                **({"service": (self.service_, False), "feed": (self.feed_, False)} if self.services else {}),
                **({"time": (self.layer_time_, False), "scenario": (self.scenario_, False), "replaces": (self.text, False)} if self.temporal else {}),
                **({"filter": (self.layer_filter_, False)} if self.filters else {}),
                **({"actions": (self.layer_actions_, False), "card": (self.info_card_, False)} if self.attachments else {}),
                **({"compose": (self.layer_compose_, False)} if self.compose else {}),
                **({"plot": (self.only_false, False)} if self.look else {}),
                **({"frozen": (self.only_true, False)} if self.frozen else {}),
                # Schema 60 (docs/adr/0413 §3): the attribute table's rules.
                **({"tableStyles": (self.table_styles_, False)} if self.forms else {}),
                # Schema 60 (docs/adr/0422 §4): the subtypes.
                **({"subtypes": (self.subtypes_, False)} if self.forms else {}),
                # Schema 60 (docs/adr/0423 §4): the value groups.
                **({"contingents": (self.contingents_, False)} if self.forms else {}),
                # Schema 60 (docs/adr/0438 §1): the record rules.
                **({"rules": (self.layer_rules_, False)} if self.forms else {}),
                # Schema 98 (docs/adr/0583 §1): the system its objects came from.
                **({"sourceCrs": (self.source_crs_, False)} if self.source_crs else {}),
                # Schema 106 (docs/adr/0633 §3): the layer as a dataset and what it was derived from.
                **({"dataset": (self.dataset_, False), "derivation": (self.derivation_, False)} if self.datasets else {}),
                # Schema 102 (docs/adr/0622 §5): what the layer holds.
                **({"geometry": (self.enum(("point", "line", "area", "annotation")), False)} if self.layer_geometry else {}),
                "type": (self.enum(("group", "layer")), True),
                "style": (self.layer_style, True),
                "locked": (self.bool, True),
                "visible": (self.bool, True),
                "children": (self.array(self.layer), True),
                "expanded": (self.bool, True),
            }
        )(v)
        if "snap" in n and n["type"] == "group":
            self.fail("bad_value", "grubun keneti olmaz; kenet yalnız katmanındır")
        if "fields" in n and n["type"] == "group":
            self.fail("bad_value", "grubun alanları olmaz; alanlar yalnız katmanındır")
        if "service" in n and "feed" in n:
            self.fail("bad_value", "katman hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz")
        if "service" in n and n["type"] == "group":
            self.fail("bad_value", "grubun servisi olmaz; servis yalnız katmanındır")
        if "feed" in n and n["type"] == "group":
            self.fail("bad_value", "grubun veri kaynağı olmaz; kaynak yalnız katmanındır")
        if "time" in n and n["type"] == "group":
            self.fail("bad_value", "grubun zamanı olmaz; zaman yalnız katmanındır")
        if "scenario" in n and n["type"] == "layer":
            self.fail("bad_value", "katman senaryo olmaz; yalnız grup senaryodur")
        if "replaces" in n and n["type"] == "group":
            self.fail("bad_value", "grup bir katmanın yerine geçmez; yalnız katman geçer")
        if "filter" in n and n["type"] == "group":
            self.fail("bad_value", "grubun süzgeci olmaz; süzgeç yalnız katmanındır")
        if "actions" in n and n["type"] == "group":
            self.fail("bad_value", "grubun eylemi olmaz; eylemler yalnız katmanındır")
        if "card" in n and n["type"] == "group":
            self.fail("bad_value", "grubun bilgi kartı olmaz; kart yalnız katmanındır")
        if "compose" in n and n["type"] == "group":
            self.fail("bad_value", "grubun katman çizimi olmaz; katman çizimi yalnız katmanındır")
        if "tableStyles" in n and n["type"] == "group":
            self.fail("bad_value", "grubun koşullu biçimi olmaz; yalnız katmanın olur")
        if ("dataset" in n or "derivation" in n) and n["type"] == "group":
            self.fail("bad_value", "grubun veri kümesi bilgisi ve türetim kaydı olmaz; yalnız katmanın olur")
        if "sourceCrs" in n and n["type"] == "group":
            self.fail("bad_value", "grubun kaynak sistemi olmaz; yalnız katmanın olur")
        if "geometry" in n and n["type"] == "group":
            self.fail("bad_value", "grubun türü olmaz; nokta, çizgi, alan ya da açıklama yalnız katmanın türüdür")
        if "contingents" in n:
            if n["type"] == "group":
                self.fail("bad_value", "grubun değer bileşimleri olmaz; yalnız katmanın olur")
            problem = contingents_problem(n.get("fields", []), n.get("subtypes"), n["contingents"])
            if problem:
                self.fail("bad_value", problem)
        if "rules" in n:
            if n["type"] == "group":
                self.fail("bad_value", "grubun kayıt kuralları olmaz; yalnız katmanın olur")
            problem = layer_rules_problem(n.get("fields", []), n["rules"])
            if problem:
                self.fail("bad_value", problem)
        if "subtypes" in n:
            if n["type"] == "group":
                self.fail("bad_value", "grubun alt türleri olmaz; yalnız katmanın olur")
            problem = subtypes_problem(n.get("fields", []), n["subtypes"])
            if problem:
                self.fail("bad_value", problem)
        return n

    # Schema 60's record rules (docs/adr/0438 §1): rule by rule; `warning` only true, the list not empty.

    def layer_rules_(self, v):
        def only_true(x):
            if self.bool(x) is not True:
                self.fail("bad_value", "warning yalnız true yazılır")
            return True

        def nonempty_texts(x):
            out = self.array(self.text)(x)
            if not out:
                self.fail("bad_value", "boş alan listesi yazılmaz")
            return out

        rule = self.fields({"id": (self.text, True), "name": (self.text, True), "rule": (self.text, True),
                            "message": (self.text, True), "warning": (only_true, False),
                            "fields": (nonempty_texts, False)})
        out = self.array(rule)(v)
        if not out:
            self.fail("bad_value", "boş liste yazılmaz; kural yoksa alan hiç yazılmaz")
        return out

    # Schema 60's value groups (docs/adr/0423 §1): field by field; `restrictive` only true, the list not empty.

    def contingents_(self, v):
        def only_true(x):
            if self.bool(x) is not True:
                self.fail("bad_value", "restrictive yalnız true yazılır")
            return True

        def cell(x):
            c = self.fields({"kind": (self.enum(("any", "empty", "code")), True), "code": (self.text, False)})(x)
            if (c["kind"] == "code") != ("code" in c):
                self.fail("bad_value", "kod yalnız code hücresinde yazılır, orada zorunludur")
            return c

        row = self.fields({"cells": (self.array(cell), True), "subtype": (self.text, False)})
        group = self.fields({"name": (self.text, True), "fields": (self.array(self.text), True),
                             "restrictive": (only_true, False), "rows": (self.array(row), True)})
        out = self.array(group)(v)
        if not out:
            self.fail("bad_value", "boş liste yazılmaz; grup yoksa alan hiç yazılmaz")
        return out

    # Schema 60's subtypes (docs/adr/0422 §1): field by field; `required` only true, the label and the lists not
    # written empty. Their fit to the layer's fields is checked with the layer.

    def dataset_(self, v):
        """Schema 106 (docs/adr/0633 §1): a layer's description as a dataset; an absent part is not written, nothing
        empty is."""
        def line(x):
            t = self.text(x)
            if not (1 <= len(t) <= 200) or t.strip() != t or any(unicodedata.category(c) == "Cc" for c in t):
                self.fail("bad_value", "satır boş, baş ya da sonda boşluklu, 200 karakterden uzun ya da denetim karakterli")
            return t

        def long_text(x):
            t = self.text(x)
            if not (1 <= len(t) <= 10000) or not t.strip():
                self.fail("bad_value", "metin boş ya da 10000 karakterden uzun")
            return t

        def email(x):
            t = line(x)
            user, at, host = t.partition("@")
            if not at or not user or "." not in host or host.startswith(".") or host.endswith(".") or any(c.isspace() for c in t):
                self.fail("bad_value", f"“{t}” bir e-posta adresi değil")
            return t

        def party(x):
            p = self.fields({"name": (line, False), "organisation": (line, False), "email": (email, False)})(x)
            if not p:
                self.fail("bad_value", "sorumlunun adı, kurumu ya da e-postası yazılmalı")
            return p

        def keywords(x):
            lst = self.array(line)(x)
            if not lst or len(lst) > 50 or len(set(lst)) != len(lst):
                self.fail("bad_value", "anahtar sözcükler boş, 50'den çok ya da yinelenmiş")
            return lst

        def day(x):
            t = self.text(x)
            if not _is_date(t):
                self.fail("bad_value", f"“{t}” bir gün değil (YYYY-AA-GG)")
            return t

        def scale(x):
            n = self.uint(32)(x)
            if n == 0:
                self.fail("bad_value", "ölçeğin paydası sıfırdan büyük olur")
            return n

        def metres(x):
            f = self.float(x)
            if not (math.isfinite(f) and f > 0):
                self.fail("bad_value", "değer sıfırdan büyük bir metre olur")
            return f

        d = self.fields({
            "title": (line, False), "description": (long_text, False), "responsible": (party, False),
            "license": (line, False), "constraints": (long_text, False), "keywords": (keywords, False),
            "updated": (day, False), "scale": (scale, False), "resolution": (metres, False),
            "accuracy": (metres, False), "completeness": (long_text, False), "statement": (long_text, False),
        })(v)
        if not d:
            self.fail("bad_value", "boş veri kümesi bilgisi yazılmaz")
        return d

    def derivation_(self, v):
        """Schema 106 (docs/adr/0633 §2): what a processing tool made the layer from."""
        def hex64(x):
            t = self.text(x)
            if len(t) != 64 or any(c not in "0123456789abcdef" for c in t):
                self.fail("bad_value", "özet 64 haneli küçük harfli SHA-256 olur")
            return t

        def nonempty(x):
            t = self.text(x)
            if not t:
                self.fail("bad_value", "boş metin yazılmaz")
            return t

        def revision(x):
            t = nonempty(x)
            if not t.isdigit():
                self.fail("bad_value", "çizim sürümü bir sayı olur")
            return t

        def inputs(x):
            lst = self.array(self.fields({"layer": (nonempty, True), "name": (self.text, True),
                                          "digest": (hex64, True), "source": (nonempty, False)}))(x)
            if not lst or len(lst) > 200 or len({i["layer"] for i in lst}) != len(lst):
                self.fail("bad_value", "girdiler boş, 200'den çok ya da bir katmanı iki kez anıyor")
            return lst

        def parameters(x):
            if type(x) is not dict or not x:
                self.fail("bad_value", "değerler boş olmayan bir harita olur")
            return self.any(x)

        return self.fields({
            "tool": (nonempty, True), "label": (nonempty, True), "toolVersion": (self.uint(32), True),
            "parameters": (parameters, False), "inputs": (inputs, False), "documentRevision": (revision, True),
            "engine": (nonempty, True), "at": (nonempty, True),
        })(v)

    def source_crs_(self, v):
        """Schema 98 (docs/adr/0583 §1): the system a layer's objects came from; `srid` and `area` written only when
        known."""
        def nonempty(x):
            t = self.text(x)
            if t.strip() == "":
                self.fail("bad_value", "boş metin yazılmaz")
            return t

        def axis(x):
            if x not in ("en", "ne", "latlon"):
                self.fail("bad_value", f"eksen sırası “{x}” olamaz: en, ne ya da latlon")
            return x

        def unit(x):
            if x not in ("metre", "degree"):
                self.fail("bad_value", f"birim “{x}” olamaz: metre ya da degree")
            return x

        def srid(x):
            n = self.uint(32)(x)
            if n == 0:
                self.fail("bad_value", "yerel sistemin EPSG kodu yoktur; srid yazılmaz")
            return n

        def wkt(x):
            t = nonempty(x)
            if len(t) > 16384:
                self.fail("bad_value", "WKT 16384 karakterden uzun")
            return t

        return self.fields({"srid": (srid, False), "name": (nonempty, True), "wkt": (wkt, True),
                            "axisOrder": (axis, True), "unit": (unit, True), "area": (self.text, False)})(v)

    def subtypes_(self, v):
        def only_true(x):
            if self.bool(x) is not True:
                self.fail("bad_value", "required yalnız true yazılır")
            return True

        def label(x):
            t = self.text(x)
            if t == "":
                self.fail("bad_value", "boş ad yazılmaz; ad yoksa alan hiç yazılmaz")
            return t

        rule = self.fields({"field": (self.text, True), "values": (self.array(self.text), False),
                            "default": (self.text, False), "required": (only_true, False)})

        def rules(x):
            out = self.array(rule)(x)
            if not out:
                self.fail("bad_value", "boş liste yazılmaz; kural yoksa alan hiç yazılmaz")
            return out

        kind = self.fields({"code": (self.text, True), "label": (label, False), "fields": (rules, False)})
        return self.fields({"field": (self.text, True), "types": (self.array(kind), True)})(v)

    # Schema 60's table rules (docs/adr/0413 §1): field by field, then the rules' rule; `bold` only true, the list not
    # empty, at most 50.

    def table_styles_(self, v):
        def only_true(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "bold yalnız true yazılır")
            return True

        rule = self.fields({"field": (self.text, False), "condition": (self.text, True), "background": (self.text, False),
                            "color": (self.text, False), "bold": (only_true, False)})
        rules = self.array(rule)(v)
        if not rules:
            self.fail("bad_value", "boş liste yazılmaz; kural yoksa alan hiç yazılmaz")
        if len(rules) > 50:
            self.fail("bad_value", f"Bir katmanın tablosunda en çok 50 biçim kuralı olur; {len(rules)} verildi.")
        hexed = lambda c: len(c) == 7 and c[0] == "#" and all(x in "0123456789abcdefABCDEF" for x in c[1:])  # noqa: E731
        for i, r in enumerate(rules, 1):
            if not r["condition"].strip():
                self.fail("bad_value", f"{i}. kuralın koşulu boş.")
            if len(r["condition"]) > 2000:
                self.fail("bad_value", f"{i}. kuralın koşulu en çok 2000 karakter olabilir.")
            if "field" in r and not r["field"].strip():
                self.fail("bad_value", f"{i}. kuralın alanı boş; satır için alanı vermeyin.")
            for key, word in (("background", "zemin"), ("color", "yazı")):
                if key in r and not hexed(r[key]):
                    self.fail("bad_value", f"{i}. kuralın {word} rengi “{r[key]}” #RRGGBB biçiminde değil.")
            if "background" not in r and "color" not in r and not r.get("bold"):
                self.fail("bad_value", f"{i}. kural hiçbir şey vermiyor: zemin rengi, yazı rengi ya da kalınlık verin.")
        return rules

    # Schema 41's composition (docs/adr/0226 §3, §4): read field by field, then checked by the contract's rule; what says
    # nothing (an opacity of 1, the normal mode, a mask's switch off) is not written.

    def layer_compose_(self, v):
        def opacity(v):
            o = self.float(v)
            if o == 1.0:
                self.fail("bad_value", "1 saydamlığı yazılmaz; katman opaksa alan boş bırakılır")
            return o

        def blend(v):
            name = self.text(v)
            if name == "normal":
                self.fail("bad_value", "normal karışım kipi yazılmaz; kip normalse alan boş bırakılır")
            if name not in BLEND_MODES:
                self.fail("bad_value", f"bilinmeyen karışım kipi “{name}”")
            return name

        def switch(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "maskenin kaynağı yalnız true yazılır; kapalıysa yazılmaz")
            return True

        mask = self.fields({"size": (self.float, True), "labels": (switch, False), "markers": (switch, False), "targets": (self.array(self.text), True)})
        c = self.fields({"mask": (mask, False), "blend": (blend, False), "opacity": (opacity, False)})(v)
        p = compose_problem(c)
        if p:
            self.fail("bad_value", p)
        return c

    # Schema 40's actions, card and attachments (docs/adr/0225): read field by field, then checked by the contract's rules;
    # an empty list and a card's switch written on are not written.

    def layer_actions_(self, v):
        xs = self.array(self.fields({"name": (self.text, True), "target": (self.text, True)}))(v)
        if not xs:
            self.fail("bad_value", "katmanın eylem listesi boş; boş liste yazılmaz")
        p = actions_problem(xs)
        if p:
            self.fail("bad_value", p)
        return xs

    def info_card_(self, v):
        c = self.fields(
            {"text": (self.text, False), "title": (self.text, False), "fields": (self.array(self.text), False), "actions": (self.bool, False), "attachments": (self.bool, False)}
        )(v)
        if "fields" in c and not c["fields"]:
            self.fail("bad_value", "kartın alan listesi boş; boş liste yazılmaz")
        if c.get("actions") is True or c.get("attachments") is True:
            self.fail("bad_value", "kartın düğmesi açıkken yazılmaz; yalnız kapalısı (false) yazılır")
        p = card_problem(c)
        if p:
            self.fail("bad_value", p)
        return c

    def attachments_(self, v):
        xs = self.array(self.fields({"url": (self.text, False), "file": (self.text, False), "name": (self.text, True), "note": (self.text, False), "asset": (self.text, False)}))(v)
        if not xs:
            self.fail("bad_value", "nesnenin ek listesi boş; boş liste yazılmaz")
        p = attachments_problem(xs)
        if p:
            self.fail("bad_value", p)
        return xs

    # Schema 35's layer filter (docs/adr/0211 §2): read field by field, then checked by the contract's rule.

    def layer_filter_(self, v):
        f = self.fields({"objects": (self.array(self.id16), False), "expression": (self.text, False)})(v)
        p = layer_filter_problem(f)
        if p:
            self.fail("bad_value", p)
        return f

    # Schema 34's time settings and scenarios (docs/adr/0210 §2): read field by field, then checked by the contract's rules.

    def layer_time_(self, v):
        t = self.fields({"start": (self.text, True), "end": (self.text, False), "key": (self.text, False), "cumulative": (self.only_true, False),
                         **({"recordedStart": (self.text, False), "recordedEnd": (self.text, False)} if self.recorded_time else {})})(v)
        p = layer_time_problem(t)
        if p:
            self.fail("bad_value", p)
        return t

    def scenario_(self, v):
        s = self.fields({"note": (self.text, False)})(v)
        p = scenario_problem(s)
        if p:
            self.fail("bad_value", p)
        return s

    # Schema 32's map services (docs/adr/0208 §2): read field by field, then checked whole by the contract's rules.

    def nonempty_texts(self, v):
        texts = self.array(self.text)(v)
        if not texts:
            self.fail("bad_value", "boş liste yazılmaz")
        return texts

    def only_false(self, v):
        if self.bool(v) is not False:
            self.fail("bad_value", "plot yalnız false yazılır; yazdırılan katmanda alan yoktur")
        return False

    def only_true(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "false yazılmaz; alan yalnız true iken vardır")
        return True

    def tile_grid(self, v):
        matrix = self.fields(
            {
                "id": (self.text, True),
                "x0": (self.float, True),
                "y0": (self.float, True),
                "resolution": (self.float, True),
                "tileWidth": (self.uint(32), True),
                "tileHeight": (self.uint(32), True),
                "matrixWidth": (self.uint(64), True),
                "matrixHeight": (self.uint(64), True),
            }
        )
        return self.fields({"srid": (self.uint(32), True), "matrices": (self.array(matrix), True)})(v)

    def service_(self, v):
        s = self.fields(
            {
                "kind": (self.enum(SERVICE_KINDS), True),
                "url": (self.text, True),
                "layers": (self.nonempty_texts, False),
                "style": (self.text, False),
                "format": (self.text, False),
                "srid": (self.uint(32), False),
                "grid": (self.tile_grid, False),
                "matrixSet": (self.text, False),
                "template": (self.text, False),
                "tileSize": (self.uint(32), False),
                "minZoom": (self.uint(32), False),
                "maxZoom": (self.uint(32), False),
                "subdomains": (self.nonempty_texts, False),
                "yFlip": (self.only_true, False),
                "transparent": (self.only_true, False),
                "version": (self.text, False),
                "params": (self.array(self.fields({"name": (self.text, True), "value": (self.text, True)})), False),
                "dynamic": (self.only_true, False),
                "attribution": (self.text, False),
                "opacity": (self.float, False),
                "connection": (self.text, False),
                "preset": (self.text, False),
                "bbox": (self.array(self.float), False),
            }
        )(v)
        if "params" in s and not s["params"]:
            self.fail("bad_value", "boş parametre listesi yazılmaz")
        if "bbox" in s and len(s["bbox"]) != 4:
            self.fail("bad_value", f"servisin kapsamı 4 sayı olmalı, {len(s['bbox'])} var")
        problem = service_problem(s)
        if problem:
            self.fail("bad_value", problem)
        return s

    def feed_(self, v):
        f = self.fields(
            {
                "kind": (self.enum(FEED_KINDS), True),
                "url": (self.text, True),
                "name": (self.text, False),
                "srid": (self.uint(32), False),
                "filter": (self.text, False),
                "bbox": (self.array(self.float), False),
                "limit": (self.uint(64), False),
                "version": (self.text, False),
                "key": (self.text, False),
                "connection": (self.text, False),
                "fetched": (self.text, False),
            }
        )(v)
        if "bbox" in f and len(f["bbox"]) != 4:
            self.fail("bad_value", f"istenen alan 4 sayı olmalı, {len(f['bbox'])} var")
        problem = feed_problem(f)
        if problem:
            self.fail("bad_value", problem)
        return f

    def connections_(self, v):
        conn = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                "origin": (self.text, True),
                "auth": (self.enum(AUTH_KINDS), True),
                "names": (self.nonempty_texts, False),
                "tokenUrl": (self.text, False),
                "scope": (self.text, False),
            }
        )
        all_ = self.array(conn)(v)
        if not all_:
            self.fail("bad_value", "boş bağlantı listesi yazılmaz")
        problem = connections_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def address_locators_(self, v):
        """The project's address data (schema 42, docs/adr/0229 §2): read field by field, an area list and a field's
        name never empty; then checked whole."""

        def field(v):
            t = self.text(v)
            if not t:
                self.fail("bad_value", "boş alan adı yazılmaz")
            return t

        optional = ("neighbourhood", "district", "province", "postcode", "building", "code")
        doors = self.fields({"layer": (self.text, True), "street": (field, True), "door": (field, True), **{k: (field, False) for k in optional}})
        streets = self.fields({"layer": (self.text, True), "name": (field, True), "neighbourhood": (field, False), "district": (field, False)})
        area = self.fields({"layer": (self.text, True), "level": (self.enum(AREA_LEVELS), True), "name": (field, True), "district": (field, False)})

        def areas(v):
            all_ = self.array(area)(v)
            if not all_:
                self.fail("bad_value", "boş bölge listesi yazılmaz")
            return all_

        locator = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                "doors": (doors, False),
                "streets": (streets, False),
                "areas": (areas, False),
                "minScore": (self.uint(32), True),
                "reach": (self.float, True),
            }
        )
        all_ = self.array(locator)(v)
        if not all_:
            self.fail("bad_value", "boş adres verisi listesi yazılmaz")
        problem = address_locators_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def networks_(self, v):
        """The project's networks (schema 33, docs/adr/0209 §2): read field by field, lists only when not empty, a
        direction's field and values only with a field direction; then checked whole. A field direction's lists come
        back as the contract's JSON form writes them: all three, an empty one too."""

        def direction(v):
            d = self.fields(
                {
                    "kind": (self.enum(DIRECTION_KINDS), True),
                    "field": (self.text, False),
                    "forward": (self.nonempty_texts, False),
                    "backward": (self.nonempty_texts, False),
                    "closed": (self.nonempty_texts, False),
                }
            )(v)
            if d["kind"] != "field":
                if len(d) > 1:
                    self.fail("bad_value", "yönün alanı ve değerleri yalnız alanla yönde yazılır")
                return d
            if "field" not in d:
                self.fail("missing_field", "alanla yönün alanı yok")
            return {"kind": "field", "field": d["field"], "forward": d.get("forward", []), "backward": d.get("backward", []), "closed": d.get("closed", [])}

        def nonempty(item, words):
            def read(v):
                all_ = self.array(item)(v)
                if not all_:
                    self.fail("bad_value", words)
                return all_

            return read

        def unit(v):
            u = self.text(v)
            if not u:
                self.fail("bad_value", "boş birim yazılmaz")
            return u

        edge = self.fields({"layer": (self.text, True), "filter": (self.text, False)})
        junction = self.fields(
            {"layer": (self.text, True), "role": (self.enum(JUNCTION_ROLES), True), "filter": (self.text, False), "closed": (self.text, False)}
        )
        def delay(v):
            d = self.float(v)
            if d == 0.0:
                self.fail("bad_value", "sıfır gecikme yazılmaz")
            return d

        period = self.fields(
            {"from": (self.uint(32), True), "to": (self.uint(32), True), "factor": (self.float, False), "field": (self.text, False)}
        )
        turn_layer = self.fields({"layer": (self.text, True), "filter": (self.text, False), "field": (self.text, False)})
        turned = self.network_turns
        cost = self.fields(
            {
                "name": (self.text, True),
                "kind": (self.enum(NETWORK_COST_KINDS), True),
                "field": (self.text, True),
                "unit": (unit, False),
                "speed": (self.float, False),
                **({"periods": (nonempty(period, "boş dönem listesi yazılmaz"), False)} if turned else {}),
            }
        )
        turns = self.fields(
            {
                "uTurns": (self.enum(U_TURNS), True),
                "straight": (delay, False),
                "right": (delay, False),
                "left": (delay, False),
                "back": (delay, False),
                "layers": (nonempty(turn_layer, "boş dönüş katmanı listesi yazılmaz"), False),
            }
        )
        levels = self.fields({"start": (self.text, True), "end": (self.text, True), "inside": (self.text, False)})
        terminal = self.fields({"id": (self.text, True), "name": (self.text, True)})
        config = self.fields({"value": (self.text, True), "terminals": (self.array(terminal), True),
                              "paths": (self.array(self.array(self.text)), True)})
        device = self.fields({"layer": (self.text, True), "terminals": (self.array(terminal), True),
                              "paths": (self.array(self.array(self.text)), True),
                              **({"by": (self.text, False), "configs": (nonempty(config, "boş yapılandırma listesi yazılmaz"), False)}
                                 if self.network_configs else {})})
        terminals = self.fields({"start": (self.text, True), "end": (self.text, True), "devices": (self.array(device), True)})
        subnetworks = self.fields({"name": (self.text, False), "attribute": (self.text, True),
                                   "reach": (self.enum(SUBNETWORK_REACHES), True),
                                   **({"keepCurrent": (self.true_only, False)} if self.network_configs else {})})
        limit = self.fields({"field": (self.text, True), "value": (self.float, True)})
        forbid = self.fields({"field": (self.text, True),
                              "values": (lambda x: self.nonempty_texts(x), True)})
        profile = self.fields({"name": (self.text, True),
                               "limits": (nonempty(limit, "boş sınır listesi yazılmaz"), False),
                               "forbidden": (nonempty(forbid, "boş yasak listesi yazılmaz"), False),
                               "topSpeed": (self.float, False), "bothWays": (self.true_only, False)})
        closures = self.fields({"from": (self.text, True), "to": (self.text, True),
                                **({"days": (self.text, False), "hours": (self.text, False)} if self.timed_closures else {})})
        network = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                "kind": (self.enum(NETWORK_KINDS), True),
                "edges": (self.array(edge), True),
                "junctions": (nonempty(junction, "boş düğüm katmanı listesi yazılmaz"), False),
                "connect": (self.enum(NETWORK_CONNECTS), True),
                "tolerance": (self.float, True),
                "direction": (direction, True),
                "costs": (nonempty(cost, "boş maliyet listesi yazılmaz"), False),
                "closed": (self.text, False),
                **({"turns": (turns, False), "levels": (levels, False)} if turned else {}),
                **({"terminals": (terminals, False), "subnetworks": (subnetworks, False)} if self.network_terminals else {}),
                **({"profiles": (nonempty(profile, "boş araç profili listesi yazılmaz"), False),
                    "closures": (closures, False)} if self.network_profiles else {}),
            }
        )
        all_ = self.array(network)(v)
        if not all_:
            self.fail("bad_value", "boş ağ listesi yazılmaz")
        problem = networks_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def ucs_frame(self, more=None):
        """A UCS's origin and angle (schema 49, docs/adr/0261 §1): the origin within 10 000 km, the angle in [0, 2π)."""
        read = self.fields({**(more or {}), "origin": (self.point, True), "angle": (self.float, True)})

        def frame(v):
            u = read(v)
            if abs(u["origin"]["x"]) > UCS_REACH or abs(u["origin"]["y"]) > UCS_REACH:
                self.fail("bad_value", "UCS'in başlangıcı 10 000 km içinde olmalı")
            if not 0.0 <= u["angle"] < 2.0 * math.pi:
                self.fail("bad_value", "UCS'in açısı [0, 2π) aralığında olmalı")
            return u

        return frame

    def ucs_(self, v):
        """The current UCS (schema 49): never the world's, which is no setting."""
        u = self.ucs_frame()(v)
        if u["origin"]["x"] == 0 and u["origin"]["y"] == 0 and u["angle"] == 0:
            self.fail("bad_value", "dünya UCS'i yazılmaz")
        return u

    def named_ucs_(self, v):
        """The named UCSs (schema 49): names trimmed, 1 to 64 characters without control characters, unique, at most
        100; an empty list is not written."""
        all_ = self.array(self.ucs_frame({"name": (self.text, True)}))(v)
        if not all_:
            self.fail("bad_value", "boş adlı UCS listesi yazılmaz")
        if len(all_) > MOST_NAMED_UCS:
            self.fail("bad_value", f"en çok {MOST_NAMED_UCS} adlı UCS olur")
        seen = set()
        for n in all_:
            name = n["name"]
            if name != name.strip() or not name or len(name) > MOST_UCS_NAME or any(unicodedata.category(c) == "Cc" for c in name):
                self.fail("bad_value", f"“{name}” bir UCS adı olamaz")
            if name in seen:
                self.fail("bad_value", f"“{name}” adlı iki UCS var")
            seen.add(name)
        return all_

    def named_views_(self, v):
        """The named views (schema 52, docs/adr/0285 §1): each a name, a middle within 10 000 km, a width and a height
        above 0 and at most 20 000 km, a twist in [0, 2π); names trimmed, 1 to 64 characters without control
        characters, unique, at most 100; an empty list is not written."""
        read = self.fields({**({"id": (self.text, False)} if self.viewmarks else {}), "name": (self.text, True), "center": (self.point, True), "width": (self.float, True), "height": (self.float, True), "twist": (self.float, True)})
        all_ = self.array(read)(v)
        if not all_:
            self.fail("bad_value", "boş adlı görünüm listesi yazılmaz")
        if len(all_) > MOST_NAMED_VIEWS:
            self.fail("bad_value", f"en çok {MOST_NAMED_VIEWS} adlı görünüm olur")
        seen, keys = set(), set()
        for w in all_:
            name = w["name"]
            if name != name.strip() or not name or len(name) > MOST_UCS_NAME or any(unicodedata.category(c) == "Cc" for c in name):
                self.fail("bad_value", f"“{name}” bir görünüm adı olamaz")
            if name in seen:
                self.fail("bad_value", f"“{name}” adlı iki görünüm var")
            seen.add(name)
            # A view's stable id (schema 81, docs/adr/0345 §2): its own, else `v-` and its name's FNV-1a; unique.
            if w.get("id") == "":
                self.fail("bad_value", "boş görünüm kimliği yazılmaz")
            if len(w.get("id", "")) > MOST_UCS_NAME or any(unicodedata.category(c) == "Cc" for c in w.get("id", "")):
                self.fail("bad_value", f"“{name}” görünümünün kimliği tutmuyor")
            key = w.get("id") or view_id_of_name(name)
            if key in keys:
                self.fail("bad_value", f"“{name}” görünümünün kimliği başka bir görünümünkiyle aynı")
            keys.add(key)
            if abs(w["center"]["x"]) > UCS_REACH or abs(w["center"]["y"]) > UCS_REACH:
                self.fail("bad_value", "görünümün ortası 10 000 km içinde olmalı")
            if not all(0.0 < w[k] <= MOST_VIEW_SIZE for k in ("width", "height")):
                self.fail("bad_value", "görünümün eni ve boyu 0'dan büyük ve en çok 20 000 km olmalı")
            if not 0.0 <= w["twist"] < 2.0 * math.pi:
                self.fail("bad_value", "görünümün dönüşü [0, 2π) aralığında olmalı")
        return all_

    def variables_(self, v):
        """The project's variables (schema 37, docs/adr/0214 §2.3): each its kind and name, its label and value when
        given (null, true/false, a float64 or text); then checked whole."""

        def value(v):
            if v is None or type(v) in (bool, float, str):
                return v
            self.fail("wrong_type", "null, true/false, float64 ya da metin olmalı")

        def nonempty_label(v):
            t = self.text(v)
            if not t:
                self.fail("bad_value", "boş etiket yazılmaz")
            return t

        variable = self.fields(
            {"name": (self.text, True), "label": (nonempty_label, False), "kind": (self.enum(VARIABLE_KINDS), True), "value": (value, False)}
        )
        all_ = self.array(variable)(v)
        if not all_:
            self.fail("bad_value", "boş değişken listesi yazılmaz")
        problem = variables_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def layer_fields_(self, v):
        """A layer's fields (schema 26, docs/adr/0199 §1): each its name and kind and what it has, `required` only
        true; the list checked whole."""
        field = self.fields(
            {
                **({"id": (self.text, False)} if self.forms else {}),
                "max": (self.text, False),
                "min": (self.text, False),
                # Schema 60 (docs/adr/0412 §4): a date and time, a time of day.
                "kind": (self.enum(FIELD_KINDS + (("datetime", "time") if self.forms else ())
                                   + (WIDE_KINDS if self.field_kinds else ())), True),
                "name": (self.text, True),
                "alias": (self.text, False),
                "scale": (self.uint(32), False),
                "length": (self.uint(32), False),
                # Schema 60 (docs/adr/0418): a retired value.
                "values": (self.array(self.fields({"code": (self.text, True), "label": (self.text, True),
                                                   **({"retired": (self.bool, False), "description": (self.text, False),
                                                       "from": (self.text, False), "until": (self.text, False)}
                                                      if self.forms else {})})), False),
                "default": (self.text, False),
                # Schema 60 (docs/adr/0408 §1): a listed text field holding several codes.
                **({"multiple": (self.bool, False)} if self.forms else {}),
                # Schema 60 (docs/adr/0419): the split and merge policies.
                **({"split": (self.enum(("copy", "clear", "default", "ratio")), False),
                    "merge": (self.enum(("first", "clear", "sum", "min", "max")), False)} if self.forms else {}),
                # Schema 60 (docs/adr/0430): the value list it links.
                **({"domain": (self.text, False)} if self.forms else {}),
                # Schema 60 (docs/adr/0438 §3, §4): unique, the left-out ends, the unit.
                **({"unique": (self.bool, False), "minExclusive": (self.bool, False),
                    "maxExclusive": (self.bool, False), "unit": (self.text, False)} if self.forms else {}),
                # Schema 107 (docs/adr/0638 §5): its values do not leave the drawing.
                **({"withheld": (self.bool, False)} if self.withheld_fields else {}),
                "required": (self.bool, False),
            }
        )
        all_ = self.array(field)(v)
        if any(f.get("withheld") is False for f in all_):
            self.fail("bad_value", "withheld yalnız true yazılır")
        if any(f.get("required") is False or f.get("multiple") is False for f in all_):
            self.fail("bad_value", "required ve multiple yalnız true yazılır")
        if any(f.get(k) is False for f in all_ for k in ("unique", "minExclusive", "maxExclusive")):
            self.fail("bad_value", "unique, minExclusive ve maxExclusive yalnız true yazılır")
        if any(c.get("retired") is False for f in all_ for c in f.get("values", [])):
            self.fail("bad_value", "retired yalnız true yazılır")
        problem = fields_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    # The project's forms (schema 60, spec §6.11, docs/adr/0401).

    def forms_(self, v):
        """The project's forms: each its id, name, layer, draft, releases and `enforce` (true only), each item by its
        type's fields; the list checked whole (`forms_problem`)."""

        def true_(x):
            if self.bool(x) is not True:
                self.fail("bad_value", "yalnız true yazılır")
            return True

        def nonempty(read):
            def check(x):
                lst = read(x)
                if not lst:
                    self.fail("bad_value", "boş liste yazılmaz")
                return lst
            return check

        widgets = ("text", "textArea", "number", "date", "check", "switch", "list", "radio", "segmented", "slider",
                   "color", "checkList", "search", "chips", "tree")
        # docs/adr/0438 §2: when a check is asked (save or submit; as the value changes it is written not at all).
        check = self.fields({"rule": (self.text, True), "message": (self.text, True), "warning": (true_, False),
                             "on": (self.enum(("save", "submit")), False)})
        # docs/adr/0404 §1: a field's list from another layer, its chain's matches.
        match = self.fields({"field": (self.text, True), "record": (self.text, True),
                             # docs/adr/0439 §4: compared folded.
                             "fold": (true_, False)})
        # docs/adr/0439: the list's order, columns, page, copies, stamp, duplicates and geometry.
        sort = self.fields({"field": (self.text, True), "descending": (true_, False)})
        copy = self.fields({"from": (self.text, True), "to": (self.text, True)})

        def tolerance(v):
            if type(v) not in (int, float) or type(v) is bool:
                self.fail("wrong_type", "sayı olmalı")
            return v

        spatial = self.fields({"relation": (self.enum(("contains", "within", "intersects", "nearest")), True),
                               "tolerance": (tolerance, False)})
        lookup = self.fields({
            "layer": (self.text, True),
            "key": (self.text, True),
            "label": (self.text, False),
            "matches": (nonempty(self.array(match)), False),
            "filter": (self.text, False),
            "sort": (sort, False),
            "columns": (nonempty(self.array(self.text)), False),
            "limit": (self.uint(32), False),
            "copy": (nonempty(self.array(copy)), False),
            "stamp": (self.text, False),
            # "warn" is the default and is not written.
            "duplicates": (self.enum(("first", "refuse")), False),
            "spatial": (spatial, False),
        })

        def item(x):
            if type(x) is not dict:
                self.fail("wrong_type", "harita olmalı")
            shapes = {
                "tab": {"id": (self.text, True), "label": (self.text, True), "items": (items, True),
                        # docs/adr/0426: a tab's visibility rule.
                        "visibleWhen": (self.text, False)},
                "group": {
                    "id": (self.text, True),
                    "label": (self.text, False),
                    "columns": (self.uint(8), False),
                    "visibleWhen": (self.text, False),
                    "collapsed": (true_, False),
                    # docs/adr/0443 §6: two columns filled a row at a time.
                    "across": (true_, False),
                    "items": (items, True),
                },
                "field": {
                    "id": (self.text, True),
                    "field": (self.text, True),
                    "label": (self.text, False),
                    "help": (self.text, False),
                    "placeholder": (self.text, False),
                    "widget": (self.enum(widgets), False),
                    "readOnly": (true_, False),
                    "required": (true_, False),
                    "visibleWhen": (self.text, False),
                    "requiredWhen": (self.text, False),
                    "readOnlyWhen": (self.text, False),
                    "default": (self.text, False),
                    "defaultExpr": (self.text, False),
                    "calculate": (self.text, False),
                    "choiceFilter": (self.text, False),
                    "unique": (true_, False),
                    "reuseLast": (true_, False),
                    "overridable": (true_, False),
                    "suggest": (true_, False),
                    "lookup": (lookup, False),
                    # docs/adr/0439 §6: a searched list's letters before it searches.
                    "minSearch": (self.uint(32), False),
                    "checks": (nonempty(self.array(check)), False),
                    # docs/adr/0437: the field shown again, read-only.
                    "mirror": (true_, False),
                    # docs/adr/0443 §2: kept out of the recorder's examples.
                    "redact": (true_, False),
                },
                "note": {"id": (self.text, True), "text": (self.text, True)},
                "related": {
                    "id": (self.text, True),
                    "label": (self.text, False),
                    "layer": (self.text, True),
                    "field": (self.text, True),
                    "visibleWhen": (self.text, False),
                    # docs/adr/0425: a table of the other layer's fields; a list is not written.
                    "view": (self.enum(("table", "cards", "chart")), False),
                    "columns": (self.array(self.text), False),
                    # docs/adr/0444 §1: the fewest and most related objects.
                    "min": (self.uint(32), False),
                    "max": (self.uint(32), False),
                },
                # docs/adr/0407 §1: the record's photos and documents.
                "attachments": {
                    "id": (self.text, True),
                    "label": (self.text, False),
                    "accept": (self.enum(("any", "photo", "document")), False),
                    "min": (self.uint(32), False),
                    "max": (self.uint(32), False),
                    "visibleWhen": (self.text, False),
                },
                # docs/adr/0433: the record's place.
                "location": {
                    "id": (self.text, True),
                    "label": (self.text, False),
                    "visibleWhen": (self.text, False),
                },
                # docs/adr/0441 §2, §6: a divider and an action opening a registered processing tool.
                "divider": {
                    "id": (self.text, True),
                    "label": (self.text, False),
                    "visibleWhen": (self.text, False),
                },
                "action": {
                    "id": (self.text, True),
                    "label": (self.text, True),
                    # docs/adr/0442 §5, §6: its kind, its target and where a tool runs.
                    "kind": (self.enum(("recompute", "step", "show")), False),
                    "tool": (self.text, False),
                    "step": (self.text, False),
                    "runs": (self.enum(("here", "background")), False),
                    "visibleWhen": (self.text, False),
                },
            }
            kind = x.get("type")
            if kind not in shapes:
                self.path.append("type")
                self.fail("bad_value", f"“{kind}” bilinmiyor ({', '.join(shapes)})")
            return {"type": kind, **self.fields({"type": (self.text, True), **shapes[kind]})(x)}

        def items(x):
            return self.array(item)(x)

        # docs/adr/0420: the designer's scenarios.
        strings = self.attrs
        expect = self.fields({"attrs": (strings, False), "shown": (self.array(self.text), False),
                              "required": (self.array(self.text), False), "readOnly": (self.array(self.text), False),
                              "issues": (self.array(self.text), False)})
        scenario = self.fields({"name": (self.text, True), "values": (strings, False), "new": (true_, False),
                                "expect": (expect, True)})
        # docs/adr/0428: the records' workflow; its fit to the layer's fields is the preflight's.
        step = self.fields({"label": (self.text, True), "from": (nonempty(self.array(self.text)), False),
                            "to": (self.text, True), "valid": (true_, False), "reason": (self.text, False)})
        workflow = self.fields({"field": (self.text, True), "initial": (self.text, True),
                                "locked": (nonempty(self.array(self.text)), False), "steps": (nonempty(self.array(step)), True),
                                # docs/adr/0429: the field each step appends its line to.
                                "log": (self.text, False),
                                # docs/adr/0432: the states a record may be kept incomplete in.
                                "drafts": (nonempty(self.array(self.text)), False),
                                # docs/adr/0442 §4: the field the steps' notes are written to.
                                "notes": (self.text, False)})
        words = self.fields({"label": (self.text, False), "help": (self.text, False), "placeholder": (self.text, False),
                             "choices": (lambda v: nonempty(self.keyed(self.text))(v), False)})
        translation = self.fields({"name": (self.text, False),
                                   "items": (lambda v: nonempty(self.keyed(words))(v), False)})
        body = self.fields({"revision": (self.uint(32), True), "items": (items, True), "clearHidden": (true_, False),
                            "steps": (true_, False),
                            "tests": (nonempty(self.array(scenario)), False), "workflow": (workflow, False),
                            # docs/adr/0443 §5: the form's words in other languages, by tag.
                            "translations": (lambda v: nonempty(self.keyed(translation))(v), False)})
        release = self.fields(
            {"version": (self.uint(32), True), "digest": (self.text, True), "body": (body, True), "note": (self.text, False),
             # docs/adr/0444 §2: the bound fields' digest at publishing.
             "fields": (self.text, False)}
        )
        form = self.fields(
            {
                "id": (self.text, True),
                "name": (self.text, True),
                "layer": (self.text, True),
                "draft": (body, True),
                "releases": (nonempty(self.array(release)), False),
                # docs/adr/0445 §1: withdrawn; written only as true.
                "withdrawn": (true_, False),
                "enforce": (true_, False),
                # docs/adr/0441 §5: the reviewers' comments.
                "comments": (nonempty(self.array(self.fields({
                    "id": (self.uint(32), True), "item": (self.text, False), "author": (self.text, False),
                    "at": (self.text, False), "text": (self.text, True), "resolved": (true_, False)}))), False),
            }
        )
        all_ = nonempty(self.array(form))(v)
        problem = forms_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def domains_(self, v):
        """Schema 60 (docs/adr/0430): the project's value lists, each its id, name, kind and its values or bounds;
        the list checked whole (`domains_problem`)."""
        choice = self.fields({"code": (self.text, True), "label": (self.text, True), "retired": (self.bool, False),
                              "description": (self.text, False), "from": (self.text, False),
                              "until": (self.text, False)})
        domain = self.fields({
            "id": (self.text, True),
            "name": (self.text, True),
            "description": (self.text, False),
            "kind": (self.enum(FIELD_KINDS + ("datetime", "time")), True),
            "values": (self.array(choice), False),
            "min": (self.text, False),
            "max": (self.text, False),
            "minExclusive": (self.bool, False),
            "maxExclusive": (self.bool, False),
            "unit": (self.text, False),
        })
        lst = self.array(domain)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        if any(c.get("retired") is False for d in lst for c in d.get("values", [])):
            self.fail("bad_value", "retired yalnız true yazılır")
        problem = domains_problem(lst)
        if problem:
            self.fail("bad_value", problem)
        return lst

    def join_views_(self, v):
        """Schema 89 (docs/adr/0527): the project's join views; `keep` written only when not `all`, `fields` only when
        some."""
        def keep(x):
            if x != "matched":
                self.fail("bad_value", f"“{x}” bilinmiyor (matched; all yazılmaz)")
            return x

        def fields(x):
            lst = self.array(self.text)(x)
            if not lst:
                self.fail("bad_value", "boş alan listesi yazılmaz")
            return lst

        def how(x):
            if x not in ("count", "sum", "min", "max", "mean"):
                self.fail("bad_value", f"“{x}” bilinmiyor (count, sum, min, max, mean)")
            return x

        def aggregates(x):
            lst = self.array(self.fields({"name": (self.text, True), "how": (how, True),
                                          "field": (self.text, False)}))(x)
            if not lst:
                self.fail("bad_value", "boş toplama listesi yazılmaz")
            return lst

        # Schema 105 (docs/adr/0632 §4): a source's table and the summing columns; before it unknown fields.
        extra = {"source": (self.fields({"path": (self.text, True), "table": (self.text, True)}), False),
                 "aggregates": (aggregates, False)} if self.join_sources else {}
        view = self.fields({
            "id": (self.text, True),
            "name": (self.text, True),
            "layer": (self.text, True),
            "key": (self.text, True),
            "with": (self.text, True),
            "withKey": (self.text, True),
            "keep": (keep, False),
            "fields": (fields, False),
            **extra,
        })
        lst = self.array(view)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        problem = join_views_problem(lst)
        if problem:
            self.fail("bad_value", problem)
        return lst

    def linear_events_(self, v):
        """Schema 97 (docs/adr/0569): the project's links of event layers to their routes; `side` written only when
        `left`, `tolerance` only when given."""
        def kind(x):
            if x not in ("point", "line"):
                self.fail("bad_value", f"“{x}” bilinmiyor (point, line)")
            return x

        def side(x):
            if x != "left":
                self.fail("bad_value", f"“{x}” bilinmiyor (left; right yazılmaz)")
            return x

        link = self.fields({
            "id": (self.text, True),
            "name": (self.text, True),
            "events": (self.text, True),
            "kind": (kind, True),
            "routes": (self.array(self.text), True),
            "routeField": (self.text, True),
            "eventRoute": (self.text, True),
            "measure": (self.text, False),
            "from": (self.text, False),
            "to": (self.text, False),
            "offset": (self.text, False),
            "side": (side, False),
            "tolerance": (self.float, False),
        })
        lst = self.array(link)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        problem = event_links_problem(lst)
        if problem:
            self.fail("bad_value", problem)
        return lst

    def lineage_(self, v):
        """Schema 101 (docs/adr/0598 §1): what the split and merged objects came from: each record a child, its
        parents (never itself, none twice), `split` or `merge` and the step's name (1–200 characters, trimmed); no
        record twice, at most 100 000."""
        def uid(x):
            if not isinstance(x, str) or not UUID_TEXT.fullmatch(x) or x == "00000000-0000-0000-0000-000000000000":
                self.fail("bad_value", f"“{x}” bir nesne kimliği değil")
            return x

        def operation(x):
            if x not in ("split", "merge"):
                self.fail("bad_value", f"“{x}” bilinmiyor (split, merge)")
            return x

        def step(x):
            if not isinstance(x, str) or not 1 <= len(x) <= 200 or x.strip() != x:
                self.fail("bad_value", "adımın adı boşluksuz başlayıp biten 1–200 karakter olmalı")
            return x

        record = self.fields({
            "child": (uid, True),
            "parents": (self.array(uid), True),
            "operation": (operation, True),
            "step": (step, True),
        })
        lst = self.array(record)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        if len(lst) > 100000:
            self.fail("bad_value", "en çok 100 000 soy kaydı olur")
        seen = set()
        for r in lst:
            parents = r["parents"]
            if not 1 <= len(parents) <= 10000 or r["child"] in parents or len(set(parents)) != len(parents):
                self.fail("bad_value", "soy kaydının ataları 1–10 000, kendisi değil, her biri bir kez")
            key = (r["child"], tuple(parents), r["operation"], r["step"])
            if key in seen:
                self.fail("bad_value", "soy kaydı önceki bir kaydın aynısı")
            seen.add(key)
        return lst

    def relations_(self, v):
        """Schema 60 (docs/adr/0405 §1): the project's relations; `onDelete` written only when not `unlink`."""
        def policy(x):
            if x not in ("keep", "cascade", "restrict"):
                self.fail("bad_value", f"“{x}” bilinmiyor (keep, cascade, restrict; unlink yazılmaz)")
            return x

        relation = self.fields({
            "id": (self.text, True),
            "name": (self.text, True),
            "parent": (self.text, True),
            "child": (self.text, True),
            "field": (self.text, True),
            "onDelete": (policy, False),
        })
        lst = self.array(relation)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        problem = relations_problem(lst)
        if problem:
            self.fail("bad_value", problem)
        return lst

    def source_bases_(self, v):
        """Schema 91 (docs/adr/0538 §1): each bound layer's base; its revision and digest 64 lower-case hex digits, its
        rows base64 (RFC 4648 with padding) of whole 32-byte rows in strictly rising lasting id order, at most 200 000."""
        import base64
        import binascii

        def hex64(x):
            t = self.text(x)
            if len(t) != 64 or any(c not in "0123456789abcdef" for c in t):
                self.fail("bad_value", "64 küçük onaltılı hane değil")
            return t

        def rows(x):
            t = self.text(x)
            try:
                raw = base64.b64decode(t, validate=True)
            except (binascii.Error, ValueError):
                self.fail("bad_value", "satırlar base64 değil")
            if base64.b64encode(raw).decode() != t:
                self.fail("bad_value", "satırlar yazıcının base64'ünde değil")
            if len(raw) % BASE_ROW or len(raw) // BASE_ROW > MAX_BASE_ROWS:
                self.fail("bad_value", f"satırlar {BASE_ROW} baytlık değil ya da {MAX_BASE_ROWS}'den çok")
            ids = [raw[i:i + 16] for i in range(0, len(raw), BASE_ROW)]
            if any(a >= b for a, b in zip(ids, ids[1:])):
                self.fail("bad_value", "satırlar kalıcı kimliklerin artan sırasında değil")
            return t

        def field_text(x):
            if not self.source_field_bases:
                self.fail("unknown_field", "kaynak tabanının alan kayıtları şema 92'den önce yok")
            t = self.text(x)
            try:
                raw = base64.b64decode(t, validate=True)
            except (binascii.Error, ValueError):
                self.fail("bad_value", "alan kayıtları base64 değil")
            if base64.b64encode(raw).decode() != t:
                self.fail("bad_value", "alan kayıtları yazıcının base64'ünde değil")
            if len(raw) > MAX_FIELD_BASE_BYTES:
                self.fail("bad_value", f"alan kayıtları {MAX_FIELD_BASE_BYTES} bayttan çok")
            return t

        base = self.fields({"source": (self.text, True), "revision": (hex64, True), "digest": (hex64, True),
                            "rows": (rows, True), "fields": (field_text, False)})
        lst = self.array(base)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        # The field records: one a row, whole, their hashes in rising order (docs/adr/0553 §1).
        for i, b in enumerate(lst):
            if "fields" not in b:
                continue
            count = len(base64.b64decode(b["rows"])) // BASE_ROW
            if not field_records_ok(base64.b64decode(b["fields"]), count):
                self.path.extend([str(i), "fields"])
                self.fail("bad_value", "alan kayıtları satır başına bir bütün kayıt, artan özetlerle olmalı")
        return lst

    def sources_(self, v):
        """Schema 74 (docs/adr/0503): the layers bound to a table of a file; `srid` written only when the table
        declares one; ids and layers once each, an absolute path, a table, the file's SHA-256."""
        def kind(x):
            if x not in ("geopackage", "postgis"):
                self.fail("bad_value", f"“{x}” bilinmeyen kaynak türü (geopackage, postgis)")
            return x

        def box(x):
            # docs/adr/0510: [minX, minY, maxX, maxY] as written, kept a list (the writer writes it back as one).
            if type(x) is not list or len(x) != 4:
                self.fail("bad_value", "alan sınırı 4 sayılık dizi olmalı")
            return [self.at(i, self.float, v) for i, v in enumerate(x)]

        def at(x):
            return self.fields({"x": (self.float, True), "y": (self.float, True)})(x)

        def pins(x):
            # docs/adr/0578 §3: the pins of an object whose row is outside the part, as an object's own are.
            return self.array(self.fields({"at": (at, False), "class": (self.text, False), "hidden": (self.bool, False),
                                           "rotation": (self.float, False)}))(x)

        def parked(x):
            return self.array(self.fields({"uid": (self.text, True), "key": (self.text, False), "pins": (pins, True)}))(x)

        source = self.fields({
            "id": (self.text, True),
            "kind": (kind, True),
            "path": (self.text, True),
            "relative": (self.text, False),
            "table": (self.text, True),
            "layer": (self.text, True),
            "revision": (self.text, True),
            "digest": (self.text, True),
            "srid": (self.uint(32), False),
            "extent": (box, False),
            "key": (self.text, False),
            "parked": (parked, False),
            "writable": (self.bool, True),
        })
        lst = self.array(source)(v)
        if not lst:
            self.fail("bad_value", "boş liste yazılmaz")
        ids, layers = set(), set()
        for s in lst:
            if not s["id"].strip() or s["id"] in ids:
                self.fail("bad_value", f"“{s['id']}” kaynak kimliği boş ya da iki kez kullanılmış")
            ids.add(s["id"])
            if "key" in s and (not s["key"].strip() or len(s["key"]) > 128 or _control(s["key"])):
                # docs/adr/0533 §1: a column's name, the key that ties a row to its object across rewrites.
                self.fail("bad_value", "anahtar alanının adı boş, çok uzun ya da denetim karakteri taşıyor")
            if "parked" in s:
                # docs/adr/0578 §3: at most 100 000 objects, each once with a lasting id and pins, a key value not empty.
                if len(s["parked"]) > 100000:
                    self.fail("bad_value", "en çok 100 000 nesnenin iğneleri park edilir")
                seen_uids = set()
                for p in s["parked"]:
                    if not p["uid"].strip() or p["uid"] in seen_uids:
                        self.fail("bad_value", "park edilen iğnenin nesne kimliği boş ya da iki kez geçiyor")
                    seen_uids.add(p["uid"])
                    if not p["pins"]:
                        self.fail("bad_value", "park edilen nesnenin iğnesi yok")
                    if "key" in p and not p["key"]:
                        self.fail("bad_value", "park edilen nesnenin anahtar değeri boş")
                    bad = pins_problem(p["pins"])
                    if bad:
                        self.fail("bad_value", bad)
            if s["kind"] == "geopackage" and "extent" in s:
                # docs/adr/0510: the area of a part's working copy, finite, its minimum below its maximum.
                e = s["extent"]
                if any(x != x or x in (float("inf"), float("-inf")) for x in e) or e[0] >= e[2] or e[1] >= e[3]:
                    self.fail("bad_value", "alan sınırının en küçüğü en büyüğünden küçük ve sonlu olmalı")
            if s["kind"] == "postgis":
                # docs/adr/0505: a database's address, no password in it.
                if not s["path"].startswith(("postgresql://", "postgres://")):
                    self.fail("bad_value", f"“{s['path']}” bir veritabanı adresi değil")
                rest = s["path"].split("://", 1)[1]
                authority, _, tail = rest.partition("/")
                user, at, _ = authority.rpartition("@")
                if (at and ":" in user) or any(kv.split("=")[0].lower() == "password"
                                               for kv in tail.partition("?")[2].split("&") if kv):
                    self.fail("bad_value", "adreste parola var; parola projeye yazılmaz")
                if "relative" in s:
                    self.fail("bad_value", "veritabanının göreli yolu olmaz")
                if "extent" in s:
                    self.fail("bad_value", "veritabanı tablosunun alan sınırı henüz olmaz")
            elif not (s["path"].startswith("/") or s["path"][1:3] == ":\\"):
                self.fail("bad_value", f"“{s['path']}” mutlak bir yol değil")
            elif "relative" in s:
                # docs/adr/0506: from the project file's folder, parts by "/", ".." only at the start.
                r, up = s["relative"], True
                if not r or len(r) > 4096 or "\0" in r or r.startswith("/") or "\\" in r or r[1:2] == ":":
                    self.fail("bad_value", f"“{r}” göreli bir yol değil")
                for part in r.split("/"):
                    if part in ("", "."):
                        self.fail("bad_value", f"“{r}” boş ya da . parçası taşıyor")
                    if part == "..":
                        if not up:
                            self.fail("bad_value", f"“{r}” ortasında .. taşıyor")
                    else:
                        up = False
                if up:
                    self.fail("bad_value", f"“{r}” bir dosyayı göstermiyor")
            if not s["table"].strip():
                self.fail("bad_value", "kaynağın tablosu boş")
            if s["layer"] in layers:
                self.fail("bad_value", f"“{s['layer']}” katmanı iki kaynağa bağlı")
            layers.add(s["layer"])
            for k, what in (("revision", "sürümü"), ("digest", "katmanın özeti")):
                if len(s[k]) != 64 or any(c not in "0123456789abcdef" for c in s[k]):
                    self.fail("bad_value", f"kaynağın {what} 64 küçük onaltılı hane değil")
        return lst

    # The project's coordinate systems and datum choices (schema 13, spec §6.4.1, docs/adr/0168).

    def three(self, v):
        a = self.array(self.float)(v)
        if len(a) != 3:
            self.fail("bad_value", f"3 sayı olmalı, {len(a)} var")
        return a

    def helmert(self, v):
        h = self.fields(
            {
                "scale": (self.float, True),
                "accuracy": (self.float, False),
                "rotation": (self.three, True),
                "convention": (self.enum(("positionVector", "coordinateFrame")), True),
                "translation": (self.three, True),
            }
        )(v)
        return {k: h[k] for k in ("translation", "rotation", "scale", "convention", "accuracy") if k in h}

    def custom_datum(self, v):
        ellipsoid = self.fields({"name": (self.text, True), "semiMajor": (self.float, True), "inverseFlattening": (self.float, True)})
        d = self.fields({"name": (self.text, True), "toWgs84": (self.helmert, False), "ellipsoid": (ellipsoid, True)})(v)
        e = d["ellipsoid"]
        out = {"name": d["name"], "ellipsoid": {"name": e["name"], "semiMajor": e["semiMajor"], "inverseFlattening": e["inverseFlattening"]}}
        if "toWgs84" in d:
            out["toWgs84"] = d["toWgs84"]
        return out

    def crs_plane(self, v):
        similarity = ("east", "north", "rotation", "scale")
        table = {"kind": (self.enum(("similarity", "affine")), True), **{k: (self.float, False) for k in "abcdef"},
                 **{k: (self.float, False) for k in similarity}}
        p = self.fields(table)(v)
        affine = p["kind"] == "affine"
        if any(k in p for k in (similarity if affine else tuple("abcdef"))):
            self.fail("unknown_field", "düzlem dönüşümünde türünün olmayan bir alanı var")
        for k in (tuple("abcdef") if affine else similarity):
            if k not in p:
                self.path.append(k)
                self.fail("missing_field", "zorunlu alan yok")
        return {"kind": p["kind"], **{k: p[k] for k in (tuple("abcdef") if affine else similarity)}}

    def crs_base(self, v):
        b = self.fields({"srid": (self.uint(32), False), "definition": (self.crs_definition_raw, False)})(v)
        return {k: b[k] for k in ("srid", "definition") if k in b}

    def crs_system(self, v):
        allowed = {
            "tm": ("kind", "datum", "customDatum", "latitudeOfOrigin", "centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"),
            "geographic": ("kind", "datum", "customDatum"),
            "local": ("kind", "base", "plane"),
        }
        s = self.fields(
            {
                "kind": (self.enum(("tm", "geographic", "local")), True),
                "datum": (self.enum(REGISTRY_DATUMS), False),
                "customDatum": (self.custom_datum, False),
                "latitudeOfOrigin": (self.float, False),
                "centralMeridian": (self.float, False),
                "scaleFactor": (self.float, False),
                "falseEasting": (self.float, False),
                "falseNorthing": (self.float, False),
                "base": (self.crs_base, False),
                "plane": (self.crs_plane, False),
            }
        )(v)
        kind = s["kind"]
        stray = [k for k in s if k not in allowed[kind]]
        if stray:
            self.path.append(stray[0])
            self.fail("unknown_field", f"“{kind}” sisteminde bu alan olmaz")
        required = {"tm": ("centralMeridian", "scaleFactor", "falseEasting", "falseNorthing"), "geographic": (), "local": ("base", "plane")}[kind]
        for k in required:
            if k not in s:
                self.path.append(k)
                self.fail("missing_field", "zorunlu alan yok")
        return {"kind": kind, **{k: s[k] for k in allowed[kind][1:] if k in s}}

    def crs_definition_raw(self, v):
        d = self.fields({"name": (self.text, True), "system": (self.crs_system, True)})(v)
        return {"name": d["name"], "system": d["system"]}

    def crs_definition(self, v):
        d = self.crs_definition_raw(v)
        problem = crs_problem(d)
        if problem:
            self.fail("bad_value", problem)
        return d

    def datum_transform(self, v):
        grid = self.fields({"id": (self.text, True), "file": (self.text, True), "size": (self.uint(64), True), "accuracy": (self.float, False)})
        t = self.fields({"to": (self.enum(REGISTRY_DATUMS), True), "from": (self.enum(REGISTRY_DATUMS), True), "grid": (grid, False),
                         "name": (self.text, True), "helmert": (self.helmert, False)})(v)
        out = {"from": t["from"], "to": t["to"], "name": t["name"]}
        if "helmert" in t:
            out["helmert"] = t["helmert"]
        if "grid" in t:
            g = t["grid"]
            out["grid"] = {k: g[k] for k in ("id", "file", "size", "accuracy") if k in g}
        return out

    def datum_transforms(self, v):
        all_ = self.array(self.datum_transform)(v)
        problem = choices_problem(all_)
        if problem:
            self.fail("bad_value", problem)
        return all_

    def layer_snap_(self, v):
        """A layer's own snapping (schema 10): exactly one of `off` (true only) and a non-empty list of known,
        distinct kinds."""
        s = self.fields({"off": (self.bool, False), "kinds": (self.array(self.text), False)})(v)
        if s.get("off") is False:
            self.fail("bad_value", "off yalnız true yazılır")
        if ("off" in s) == ("kinds" in s):
            self.fail("bad_value", "katmanın keneti ya off ya kinds taşır, tam biri")
        kinds = s.get("kinds", [])
        if "kinds" in s and not kinds:
            self.fail("bad_value", "kenet türleri boş olamaz")
        for i, k in enumerate(kinds):
            if k not in LAYER_SNAP_KINDS:
                self.fail("bad_value", f"bilinmeyen kenet türü: {k}")
            if k in kinds[:i]:
                self.fail("bad_value", f"kenet türü iki kez yazılmış: {k}")
        return s

    def layer_style(self, v):
        return self.fields(
            {
                "fill": (self.text, False),
                "color": (self.text, True),
                "label": (self.label_style, False),
                **({"labels": (self.layer_labels_, False)} if self.labels else {}),
                "point": (self.fields({"size": (self.float, True), "symbol": (self.enum(("ring", "cross", "triangle")), True)}), False),
                "lineType": (self.enum(("continuous", "dashed", "dashdot", "dotted")), True),
                "renderer": (self.not_null, False),
                "lineWeight": (self.float, True),
                "pickInterior": (self.bool, False),
                **({"linePattern": (self.text, False)} if self.patterns else {}),
            }
        )(v)

    def line_type_scale_(self, v):
        """The project's line type scale (schema 56, docs/adr/0313 §3): above 0, at most 1000, never 1 (its absence)."""
        self.float(v)
        if not (0.0 < v <= 1000.0) or v == 1.0:
            self.fail("bad_value", "çizgi tipi ölçeği 0'dan büyük ve en çok 1000 olmalı; 1 yazılmaz")
        return v

    def hatch_patterns_(self, v):
        """The project's own hatch patterns (schema 71, docs/adr/0331 §1): a name trimmed, not empty, without a comma, at
        most 64 letters, once (case aside); 1 to 64 families, each at most 16 dashes not all 0, its numbers finite and
        at most 10⁶ in size, its lines' distance not 0; the unit mm, inch or drawing; an empty list is not written."""
        family = self.fields({"angle": (self.float, True), "origin": (self.pair, True), "offset": (self.pair, True),
                              "dashes": (self.array(self.float), False)})
        read = self.fields({"name": (self.text, True), "description": (self.text, False), "unit": (self.enum(("mm", "inch", "drawing")), True),
                            "lines": (self.array(family), True), "source": (self.text, False)})
        all_ = self.array(read)(v)
        if not all_:
            self.fail("bad_value", "boş liste yazılmaz; tarama deseni yoksa alan hiç yazılmaz")
        seen = set()
        fits = lambda x: math.isfinite(x) and abs(x) <= MAX_PATTERN_SIZE  # noqa: E731
        for d in all_:
            name = d["name"]
            if not name or name != name.strip() or "," in name or len(name) > MAX_PATTERN_NAME:
                self.fail("bad_value", f"“{name}” bir tarama deseni adı olamaz")
            if name.lower() in seen:
                self.fail("bad_value", f"“{name}” tarama deseni iki kez var")
            seen.add(name.lower())
            if "description" in d and not d["description"]:
                self.fail("bad_value", "boş açıklama yazılmaz")
            if not 1 <= len(d["lines"]) <= MAX_PATTERN_LINES:
                self.fail("bad_value", f"“{name}” tarama deseninin ailesi en az 1, en çok {MAX_PATTERN_LINES} olur")
            for l in d["lines"]:
                dashes = l.get("dashes", [])
                if len(dashes) > MAX_PATTERN_DASHES or not all(fits(x) for x in [l["angle"], *l["origin"], *l["offset"], *dashes]):
                    self.fail("bad_value", f"“{name}” tarama deseninin bir ailesi geçersiz")
                if l["offset"][1] == 0.0 or (dashes and all(x == 0.0 for x in dashes)):
                    self.fail("bad_value", f"“{name}” tarama deseninin bir ailesi çizmiyor")
        return all_

    def line_types_(self, v):
        """The project's line types (schema 56, docs/adr/0313 §1): a name trimmed, not empty, once (case aside); at most
        12 elements, a text or a shape of length 0, the first that draws not a gap; the unit paper or drawing; an empty
        list is not written."""
        place = {"scale": (self.float, False), "rotation": (self.float, False), "absolute": (self.bool, False),
                 "upright": (self.bool, False), "x": (self.float, False), "y": (self.float, False)}
        element = self.fields({
            "length": (self.float, True),
            "text": (self.fields({"text": (self.text, True), "style": (self.text, True), **place}), False),
            "shape": (self.fields({"name": (self.text, True), "file": (self.text, True), **place}), False),
        })
        read = self.fields({"name": (self.text, True), "description": (self.text, False), "elements": (self.array(element), True),
                            "unit": (self.enum(("paper", "drawing")), True), "source": (self.text, False)})
        all_ = self.array(read)(v)
        if not all_:
            self.fail("bad_value", "boş liste yazılmaz; çizgi tipi yoksa alan hiç yazılmaz")
        seen = set()
        for t in all_:
            name = t["name"]
            if not name or name != name.strip():
                self.fail("bad_value", f"“{name}” bir çizgi tipi adı olamaz")
            if name.lower() in seen:
                self.fail("bad_value", f"“{name}” çizgi tipi iki kez var")
            seen.add(name.lower())
            els = t["elements"]
            if len(els) > MOST_PATTERN_ELEMENTS:
                self.fail("bad_value", f"“{name}” çizgi tipinin deseni en çok {MOST_PATTERN_ELEMENTS} öğe olur")
            for e in els:
                if ("text" in e or "shape" in e) and e["length"] != 0.0:
                    self.fail("bad_value", f"“{name}” çizgi tipinde yazı ya da şeklin uzunluğu 0 olmalı")
                for c in (e.get("text"), e.get("shape")):
                    if c is not None and c.get("scale", 1.0) <= 0.0:
                        self.fail("bad_value", f"“{name}” çizgi tipinde bir yazının ya da şeklin ölçeği 0'dan büyük olmalı")
            drawn = [e["length"] for e in els if "text" not in e and "shape" not in e]
            if drawn and drawn[0] < 0.0:
                self.fail("bad_value", f"“{name}” çizgi tipinin deseni bir çizgi ya da noktayla başlamalı")
        return all_

    def label_style(self, v):
        engine = {}
        if self.labels:
            # The label engine's fields (schema 36, docs/adr/0212 §2).
            engine = {
                "text": (self.text, False),
                "color": (self.text, False),
                "italic": (self.bool, False),
                "align": (self.enum(("left", "center", "right")), False),
                "point": (self.enum(("around", "center")), False),
                "line": (self.enum(("parallel", "curved", "horizontal", "contour")), False),
                "area": (self.enum(("horizontal", "free", "perimeter", "boundary", "parcel", "corner")), False),
                "position": (self.enum(("on", "above", "below", "sides")), False),
                "distance": (self.float, False),
                "repeat": (self.float, False),
                "maxAngle": (self.float, False),
                "curved": (self.bool, False),
                "mergeLines": (self.bool, False),
                "inside": (self.bool, False),
                "outside": (self.bool, False),
                "halo": (self.fields({"width": (self.float, True), "color": (self.text, False)}), False),
                "background": (
                    self.fields({"shape": (self.enum(("rect", "round", "ellipse")), True), "fill": (self.text, False), "stroke": (self.text, False), "padding": (self.float, False)}),
                    False,
                ),
                "shadow": (self.fields({"dx": (self.float, True), "dy": (self.float, True), "color": (self.text, False), "opacity": (self.float, False)}), False),
                "callout": (
                    self.fields({"kind": (self.enum(("straight", "manhattan")), True), "color": (self.text, False), "width": (self.float, False), "minLength": (self.float, False)}),
                    False,
                ),
                "stack": (self.fields({"mode": (self.enum(("ifNeeded", "always")), True), "chars": (self.uint(32), True), "at": (self.text, False)}), False),
                "abbreviate": (
                    self.fields({"always": (self.bool, False), "words": (self.array(self.fields({"word": (self.text, True), "short": (self.text, True)})), True)}),
                    False,
                ),
                "shrink": (self.float, False),
                "priority": (self.uint(8), False),
                "overlap": (self.enum(("never", "ifNeeded", "always")), False),
                "duplicates": (self.float, False),
            }
        s = self.fields(
            {
                "ink": (self.enum(("fg", "fg-dim", "label")), False),
                "grow": (self.float, False),
                "size": (self.float, True),
                "weight": (self.uint(16), False),
                "maxSize": (self.float, False),
                "maxScale": (self.float, False),
                "minScale": (self.float, False),
                "template": (self.text, False),
                "placement": (self.enum(("center", "corner", "beside", "along")), True),
                "minFeaturePx": (self.float, False),
                **engine,
            }
        )(v)
        # The old fields keep the old readers' leniency; a schema 36 style is checked whole.
        if self.labels:
            p = label_style_problem(s)
            if p:
                self.fail("bad_value", p)
        return s

    # Schema 36's labelling (docs/adr/0212 §2): read field by field, then checked by the contract's rule.

    def layer_labels_(self, v):
        cls = self.fields({"name": (self.text, True), "when": (self.text, False), "style": (self.label_style, True)})
        l = self.fields(
            {
                "mode": (self.enum(("single", "rules", "off")), True),
                "classes": (self.array(cls), False),
                "obstacle": (self.fields({"kind": (self.enum(("interior", "boundary")), False), "weight": (self.uint(8), True)}), False),
            }
        )(v)
        if "classes" in l and not l["classes"]:
            self.fail("bad_value", "sınıf listesi boş; boş liste yazılmaz")
        p = layer_labels_problem(l)
        if p:
            self.fail("bad_value", p)
        return l

    def label_pins_(self, v):
        pins = self.array(self.fields({"at": (self.point, False), "class": (self.text, False), "hidden": (self.bool, False), "rotation": (self.float, False)}))(v)
        if not pins:
            self.fail("bad_value", "etiket iğneleri boş; boş liste yazılmaz")
        p = pins_problem(pins)
        if p:
            self.fail("bad_value", p)
        return pins

    def uid(self, v):
        text = self.id16(v)
        if text in self.seen:
            self.fail("duplicate_uid", f"kalıcı kimlik {text} iki nesnede var")
        self.seen.add(text)
        self.uids.append(text)
        return text

    def entity(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "nesne tek anahtarlı bir harita olmalı")
        if len(v) != 1:
            self.fail("bad_value", f"nesne haritasında tek anahtar (tür) olmalı, {len(v)} var")
        ((kind, body),) = v.items()
        if kind not in ENTITY_KINDS or (kind == "insert" and not self.blocks) or (kind == "leader" and not self.leaders) or (kind == "table" and not self.tables) or (kind == "image" and not self.images) or (kind == "raster" and not self.rasters) or (kind == "pointcloud" and not self.point_clouds) or (kind == "nurbs" and not self.nurbs) or (kind == "record" and not self.forms) or (kind == "wall" and not self.walls) or (kind == "opening" and not self.openings) or (kind == "space" and not self.spaces) or (kind in ARCH_ELEMENTS and not self.arch_elements) or (kind == "tag" and not self.tags) or (kind == "viewmark" and not self.viewmarks):
            self.path.append(kind)
            self.fail("unknown_kind", f"“{kind}” nesne türü bilinmiyor")
        # A block definition holds no table (docs/adr/0184 §1).
        if kind == "table" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında tablo olamaz")
        # Nor a picture (docs/adr/0192 §1).
        if kind == "image" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında resim olamaz")
        # Nor a raster (docs/adr/0204 §2).
        if kind == "raster" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında raster olamaz")
        # Nor a point cloud (docs/adr/0207 §3).
        if kind == "pointcloud" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında nokta bulutu olamaz")
        # Nor a wall (docs/adr/0339 §4).
        if kind == "wall" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında duvar olamaz")
        # Nor a grid, a column, a stair or a ramp (docs/adr/0343 §1).
        if kind in ARCH_ELEMENTS and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında aks, kolon, merdiven ya da rampa olamaz")
        # Nor a tag: its object is the drawing's (docs/adr/0344 §1).
        if kind == "tag" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında etiket olamaz")
        # Nor a view mark: its target is the project's (docs/adr/0345 §1).
        if kind == "viewmark" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında görünüm işareti olamaz")
        # Nor a space: its walls are the drawing's (docs/adr/0342 §1).
        if kind == "space" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında mahal olamaz")
        # Nor an opening: its wall is the drawing's (docs/adr/0341 §1).
        if kind == "opening" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında açıklık olamaz")
        # Nor a record without a geometry (docs/adr/0410 §2).
        if kind == "record" and self.inside is not None:
            self.path.append(kind)
            self.fail("bad_value", "blok tanımında geometrisiz kayıt olamaz")
        table = dict(self.common())
        table.update(ENTITY_KINDS[kind](self))
        self.path.append(kind)
        try:
            fields = self.fields(table)(body)
            # One way to write an insert: its y factor only when it is not the x one (docs/adr/0315 §3).
            if kind == "insert" and fields.get("yScale") == fields["scale"]:
                self.path.append("yScale")
                self.fail("bad_value", "y ölçeği x ölçeğine eşitse yazılmaz")
            if kind in ("polyline", "polygon"):
                self.same_length(fields)
            if kind == "polyline":
                self.polyline_extras_rules(fields)
                if "spaceSeparator" in fields:
                    self.space_separator_rules(fields)
            if kind == "nurbs":
                self.nurbs_rules(fields)
            if kind == "wall":
                self.wall_rules(fields)
            if kind == "opening":
                self.opening_rules(fields)
            if kind == "space":
                self.space_rules(fields)
            if kind in ARCH_ELEMENTS:
                self.arch_element_rules(kind, fields[kind])
            self.look_rules(fields)
            if "wipeout" in fields and kind != "polygon":
                self.path.append("wipeout")
                self.fail("bad_value", f"maske yalnız alanda olur; bu nesne bir {kind}")
            if kind == "dimension":
                self.dimension_rules(fields)
            if kind == "text" and ("labelOf" in fields) != ("labelScale" in fields):
                self.path.append("labelOf" if "labelOf" in fields else "labelScale")
                self.fail("bad_value", "bağlı yazının nesnesi ve ölçeği birlikte verilir")
            if kind == "text" and "runs" in fields:
                self.path.append("runs")
                self.runs_rules(fields["text"], fields["runs"])
                self.path.pop()
            if kind == "text" and "layout" in fields:
                self.path.append("layout")
                self.layout_rules(fields["layout"], "boxWidth" in fields)
                self.path.pop()
            # A leader with content blocks has no note (docs/adr/0326 §2).
            if kind == "leader" and "blocks" in fields and "text" in fields:
                self.path.append("blocks")
                self.fail("bad_value", "blok içerikli kılavuzun notu olmaz")
            if kind == "text" and "fields" in fields:
                self.path.append("fields")
                self.text_fields_rules(fields["text"], fields["fields"])
                self.path.pop()
            # A text along a curve or linked to an object is not annotative (docs/adr/0323 §1).
            if kind == "text" and "annotative" in fields and ("path" in fields or "labelOf" in fields):
                self.path.append("annotative")
                self.fail("bad_value", "eğri boyunca ve nesneye bağlı yazı ölçekli olamaz")
            # A solid or gradient hatch has no pattern to scale (docs/adr/0331 §3).
            if kind == "hatch" and "annotative" in fields and fields["pattern"]["type"] in ("solid", "gradient"):
                self.path.append("annotative")
                self.fail("bad_value", "dolu ve degrade tarama ölçekli olamaz")
            if kind == "text" and "path" in fields:
                self.path.append("path")
                self.text_path_rules(fields)
                self.path.pop()
            if kind == "table":
                self.table_sources(fields)
                self.table_rules(fields)
            if kind == "image":
                self.image_rules(fields)
            if kind == "raster":
                self.raster_rules(fields)
            if kind == "pointcloud":
                self.cloud_rules(fields)
            # Bold, italic and a slant need a typeface (docs/adr/0183 §2); a table's face is a text's.
            if kind in ("text", "table") and "font" not in fields and any(k in fields for k in ("bold", "italic", "oblique")):
                self.path.append(next(k for k in ("textStyle", "bold", "italic", "oblique") if k in fields))
                self.fail("bad_value", "Kalın, eğik ve yatık yazı bir yazı tipiyle olur; yazının yazı tipi yok")
            # The drawing's own insert names a definition read before it (`blocks` comes before `entities`).
            if kind == "insert" and self.inside is None and fields["block"] not in self.index:
                self.path.append("block")
                self.fail("unknown_block", "yerleştirilen blok çizimde tanımlı değil")
            if kind == "insert" and self.inside is None:
                self.values_rule(fields, self.dynamics.get(fields["block"]))
                self.places_rule(fields, self.movable.get(fields["block"], set()))
        finally:
            self.path.pop()
        # An opening's map is its own fields, as the contract's JSON has them (docs/adr/0341 §1).
        if kind == "opening":
            fields = {**{k: v for k, v in fields.items() if k != "opening"}, **fields["opening"]}
        if kind == "space":
            fields = {**{k: v for k, v in fields.items() if k != "space"}, **fields["space"]}
        if kind in ARCH_ELEMENTS or kind in ("tag", "viewmark"):
            fields = {**{k: v for k, v in fields.items() if k != kind}, **fields[kind]}
        if self.inside is None:
            fields.pop("uid")
            return {"kind": kind, "id": len(self.uids), **fields}
        self.inside += 1
        return {"kind": kind, "id": self.inside, **fields}

    # Blocks (§6.9, docs/adr/0144).

    def definitions(self, v):
        if type(v) is list and not v:
            self.fail("bad_value", "blok listesi boş; tanımı olmayan çizimde alan yazılmaz")
        blocks = self.array(self.definition)(v)
        self.check_blocks(blocks)
        self.index = {b["id"]: i for i, b in enumerate(blocks)}
        self.movable = {b["id"]: {a["tag"] for a in b.get("attributes", []) if a.get("movable")} for b in blocks}
        self.dynamics = {b["id"]: b.get("dynamic") for b in blocks}
        return blocks

    def definition(self, v):
        def content(x):
            self.inside = 0
            try:
                return self.array(self.entity)(x)
            finally:
                self.inside = None

        def attributes(x):
            if type(x) is list and not x:
                self.fail("bad_value", "öznitelik listesi boş; öznitelik tanımı yoksa alan yazılmaz")
            return self.array(self.attribute)(x)

        d = self.fields(
            {
                "id": (self.id16, True),
                "base": (self.point, True),
                "name": (self.text, True),
                "entities": (content, True),
                "attributes": (attributes, False),
                "description": (self.text, False),
                # The unit it is drawn in (schema 57, docs/adr/0315 §4).
                **({"units": (self.enum(("mm", "cm", "m", "in", "ft", "unitless")), False)} if self.block_extras else {}),
                # Its library card (schema 58, docs/adr/0317 §4).
                **({"card": (self.card, False)} if self.block_cards else {}),
                # Its parameters and actions (schema 59, docs/adr/0318).
                **({"dynamic": (self.dynamic, False)} if self.dynamic_blocks else {}),
                # Its external reference source (schema 83, docs/adr/0348 §1).
                **({"xref": (self.xref, False)} if self.xrefs else {}),
            }
        )(v)
        if "xref" in d and "objects" in d["xref"] and len(d["xref"]["objects"]) != len(d["entities"]):
            self.fail("bad_value", f"dış başvurunun {len(d['xref']['objects'])} nesne kimliği var, tanımının {len(d['entities'])} nesnesi")
        return {k: d[k] for k in ("id", "name", "base", "entities", "attributes", "description", "units", "card", "dynamic", "xref") if k in d}

    def xref(self, v):
        """An external reference's source (docs/adr/0348 §1): its path, relative path, kind, digest, unloaded, layer group
        and parent; a default is not written (`attach`, `unloaded` false)."""
        def filled(x):
            t = self.text(x)
            if not t.strip():
                self.fail("bad_value", "boş olamaz")
            return t

        def digest(x):
            t = self.text(x)
            if len(t) != 64 or any(c not in "0123456789abcdef" for c in t):
                self.fail("bad_value", "dış başvurunun özeti 64 küçük onaltılık rakam olmalı")
            return t

        def uuid_text(x):
            t = self.text(x)
            if not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", t):
                self.fail("bad_value", "bir blok kimliği değil")
            return t

        def nonempty_uids(x):
            out = self.array(uuid_text)(x)
            if not out:
                self.fail("bad_value", "boş liste yazılmaz")
            return out

        def page(x):
            n = self.uint(32)(x)
            if n == 0:
                self.fail("bad_value", "PDF'in sayfası 1'den başlar")
            return n

        def underlay(x):
            # A PDF underlay's look (schema 85, docs/adr/0351 §2): a field at its default not written, nor a look that
            # changes nothing.
            def fade(f):
                n = self.uint(8)(f)
                if not 1 <= n <= MAX_UNDERLAY_FADE:
                    self.fail("bad_value", f"soldurma 1…{MAX_UNDERLAY_FADE} olmalı (0 yazılmaz)")
                return n

            def no_plot(p):
                if self.bool(p) is not False:
                    self.fail("bad_value", "plot yalnız false yazılır; yazdırılan altlıkta alan yoktur")
                return False

            out = self.fields({"fade": (fade, False), "monochrome": (self.only_true, False), "plot": (no_plot, False)})(x)
            if not out:
                self.fail("bad_value", "değişmeyen altlık görünüşü yazılmaz")
            return out

        out = self.fields({"path": (filled, True), "relative": (filled, False), "kind": (self.enum(("overlay",)), False),
                           "digest": (digest, True), "unloaded": (self.only_true, False), "group": (filled, False),
                           "parent": (uuid_text, False), "objects": (nonempty_uids, False),
                           **({"page": (page, False), "underlay": (underlay, False)} if self.clipping else {})})(v)
        if "underlay" in out and "page" not in out:
            self.fail("bad_value", "sayfası olmayan dış başvurunun altlık görünüşü olmaz")
        return out

    def insert_clipping(self, v):
        """An insert's clipping (schema 85, docs/adr/0351 §1): its corners in the definition, three or more, with an
        area; `inverted` only true, `frame` only show or plot."""
        out = self.fields({"boundary": (self.array(self.jpoint), True), "inverted": (self.only_true, False),
                           "frame": (self.enum(("show", "plot")), False)})(v)
        ring = [(p["x"], p["y"]) for p in out["boundary"]]
        if len(ring) < 3:
            self.fail("bad_value", "kırpma sınırı en az üç köşeli olmalı")
        if len(ring) > MAX_CLIPPING_CORNERS:
            self.fail("bad_value", f"kırpma sınırı en çok {MAX_CLIPPING_CORNERS} köşeli olabilir")
        n = len(ring)
        if sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n)) == 0.0:
            self.fail("bad_value", "kırpma sınırının alanı yok")
        return out

    # Dynamic blocks (schema 59, docs/adr/0318): the part is the contract's JSON, points as {x, y}, numbers floats,
    # ids whole numbers, a field at its default (an empty list, a free rule) not written.

    def jpoint(self, v):
        return self.fields({"x": (self.float, True), "y": (self.float, True)})(v)

    def ids(self, v):
        return self.array(self.uint(32))(v)

    def rule(self, v):
        def floats(x):
            if type(x) is list and not x:
                self.fail("bad_value", "boş liste yazılmaz")
            return self.array(self.float)(x)

        r = self.fields({"min": (self.float, False), "max": (self.float, False), "step": (self.float, False),
                         "list": (floats, False), "expression": (self.text, False)})(v)
        if not r:
            self.fail("bad_value", "serbest kural yazılmaz")
        return r

    def dynamic(self, v):
        def kinded(kinds):
            def check(x):
                if type(x) is not dict:
                    self.fail("wrong_type", "harita olmalı")
                kind = x.get("kind")
                if kind not in kinds:
                    self.path.append("kind")
                    self.fail("bad_value", f"“{kind}” bilinmiyor ({', '.join(kinds)})")
                return {"kind": kind, **self.fields({"kind": (self.text, True), **kinds[kind]})(x)}
            return check

        def states(x):
            def state(s):
                def hidden(h):
                    if type(h) is list and not h:
                        self.fail("bad_value", "boş liste yazılmaz")
                    return self.ids(h)
                return self.fields({"name": (self.text, True), "hidden": (hidden, False)})(s)
            return self.array(state)(x)

        def rows(x):
            def row(r):
                def values(m):
                    if type(m) is not dict:
                        self.fail("wrong_type", "harita olmalı")
                    return {k: self.at(k, self.json_value, x) for k, x in m.items()}
                return self.fields({"label": (self.text, True), "values": (values, True)})(r)
            return self.array(row)(x)

        rule = {"rule": (self.rule, False)}
        parameters = {
            "linear": {"name": (self.text, True), "from": (self.jpoint, True), "to": (self.jpoint, True), **rule},
            "polar": {"name": (self.text, True), "from": (self.jpoint, True), "to": (self.jpoint, True), **rule},
            "angle": {"name": (self.text, True), "center": (self.jpoint, True), "base": (self.float, True), **rule},
            "point": {"name": (self.text, True), "at": (self.jpoint, True)},
            "flip": {"name": (self.text, True), "a": (self.jpoint, True), "b": (self.jpoint, True)},
            "visibility": {"name": (self.text, True), "states": (states, True)},
            "lookup": {"name": (self.text, True), "rows": (rows, True)},
        }
        acts = {"parameter": (self.text, True), "pieces": (self.ids, True)}
        actions = {
            "move": acts, "scale": acts, "rotate": acts, "flip": acts,
            "stretch": {**acts, "min": (self.jpoint, True), "max": (self.jpoint, True)},
            "array": {**acts, "spacing": (self.float, True)},
        }

        def action_list(x):
            if type(x) is list and not x:
                self.fail("bad_value", "boş eylem listesi yazılmaz")
            return self.array(kinded(actions))(x)

        return self.fields({"parameters": (self.array(kinded(parameters)), True), "actions": (action_list, False)})(v)

    def json_value(self, v):
        """A parameter's value in the JSON form: true or false, a float, a text or a point {x, y}."""
        if type(v) in (bool, float, str):
            return v
        if type(v) is dict:
            return self.jpoint(v)
        self.fail("wrong_type", "değer true/false, float, metin ya da {x, y} noktası olmalı")

    def param_values(self, v):
        """An insert's parameter values (schema 59): name → true/false, a float, a text or a point [x, y]; not empty."""
        if type(v) is not dict:
            self.fail("wrong_type", "harita olmalı")
        if not v:
            self.fail("bad_value", "boş parametre değerleri yazılmaz")
        out = {}
        for k, x in v.items():
            self.path.append(k)
            if type(x) in (bool, float, str):
                out[k] = x
            elif type(x) is list:
                out[k] = self.point(x)
            else:
                self.fail("wrong_type", "değer true/false, float, metin ya da [x, y] olmalı")
            self.path.pop()
        return out

    def values_rule(self, fields, dynamic):
        """An insert's values only for its block's parameters, each one of its (docs/adr/0318 §3)."""
        values = fields.get("parameters")
        if values is None:
            return
        problem = "blok dinamik değil" if dynamic is None else values_problem(dynamic, values)
        if problem:
            self.path.append("parameters")
            self.fail("bad_value", f"parametre değerleri: {problem}")

    def card(self, v):
        """A block's card (schema 58): a version from 1, a category and tags trimmed and not blank, each tag once
        (Turkish case folded); an empty tag list is not written."""
        def tags(x):
            if type(x) is list and not x:
                self.fail("bad_value", "boş etiket listesi yazılmaz")
            return self.array(self.text)(x)

        c = self.fields({"category": (self.text, False), "tags": (tags, False), "version": (self.uint(32), True)})(v)
        if c["version"] < 1 or c["version"] > 0xFFFFFFFF:
            self.path.append("version")
            self.fail("bad_value", "sürüm en az 1 olmalı")
        cat = c.get("category")
        if cat is not None and (not cat.strip() or cat.strip() != cat):
            self.path.append("category")
            self.fail("bad_value", f"“{cat}” kategorisi boş ya da başında, sonunda boşluk var")
        seen = set()
        for t in c.get("tags", []):
            if not t.strip() or t.strip() != t:
                self.path.append("tags")
                self.fail("bad_value", f"“{t}” etiketi boş ya da başında, sonunda boşluk var")
            if name_key(t) in seen:
                self.path.append("tags")
                self.fail("bad_value", f"“{t}” etiketi iki kez var")
            seen.add(name_key(t))
        return {k: c[k] for k in ("category", "tags", "version") if k in c}

    def attribute(self, v):
        table = {
            "p": (self.point, True),
            "tag": (self.text, True),
            "value": (self.text, False),
            "height": (self.float, True),
            "prompt": (self.text, False),
            "rotation": (self.float, True),
        }
        if self.texts:
            table.update(text_extras(self, mask=False))
        if self.block_cards:
            # Its options (schema 58, docs/adr/0317 §1): a flag only when on.
            for flag in ("invisible", "constant", "verify", "preset", "movable", "multiline"):
                table[flag] = (self.true_only, False)
            table["boxWidth"] = (self.box_width, False)
            table["field"] = (self.text, False)
        a = self.fields(table)(v)
        problem = attribute_options_problem(a)
        if problem:
            self.fail("bad_value", problem)
        return {k: a[k] for k in ("tag", "prompt", "value", "p", "height", "rotation", "align", "widthFactor", "invisible", "constant", "verify", "preset", "movable", "multiline", "boxWidth", "field") if k in a}

    def true_only(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "kapalı seçenek yazılmaz")
        return True

    def width_factor(self, v):
        x = self.float(v)
        if not (0.0 < x <= MAX_WIDTH_FACTOR):
            self.fail("bad_value", f"genişlik çarpanı {x}; 0'dan büyük, en çok {MAX_WIDTH_FACTOR:g} olmalı")
        return x

    def mask(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "zemin false yazılmaz; zeminsiz yazıda alan yoktur")
        return True

    def box_width(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"çok satırlı yazının kutu genişliği {x:g}; sıfırdan büyük olmalı")
        return x

    def line_spacing(self, v):
        x = self.float(v)
        if not MIN_LINE_SPACING <= x <= MAX_LINE_SPACING:
            self.fail("bad_value", f"çok satırlı yazının satır aralığı {x:g}; {MIN_LINE_SPACING:g} ile {MAX_LINE_SPACING:g} arasında olmalı")
        return x

    def true_flag(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "false yazılmaz; biçimsiz dilimde alan yoktur")
        return True

    def runs(self, v):
        """A multi-line text's letter formats (§6.6, docs/adr/0182): a list that is not empty, each run its range and format."""
        if type(v) is list and not v:
            self.fail("bad_value", "biçim dilimi listesi boş; dilimsiz yazıda alan yazılmaz")
        run = self.fields(
            {
                "start": (self.uint(32), True),
                "end": (self.uint(32), True),
                "bold": (self.true_flag, False),
                "italic": (self.true_flag, False),
                "underline": (self.true_flag, False),
                "script": (self.enum(("super", "sub", "stack") if self.paragraph_layout else ("super", "sub")), False),
                "color": (self.text, False),
            }
        )
        return self.array(run)(v)

    def runs_rules(self, text, runs):
        """The runs inside the text's letters (Unicode scalar values), in order, apart, each with a format, touching runs
        of one format joined (docs/adr/0182 §1)."""
        letters = len(text)
        keys = ("bold", "italic", "underline", "script", "color")
        before = None
        for i, r in enumerate(runs):
            n = i + 1
            if r["start"] >= r["end"] or r["end"] > letters:
                self.fail("bad_value", f"{n}. biçim dilimi {r['start']}–{r['end']}; yazının {letters} harfi içinde, başı sonundan önce olmalı")
            if not any(k in r for k in keys):
                self.fail("bad_value", f"{n}. biçim diliminin biçimi yok; biçimsiz dilim yazılmaz")
            if r.get("script") == "stack" and not any(c in "/#^" for c in text[r["start"]:r["end"]]):
                self.fail("bad_value", f"{n}. biçim dilimi kesir, ama harflerinde ayıraç (/, # ya da ^) yok")
            if r.get("color") == "":
                self.fail("bad_value", f"{n}. biçim diliminin rengi boş")
            if before is not None:
                if r["start"] < before["end"]:
                    self.fail("bad_value", f"{n}. biçim dilimi öncekiyle örtüşüyor; dilimler sıralı ve ayrı olmalı")
                if r["start"] == before["end"] and all(r.get(k) == before.get(k) for k in keys):
                    self.fail("bad_value", f"{n}. biçim dilimi aynı biçimdeki öncekine bitişik; ikisi tek dilimdir")
            before = r

    def layout_of(self, v):
        """A text's indents, tab stops and columns (docs/adr/0324 §1), in its height (a column's height in metres)."""
        def columns(c):
            return self.fields({"count": (self.uint(32), True), "gap": (self.float, True), "height": (self.float, True)})(c)

        floats = lambda a: [self.float(x) for x in self.array(lambda x: x)(a)]  # noqa: E731
        return self.fields({"indent": (self.float, False), "leftIndent": (self.float, False), "tabs": (floats, False), "columns": (columns, False)})(v)

    def text_fields_of(self, v):
        """A text's fields (docs/adr/0325 §1): ranges of its letters bound to an object's or the project's value."""
        def uid(u):
            if type(u) is not str or not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", u):
                self.fail("bad_value", "nesnenin kimliği küçük harfli, tireli bir UUID olmalı")
            return u

        def source(src):
            if type(src) is not dict:
                self.fail("wrong_type", "harita olmalı")
            if src.get("kind") == "object":
                return self.fields({"kind": (self.text, True), "uid": (uid, True), "value": (self.enum(FIELD_OBJECT_VALUES), True), "name": (self.text, False)})(src)
            if src.get("kind") == "project":
                return self.fields({"kind": (self.text, True), "value": (self.enum(FIELD_PROJECT_VALUES), True), "name": (self.text, False)})(src)
            self.fail("bad_value", f"alanın kaynağı {src.get('kind')!r}: object ya da project olmalı")

        fmt = self.fields({"decimals": (self.uint(8), False), "unit": (self.enum(FIELD_UNITS), False), "comma": (self.true_flag, False), "prefix": (self.text, False), "suffix": (self.text, False)})
        one = self.fields({"start": (self.uint(32), True), "end": (self.uint(32), True), "source": (source, True), "format": (fmt, False), "broken": (self.enum(FIELD_BROKEN), False)})
        return self.array(one)(v)

    def text_fields_rules(self, text, fields):
        """The fields as the contract allows them (`text_fields_problem`): some, in the letters, in order, apart, the
        names they need, a format that sets something, a unit of their value's kind, a broken one's letters ####."""
        letters = len(text)
        if not fields:
            self.fail("bad_value", "boş alan listesi yazılmaz")
        if len(fields) > MAX_FIELDS:
            self.fail("bad_value", f"bir yazıda en çok {MAX_FIELDS} alan olur")
        last = 0
        for i, f in enumerate(fields):
            n = i + 1
            if f["start"] >= f["end"] or f["end"] > letters or (i > 0 and f["start"] < last):
                self.fail("bad_value", f"{n}. alan {f['start']}–{f['end']}: yazının {letters} harfi içinde, sırayla ve ayrı olmalı")
            last = f["end"]
            src = f["source"]
            if self.field_source_problem(src):
                self.fail("bad_value", f"{n}. alan: {self.field_source_problem(src)}")
            fm = f.get("format")
            if fm is not None:
                if fm.get("decimals", 0) > 10:
                    self.fail("bad_value", f"{n}. alan: basamak en çok 10 olur")
                if not fm:
                    self.fail("bad_value", f"{n}. alan: boş biçim yazılmaz")
                unit = fm.get("unit")
                if unit is not None:
                    if src["kind"] == "project":
                        self.fail("bad_value", f"{n}. alan: projenin değeri birimle yazılmaz")
                    number = src["value"] in ("area", "perimeter", "length", "x", "y")
                    if not number or (unit in ("m2", "donum", "ha")) != (src["value"] == "area"):
                        self.fail("bad_value", f"{n}. alan: {src['value']} değeri {unit} biriminde yazılamaz")
            if "broken" in f and text[f["start"]:f["end"]] != "####":
                self.fail("bad_value", f"{n}. alan kopuk: harfleri #### olmalı")

    def layout_rules(self, layout, boxed):
        """The layout as the contract allows it (`ParagraphLayout::problem`): not empty, in range, columns in a box."""
        fits = lambda x: math.isfinite(x) and abs(x) <= MAX_PARAGRAPH_HEIGHTS  # noqa: E731
        if not (layout.get("indent") or layout.get("leftIndent") or layout.get("tabs") or "columns" in layout):
            self.fail("bad_value", "boş yazı düzeni yazılmaz; düzensiz yazıda alan yoktur")
        left, first = layout.get("leftIndent", 0.0), layout.get("indent", 0.0)
        if not fits(left) or left < 0.0:
            self.fail("bad_value", f"sol girinti {left}: 0 ile {MAX_PARAGRAPH_HEIGHTS:g} yükseklik arasında olmalı")
        if not fits(first) or left + first < 0.0:
            self.fail("bad_value", f"ilk satırın girintisi {first}: sol girintiden fazla geri gidemez")
        if layout.get("indent") == 0.0 or layout.get("leftIndent") == 0.0:
            self.fail("bad_value", "sıfır girinti yazılmaz")
        tabs = layout.get("tabs", [])
        if len(tabs) > MAX_TABS:
            self.fail("bad_value", f"en çok {MAX_TABS} sekme durağı olur")
        last = 0.0
        for t in tabs:
            if not fits(t) or t <= last:
                self.fail("bad_value", "sekme durakları sıfırdan büyük, küçükten büyüğe ve birer kez olmalı")
            last = t
        c = layout.get("columns")
        if c is not None:
            if not boxed:
                self.fail("bad_value", "sütunlar kutu genişliği ister")
            if not 2 <= c["count"] <= 16:
                self.fail("bad_value", f"sütun sayısı {c['count']}: 2 ile 16 arasında olmalı")
            if not fits(c["gap"]) or c["gap"] < 0.0:
                self.fail("bad_value", f"sütun aralığı {c['gap']}: sıfır ya da pozitif olmalı")
            if not (math.isfinite(c["height"]) and c["height"] > 0.0):
                self.fail("bad_value", f"sütun yüksekliği {c['height']}: sıfırdan büyük olmalı")

    def label_scale(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"bağlı yazının ölçeği 1:{x:g}; sıfırdan büyük olmalı")
        return x

    def dimension_mask(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "zemin false yazılmaz; zeminsiz ölçüde alan yoktur")
        return True

    def annotative_of(self, kind):
        """An annotation's scales (docs/adr/0323): its own scale, the others ascending, each with what its kind sets apart."""
        values = {"dimension": ("offset", "textShift"), "hatch": ()}.get(kind, ("point", "rotation"))

        def scale(v):
            x = self.float(v)
            if not (math.isfinite(x) and 0.0 < x <= MAX_ANNOTATIVE_SCALE):
                self.fail("bad_value", f"ölçek {x} olamaz: sıfırdan büyük, en çok {MAX_ANNOTATIVE_SCALE:g}")
            return x

        def entry(v):
            # A point as JSON writes it: {x, y} (the scales are their JSON, as the writer writes them).
            xy = self.fields({"x": (self.float, True), "y": (self.float, True)})
            table = {"scale": (scale, True), "point": (xy, False), "rotation": (self.float, False),
                     "offset": (self.float, False), "textShift": (self.text_shift, False)}
            e = self.fields(table)(v)
            for key in e:
                if key != "scale" and key not in values:
                    self.path.append(key)
                    self.fail("bad_value", f"{key} bu nesnenin ölçeğinde olmaz")
            for key in ("rotation", "offset"):
                if key in e and not math.isfinite(e[key]):
                    self.path.append(key)
                    self.fail("bad_value", "sonlu bir sayı olmalı")
            return e

        def read(v):
            a = self.fields({"at": (scale, True), "scales": (self.array(entry), False)})(v)
            scales = a.get("scales")
            if scales is not None:
                if not scales:
                    self.fail("bad_value", "boş ölçek listesi yazılmaz")
                if len(scales) > MAX_ANNOTATIVE_SCALES:
                    self.fail("bad_value", f"kendi ölçeğinden başka en çok {MAX_ANNOTATIVE_SCALES} ölçek olur")
                last = None
                for e in scales:
                    if abs(e["scale"] - a["at"]) <= 1e-9 * max(e["scale"], a["at"]):
                        self.fail("bad_value", "nesnenin kendi ölçeği öbür ölçeklerde yinelenmez")
                    if last is not None and (e["scale"] <= last or abs(e["scale"] - last) <= 1e-9 * e["scale"]):
                        self.fail("bad_value", "ölçekler küçükten büyüğe ve birer kez yazılır")
                    last = e["scale"]
            return a

        return read

    def overrides_of(self, names):
        """A text's or a dimension's overrides (docs/adr/0322 §5): not empty, each one of the kind's names, once."""
        def read(v):
            o = self.array(self.text)(v)
            if not o:
                self.fail("bad_value", "boş geçersiz kılma listesi yazılmaz")
            for i, n in enumerate(o):
                if n not in names:
                    self.fail("bad_value", f"“{n}” stilin bir alanı değil")
                if n in o[:i]:
                    self.fail("bad_value", f"“{n}” geçersiz kılınanlarda iki kez var")
            return o
        return read

    def dimension_options(self, v):
        """A dimension's options (docs/adr/0321 §1, §2): none empty; an increment over 0, a break size over 0 and at most
        100, a tolerance's values 0 or more (a symmetric one has no lower), at most 8 decimals; lengths at most 10⁶ m."""
        o = self.fields({
            "fit": (self.enum(("auto", "outside")), False),
            "round": (self.float, False),
            "zeros": (self.enum(("leading", "trailing", "both")), False),
            "lengthStyle": (self.enum(("decimal", "architectural", "engineering", "fractional")), False),
            "alt": (self.fields({"unit": (self.enum(("mm", "cm", "m", "in", "ft")), True), "decimals": (self.uint(32), False)}), False),
            "tolerance": (self.fields({"kind": (self.enum(("symmetric", "deviation", "limits")), True), "upper": (self.float, True),
                                       "lower": (self.float, False), "decimals": (self.uint(32), False)}), False),
            "breakSize": (self.float, False),
        })(v)
        bad = lambda words: self.fail("bad_value", words)  # noqa: E731
        holds = lambda x, positive: math.isfinite(x) and abs(x) <= 1e6 and (x > 0 if positive else x >= 0)  # noqa: E731
        if not o:
            bad("Ölçünün seçenekleri boş: hiçbiri yoksa alan yazılmaz.")
        if "round" in o and not holds(o["round"], True):
            bad("yuvarlama artışı sıfırdan büyük, en çok 10⁶ m olmalı")
        if "breakSize" in o and not (math.isfinite(o["breakSize"]) and 0 < o["breakSize"] <= 100):
            bad("kırma genişliği sıfırdan büyük, en çok 100 olmalı")
        if o.get("alt", {}).get("decimals", 0) > 8:
            bad("alternatif birimin basamağı en çok 8")
        t = o.get("tolerance")
        if t is not None:
            if not holds(t["upper"], False) or ("lower" in t and not holds(t["lower"], False)):
                bad("toleransın değerleri 0 ya da büyük olmalı")
            if t["kind"] == "symmetric" and "lower" in t:
                bad("simetrik toleransın alt değeri yoktur")
            if t.get("decimals", 0) > 8:
                bad("toleransın basamağı en çok 8")
        return o

    def text_shift(self, v):
        s = self.fields({"along": (self.float, True), "across": (self.float, True)})(v)
        if not all(math.isfinite(x) and abs(x) <= 1e6 for x in s.values()):
            self.fail("bad_value", "değerin kayması sonlu ve en çok 10⁶ m olmalı")
        return s

    def dimension_breaks(self, v):
        b = self.array(self.fields({"line": (self.enum(("dimension", "first", "second")), True), "t": (self.float, True)}))(v)
        if not b:
            self.fail("bad_value", "boş kırma listesi yazılmaz")
        if len(b) > 64:
            self.fail("bad_value", f"ölçünün {len(b)} kırması var; en çok 64")
        if any(not 0.0 <= x["t"] <= 1.0 for x in b):
            self.fail("bad_value", "kırmanın yeri (t) 0 ile 1 arasında olmalı")
        return b

    def dimension_rules(self, d):
        """What a dimension's style needs, and what only a slope has (docs/adr/0147)."""
        style = d.get("style")
        needs = ["c"] if style in ("arcLength", "jogged") else ["za", "zb"] if style == "slope" else ["za"] if style == "elevation" else []
        for key in needs:
            if key not in d:
                self.path.append(key)
                self.fail("missing_field", "zorunlu alan yok")
        # A level's one elevation (docs/adr/0322).
        if style == "elevation" and "zb" in d:
            self.path.append("zb")
            self.fail("bad_value", "kot ölçüsünün yalnız bir kotu (za) olur")
        if style not in ("slope", "elevation"):
            for key in ("za", "zb"):
                if key in d:
                    self.path.append(key)
                    self.fail("bad_value", "kot (za, zb) yalnız eğim ve kot ölçüsünde yazılır")
        if style == "ordinate" and "angle" in d and d["angle"] not in (0.0, 90.0):
            self.path.append("angle")
            self.fail("bad_value", f"koordinat ölçüsünün ekseni {d['angle']}; 0 (Y) ya da 90 (X) olmalı")

    # Tables (schema 22, spec §6.6, docs/adr/0184).

    def merges(self, v):
        ranges = self.array(self.fields({k: (self.uint(32), True) for k in ("row", "col", "rows", "cols")}))(v)
        if not ranges:
            self.fail("bad_value", "birleşik alan listesi boş; birleşik alanı olmayan tabloda alan yazılmaz")
        return ranges

    def table_header(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "başlık false yazılmaz; başlıksız tabloda alan yoktur")
        return True

    def table_source(self, v):
        d = self.fields(
            {
                "kind": (self.enum(("coordinates", "areas", "attributes", "file")), True),
                "objects": (self.array(self.id16), False),
                "name": (self.text, False),
                "sheet": (self.text, False),
            }
        )(v)
        if d["kind"] == "file":
            if "objects" in d:
                self.fail("bad_value", "dosyadan gelen tablonun kaynağında nesne yazılmaz")
            if "name" not in d:
                self.path.append("name")
                self.fail("missing_field", "“name” alanı yok")
        else:
            if "name" in d or "sheet" in d:
                self.fail("bad_value", "nesnelerden gelen tablonun kaynağında dosya adı yazılmaz")
            if "objects" not in d:
                self.path.append("objects")
                self.fail("missing_field", "“objects” alanı yok")
        return d

    def text_path(self, v):
        """A text's curve (schema 25, §6.6, docs/adr/0196 §1): its vertices after its point, in its frame, and each
        edge's bulge."""
        return self.fields({"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False)})(v)

    def text_path_rules(self, t):
        """The contract's `text_path_problem` (kentos_contracts::entity), in its order; the numbers are finite already."""
        c = t["path"]
        if not c["pts"]:
            self.fail("bad_value", "Eğri boyunca yazının eğrisinde köşe yok.")
        if "bulges" in c and len(c["bulges"]) != len(c["pts"]):
            self.fail("bad_value", f"Eğri boyunca yazının kavis sayısı ({len(c['bulges'])}) köşe sayısından ({len(c['pts'])}) farklı.")
        if all(q["x"] == 0.0 and q["y"] == 0.0 for q in c["pts"]):
            self.fail("bad_value", "Eğri boyunca yazının eğrisinin uzunluğu sıfır: bütün köşeleri yazının noktasında.")
        if "\n" in t["text"] or "boxWidth" in t or "lineSpacing" in t:
            self.fail("bad_value", "Eğri boyunca yazı tek satırdır: satır sonu, kutu genişliği ve satır aralığı olmaz.")
        if "labelOf" in t:
            self.fail("bad_value", "Nesneye bağlı yazının eğrisi olmaz.")

    def image_rules(self, i):
        """The contract's `ImageFields::problem` (kentos_contracts::image), in its order; the numbers are finite already."""
        if not (0.0 < i["width"] <= MAX_IMAGE_SIZE and 0.0 < i["height"] <= MAX_IMAGE_SIZE):
            self.fail("bad_value", f"Resmin genişliği ve yüksekliği sıfırdan büyük ve en çok {MAX_IMAGE_SIZE:g} m olmalı.")
        asset = i.get("asset", "").strip() or None
        file = i.get("file", "").strip() or None
        if asset and file:
            self.fail("bad_value", "Resmin kaynağı ya gömülü varlık (asset) ya bağlı dosya (file) olmalı, ikisi birden değil.")
        if not ((asset and "file" not in i) or (file and "asset" not in i)):
            self.fail("bad_value", "Resmin kaynağı yok: gömülü varlığın kimliğini (asset) ya da bağlı dosyanın yolunu (file) verin.")
        if file and (len(file) > MAX_IMAGE_PATH or any(ord(c) < 32 or 0x7F <= ord(c) <= 0x9F for c in file)):
            self.fail("bad_value", f"Bağlı dosyanın yolu en çok {MAX_IMAGE_PATH} harf olmalı ve denetim karakteri içermemeli.")
        clip = i.get("clip")
        if clip is not None and not (3 <= len(clip) <= MAX_CLIP_CORNERS and all(0.0 <= q["x"] <= 1.0 and 0.0 <= q["y"] <= 1.0 for q in clip)):
            self.fail("bad_value", f"Resmin kırpma sınırı en az 3, en çok {MAX_CLIP_CORNERS} köşe olmalı, köşeleri resmin kesirleriyle 0 ile 1 arasında.")
        o = i.get("opacity")
        if o is not None and not (MIN_IMAGE_OPACITY <= o <= MAX_IMAGE_OPACITY):
            self.fail("bad_value", f"Resmin donukluğu {MIN_IMAGE_OPACITY} ile {MAX_IMAGE_OPACITY} arasında olmalı; {o} verildi.")

    def raster_side(self, most):
        def check(v):
            n = self.uint(32)(v)
            if n > most:
                self.fail("bad_value", f"{n} aralık dışında (0…{most})")
            return n

        return check

    def raster_affine(self, v):
        a = self.array(self.float)(v)
        if len(a) != 6:
            self.fail("bad_value", f"rasterin dönüşümü 6 sayı olmalı, {len(a)} var")
        return a

    def raster_style(self, v):
        """A raster's look (§6.6, docs/adr/0204 §4): `render` and `bands`; the rest only when not their default."""
        def flag(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur")
            return True

        def stretch(v):
            if self.text(v) == "none":
                self.fail("bad_value", "gerdirme “none” yazılmaz; gerdirmesiz görünüşte alan yoktur")
            return self.enum(RASTER_STRETCHES)(v)

        def resampling(v):
            if self.text(v) == "bilinear":
                self.fail("bad_value", "örnekleme “bilinear” yazılmaz; çift doğrusal görünüşte alan yoktur")
            return self.enum(RASTER_RESAMPLINGS)(v)

        return self.fields(
            {
                "render": (self.enum(RASTER_RENDERS), True),
                "bands": (self.array(self.raster_side(MAX_RASTER_BANDS)), True),
                "stretch": (stretch, False),
                "min": (self.float, False),
                "max": (self.float, False),
                "ramp": (self.text, False),
                "invert": (flag, False),
                "azimuth": (self.float, False),
                "altitude": (self.float, False),
                "zFactor": (self.float, False),
                "nodata": (self.float, False),
                "resampling": (resampling, False),
                # Schema 38 (docs/adr/0243 §5): a mesh's lines.
                **({"edges": (self.text, False)} if self.multidim else {}),
            }
        )(v)

    def raster_dataset(self, v):
        """A raster's NetCDF variable and slice (§6.6, docs/adr/0243 §6): `variable`; `vector`, `mesh`, `dims` and
        `followTime` only when given (no empty list, no false)."""
        def dims(v):
            d = self.array(self.dataset_dim)(v)
            if not d:
                self.fail("bad_value", "boş dilim boyutu listesi yazılmaz")
            return d

        def flag(words):
            def check(v):
                if self.bool(v) is not True:
                    self.fail("bad_value", words)
                return True
            return check

        return self.fields(
            {
                "variable": (self.text, True),
                "vector": (self.text, False),
                "mesh": (self.text, False),
                "dims": (dims, False),
                "followTime": (flag("followTime false yazılmaz"), False),
            }
        )(v)

    def dataset_dim(self, v):
        def flag(v):
            if self.bool(v) is not True:
                self.fail("bad_value", "time false yazılmaz")
            return True

        return self.fields(
            {
                "name": (self.text, True),
                "index": (self.uint(32), True),
                "values": (self.array(self.float), True),
                "time": (flag, False),
                "units": (self.text, False),
            }
        )(v)

    def raster_rules(self, r):
        """The contract's `RasterFields::problem` and `RasterStyle::problem` (kentos_contracts::raster), in their order;
        the numbers are finite already."""
        x0, a, b, y0, c, d = r["affine"]
        det = a * d - b * c
        if not (math.isfinite(det) and det != 0.0):
            self.fail("bad_value", "Rasterin dönüşümü tersinmiyor: pikselin iki kenarı aynı doğrultuda ya da sıfır.")
        if not (1 <= r["width"] <= MAX_RASTER_SIDE and 1 <= r["height"] <= MAX_RASTER_SIDE):
            self.fail("bad_value", f"Rasterin genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı.")
        if not 1 <= r["bands"] <= MAX_RASTER_BANDS:
            self.fail("bad_value", f"Rasterin 1 ile {MAX_RASTER_BANDS} arasında bandı olmalı.")
        asset = r.get("asset", "").strip() or None
        file = r.get("file", "").strip() or None
        url = r.get("url", "").strip() or None
        # One source given, the others absent (schema 31's address beside the asset and the file, docs/adr/0207 §1).
        given = (asset is not None) + (file is not None) + (url is not None)
        named = ("asset" in r) + ("file" in r) + ("url" in r)
        if given == 0:
            self.fail("bad_value", "Rasterin kaynağı yok: gömülü varlığın kimliğini (asset), bağlı dosyanın yolunu (file) ya da adresini (url) verin.")
        if given != 1 or named != 1:
            self.fail("bad_value", "Rasterin kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.")
        if url:
            self.url_rule(url)
        if file and (len(file) > MAX_RASTER_PATH or any(ord(ch) < 32 or 0x7F <= ord(ch) <= 0x9F for ch in file)):
            self.fail("bad_value", f"Bağlı dosyanın yolu en çok {MAX_RASTER_PATH} harf olmalı ve denetim karakteri içermemeli.")
        st = r["style"]
        need, alpha = (3, True) if st["render"] == "rgb" else (1, False)
        n = len(st["bands"])
        if not (n == need or (alpha and n == need + 1)):
            self.fail("bad_value", "RGB görünüş üç bant ister (dördüncüsü alfa olabilir)." if st["render"] == "rgb" else "Bu görünüş tek bant ister.")
        wrong = [x for x in st["bands"] if x == 0 or x > r["bands"]]
        if wrong:
            self.fail("bad_value", f"Rasterin {r['bands']} bandı var; {wrong[0]}. bant gösterilemez.")
        if st.get("stretch") == "manual" and not ("min" in st and "max" in st and st["min"] < st["max"]):
            self.fail("bad_value", "Elle gerdirmenin en küçüğü ve en büyüğü sonlu sayılar olmalı, en küçük en büyükten küçük.")
        if "ramp" in st and st["ramp"] not in RASTER_RAMPS:
            self.fail("bad_value", f"“{st['ramp']}” diye bir renk rampası yok; {', '.join(RASTER_RAMPS)} rampalarından biri seçilmeli.")
        az, alt, z = st.get("azimuth", 315.0), st.get("altitude", 45.0), st.get("zFactor", 1.0)
        if not (0.0 <= az <= 360.0 and 0.0 <= alt <= 90.0 and z > 0.0):
            self.fail("bad_value", "Gölgeli kabartmanın ışığı 0–360° doğrultudan, 0–90° yükseklikten gelmeli; yükseklik çarpanı sıfırdan büyük olmalı.")
        if "edges" in st:
            c = st["edges"]
            if not (len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:])):
                self.fail("bad_value", f"Ağ çizgilerinin rengi #RRGGBB olmalı; “{c}” verildi.")
        d = r.get("dataset")
        if d is not None:
            self.dataset_rules(d)
            if r["bands"] != 1:
                self.fail("bad_value", "Veri setini gösteren rasterin tek bandı olur.")
        if "edges" in st and (d is None or "mesh" not in d):
            self.fail("bad_value", "Ağ çizgileri yalnız mesh'te çizilir.")
        o = r.get("opacity")
        if o is not None and not (MIN_RASTER_OPACITY <= o <= MAX_RASTER_OPACITY):
            self.fail("bad_value", f"Rasterin donukluğu {MIN_RASTER_OPACITY} ile {MAX_RASTER_OPACITY} arasında olmalı; {o} verildi.")
        g = r.get("georef")
        if g is not None:
            self.path.append("georef")
            self.georef_rules(g, r)
            self.path.pop()
            if d is not None:
                self.fail("bad_value", "Veri setini gösteren raster oturtulmaz; oturtma kaydı olamaz.")
        t = r.get("tiles")
        if t is not None:
            if "file" not in r or not r["file"].strip():
                self.fail("bad_value", "Karo tablosunu gösteren rasterin kaynağı bağlı GeoPackage dosyasıdır (file); gömülü ya da adresten olamaz.")
            name = t["table"]
            if not name.strip() or len(name) > MAX_TILE_TABLE_NAME or any(unicodedata.category(c) == "Cc" for c in name):
                self.fail("bad_value", f"Karo tablosunun adı boş olamaz, en çok {MAX_TILE_TABLE_NAME} harftir ve denetim karakteri içermez.")
            if d is not None:
                self.fail("bad_value", "Karo tablosunu gösteren raster veri seti gösteremez.")
            if not ((r["bands"] == 4 and r["sample"] == "u8") or (r["bands"] == 1 and r["sample"] in ("u16", "i16", "f32"))):
                self.fail("bad_value", "Karo tablosunu gösteren raster ya 4 bantlı 8 bitlik (resim) ya da tek bantlı U16, I16 veya F32 (kapsam) olur.")

    def raster_tiles(self, v):
        """A raster's GeoPackage tile table (§6.6, docs/adr/0627 §3): its table, the zoom of the raster's own pixels and
        the raster's first column and row in that zoom's matrix of pixels (not negative)."""
        return self.fields({"table": (self.text, True), "zoom": (self.uint(32), True),
                            "x": (self.uint(63), True), "y": (self.uint(63), True)})(v)

    def raster_georef(self, v):
        """A raster's georeference record (§6.6, docs/adr/0518 §1), the contract's JSON: the method, the points (pixel
        and target pairs of floats; `check` and `off` only when true; a name when given), the targets' system when not
        the project's, the source (size, one of asset, file, url; its SHA-256 as hex), m0 and the check points' errors
        when there are, and a resampled raster's output grid (`resampling` only when nearest)."""
        def pair(x):
            a = self.array(self.float)(x)
            if len(a) != 2:
                self.fail("bad_value", f"2 sayı olmalı, {len(a)} var")
            return a

        def six(x):
            a = self.array(self.float)(x)
            if len(a) != 6:
                self.fail("bad_value", f"6 sayı olmalı, {len(a)} var")
            return a

        point = self.fields({"name": (self.text, False), "pixel": (pair, True), "target": (pair, True),
                             "check": (self.only_true, False), "off": (self.only_true, False)})
        source = self.fields({"width": (self.uint(32), True), "height": (self.uint(32), True), "asset": (self.text, False),
                              "file": (self.text, False), "url": (self.text, False), "digest": (self.text, False)})
        output = self.fields({"affine": (six, True), "width": (self.uint(32), True), "height": (self.uint(32), True),
                              "pixel": (self.float, True), "resampling": (self.enum(("nearest",)), False),
                              "digest": (self.text, False)})
        return self.fields({"method": (self.enum(GEOREF_METHODS), True), "points": (self.array(point), True),
                            "srid": (self.uint(32), False), **({"crs": (self.text, False)} if self.georef_crs else {}),
                            "source": (source, True), "m0": (self.float, False),
                            "checkRmse": (self.float, False), "checkMax": (self.float, False),
                            "output": (output, False)})(v)

    def georef_rules(self, g, r):
        """The contract's `RasterGeoref::problem` and `misfit` (kentos_contracts::raster_georef), in their order."""
        pts = g["points"]
        if not 1 <= len(pts) <= MAX_GEOREF_POINTS:
            self.fail("bad_value", f"Kaydın 1 ile {MAX_GEOREF_POINTS} arasında noktası olmalı.")
        for p in pts:
            if not all(math.isfinite(x) for x in p["pixel"] + p["target"]):
                self.fail("bad_value", "Noktanın pikseli ve hedefi sonlu sayılar olmalı.")
            if p.get("check") and p.get("off"):
                self.fail("bad_value", "Kullanılmayan nokta denetim noktası olamaz; denetim noktası Kullan'da kalır.")
            n = p.get("name")
            if n is not None and (n.strip() == "" or n.strip() != n or len(n) > MAX_GEOREF_NAME or any(unicodedata.category(ch) == "Cc" for ch in n)):
                self.fail("bad_value", f"Noktanın adı kırpılmış, boş olmayan ve en çok {MAX_GEOREF_NAME} harf olmalı.")
        control = sum(1 for p in pts if not p.get("check") and not p.get("off"))
        checks = sum(1 for p in pts if p.get("check"))
        if control < GEOREF_NEED[g["method"]]:
            self.fail("bad_value", f"{g['method']} en az {GEOREF_NEED[g['method']]} kontrol noktası ister; kayıtta {control} var.")
        if g.get("srid") == 0:
            self.fail("bad_value", "Hedeflerin sistemi verildiyse bir EPSG kodu olmalı.")
        c = g.get("crs")
        if c is not None and "srid" in g:
            self.fail("bad_value", "Hedeflerin sistemi ya EPSG kodu (srid) ya tanım metni (crs) olarak verilir, ikisi birden değil.")
        if c is not None and (c.strip() == "" or len(c.encode()) > MAX_GEOREF_CRS):
            self.fail("bad_value", f"Hedeflerin sisteminin tanımı boş olmamalı, en çok {MAX_GEOREF_CRS // 1024} KB olmalı.")
        s = g["source"]
        if not (1 <= s["width"] <= MAX_RASTER_SIDE and 1 <= s["height"] <= MAX_RASTER_SIDE):
            self.fail("bad_value", f"Kaynağın genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı.")
        named = [s[k] for k in ("asset", "file", "url") if k in s]
        if len(named) != 1:
            self.fail("bad_value", "Kaynak ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.")
        if any(t.strip() == "" or len(t) > MAX_RASTER_PATH or any(unicodedata.category(ch) == "Cc" for ch in t) for t in named):
            self.fail("bad_value", f"Kaynağın adı boş olmamalı, en çok {MAX_RASTER_PATH} harf olmalı.")

        def digest_bad(d):
            return d is not None and (len(d) != 64 or any(ch not in "0123456789abcdef" for ch in d))

        if digest_bad(s.get("digest")):
            self.fail("bad_value", "SHA-256 özeti 64 küçük onaltılık hane olmalı.")
        for k in ("m0", "checkRmse", "checkMax"):
            if k in g and not (math.isfinite(g[k]) and g[k] >= 0.0):
                self.fail("bad_value", "m0 ve denetim hataları sonlu ve eksi olmayan sayılar olmalı.")
        if ("checkRmse" in g) != ("checkMax" in g) or ("checkRmse" in g) != (checks > 0):
            self.fail("bad_value", "Denetim noktası varken denetimin karesel ortalama hatası ve en büyük hatası birlikte yazılır, yokken hiçbiri.")
        places = g["method"] in ("helmert", "affine")
        o = g.get("output")
        if o is not None and places:
            self.fail("bad_value", "Bu yöntem rasteri yeniden örneklemez; çıktı ızgarası yazılmaz.")
        if o is None and not places:
            self.fail("bad_value", "Bu yöntem rasteri yeniden örnekler; çıktı ızgarası yazılır.")
        if o is not None:
            x0, a, b, y0, c, d = o["affine"]
            px = o["pixel"]
            if not (math.isfinite(px) and px > 0.0 and math.isfinite(x0) and math.isfinite(y0) and a == px and b == 0.0 and c == 0.0 and d == -px):
                self.fail("bad_value", "Çıktı ızgarası kuzeye bakan ve pikseli kare olmalı: [x₀, s, 0, y₀, 0, −s].")
            if not (1 <= o["width"] <= MAX_RASTER_SIDE and 1 <= o["height"] <= MAX_RASTER_SIDE):
                self.fail("bad_value", f"Çıktının genişliği ve yüksekliği 1 ile {MAX_RASTER_SIDE} piksel arasında olmalı.")
            if digest_bad(o.get("digest")):
                self.fail("bad_value", "SHA-256 özeti 64 küçük onaltılık hane olmalı.")
            if o["affine"] != r["affine"] or o["width"] != r["width"] or o["height"] != r["height"]:
                self.fail("bad_value", "Kaydın çıktı ızgarası rasterin dönüşümü ve boyu değil.")
        elif s["width"] != r["width"] or s["height"] != r["height"]:
            self.fail("bad_value", "Kaydın kaynağının boyu rasterin boyu değil.")

    def dataset_rules(self, d):
        """The contract's `RasterDataset::problem` (kentos_contracts::raster), in its order."""
        def name_ok(t):
            return t.strip() != "" and len(t) <= MAX_DATASET_NAME and not any(unicodedata.category(ch) == "Cc" for ch in t)

        if not name_ok(d["variable"]) or ("vector" in d and not name_ok(d["vector"])) or ("mesh" in d and not name_ok(d["mesh"])):
            self.fail("bad_value", f"Veri setinin değişken adları 1 ile {MAX_DATASET_NAME} harf arasında olmalı ve denetim karakteri içermemeli.")
        dims = d.get("dims", [])
        if len(dims) > MAX_DATASET_DIMS:
            self.fail("bad_value", f"Veri setinin en çok {MAX_DATASET_DIMS} dilim boyutu olabilir.")
        for dim in dims:
            if not name_ok(dim["name"]):
                self.fail("bad_value", "Dilim boyutunun adı boş olamaz.")
            vals = dim["values"]
            if not vals or len(vals) > MAX_DIM_VALUES or not all(math.isfinite(x) for x in vals):
                self.fail("bad_value", f"“{dim['name']}” boyutunun 1 ile {MAX_DIM_VALUES} arasında sonlu değeri olmalı.")
            if dim["index"] >= len(vals):
                self.fail("bad_value", f"“{dim['name']}” boyutunun {len(vals)} değeri var; gösterilen {dim['index'] + 1}. değer yok.")
            if dim.get("time") and any(vals[k + 1] < vals[k] for k in range(len(vals) - 1)):
                self.fail("bad_value", f"“{dim['name']}” zaman boyutunun değerleri azalmamalı.")
        if sum(1 for dim in dims if dim.get("time")) > 1:
            self.fail("bad_value", "Veri setinin en çok bir zaman boyutu olabilir.")
        if d.get("followTime") and not any(dim.get("time") for dim in dims):
            self.fail("bad_value", "Zaman sürgüsünü izlemek için veri setinin zaman boyutu olmalı.")

    def url_rule(self, u):
        """The contract's `url_problem`: an HTTP or HTTPS address, its letters few and printable."""
        lower = u.strip().lower()
        if not (lower.startswith("http://") or lower.startswith("https://")) or len(u.strip()) < 9:
            self.fail("bad_value", f"“{u}” bir HTTP ya da HTTPS adresi değil (http:// ya da https:// ile başlamalı).")
        if len(u) > MAX_CLOUD_PATH or any(ord(ch) < 32 or 0x7F <= ord(ch) <= 0x9F for ch in u):
            self.fail("bad_value", f"Adres en çok {MAX_CLOUD_PATH} harf olmalı ve denetim karakteri içermemeli.")

    def bounds6(self, v):
        a = self.array(self.float)(v)
        if len(a) != 6:
            self.fail("bad_value", f"kapsam 6 sayı olmalı, {len(a)} var")
        return a

    def cloud_source(self, v):
        """A point cloud's file (§6.6, docs/adr/0207 §3): its one source, its format, points and bounds."""
        return self.fields(
            {
                "asset": (self.text, False),
                "file": (self.text, False),
                "url": (self.text, False),
                "format": (self.enum(CLOUD_FORMATS), True),
                "count": (self.uint(64), True),
                "bounds": (self.bounds6, True),
            }
        )(v)

    def cloud_style(self, v):
        """A point cloud's look (§6.6, docs/adr/0207 §5): `render` and `size`; the rest only when not their default."""
        def only_true(words):
            def check(v):
                if self.bool(v) is not True:
                    self.fail("bad_value", words)
                return True

            return check

        def hidden(v):
            h = self.array(lambda x: self.uint(8)(x))(v)
            if not h:
                self.fail("bad_value", "gizlenen sınıf listesi boş; gizlenen sınıf yoksa alan yazılmaz")
            return h

        def size_unit(v):
            if self.text(v) == "px":
                self.fail("bad_value", "boyun birimi “px” yazılmaz; piksel boyda alan yoktur")
            return self.enum(("m",))(v)

        def shape(v):
            if self.text(v) == "round":
                self.fail("bad_value", "biçim “round” yazılmaz; yuvarlak noktada alan yoktur")
            return self.enum(("square",))(v)

        return self.fields(
            {
                "render": (self.enum(CLOUD_RENDERS), True),
                "ramp": (self.text, False),
                "invert": (only_true("ters çevirme false yazılmaz; çevrilmemiş rampada alan yoktur"), False),
                "min": (self.float, False),
                "max": (self.float, False),
                "hidden": (hidden, False),
                "rgb8": (only_true("rgb8 false yazılmaz; 16 bitlik renklerde alan yoktur"), False),
                "size": (self.float, True),
                "sizeUnit": (size_unit, False),
                "shape": (shape, False),
            }
        )(v)

    def cloud_rules(self, c):
        """The contract's `PointCloudFields::problem` and `PointCloudStyle::problem` (kentos_contracts::pointcloud), in
        their order; the numbers are finite already."""
        def bad(b):
            return not (b[0] <= b[3] and b[1] <= b[4] and b[2] <= b[5])

        sources = c["sources"]
        if not sources:
            self.fail("bad_value", "Nokta bulutunun en az bir dosyası olmalı.")
        if len(sources) > MAX_CLOUD_SOURCES:
            self.fail("bad_value", f"Sanal bulutun en çok {MAX_CLOUD_SOURCES} dosyası olabilir.")
        for i, f in enumerate(sources, 1):
            given = [f.get(k, "").strip() or None for k in ("asset", "file", "url")]
            if sum(g is not None for g in given) != 1 or sum(k in f for k in ("asset", "file", "url")) != 1:
                self.fail("bad_value", f"{i}. dosyanın kaynağı ya gömülü varlık (asset), ya bağlı dosya (file), ya adres (url) olmalı; yalnız biri.")
            path = given[1]
            if path and (len(path) > MAX_CLOUD_PATH or any(ord(ch) < 32 or 0x7F <= ord(ch) <= 0x9F for ch in path)):
                self.fail("bad_value", f"Bağlı dosyanın yolu en çok {MAX_CLOUD_PATH} harf olmalı ve denetim karakteri içermemeli.")
            if given[2]:
                self.url_rule(given[2])
            if bad(f["bounds"]):
                self.fail("bad_value", f"{i}. dosyanın kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.")
        if bad(c["bounds"]):
            self.fail("bad_value", "Nokta bulutunun kapsamı sonlu altı sayı olmalı, her eksende en küçük en büyükten büyük olmamalı.")
        if sum(f["count"] for f in sources) != c["count"]:
            self.fail("bad_value", "Nokta bulutunun nokta sayısı dosyalarınkinin toplamı olmalı.")
        st = c["style"]
        if "ramp" in st and st["ramp"] not in RASTER_RAMPS:
            self.fail("bad_value", f"“{st['ramp']}” diye bir renk rampası yok; {', '.join(RASTER_RAMPS)} rampalarından biri seçilmeli.")
        lo, hi = st.get("min"), st.get("max")
        if (lo is None) != (hi is None) or (lo is not None and not lo < hi):
            self.fail("bad_value", "Görünüşün aralığı iki sonlu sayı olmalı, en küçük en büyükten küçük.")
        if st["render"] in ("elevation", "intensity") and lo is None:
            self.fail("bad_value", "Rampalı görünüşün aralığı (en küçük ve en büyük) verilmeli.")
        if not MIN_POINT_SIZE <= st["size"] <= MAX_POINT_SIZE:
            self.fail("bad_value", "Noktanın boyu 0.5 ile 32 arasında olmalı.")
        h = st.get("hidden", [])
        if any(a >= b for a, b in zip(h, h[1:])):
            self.fail("bad_value", "Gizlenen sınıflar küçükten büyüğe ve birer kez yazılmalı.")
        o = c.get("opacity")
        if o is not None and not (MIN_CLOUD_OPACITY <= o <= MAX_CLOUD_OPACITY):
            self.fail("bad_value", f"Nokta bulutunun donukluğu 0.1 ile 1 arasında olmalı; {o} verildi.")

    def field_source_problem(self, src):
        """A field's source's names as the contract wants them (`FieldSource::problem`): an attribute's, a variable's
        and a block count's name, no other."""
        needs = src["value"] in ("attribute", "variable", "blockCount")
        named = bool(src.get("name", "").strip())
        if needs and not named:
            return "ad öznitelik, değişken ve blok sayısı alanında zorunlu"
        if named and not needs:
            return "ad yalnız öznitelik, değişken ve blok sayısı alanında verilir"
        return None

    # A table's cells beyond their words (schema 68, docs/adr/0327).

    def cell_styles(self, v):
        one = self.fields({**{k: (self.uint(32), True) for k in ("row", "col", "rows", "cols")},
                           "align": (self.enum(("left", "center", "right")), False), "bold": (self.bool, False),
                           "color": (self.text, False), "fill": (self.text, False), "decimals": (self.uint(32), False)})
        styles = self.array(one)(v)
        if not styles:
            self.fail("bad_value", "boş biçim listesi yazılmaz")
        return styles

    def cell_formulas(self, v):
        formulas = self.array(self.fields({"row": (self.uint(32), True), "col": (self.uint(32), True), "formula": (self.text, True)}))(v)
        if not formulas:
            self.fail("bad_value", "boş formül listesi yazılmaz")
        return formulas

    def cell_fields(self, v):
        def source(src):
            if type(src) is not dict:
                self.fail("wrong_type", "harita olmalı")
            if src.get("kind") == "object":
                return self.fields({"kind": (self.text, True), "uid": (self.uid_text, True), "value": (self.enum(FIELD_OBJECT_VALUES), True), "name": (self.text, False)})(src)
            if src.get("kind") == "project":
                return self.fields({"kind": (self.text, True), "value": (self.enum(FIELD_PROJECT_VALUES), True), "name": (self.text, False)})(src)
            self.fail("bad_value", f"alanın kaynağı {src.get('kind')!r}: object ya da project olmalı")

        fmt = self.fields({"decimals": (self.uint(8), False), "unit": (self.enum(FIELD_UNITS), False), "comma": (self.true_flag, False), "prefix": (self.text, False), "suffix": (self.text, False)})
        one = self.fields({"row": (self.uint(32), True), "col": (self.uint(32), True), "source": (source, True), "format": (fmt, False), "broken": (self.enum(FIELD_BROKEN), False)})
        fields = self.array(one)(v)
        if not fields:
            self.fail("bad_value", "boş alan listesi yazılmaz")
        return fields

    def uid_text(self, u):
        if type(u) is not str or not UUID_TEXT.fullmatch(u):
            self.fail("bad_value", "nesnenin kimliği küçük harfli, tireli bir UUID olmalı")
        return u

    def table_link(self, v):
        copy_ = lambda rs: [[self.text(w) for w in self.array(lambda x: x)(r)] for r in self.array(lambda x: x)(rs)]  # noqa: E731
        d = self.fields({"path": (self.text, False), "digest": (self.text, False), "copy": (copy_, False),
                         "overrides": (self.enum(("keep", "replace")), False)})(v)
        if d.get("overrides") == "keep":
            self.path.append("overrides")
            self.fail("bad_value", "keep yazılmaz; bağın varsayılanıdır")
        if not d:
            self.fail("bad_value", "boş bağ yazılmaz")
        return d

    def table_schedule(self, v):
        if self.tags and isinstance(v, dict) and v.get("kind") == "schedule":
            return self.arch_schedule(v)
        d = self.fields({"kind": (self.enum(("blocks",)), True), "names": (self.array(self.text), False), "by": (self.text, False)})(v)
        if "names" in d and not d["names"]:
            self.path.append("names")
            self.fail("bad_value", "boş ad listesi yazılmaz")
        return d

    def arch_schedule(self, v):
        """A schedule of the drawing's objects (schema 80, docs/adr/0344 §3): its kind, columns, filter, sort, grouping,
        total and the members the document wrote; a flag only when true, no empty list."""
        def true_only(x):
            x = self.bool(x)
            if x is not True:
                self.fail("bad_value", "false yazılmaz; varsayılandır")
            return x
        column = self.fields({"field": (self.enum(SCHEDULE_FIELDS), True), "title": (self.text, False), "attr": (self.text, False)})
        flt = self.fields({"levels": (self.array(self.text), False), "phases": (self.array(self.enum(("existing", "demolish", "new", "temporary"))), False),
                           "forms": (self.array(self.enum(("door", "window", "opening"))), False), "layers": (self.array(self.text), False)})
        sort = self.fields({"column": (self.uint(32), True), "descending": (true_only, False)})
        d = self.fields({"kind": (self.enum(("schedule",)), True), "of": (self.enum(("openings", "spaces", "columns", "walls")), True),
                         "columns": (self.array(column), True), "filter": (flt, False), "sort": (self.array(sort), False),
                         "group": (true_only, False), "total": (true_only, False),
                         "members": (self.array(self.array(self.text)), False)})(v)
        if not 1 <= len(d["columns"]) <= 30:
            self.path.append("columns")
            self.fail("bad_value", "çizelgenin 1 ile 30 arası sütunu olur")
        for c in d["columns"]:
            if (c["field"] == "attr") != ("attr" in c):
                self.fail("bad_value", "öznitelik adı yalnız ve her öznitelik sütununda verilir")
        for s in d.get("sort", []):
            if s["column"] >= len(d["columns"]):
                self.fail("bad_value", "sıralama olmayan bir sütunu istiyor")
        for key in ("sort", "members"):
            if key in d and not d[key]:
                self.path.append(key)
                self.fail("bad_value", "boş liste yazılmaz")
        return d

    def tag_fields(self, v):
        """A tag's fields (schema 80, docs/adr/0344 §1), as the contract's JSON has them; the frame only when not a circle,
        the leader only when on, no empty words."""
        def not_circle(x):
            x = self.enum(("circle", "hexagon", "rectangle", "rounded", "none"))(x)
            if x == "circle":
                self.fail("bad_value", "varsayılan çerçeve (circle) yazılmaz")
            return x
        def true_only(x):
            x = self.bool(x)
            if x is not True:
                self.fail("bad_value", "false yazılmaz; varsayılandır")
            return x
        f = self.fields({"of": (self.text, True), "at": (self.jpoint, True), "content": (self.text, False),
                         "shape": (not_circle, False), "size": (self.float, False), "leader": (true_only, False),
                         "text": (self.text, False), "anchor": (self.jpoint, False),
                         "broken": (self.enum(("targetGone",)), False)})(v)
        bad = lambda words: self.fail("bad_value", words)  # noqa: E731
        if not f["of"].strip():
            bad("Etiketin nesnesi verilmeli.")
        if not (math.isfinite(f["at"]["x"]) and math.isfinite(f["at"]["y"])):
            bad("Etiketin yerinin koordinatları sonlu olmalı.")
        if "content" in f:
            c = f["content"]
            if not c.strip() or len(c) > 200 or c.count("\n") + 1 > 6:
                bad("Etiketin şablonu boş olamaz, en çok 200 harf ve 6 satırdır.")
            for name in [part.split("}")[0] for part in c.split("{")[1:]]:
                if not (name in TAG_PLACES or (name.startswith("attr:") and name[5:].strip())):
                    bad(f"Şablonda bilinmeyen yer “{{{name}}}”.")
        if "size" in f and not (math.isfinite(f["size"]) and 0 < f["size"] <= 50):
            bad("Etiketin yazı yüksekliği sıfırdan büyük, en çok 50 mm olmalı.")
        if f.get("text") == "":
            bad("boş yazı yazılmaz")
        return f

    def viewmark_fields(self, v):
        """A view mark's fields (schema 81, docs/adr/0345 §1), as the contract's JSON has them; a flag only when true,
        no empty number or words; then the contract's rule (`ViewMarkFields::problem`)."""
        def true_only(x):
            x = self.bool(x)
            if x is not True:
                self.fail("bad_value", "false yazılmaz; varsayılandır")
            return x
        target = self.fields({"view": (self.text, False), "sheet": (self.text, False), "item": (self.text, False)})
        words = self.fields({"top": (self.text, True), "bottom": (self.text, True)})
        f = self.fields({"form": (self.enum(("section", "elevation", "detail")), True), "points": (self.array(self.jpoint), True),
                         "radius": (self.float, False), "at": (self.jpoint, False), "angle": (self.float, False),
                         "flip": (true_only, False), "mark": (self.text, False), "target": (target, False),
                         "size": (self.float, False), "words": (words, False), "broken": (self.enum(("targetGone",)), False)})(v)
        bad = lambda words: self.fail("bad_value", words)  # noqa: E731
        pts = f["points"]
        if not all(math.isfinite(p["x"]) and math.isfinite(p["y"]) for p in pts):
            bad("İşaretin noktalarının koordinatları sonlu olmalı.")
        if f["form"] == "section":
            if not 2 <= len(pts) <= 32:
                bad("Kesitin kesme çizgisi 2 ile 32 arası nokta ister.")
            if any(pts[k] == pts[k + 1] for k in range(len(pts) - 1)):
                bad("Kesme çizgisinin art arda iki noktası aynı olamaz.")
        elif len(pts) != 1:
            bad("Görünüş ve detay tek nokta ister.")
        if f["form"] == "detail":
            if not ("radius" in f and math.isfinite(f["radius"]) and f["radius"] > 0):
                bad("Detayın sınırının yarıçapı sıfırdan büyük olmalı.")
        elif "radius" in f:
            bad("Yarıçap yalnız detayın sınırınındır.")
        if "at" in f and not (math.isfinite(f["at"]["x"]) and math.isfinite(f["at"]["y"])):
            bad("Balonun yerinin koordinatları sonlu olmalı.")
        if "angle" in f and not math.isfinite(f["angle"]):
            bad("Bakış yönü sonlu olmalı.")
        if f.get("mark") == "":
            bad("boş numara yazılmaz")
        if len(f.get("mark", "")) > 16 or any(unicodedata.category(c) == "Cc" for c in f.get("mark", "")):
            bad("İşaretin numarası en çok 16 harftir, denetim karakteri olmaz.")
        if "size" in f and not (math.isfinite(f["size"]) and 0 < f["size"] <= 50):
            bad("İşaretin yazı yüksekliği sıfırdan büyük, en çok 50 mm olmalı.")
        t = f.get("target")
        if t is not None:
            if any(not x.strip() for x in t.values()):
                bad("Hedefin kimlikleri boş olamaz.")
            if set(t) not in ({"view"}, {"sheet", "item"}):
                bad("Hedef ya bir adlı görünümdür ya da bir paftadaki görünüm penceresi.")
        w = f.get("words")
        if w is not None:
            if not w["top"] and not w["bottom"]:
                bad("boş yazı yazılmaz")
            if any(len(x) > 128 or any(unicodedata.category(c) == "Cc" for c in x) for x in w.values()):
                bad("İşaretin yazısı en çok 128 harftir, denetim karakteri olmaz.")
        return f

    def table_sources(self, t):
        """A table's schedule is its source; a link joins its file source (docs/adr/0327 §4–§5)."""
        if "schedule" in t:
            if "source" in t:
                self.path.append("schedule")
                self.fail("bad_value", "çizelge yalnız blok çizelgesidir ve kaynakla birlikte yazılmaz")
            t["source"] = t.pop("schedule")
        if "link" in t:
            link = t.pop("link")
            src = t.get("source")
            if src is None or src["kind"] != "file":
                self.path.append("link")
                self.fail("bad_value", "bağ yalnız dosyadan gelen tablonun kaynağıyla yazılır")
            src.update(link)

    def cells_rules(self, t, n, m, refuse):
        """The contract's `cells_problem_of` (kentos_contracts::table_cells), in its order."""
        merges = t.get("merges", [])

        def hidden(r, c):
            return any(g["row"] <= r < g["row"] + g["rows"] and g["col"] <= c < g["col"] + g["cols"] and (g["row"], g["col"]) != (r, c) for g in merges)

        styles = t.get("styles", [])
        if len(styles) > MAX_CELL_STYLES:
            refuse("styles", f"bir tabloda en çok {MAX_CELL_STYLES} biçim olur")
        for k, st in enumerate(styles):
            if st["rows"] == 0 or st["cols"] == 0 or st["row"] + st["rows"] > n or st["col"] + st["cols"] > m:
                refuse("styles", f"{k + 1}. biçimin aralığı tablonun içinde olmalı")
            for what in ("color", "fill"):
                if what in st and not COLOUR.fullmatch(st[what]):
                    refuse("styles", f"{k + 1}. biçimin rengi “{st[what]}”; #rrggbb olmalı")
            if st.get("decimals", 0) > MAX_CELL_DECIMALS:
                refuse("styles", f"{k + 1}. biçimin basamağı en çok {MAX_CELL_DECIMALS} olur")
            if not any(x in st for x in ("align", "bold", "color", "fill", "decimals")):
                refuse("styles", f"{k + 1}. biçim hiçbir şey vermiyor")
        formulas = t.get("formulas", [])
        if len(formulas) > MAX_FORMULAS:
            refuse("formulas", f"bir tabloda en çok {MAX_FORMULAS} formül olur")
        seen = set()
        for f in formulas:
            r, c, words = f["row"], f["col"], f["formula"]
            if not (r < n and c < m):
                refuse("formulas", "formülün hücresi tablonun içinde olmalı")
            if hidden(r, c):
                refuse("formulas", "formülün hücresi birleşik bir alanın içinde")
            if not (words.startswith("=") and len(words.encode("utf-8")) > 1) or len(words) > MAX_FORMULA_LETTERS or any(unicodedata.category(x) == "Cc" for x in words):
                refuse("formulas", "formül “=” ile başlayan, tek satırlık bir yazıdır")
            if (r, c) in seen:
                refuse("formulas", "bir hücreye iki formül verildi")
            seen.add((r, c))
        fields = t.get("fields", [])
        if len(fields) > MAX_CELL_FIELDS:
            refuse("fields", f"bir tabloda en çok {MAX_CELL_FIELDS} alan olur")
        fielded = set()
        for f in fields:
            r, c = f["row"], f["col"]
            if not (r < n and c < m) or hidden(r, c):
                refuse("fields", "alanın hücresi tablonun içinde ve birleşik alanın sol üst hücresi olmalı")
            if (r, c) in seen:
                refuse("fields", "bir hücre hem formül hem alan olamaz")
            if (r, c) in fielded:
                refuse("fields", "bir hücreye iki alan verildi")
            fielded.add((r, c))
            why = self.field_source_problem(f["source"])
            if why:
                refuse("fields", why)
            fm = f.get("format")
            if fm is not None:
                if fm.get("decimals", 0) > 10:
                    refuse("fields", "alanın basamağı en çok 10 olur")
                if not fm:
                    refuse("fields", "boş biçim yazılmaz")

    def table_rules(self, t):
        """The contract's `TableShape::problem` (kentos_contracts::table), in its order: what it names is refused at its field."""
        def refuse(field, words):
            self.path.append(field)
            self.fail("bad_value", words)

        holds = lambda x: math.isfinite(x) and 0.0 < x <= MAX_TABLE_SIZE  # noqa: E731
        if not holds(t["height"]):
            refuse("height", f"tablonun yazı yüksekliği {t['height']}; sıfırdan büyük ve sonlu olmalı")
        rows, columns, cells = t["rows"], t["columns"], t["cells"]
        n, m = len(rows), len(columns)
        if not 0 < n <= MAX_TABLE_ROWS:
            refuse("rows", f"tablonun {n} satırı var")
        if not 0 < m <= MAX_TABLE_COLUMNS:
            refuse("columns", f"tablonun {m} sütunu var")
        if n * m > MAX_TABLE_CELLS:
            refuse("cells", f"tablonun {n * m} hücresi var")
        if not all(holds(h) for h in rows):
            refuse("rows", "bir satırın yüksekliği sıfırdan büyük ve sonlu değil")
        if not all(holds(w) for w in columns):
            refuse("columns", "bir sütunun genişliği sıfırdan büyük ve sonlu değil")
        if len(cells) != n or any(len(r) != m for r in cells):
            refuse("cells", "hücreler satır ve sütun sayısı kadar değil")
        for r in cells:
            for words in r:
                if any(unicodedata.category(c) == "Cc" for c in words) or len(words) > MAX_CELL_LETTERS:
                    refuse("cells", "hücrede satır sonu ya da denetim karakteri var ya da harf çok")
        merges = t.get("merges", [])
        for k, g in enumerate(merges):
            if g["rows"] == 0 or g["cols"] == 0 or g["rows"] * g["cols"] < 2 or g["row"] + g["rows"] > n or g["col"] + g["cols"] > m:
                refuse("merges", f"{k + 1}. birleşik alan tablonun içinde ve birden çok hücre değil")
            for o in merges[:k]:
                if o["row"] < g["row"] + g["rows"] and g["row"] < o["row"] + o["rows"] and o["col"] < g["col"] + g["cols"] and g["col"] < o["col"] + o["cols"]:
                    refuse("merges", f"{k + 1}. birleşik alan öncekiyle örtüşüyor")
            for i in range(g["row"], g["row"] + g["rows"]):
                for j in range(g["col"], g["col"] + g["cols"]):
                    if (i, j) != (g["row"], g["col"]) and cells[i][j]:
                        refuse("merges", "birleşik alanın içindeki bir hücre boş değil")
        if "frame" in t:
            f = t["frame"]
            if not (math.isfinite(f) and f > 0.0 and 2.0 * f < min(math.fsum(columns), math.fsum(rows))):
                refuse("frame", f"tablonun çerçeve kalınlığı {f}; sıfırdan büyük, tablonun eninin ve boyunun yarısından küçük olmalı")
        if "aligns" in t and len(t["aligns"]) != m:
            refuse("aligns", "her sütunun hizası verilmeli")
        src = t.get("source")
        if src is not None:
            if src["kind"] == "blocks":
                if any(not x.strip() for x in src.get("names", [])) or ("by" in src and not src["by"].strip()):
                    refuse("source", "blok çizelgesinin blok ve öznitelik adları boş olamaz")
            elif src["kind"] == "file":
                if "path" in src and (not src["path"].strip() or any(unicodedata.category(c) == "Cc" for c in src["path"])):
                    refuse("source", "tablonun bağlı dosyasının yolu boş olamaz")
                if "digest" in src and not SHA256_HEX.fullmatch(src["digest"]):
                    refuse("source", "bağlı dosyanın özeti 64 onaltılık rakamlık bir SHA-256 olmalı")
                if "copy" in src and (len(src["copy"]) > MAX_TABLE_ROWS or any(len(r) > MAX_TABLE_COLUMNS or any(any(unicodedata.category(c) == "Cc" for c in w) or len(w) > MAX_CELL_LETTERS for w in r) for r in src["copy"])):
                    refuse("source", "dosyanın gömülü kopyası bir tablonun sınırlarını aşıyor")
                name = src["name"]
                if not name.strip() or any(unicodedata.category(c) == "Cc" for c in name):
                    refuse("source", "kaynak dosyanın adı boş olamaz")
                if "sheet" in src and not src["sheet"].strip():
                    refuse("source", "kaynak sayfanın adı boş olamaz")
            elif src["kind"] == "schedule":
                pass  # Its columns are checked as it is read (`arch_schedule`); its rows are the document's.
            elif not 0 < len(src["objects"]) <= MAX_SOURCE_OBJECTS:
                refuse("source", f"tablonun kaynağı {len(src['objects'])} nesne gösteriyor")
        self.cells_rules(t, n, m, refuse)

    # Hatches (docs/adr/0186).

    def hatch_pattern(self, v):
        table = {"type": (self.enum(HATCH_TYPES if self.hatches else HATCH_TYPES[:3]), True), "angle": (self.float, True), "spacing": (self.float, True)}
        if self.hatches:
            table.update(
                {
                    "name": (self.text, False),
                    "scale": (self.float, False),
                    "lines": (self.array(self.pattern_line), False),
                    "gradient": (self.hatch_gradient, False),
                }
            )
        if self.hatch_extras:
            # Schema 71 (docs/adr/0331 §2): where its lines are anchored; only lines' and a pattern's.
            table["origin"] = (self.point, False)
        p = self.fields(table)(v)
        self.pattern_rules(p)
        if "origin" in p:
            if p["type"] in ("solid", "gradient"):
                self.fail("bad_value", "dolu ve degrade taramanın başlangıç noktası olmaz")
            if not all(math.isfinite(p["origin"][k]) for k in ("x", "y")):
                self.fail("bad_value", "desenin başlangıç noktası sonlu olmalı")
        return p

    def pair(self, v):
        q = self.point(v)
        return [q["x"], q["y"]]

    def dashes(self, v):
        if type(v) is list and not v:
            self.fail("bad_value", "kesik listesi boş; bütün çizgili ailede alan yazılmaz")
        return self.array(self.float)(v)

    def pattern_line(self, v):
        return self.fields({"angle": (self.float, True), "origin": (self.pair, True), "offset": (self.pair, True), "dashes": (self.dashes, False)})(v)

    def gradient_inverted(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "degradenin “inverted”ı yanlışken yazılmaz")
        return True

    def hatch_gradient(self, v):
        return self.fields({"shape": (self.enum(GRADIENT_SHAPES), True), "color2": (self.text, True), "inverted": (self.gradient_inverted, False)})(v)

    def assoc_ids(self, v):
        if type(v) is list and not v:
            self.fail("bad_value", "nesne listesi boş; nesnesi olmayan listenin alanı yazılmaz")
        return self.array(self.id16)(v)

    def hatch_assoc(self, v):
        # Schema 69 (docs/adr/0328 §3): a face's tie names its line work (`bounds`) and no closed object.
        table = {"outer": (self.id16, not self.hatch_boundaries), "islands": (self.assoc_ids, False), "cutouts": (self.assoc_ids, False), "seed": (self.point, True)}
        if self.hatch_boundaries:
            table["bounds"] = (self.assoc_ids, False)
        if self.hatch_gaps:
            table["gap"] = (self.float, False)
        a = self.fields(table)(v)
        if "gap" in a and ("outer" in a or not 0.0 < a["gap"] <= MAX_HATCH_GAP):
            self.fail("bad_value", "taramanın boşluk toleransı yalnız yüzün ilişkisinde, 0'dan büyük ve en çok 1000 m olur")
        if ("outer" in a) == ("bounds" in a):
            self.fail("bad_value", "taramanın ilişkisi ya kapalı nesneyi (outer) ya da sınır nesnelerini (bounds) gösterir")
        named = [*([a["outer"]] if "outer" in a else []), *a.get("bounds", []), *a.get("islands", []), *a.get("cutouts", [])]
        if len(named) - ("outer" in a) > MAX_ASSOC_OBJECTS:
            self.fail("bad_value", f"taramanın ilişkisi {len(named)} nesne gösteriyor; en çok {MAX_ASSOC_OBJECTS} olmalı")
        if len(set(named)) != len(named):
            self.fail("bad_value", "taramanın ilişkisi bir nesneyi iki kez gösteriyor")
        return a

    # A hatch's exact boundary (schema 69, docs/adr/0328 §1): the contract's form (kentos_contracts::boundary_problem),
    # in its order; whether a curve ends at its loop's points is the geometry's, not the file's.

    def xy(self, v):
        return self.fields({"x": (self.float, True), "y": (self.float, True)})(v)

    def hatch_curve(self, v):
        if type(v) is not dict:
            self.fail("wrong_type", "harita olmalı")
        if v.get("kind") == "ellipse":
            c = self.fields({"kind": (self.text, True), "c": (self.xy, True), "major": (self.xy, True), "ratio": (self.float, True), "t0": (self.float, True), "t1": (self.float, True)})(v)
            nums = [c["c"]["x"], c["c"]["y"], c["major"]["x"], c["major"]["y"], c["ratio"], c["t0"], c["t1"]]
            if not all(math.isfinite(x) for x in nums):
                self.fail("bad_value", "elips kenarında sonlu olmayan bir sayı var")
            if not 0.0 < c["ratio"] <= 1.0 or (c["major"]["x"] == 0.0 and c["major"]["y"] == 0.0):
                self.fail("bad_value", "elips kenarının büyük ekseni sıfır olamaz, oranı 0 ile 1 arasında olmalı")
            sweep = abs(c["t1"] - c["t0"])
            if sweep == 0.0 or sweep > 2 * math.pi * (1 + 1e-12):
                self.fail("bad_value", "elips kenarının açıklığı sıfır olamaz ve bir turu aşamaz")
            return c
        if v.get("kind") == "nurbs":
            c = self.fields({"kind": (self.text, True), "degree": (self.uint(32), True), "ctrl": (self.array(self.xy), True), "knots": (self.array(self.float), True), "weights": (self.array(self.float), False)})(v)
            p, ctrl, knots = c["degree"], c["ctrl"], c["knots"]
            n = len(ctrl)
            if not 1 <= p <= MAX_CURVE_DEGREE:
                self.fail("bad_value", f"NURBS kenarının derecesi {p}; 1 ile {MAX_CURVE_DEGREE} arasında olmalı")
            if n <= p or n > MAX_CURVE_POINTS:
                self.fail("bad_value", f"NURBS kenarının {n} denetim noktası var")
            if len(knots) != n + p + 1:
                self.fail("bad_value", f"NURBS kenarının {len(knots)} düğümü var; {n + p + 1} olmalı")
            if (not all(math.isfinite(q["x"]) and math.isfinite(q["y"]) for q in ctrl) or not all(math.isfinite(k) for k in knots)
                    or any(b < a for a, b in zip(knots, knots[1:])) or knots[p] >= knots[n]):
                self.fail("bad_value", "NURBS kenarının noktaları sonlu, düğümleri artan olmalı")
            if "weights" in c and (len(c["weights"]) != n or not all(math.isfinite(w) and w > 0 for w in c["weights"])):
                self.fail("bad_value", "NURBS kenarının ağırlıkları denetim noktaları kadar ve sıfırdan büyük olmalı")
            return c
        self.fail("bad_value", f"eğri kenarı “{v.get('kind')}” bilinmiyor (ellipse, nurbs)")

    def hatch_loop(self, v):
        def curve_or_null(c):
            return None if c is None else self.hatch_curve(c)
        lp = self.fields({"pts": (self.array(self.xy), True), "bulges": (self.array(self.float), False), "curves": (self.array(curve_or_null), False)})(v)
        pts, n = lp["pts"], len(lp["pts"])
        if n == 0 or n > MAX_LOOP_EDGES:
            self.fail("bad_value", f"tarama sınırının bir halkasında {n} nokta var")
        if not all(math.isfinite(q["x"]) and math.isfinite(q["y"]) for q in pts):
            self.fail("bad_value", "tarama sınırında sonlu olmayan bir nokta var")
        bulges = lp.get("bulges")
        if bulges is not None and (len(bulges) != n or not all(math.isfinite(b) for b in bulges)):
            self.fail("bad_value", "tarama sınırının bükümleri noktaları kadar ve sonlu olmalı")
        curves = lp.get("curves")
        if curves is not None:
            if len(curves) != n:
                self.fail("bad_value", "tarama sınırının eğrileri noktaları kadar olmalı")
            for i, c in enumerate(curves):
                if c is not None and bulges is not None and bulges[i] != 0.0:
                    self.fail("bad_value", "tarama sınırının eğri kenarının bükümü 0 olmalı")

        def curved(i):
            return (curves is not None and curves[i] is not None) or (bulges is not None and bulges[i] != 0.0)
        if n == 1 and not (curves is not None and curves[0] is not None):
            self.fail("bad_value", "tek noktalı halka kapalı bir eğri olmalı")
        if n == 2 and not (curved(0) or curved(1)):
            self.fail("bad_value", "iki noktalı halkanın en az bir kenarı eğri olmalı")
        return lp

    def hatch_boundary(self, v):
        loops = self.array(self.hatch_loop)(v)
        if not loops or len(loops) > MAX_BOUNDARY_LOOPS:
            self.fail("bad_value", f"tarama sınırında {len(loops)} halka var; en az 1 olmalı")
        return loops

    def sub_place(self, v):
        """A place on an object (docs/adr/0249 §1): its `kind`, then the numbers that kind has, each at most
        SUBREF_INDEX_LIMIT; a number its kind does not have is an unknown field."""
        n = 0
        kinds = tuple(k for k in SUBREF_PLACES if k != "along" or self.subref_extras)
        kind = None
        if type(v) is dict and "t" in v and not self.subref_extras:
            # The shortest key comes first: before schema 51 an along place's t is an unknown field.
            return self.fields({"kind": (self.text, True)})(v)
        if type(v) is dict and "kind" in v:
            kind = self.at("kind", self.enum(kinds), v["kind"])
            n = SUBREF_PLACES[kind]
        table = {"kind": (self.text, True)}
        for key in ("part", "ring", "index")[:n]:
            table[key] = (self.sub_index, True)
        if kind == "along":
            table["t"] = (self.fraction, True)
        return self.fields(table)(v)

    def fraction(self, v):
        x = self.float(v)
        if not (0.0 <= x <= 1.0):
            self.fail("bad_value", f"{x} 0 ile 1 arasında olmalı")
        return x

    def sub_path(self, v):
        path = self.array(self.sub_index)(v)
        if not path:
            self.fail("bad_value", "Başvurunun blok yolu boş: boşsa alan yazılmaz.")
        if len(path) > SUBREF_PATH_DEPTH:
            self.fail("bad_value", f"Başvurunun blok yolu {len(path)} düzey; en çok {SUBREF_PATH_DEPTH} olmalı.")
        return path

    def sub_index(self, v):
        x = self.uint(64)(v)
        if x > SUBREF_INDEX_LIMIT:
            self.fail("bad_value", f"{x} aralık dışında (0…{SUBREF_INDEX_LIMIT})")
        return x

    def subref(self, v):
        table = {"uid": (self.id16, True), "place": (self.sub_place, True), "broken": (self.text, False)}
        if self.subref_extras:
            table["path"] = (self.sub_path, False)
        r = self.fields(table)(v)
        if "broken" in r and r["broken"] not in SUBREF_BROKEN:
            self.fail("bad_value", f"Başvurunun kopma nedeni “{r['broken']}” bilinmiyor.")
        return r

    def dimension_refs(self, v):
        """What a dimension's points follow (docs/adr/0249 §4): `a`, `b`, `c`; references none of its points has
        are not written."""
        table = {"a": (self.subref, False), "b": (self.subref, False), "c": (self.subref, False)}
        if self.dim_rules:
            # Schema 60 (docs/adr/0320 §1): how the points no place names follow.
            # Schema 62 (docs/adr/0322): the axis's pair.
            table["rule"] = (self.enum(("curve", "edges", "pair") if self.dim_marks else ("curve", "edges")), False)
        refs = self.fields(table)(v)
        if not any(k in refs for k in "abc"):
            self.fail("bad_value", "Ölçünün başvuruları boş: hiç başvuru yoksa alan yazılmaz.")
        return refs

    def pattern_rules(self, p):
        """A pattern's and a gradient's fields (docs/adr/0186 §1) as the commands check them; the first three kinds as
        they always were (their angle and spacing unread here)."""
        kind = p["type"]
        if kind not in ("pattern", "gradient") and not any(k in p for k in ("name", "scale", "lines", "gradient")):
            return
        fits = lambda x: math.isfinite(x) and abs(x) <= MAX_PATTERN_SIZE
        if kind != "pattern" and any(k in p for k in ("name", "scale", "lines")):
            self.fail("bad_value", "yalnız desen türündeki taramanın adı, ölçeği ve çizgi aileleri olur")
        if kind != "gradient" and "gradient" in p:
            self.fail("bad_value", "yalnız degrade taramanın ikinci rengi ve biçimi olur")
        if kind in ("lines", "cross") and not 0.0 < p["spacing"] <= MAX_PATTERN_SIZE:
            self.fail("bad_value", f"çizgi aralığı {p['spacing']}; sıfırdan büyük ve sonlu olmalı")
        if kind == "pattern":
            name = p.get("name", "")
            if all(c in WHITE_SPACE for c in name) or len(name) > MAX_PATTERN_NAME:
                self.fail("bad_value", f"desenin adı boş olamaz ve en çok {MAX_PATTERN_NAME} harf olabilir")
            scale = p.get("scale", math.nan)
            if not 0.0 < scale <= MAX_PATTERN_SIZE:
                self.fail("bad_value", f"desenin ölçeği {scale}; sıfırdan büyük ve sonlu olmalı")
            lines = p.get("lines", [])
            if not 1 <= len(lines) <= MAX_PATTERN_LINES:
                self.fail("bad_value", f"desenin {len(lines)} çizgi ailesi var; en az 1, en çok {MAX_PATTERN_LINES} olmalı")
            for n, l in enumerate(lines, 1):
                dashes = l.get("dashes", [])
                if not all(fits(x) for x in [l["angle"], *l["origin"], *l["offset"], *dashes]):
                    self.fail("bad_value", f"desenin {n}. çizgi ailesinde sonlu olmayan ya da çok büyük bir sayı var")
                if l["offset"][1] == 0.0:
                    self.fail("bad_value", f"desenin {n}. çizgi ailesinin çizgileri arası 0")
                if len(dashes) > MAX_PATTERN_DASHES:
                    self.fail("bad_value", f"desenin {n}. çizgi ailesinde {len(dashes)} kesik var; en çok {MAX_PATTERN_DASHES} olmalı")
                if dashes and all(d == 0.0 for d in dashes):
                    self.fail("bad_value", f"desenin {n}. çizgi ailesinin kesiklerinin hepsi 0")
        if kind == "gradient":
            g = p.get("gradient")
            if g is None:
                self.fail("bad_value", "degrade taramanın ikinci rengi ve biçimi verilmeli")
            c = g["color2"]
            if not (len(c) == 7 and c[0] == "#" and all(x in "0123456789abcdefABCDEF" for x in c[1:])):
                self.fail("bad_value", f"degradenin ikinci rengi “{c}”; #RRGGBB biçiminde olmalı")

    def json_point(self, v):
        """A point as its JSON writes it: {x, y}."""
        return self.fields({"x": (self.float, True), "y": (self.float, True)})(v)

    def json_subref(self, v):
        """A place on an object as its JSON writes it (docs/adr/0249, 0326 §3): the object's id as text."""
        def uid(u):
            if type(u) is not str or not UUID_TEXT.fullmatch(u):
                self.fail("bad_value", "başvurunun nesnesi küçük harfli, tireli bir UUID olmalı")
            return u

        def place(p):
            if type(p) is not dict or "kind" not in p:
                self.fail("bad_value", "başvurunun yeri bir harita olmalı")
            return p

        return self.fields({"uid": (uid, True), "place": (place, True), "path": (self.array(self.uint(32)), False),
                            "broken": (self.text, False)})(v)

    def leader_arms(self, v):
        """A leader's other arms (docs/adr/0326 §1): some, each with its tip and bends, at most 63."""
        arms = self.array(self.fields({"pts": (self.array(self.json_point), True), "tip": (self.json_subref, False)}))(v)
        if not arms:
            self.fail("bad_value", "boş kol listesi yazılmaz")
        if len(arms) + 1 > MAX_LEADER_ARMS:
            self.fail("bad_value", f"bir kılavuzun en çok {MAX_LEADER_ARMS} kolu olur")
        for i, a in enumerate(arms):
            if not a["pts"]:
                self.fail("bad_value", f"{i + 2}. kolun ucu yok: her kolun en az bir noktası olmalı")
        return arms

    def leader_blocks(self, v):
        """A leader's content blocks: some inserts' ids, each once."""
        ids = self.array(self.text)(v)
        if not ids:
            self.fail("bad_value", "boş blok listesi yazılmaz")
        if len(set(ids)) != len(ids):
            self.fail("bad_value", "kılavuzun bir bloğu iki kez verilmiş")
        for u in ids:
            if not UUID_TEXT.fullmatch(u):
                self.fail("bad_value", "bloğun kimliği küçük harfli, tireli bir UUID olmalı")
        return ids

    def leader_styles(self, v):
        """The project's leader styles (docs/adr/0326 §4): the note's paper height over 0 and at most 1000 mm."""
        styles = self.array(self.fields({"id": (self.text, True), "name": (self.text, True), "height": (self.float, True),
                                         "arrow": (self.enum(LEADER_ARROWS_30), False), "arrowSize": (self.leader_arrow_size, False),
                                         "mask": (self.true_flag, False)}))(v)
        for st in styles:
            if not 0 < st["height"] <= 1000:
                self.fail("bad_value", f"“{st['name']}” kılavuz stilinin not yüksekliği {st['height']:g} mm; sıfırdan büyük, en çok 1000 olmalı")
        return styles

    # ── Walls (schema 75, docs/adr/0339) ──────────────────────────────────────────────────────────────────────────

    def wall_colour(self, what):
        def read(v):
            c = self.text(v)
            if not COLOUR.fullmatch(c):
                self.fail("bad_value", f"{what} “{c}”; #RRGGBB olmalı")
            return c
        return read

    def wall_component(self, v):
        def weight(v):
            w = self.float(v)
            if not 0.0 <= w <= MAX_LINE_WEIGHT:
                self.fail("bad_value", f"bileşenin çizgi kalınlığı {w:g}; 0 ile 100 mm arasında olmalı")
            return w
        edge = self.fields({"color": (self.wall_colour("bileşenin çizgi rengi"), False), "lineWeight": (weight, False),
                            "lineType": (self.enum(("continuous", "dashed", "dashdot", "dotted")), False)})
        hatch = self.fields({"pattern": (self.text, True), "scale": (self.float, True), "angle": (self.float, True),
                             "color": (self.wall_colour("bileşenin tarama rengi"), False)})
        def priority(v):
            p = self.uint(8)(v)
            if not 1 <= p <= 9:
                self.fail("bad_value", f"bileşenin önceliği {p}; 1 ile 9 arasında olmalı")
            return p
        return self.fields({"name": (self.text, True), "material": (self.text, False), "thickness": (self.float, False),
                            "edge": (edge, False), "hatch": (hatch, False), "detail": (self.enum(WALL_DETAILS), True),
                            **({"priority": (priority, False)} if self.wall_joins else {})})(v)

    def wall_components_rules(self, comps, width, owner):
        """A wall's (or a style's, `width` None) components: names, thicknesses above 0, at most one without (it takes
        the rest), hatches' scales above 0; with a width, the given ones leave the rest room, or add up to it."""
        if len(comps) > WALL_MAX_COMPONENTS:
            self.fail("bad_value", f"{owner} en çok {WALL_MAX_COMPONENTS} bileşeni olur; {len(comps)} var")
        rest = [c for c in comps if "thickness" not in c]
        if len(rest) > 1:
            self.fail("bad_value", f"{owner} kalınlığı verilmeyen iki bileşeni var; genişliğin kalanını yalnız biri alır")
        for c in comps:
            if not c["name"].strip():
                self.fail("bad_value", f"{owner} bir bileşeninin adı boş")
            t = c.get("thickness")
            if t is not None and not (math.isfinite(t) and 0 < t <= WALL_MAX_WIDTH):
                self.fail("bad_value", f"“{c['name']}” bileşeninin kalınlığı {t:g}; sıfırdan büyük, en çok {WALL_MAX_WIDTH:g} m olmalı")
            h = c.get("hatch")
            if h is not None and (not h["pattern"].strip() or not (math.isfinite(h["scale"]) and h["scale"] > 0) or not math.isfinite(h["angle"])):
                self.fail("bad_value", f"“{c['name']}” bileşeninin taraması: desen dolu, ölçek sıfırdan büyük, açı sonlu olmalı")
        if width is None or not comps:
            return
        fixed = sum(c["thickness"] for c in comps if "thickness" in c)
        slack = 1e-9 * max(1.0, width)
        if rest and not fixed < width - slack:
            self.fail("bad_value", f"{owner} genişliği {width:g} m; kalınlığı belli bileşenler {fixed:g} m, kalana yer kalmıyor")
        if not rest and abs(fixed - width) > slack:
            self.fail("bad_value", f"{owner} genişliği {width:g} m, bileşenlerinin toplamı {fixed:g} m; eşit olmalı")

    def wall_rules(self, w):
        """A wall (schema 75, docs/adr/0339 §1): two vertices or more (a closed one three, or two with an arc), as many
        bulges as vertices with an open wall's last 0, no edge of no length, a width above 0 and at most 100 m, an offset
        written only when not 0, its components fitting its width, a height above 0."""
        pts, n = w["pts"], len(w["pts"])
        closed = w.get("closed", False)
        if not 2 <= n <= WALL_MAX_POINTS:
            self.fail("bad_value", f"duvarın yolunun {n} noktası var; en az 2, en çok {WALL_MAX_POINTS}")
        bulges = w.get("bulges")
        if bulges is not None:
            if len(bulges) != n:
                self.fail("bad_value", f"duvarın {n} noktası, {len(bulges)} bükümü var; eşit olmalı")
            if not closed and bulges[-1] != 0:
                self.fail("bad_value", "açık duvarın son bükümü 0 olmalı")
        for i in range(n if closed else n - 1):
            a, b = pts[i], pts[(i + 1) % n]
            if a["x"] == b["x"] and a["y"] == b["y"]:
                self.fail("bad_value", f"duvarın {i + 1}. ve {(i + 1) % n + 1}. noktası aynı yerde")
        if closed and n < 3 and not any(x != 0 for x in (bulges or [])):
            self.fail("bad_value", "kapalı duvar düz kenarlarla en az üç noktadır")
        if not (math.isfinite(w["width"]) and 0 < w["width"] <= WALL_MAX_WIDTH):
            self.fail("bad_value", f"duvarın genişliği {w['width']:g}; sıfırdan büyük, en çok {WALL_MAX_WIDTH:g} m olmalı")
        if "offset" in w and (w["offset"] == 0 or not abs(w["offset"]) <= 1000):
            self.fail("bad_value", "duvarın ötelemesi 0 iken yazılmaz, en çok 1000 m")
        if "components" in w and not w["components"]:
            self.fail("bad_value", "boş bileşen listesi yazılmaz")
        self.wall_components_rules(w.get("components", []), w["width"], "duvarın")
        if "height" in w and not w["height"] > 0:
            self.fail("bad_value", f"duvarın yüksekliği {w['height']:g}; sıfırdan büyük olmalı")
        if "arch" in w and not w["arch"]:
            self.fail("bad_value", "boş mimari bilgi yazılmaz")
        if w.get("arch", {}).get("level", "x").strip() == "":
            self.fail("bad_value", "duvarın katı boş olamaz")

    def space_ring(self, v):
        r = self.fields({"pts": (self.array(self.jpoint), True), "bulges": (self.array(self.float), False)})(v)
        n = len(r["pts"])
        curved = any(b != 0 for b in r.get("bulges", []))
        if n < 3 and not (n == 2 and curved):
            self.fail("bad_value", f"Mahalin halkası en az üç köşedir; {n} verildi.")
        if "bulges" in r and len(r["bulges"]) != n:
            self.fail("bad_value", f"Halkanın {n} köşesi, {len(r['bulges'])} bükümü var; eşit olmalı.")
        return r

    def space_part(self, v):
        return self.fields({"outer": (self.space_ring, True), "holes": (self.array(self.space_ring), False)})(v)

    def space_fields(self, v):
        """A space's fields beside its parts (schema 78, docs/adr/0342 §1): each only when not its default."""
        def text_not_blank(x):
            x = self.text(x)
            if x.strip() == "":
                self.fail("bad_value", "Boş metin yazılmaz; alanı kaldırın ya da bir değer yazın.")
            return x
        def method(x):
            m = self.enum(SPACE_METHODS)(x)
            if m == "net":
                self.fail("bad_value", "varsayılan değer ('net') yazılmaz")
            return m
        def columns(x):
            if type(x) is not str:
                self.fail("bad_value", "kolon tercihi metin olmalı")
            value = self.enum(("ignore", "net", "all"))(x)
            if value == "ignore":
                self.fail("bad_value", "varsayılan kolon tercihi ('ignore') yazılmaz")
            return value
        table = {
            "seed": (self.jpoint, False),
            "method": (method, False),
            "number": (text_not_blank, False),
            "name": (text_not_blank, False),
            "usage": (text_not_blank, False),
            "arch": (self.arch_info, False),
            "height": (self.float, False),
            "finishes": (self.fields({"floor": (text_not_blank, False), "wall": (text_not_blank, False),
                                      "ceiling": (text_not_blank, False)}), False),
            "broken": (self.enum(("lostEnclosure", "noFreeArea", "invalidBoundarySource", "sourceUnavailable") if self.space_sources else ("lostEnclosure", "noFreeArea", "invalidBoundarySource") if self.space_columns else ("lostEnclosure",)), False),
        }
        if self.space_columns:
            table["columns"] = (columns, False)
        if self.space_sources:
            def sources(value):
                ids = self.array(self.uid_text)(value)
                if not ids or len(ids) > SPACE_MAX_POINTS or ids != sorted(set(ids)) or "00000000-0000-0000-0000-000000000000" in ids:
                    self.fail("bad_value", "Kaynak listesi boş olmayan, sıralı ve tekrarsız en çok 10000 UUID olmalı")
                return ids
            table["sources"] = (sources, False)
        if self.space_scope:
            def scope(value):
                if type(value) is not str:
                    self.fail("bad_value", "kaynak kapsamı metin olmalı")
                return self.enum(("unassigned",))(value)
            table["scope"] = (scope, False)
        return self.fields(table)(v)

    def space_rules(self, s):
        """A space (schema 78, docs/adr/0342 §1): one part or more, at most 10 000 vertices, a height above 0."""
        if not s["parts"]:
            self.fail("bad_value", "Mahalin sınırı yok: en az bir halka verin.")
        total = sum(len(p["outer"]["pts"]) + sum(len(h["pts"]) for h in p.get("holes", [])) for p in s["parts"])
        if total > SPACE_MAX_POINTS:
            self.fail("bad_value", f"Mahalin halkalarında {total} köşe var; en çok {SPACE_MAX_POINTS}.")
        if "sources" in s["space"] and "seed" not in s["space"]:
            self.fail("bad_value", "Seçilmiş kaynaklar ilişkili mahal gerektirir")
        if "columns" in s["space"] and "seed" not in s["space"]:
            self.fail("bad_value", "Kolon alanı yalnız duvarlardan bulunan ilişkili mahalda düşülür")
        if "scope" in s["space"] and ("seed" not in s["space"] or "level" in s["space"].get("arch", {})):
            self.fail("bad_value", "Atanmamış kaynak kapsamı katı olmayan ilişkili mahal gerektirir")
        h = s["space"].get("height")
        if h is not None and not (math.isfinite(h) and 0 < h <= 1000):
            self.fail("bad_value", "Mahalin yüksekliği sıfırdan büyük, en çok 1000 m olmalı.")
        if s["space"].get("arch") == {}:
            self.fail("bad_value", "boş mimari bilgi yazılmaz")
        if s["space"].get("finishes") == {}:
            self.fail("bad_value", "boş kaplama yazılmaz")

    def opening_fields(self, v):
        """An opening's fields beside its wall, place, width and jambs (schema 77, docs/adr/0341 §1): each only when it
        is not its default (a centred anchor, one leaf, a swinging operation, the start's hand, the left side, the
        swing's arc drawn), its form always."""
        def unless(default, read):
            def f(x):
                x = read(x)
                if x == default:
                    self.fail("bad_value", f"varsayılan değer ({default!r}) yazılmaz")
                return x
            return f
        return self.fields({
            "anchor": (unless("center", self.enum(("start", "center", "end"))), False),
            "form": (self.enum(OPENING_FORMS), True),
            "leaves": (unless(1, self.uint(8)), False),
            "operation": (unless("swing", self.enum(("swing", "slide", "fixed"))), False),
            "hand": (unless("start", self.enum(("start", "end"))), False),
            "side": (unless("left", self.enum(("left", "right"))), False),
            "swing": (self.float, False),
            "showSwing": (unless(True, self.bool), False),
            "frame": (self.float, False),
            "sill": (self.float, False),
            "head": (self.float, False),
            "style": (self.text, False),
            "number": (self.text, False),
            "broken": (self.enum(("hostGone", "offWall", "overlap")), False),
        })(v)

    def opening_rules(self, o):
        """An opening (schema 77, docs/adr/0341 §1): a wall, a finite place along it, a width above 0 and at most
        100 m, one or two leaves, a swing above 0° and at most 180°, a frame 0 or more and under half the width, finite
        sill and head with the head above the sill, a number not blank, four finite jamb points."""
        f = o["opening"]
        bad = lambda words: self.fail("bad_value", words)  # noqa: E731
        if o["host"].strip() == "":
            bad("Açıklığın duvarı belirtilmeli.")
        if not math.isfinite(o["at"]):
            bad("Açıklığın konumu sonlu olmalı.")
        w = o["width"]
        if not (math.isfinite(w) and 0 < w <= OPENING_MAX_WIDTH):
            bad(f"Açıklığın genişliği {w:g}; sıfırdan büyük, en çok {OPENING_MAX_WIDTH:g} m olmalı.")
        if f.get("leaves", 1) not in (1, 2):
            bad(f"Açıklığın kanat sayısı {f['leaves']}; 1 ya da 2 olmalı.")
        if "swing" in f and not (math.isfinite(f["swing"]) and 0 < f["swing"] <= 180):
            bad("Kanadın açılma açısı sıfırdan büyük, en çok 180° olmalı.")
        if "frame" in f and not (math.isfinite(f["frame"]) and 0 <= f["frame"] and 2 * f["frame"] < w):
            bad("Kasa genişliği sıfır ya da daha büyük, açıklığın yarısından küçük olmalı.")
        for k in ("sill", "head"):
            if k in f and not (math.isfinite(f[k]) and abs(f[k]) <= 1000):
                bad("Açıklığın denizlik ya da lento yüksekliği sonlu olmalı.")
        if "sill" in f and "head" in f and f["head"] <= f["sill"]:
            bad("Lento denizlikten yüksek olmalı.")
        if "number" in f and f["number"].strip() == "":
            bad("Açıklığın numarası boş olamaz.")
        if len(o["place"]) != 4:
            bad(f"Açıklığın pervazları dört noktadır; {len(o['place'])} verildi.")

    def arch_element(self, kind):
        """A grid's, column's, stair's or ramp's fields (schema 79, docs/adr/0343 §1), as the contract's JSON has them:
        each only when it is not its default (both bubbles, a rectangle, no turn, not filled, no numbers)."""
        def unless(default, read):
            def f(x):
                x = read(x)
                if x == default:
                    self.fail("bad_value", f"varsayılan değer ({default!r}) yazılmaz")
                return x
            return f
        points = self.array(self.jpoint)
        arch = (self.arch_info, False)

        def railings(x):
            # Schema 115 (docs/adr/0705): the sides a stair's or ramp's railings run on.
            if not self.railings:
                self.fail("bad_value", "korkuluk (railings) şema 115 ister")
            return self.enum(("left", "right", "both"))(x)
        tables = {
            "grid": {"pts": (points, True), "bulges": (self.array(self.float), False), "name": (self.text, True),
                     "bubbles": (unless("both", self.enum(("both", "start", "end", "none"))), False),
                     "bubbleSize": (self.float, False), "arch": arch},
            "column": {"at": (self.jpoint, True), "shape": (unless("rect", self.enum(("rect", "round"))), False),
                       "width": (self.float, True), "depth": (self.float, False), "rotation": (unless(0.0, self.float), False),
                       "type": (self.text, False), "tag": (self.text, False), "fill": (unless(False, self.bool), False),
                       "grids": (self.array(self.text), False), "place": (self.text, False),
                       "broken": (self.enum(("gridGone",)), False), "arch": arch},
            "stair": {"pts": (points, True), "width": (self.float, True), "risers": (self.uint(32), True),
                      "riserHeight": (self.float, True), "tread": (self.float, True), "landing": (self.float, False),
                      "cut": (self.uint(32), False), "numbering": (unless(False, self.bool), False),
                      "railings": (railings, False), "arch": arch},
            "ramp": {"pts": (points, True), "width": (self.float, True), "rise": (self.float, True),
                     "slope": (self.float, False), "base": (self.float, False), "landing": (self.float, False),
                     "railings": (railings, False), "arch": arch},
        }
        return self.fields(tables[kind])

    def arch_element_rules(self, kind, f):
        """The contract's problems of a grid, column, stair or ramp (docs/adr/0343 §1)."""
        bad = lambda words: self.fail("bad_value", words)  # noqa: E731
        def positive(v, whose, top=100.0):
            if not (math.isfinite(v) and 0 < v <= top):
                bad(f"{whose} sıfırdan büyük, en çok {top:g} olmalı.")
        def finite_points(pts):
            if any(not (math.isfinite(p["x"]) and math.isfinite(p["y"])) for p in pts):
                bad("Koordinatlar sonlu olmalı.")
        def flight(pts, what):
            if not 2 <= len(pts) <= 4:
                bad(f"{what} yolu 2, 3 ya da 4 noktadır; {len(pts)} verildi.")
            finite_points(pts)
            if any(a == b for a, b in zip(pts, pts[1:])):
                bad(f"{what} yolunun ard arda iki noktası aynı.")
        if f.get("arch") == {}:
            bad("boş mimari bilgi yazılmaz")
        if kind == "grid":
            if len(f["pts"]) != 2:
                bad(f"Aks iki noktadır; {len(f['pts'])} verildi.")
            finite_points(f["pts"])
            if f["pts"][0] == f["pts"][1]:
                bad("Aksın başı ve sonu aynı.")
            if "bulges" in f and (len(f["bulges"]) != 2 or not all(math.isfinite(b) for b in f["bulges"])):
                bad("Aksın iki sonlu bükümü olur.")
            if f["name"].strip() == "":
                bad("Aksın etiketi boş olamaz.")
            if "bubbleSize" in f:
                positive(f["bubbleSize"], "Balonun çapı")
        elif kind == "column":
            finite_points([f["at"]])
            positive(f["width"], "Kolonun eni")
            if "depth" in f:
                positive(f["depth"], "Kolonun boyu")
            if not math.isfinite(f.get("rotation", 0.0)):
                bad("Kolonun dönüşü sonlu olmalı.")
            g = f.get("grids")
            if g is not None and (len(g) != 2 or g[0] == g[1]):
                bad("Kolon iki ayrı aksın kesişimine bağlanır.")
            for k in ("type", "tag"):
                if k in f and f[k].strip() == "":
                    bad("Boş metin yazılmaz.")
        elif kind == "stair":
            flight(f["pts"], "Merdivenin")
            for k, whose in (("width", "Merdivenin genişliği"), ("riserHeight", "Rıht yüksekliği"), ("tread", "Basamak derinliği")):
                positive(f[k], whose)
            if not 1 <= f["risers"] <= 400:
                bad(f"Rıht sayısı {f['risers']}; 1 ile 400 arasında olmalı.")
            if "landing" in f:
                positive(f["landing"], "Sahanlığın boyu")
            if "cut" in f and not 1 <= f["cut"] <= f["risers"]:
                bad("Kesme çizgisi rıhtların arasında olmalı.")
        else:
            flight(f["pts"], "Rampanın")
            positive(f["width"], "Rampanın genişliği")
            if not (math.isfinite(f["rise"]) and abs(f["rise"]) <= 1000):
                bad("Rampanın yüksekliği sonlu ve en çok 1000 m olmalı.")
            if "slope" in f:
                positive(f["slope"], "Rampanın eğimi", 1000.0)
            if "base" in f and not math.isfinite(f["base"]):
                bad("Rampanın başlangıç kotu sonlu olmalı.")
            if "landing" in f:
                positive(f["landing"], "Sahanlığın boyu")

    def arch_info(self, v):
        return self.fields({"level": (self.text, False), "phase": (self.enum(ARCH_PHASES), False),
                            "status": (self.enum(ARCH_STATUSES), False), "discipline": (self.enum(ARCH_DISCIPLINES), False)})(v)

    def wall_joins_(self, v):
        """A wall's joins (schema 76, docs/adr/0340 §1): an automatic end is not written, nor are both."""
        j = self.fields({"start": (self.enum(WALL_JOINS), False), "end": (self.enum(WALL_JOINS), False)})(v)
        if any(x == "auto" for x in j.values()) or not j:
            self.fail("bad_value", "otomatik birleşim yazılmaz; iki ucu da otomatikse alan yazılmaz")
        return j

    def only_false(self, v):
        if self.bool(v):
            self.fail("bad_value", "temizlik açıkken yazılmaz; alan yalnız false iken yazılır")
        return False

    def wall_caps(self, v):
        if self.text(v) != "open":
            self.fail("bad_value", "uçlar kapalıyken yazılmaz; alan yalnız open iken yazılır")
        return "open"

    def wall_styles_(self, v):
        """The project's wall styles (docs/adr/0339 §2): ids and names one of a kind, each's components holding."""
        styles = self.array(self.fields({"id": (self.text, True), "name": (self.text, True),
                                         "components": (self.array(self.wall_component), True),
                                         "caps": (self.wall_caps, False), "fill": (self.wall_colour("duvar stilinin dolgu rengi"), False),
                                         "width": (self.float, False)}))(v)
        if not styles:
            self.fail("bad_value", "boş duvar stili listesi yazılmaz")
        ids, names = set(), set()
        for st in styles:
            if st["id"] in ids or not st["id"]:
                self.fail("bad_value", f"duvar stilinin kimliği “{st['id']}” boş ya da iki kez var")
            ids.add(st["id"])
            name = st["name"]
            if not name.strip() or name != name.strip() or name.lower() in names or name.lower() == "standart":
                self.fail("bad_value", f"duvar stilinin adı “{name}” boş, kenarı boşluklu, Standart ya da iki kez var")
            names.add(name.lower())
            self.wall_components_rules(st["components"], None, f"“{name}” duvar stilinin")
            if "width" in st:
                fixed = sum(c["thickness"] for c in st["components"] if "thickness" in c)
                if st["components"] and all("thickness" in c for c in st["components"]):
                    self.fail("bad_value", f"“{name}” duvar stilinin genişliği bileşenlerinin toplamıdır, ayrıca yazılmaz")
                if not fixed < st["width"] <= WALL_MAX_WIDTH:
                    self.fail("bad_value", f"“{name}” duvar stilinin genişliği {st['width']:g} m; {fixed:g} m'den büyük olmalı")
        return styles

    def arch_(self, v):
        """The project's levels and the walls' detail (docs/adr/0339 §3); an empty one is not written."""
        a = self.fields({"levels": (self.array(self.fields({"id": (self.text, True), "name": (self.text, True),
                                                            "elevation": (self.float, True), "height": (self.float, False)})), False),
                         "detail": (self.enum(WALL_DETAILS), False),
                         # Schema 78 (docs/adr/0342 §1): the project's space method.
                         **({"spaceMethod": (self.enum(SPACE_METHODS), False)} if self.spaces else {}),
                         # Schema 82 (docs/adr/0346 §2): the phases the drawing shows.
                         **({"phaseView": (self.enum(("all", "existing", "demolition", "proposed")), False)} if self.phases else {}),
                         **({key:(self.any, False) for key in ("spaceNames","spaceUsages","spaceNumberPrefixes")} if self.space_choices else {})})(v)
        if not a:
            self.fail("bad_value", "boş mimari ayar yazılmaz")
        ids, names = set(), set()
        for lv in a.get("levels", []):
            if not lv["id"] or lv["id"] in ids or not lv["name"].strip() or lv["name"].lower() in names or lv["name"].lower() == "standart":
                self.fail("bad_value", f"kat “{lv['name']}”: kimliği ve adı dolu ve tek olmalı")
            ids.add(lv["id"])
            names.add(lv["name"].lower())
            if "height" in lv and not lv["height"] > 0:
                self.fail("bad_value", f"“{lv['name']}” katının yüksekliği sıfırdan büyük olmalı")
        if "levels" in a and not a["levels"]:
            self.fail("bad_value", "boş kat listesi yazılmaz")
        for key in ("spaceNames", "spaceUsages"):
            if key not in a:
                continue
            values = a[key]
            if type(values) is not list or not 1 <= len(values) <= 256:
                self.fail("bad_value", f"{key}: 1–256 metin olmalı; boş liste yazılmaz")
            seen = set()
            for value in values:
                if type(value) is not str or not value or value.strip() != value or len(value) > 256 or any(unicodedata.category(c) == "Cc" for c in value):
                    self.fail("bad_value", f"{key}: boş/kenarı boşluklu/kontrol karakterli veya uzun değer")
                if value in seen:
                    self.fail("bad_value", f"{key}: yinelenen tam değer")
                seen.add(value)
        if "spaceNumberPrefixes" in a:
            prefixes = a["spaceNumberPrefixes"]
            if type(prefixes) is not dict or not prefixes:
                self.fail("bad_value", "kat önekleri boş olmayan harita olmalı")
            for level, prefix in prefixes.items():
                if level not in ids:
                    self.fail("bad_value", "mahal önekinin katı projede yok")
                if type(prefix) is not str or not prefix or prefix.strip() != prefix or len(prefix) > 253 or any(unicodedata.category(c) == "Cc" for c in prefix):
                    self.fail("bad_value", "geçersiz mahal numara öneki")
        return a

    def leader_points(self, v):
        pts = self.array(self.point)(v)
        if len(pts) < 2:
            self.fail("bad_value", f"kılavuzun {len(pts)} köşesi var; en az iki olmalı")
        return pts

    def leader_height(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"kılavuzun yüksekliği {x}; 0'dan büyük olmalı")
        return x

    def note(self, v):
        t = self.text(v)
        if t == "":
            self.fail("bad_value", "kılavuzun notu boş; notsuz kılavuzun not alanı yazılmaz")
        return t

    def scale(self, v):
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"blok ölçeği {x}; pozitif olmalı")
        return x

    def attribute_places(self, v):
        """An insert's moved attributes (schema 58, docs/adr/0317 §1): tag → point, not empty."""
        if type(v) is dict and not v:
            self.fail("bad_value", "boş öznitelik yerleri yazılmaz")
        if type(v) is not dict:
            self.fail("wrong_type", "harita olmalı")
        out = {}
        for k, p in v.items():
            self.path.append(k)
            out[k] = self.point(p)
            self.path.pop()
        return out

    def places_rule(self, fields, movable):
        """A place only for a movable attribute of the insert's block (docs/adr/0317 §1)."""
        for tag in fields.get("attributePlaces", {}):
            if tag not in movable:
                self.path.append("attributePlaces")
                self.fail("bad_value", f"“{tag}” özniteliği blokta taşınabilir değil; yeri yazılmaz")

    def y_scale(self, v):
        """An insert's own y factor (schema 57, docs/adr/0315 §3): positive."""
        x = self.float(v)
        if not x > 0.0:
            self.fail("bad_value", f"blok y ölçeği {x}; pozitif olmalı")
        return x

    def mirror(self, v):
        if self.bool(v) is not True:
            self.fail("bad_value", "aynalama false yazılmaz; aynalı olmayan yerleştirmede alan yoktur")
        return True

    def check_blocks(self, blocks):
        """The block rules over the whole list, in their order: each definition's name, id and tags; the inserts in
        the definitions; then the nesting. The first broken one is said at its definition."""
        ids, names = {}, {}
        for i, b in enumerate(blocks):
            self.path.append(str(i))
            if all(c in WHITE_SPACE for c in b["name"]):
                self.path.append("name")
                self.fail("bad_value", "blok adı boş ya da yalnız boşluk olamaz")
            if b["id"] in ids:
                self.path.append("id")
                self.fail("duplicate_block", f"blok kimliği {b['id']} {ids[b['id']] + 1}. tanımda da var")
            ids[b["id"]] = i
            key = name_key(b["name"])
            if key in names:
                self.path.append("name")
                self.fail("duplicate_block", f"“{b['name']}” adı {names[key] + 1}. tanımda da var")
            names[key] = i
            if "dynamic" in b:
                problem = dynamic_problem(b["dynamic"], {e["id"] for e in b["entities"]})
                if problem:
                    self.path.append("dynamic")
                    self.fail("bad_value", problem)
            tags = {}
            for a, attribute in enumerate(b.get("attributes", [])):
                self.path += ["attributes", str(a), "tag"]
                if not attribute["tag"]:
                    self.fail("bad_value", "öznitelik etiketi boş")
                if attribute["tag"] in tags:
                    self.fail("bad_value", f"“{attribute['tag']}” etiketi {tags[attribute['tag']] + 1}. öznitelikte de var")
                tags[attribute["tag"]] = a
                del self.path[-3:]
            self.path.pop()
        for i, b in enumerate(blocks):
            for j, e in enumerate(b["entities"]):
                if e["kind"] == "insert" and e["block"] not in ids:
                    self.path += [str(i), "entities", str(j), "insert", "block"]
                    self.fail("unknown_block", "yerleştirilen blok çizimde tanımlı değil")
                if e["kind"] == "insert":
                    movable = {a["tag"] for a in blocks[ids[e["block"]]].get("attributes", []) if a.get("movable")}
                    self.path += [str(i), "entities", str(j), "insert"]
                    self.values_rule(e, blocks[ids[e["block"]]].get("dynamic"))
                    self.places_rule(e, movable)
                    del self.path[-4:]
        depth = [0] * len(blocks)
        stack = []

        def visit(i):
            stack.append(i)
            deepest = 0
            for e in blocks[i]["entities"]:
                if e["kind"] != "insert":
                    continue
                k = ids[e["block"]]
                if depth[k] == 0:
                    if k in stack:
                        self.path.append(str(k))
                        self.fail("block_cycle", f"“{blocks[k]['name']}” bloğu kendini içeriyor")
                    if len(stack) == MAX_BLOCK_DEPTH:
                        self.path.append(str(stack[0]))
                        self.fail("block_too_deep", f"bloklar {MAX_BLOCK_DEPTH} düzeyden derin iç içe")
                    visit(k)
                if len(stack) + depth[k] > MAX_BLOCK_DEPTH:
                    self.path.append(str(stack[0]))
                    self.fail("block_too_deep", f"bloklar {MAX_BLOCK_DEPTH} düzeyden derin iç içe")
                deepest = max(deepest, depth[k])
            stack.pop()
            depth[i] = deepest + 1

        for i in range(len(blocks)):
            if depth[i] == 0:
                visit(i)
        # No definition opens into more than a million pieces, nested inserts counted through, attributes with them
        # (docs/adr/0316 §4); shallowest first, the first in the list's order over the limit named.
        size = [0] * len(blocks)
        for i in sorted(range(len(blocks)), key=lambda i: (depth[i], i)):
            n = len(blocks[i].get("attributes", []))
            for e in blocks[i]["entities"]:
                n += size[ids[e["block"]]] if e["kind"] == "insert" else 1
            size[i] = n
        for i in range(len(blocks)):
            if size[i] > MAX_BLOCK_PIECES:
                self.path.append(str(i))
                self.fail("block_too_large", f"“{blocks[i]['name']}” bloğu {MAX_BLOCK_PIECES} parçadan çoğuna açılıyor")

    def common(self):
        return {
            **({"uid": (self.uid, True)} if self.inside is None else {}),
            "attrs": (self.attrs, True),
            "color": (self.text, False),
            "label": (self.text, False),
            "symbol": (self.text, False),
            "layerId": (self.text, True),
            **({"lineWeight": (self.line_weight, False)} if self.weights else {}),
            # Labels pinned by hand (schema 36, docs/adr/0212 §2): the drawing's objects only.
            **({"labelPins": (self.label_pins_, False)} if self.labels and self.inside is None else {}),
            # Photos and documents (schema 40, docs/adr/0225 §1): the drawing's objects only.
            **({"attachments": (self.attachments_, False)} if self.attachments and self.inside is None else {}),
            # An object's look (schema 48, docs/adr/0260): the order the drawing's objects', `byLayer` a block's.
            **({"lineType": (self.enum(LINE_TYPES), False), "lineTypeScale": (self.float, False), "transparency": (self.float, False)} if self.look else {}),
            **({"order": (self.order, False)} if self.look and self.inside is None else {}),
            **({"byLayer": (self.array(self.enum(LOOK_PARTS)), False)} if self.look and self.inside is not None else {}),
            # An area's mask (schema 54, docs/adr/0310): `{ frame }`; only an area has it.
            **({"wipeout": (self.fields({"frame": (self.bool, True)}), False)} if self.wipeouts else {}),
            # Its own project line type (schema 56, docs/adr/0313 §1).
            **({"linePattern": (self.text, False)} if self.patterns else {}),
        }

    def order(self, v):
        """An object's drawing order (schema 48): a 32-bit whole number."""
        if type(v) is not int:
            self.fail("wrong_type", "tam sayı olmalı")
        if not -(2**31) <= v < 2**31:
            self.fail("bad_value", f"çizim sırası {v} aralık dışında")
        return v

    def look_rules(self, obj):
        """An object's look (schema 48, docs/adr/0260 §1–§3): a scale above 0 and at most 1000, a transparency from 0
        to 0.9; the by-layer parts in order, once, none with a value of its own."""
        s = obj.get("lineTypeScale")
        if s is not None and not 0 < s <= 1000:
            self.fail("bad_value", f"çizgi tipi ölçeği {s}; 0'dan büyük, en çok 1000 olmalı")
        t = obj.get("transparency")
        if t is not None and not 0 <= t <= 0.9:
            self.fail("bad_value", f"saydamlık {t}; 0 ile 0,9 arası olmalı")
        parts = obj.get("byLayer", [])
        if "byLayer" in obj and not parts:
            self.fail("bad_value", "boş katmana göre listesi yazılmaz")
        idx = [LOOK_PARTS.index(p) for p in parts]
        if idx != sorted(set(idx)):
            self.fail("bad_value", "katmana göre parçalar sırayla ve birer kez yazılır")
        if any(p in obj for p in parts):
            self.fail("bad_value", "kendi değeri olan parça katmana göre olamaz")

    def line_weight(self, v):
        w = self.float(v)
        if not 0.0 <= w <= MAX_LINE_WEIGHT:
            self.fail("bad_value", f"çizgi kalınlığı {w} mm; 0 ile {MAX_LINE_WEIGHT:g} arasında olmalı")
        return w

    def elevation(self, v):
        """A vertex's elevation: a float, or `null` for a vertex without one (§6.6)."""
        return None if v is None else self.float(v)

    def same_length(self, obj):
        """One elevation per vertex (§6.6): `zs` as long as `pts`. Keys come in encoded order, `zs` before `pts`, so this is checked once the map is read."""
        if "zs" in obj and len(obj["zs"]) != len(obj["pts"]):
            self.path.append("zs")
            self.fail("bad_value", f"{len(obj['zs'])} kot var ama {len(obj['pts'])} köşe var; her köşenin bir kotu olmalı (kotsuz köşe için null)")
        # One measure per vertex (schema 39, §6.6), alike.
        if "ms" in obj and len(obj["ms"]) != len(obj["pts"]):
            self.path.append("ms")
            self.fail("bad_value", f"{len(obj['ms'])} ölçü var ama {len(obj['pts'])} köşe var; her köşenin bir ölçüsü olmalı (ölçüsüz köşe için null)")

    def space_separator_role(self, value):
        if type(value) is not dict:
            self.fail("bad_value", "mahal ayırıcı rolü harita olmalı")
        return self.arch_info(value)

    def space_separator_rules(self, obj):
        """Independent schema 111 role constraints; ordinary paths are unchanged."""
        pts=obj["pts"]
        if not 2 <= len(pts) <= 10_000 or any(a==b for a,b in zip(pts,pts[1:])) or (obj.get("closed") and pts[0]==pts[-1]):
            self.fail("bad_value", "mahal ayırıcının köşeleri/kenarları geçersiz")
        if any(k in obj for k in ("holes","parts","widths","zs","ms")):
            self.fail("bad_value", "mahal ayırıcı tek parçalı, düzlemsel ve genişliksiz olmalı")
        if "bulges" in obj and len(obj["bulges"])!=len(pts):
            self.fail("bad_value", "mahal ayırıcı büküm sayısı köşe sayısıyla eşleşmeli")

    def width_pair(self, v):
        """A polyline edge's start and end width (schema 46, docs/adr/0258 §2): two numbers."""
        if type(v) is not list:
            self.fail("wrong_type", "genişlik [başlangıç, bitiş] dizisi olmalı")
        if len(v) != 2:
            self.fail("bad_value", f"genişlik 2 sayı olmalı (başlangıç, bitiş), {len(v)} var")
        return [self.at(0, self.float, v[0]), self.at(1, self.float, v[1])]

    def degree(self, v):
        """A NURBS curve's degree (schema 47): a whole number from 1 to 11."""
        if type(v) is not int:
            self.fail("wrong_type", "tam sayı olmalı")
        if not 1 <= v <= NURBS_MAX_DEGREE:
            self.fail("bad_value", f"derece {v}; 1 ile {NURBS_MAX_DEGREE} arasında olmalı")
        return v

    def tangents(self, v):
        """A NURBS curve's fit points' end tangents (schema 47): two entries, each a point or `null`."""
        if type(v) is not list:
            self.fail("wrong_type", "teğetler [başlangıç, bitiş] dizisi olmalı")
        if len(v) != 2:
            self.fail("bad_value", f"teğetler 2 öğe olmalı, {len(v)} var")
        return [None if t is None else self.at(i, self.point, t) for i, t in enumerate(v)]

    def nurbs_rules(self, obj):
        """A NURBS curve's numbers make a curve (schema 47, docs/adr/0259 §1): at least degree + 1 control points, as
        many knots as control points + degree + 1, never decreasing, a span that is not empty, an inner knot at most
        degree times, one positive weight per control point, a fit tolerance not below 0."""
        p, n, knots = obj["degree"], len(obj["ctrl"]), obj["knots"]
        if not p + 1 <= n <= NURBS_MAX_POINTS:
            self.fail("bad_value", f"{p}. dereceden eğrinin {n} denetim noktası var; en az {p + 1}, en çok {NURBS_MAX_POINTS}")
        if len(knots) != n + p + 1:
            self.fail("bad_value", f"{len(knots)} düğüm var; denetim noktası + derece + 1 = {n + p + 1} olmalı")
        if any(b < a for a, b in zip(knots, knots[1:])):
            self.fail("bad_value", "düğümler azalmamalı")
        if not knots[n] > knots[p]:
            self.fail("bad_value", "eğrinin aralığı boş")
        run, inner = 1, knots[1:-1]
        for a, b in zip(inner, inner[1:]):
            run = run + 1 if a == b else 1
            if run > p:
                self.fail("bad_value", f"iç düğüm {run} kez yinelenmiş; en çok derece ({p}) kez")
        if "weights" in obj:
            w = obj["weights"]
            if len(w) != n:
                self.fail("bad_value", f"{len(w)} ağırlık var ama {n} denetim noktası var")
            if any(not x > 0 for x in w):
                self.fail("bad_value", "ağırlıklar sıfırdan büyük olmalı")
        if obj.get("fitTolerance", 0.0) < 0:
            self.fail("bad_value", "geçiş toleransı sıfır ya da büyük olmalı")

    def polyline_extras_rules(self, obj):
        """A closed and a wide polyline (schema 46, docs/adr/0258): of one part; a closed one of two vertices or more;
        widths one pair for each vertex, not below 0, the last pair 0 on an open line and not all 0."""
        closed = obj.get("closed", False)
        widths = obj.get("widths")
        if (closed or widths is not None) and "parts" in obj:
            self.path.append("parts")
            self.fail("bad_value", "kapalı ya da genişlikli çoklu çizgi tek parçalıdır")
        n = len(obj["pts"])
        if closed and n < 2:
            self.path.append("closed")
            self.fail("bad_value", f"kapalı çoklu çizginin {n} köşesi var; en az iki olmalı")
        if widths is None:
            return
        self.path.append("widths")
        if len(widths) != n:
            self.fail("bad_value", f"{len(widths)} genişlik var ama {n} köşe var; her köşeden çıkan kenara bir genişlik çifti")
        for i, (a, b) in enumerate(widths):
            if not (a >= 0 and b >= 0):
                self.path.append(str(i))
                self.fail("bad_value", "genişlik sıfır ya da büyük olmalı")
        if not closed and widths[-1] != [0.0, 0.0]:
            self.path.append(str(n - 1))
            self.fail("bad_value", "açık çoklu çizginin son köşesinden kenar çıkmaz; son genişlik [0, 0] olmalı")
        if all(a == 0 and b == 0 for a, b in widths):
            self.fail("bad_value", "bütün genişlikler sıfır; genişliksiz çizginin alanı yazılmaz")
        self.path.pop()

    def ring(self, v):
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        ring = self.fields(table)(v)
        self.same_length(ring)
        return ring

    def line_part(self, v):
        """A part of a multi-part polyline past its first (schema 17, §6.6): two vertices or more, its arcs and
        elevations; no holes."""
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        # A polyline's part's measures from schema 39 on (docs/adr/0224).
        if self.measures:
            table["ms"] = (self.array(self.elevation), False)
        part = self.fields(table)(v)
        if len(part["pts"]) < 2:
            self.path.append("pts")
            self.fail("bad_value", f"çoklu çizginin parçasının {len(part['pts'])} köşesi var; en az iki olmalı")
        self.same_length(part)
        return part

    def point_part(self, v):
        """A point of a multi-point object past its first (schema 17, §6.6): its place and elevation."""
        return self.fields({"p": (self.point, True), "z": (self.float, False)})(v)

    def part(self, v):
        """A part of a multi-part area past its first (§6.6): its ring, arcs, holes and elevations, as the area's own."""
        table = {"pts": (self.array(self.point), True), "bulges": (self.array(self.float), False), "holes": (self.array(self.ring), False)}
        if self.elevations:
            table["zs"] = (self.array(self.elevation), False)
        part = self.fields(table)(v)
        self.same_length(part)
        return part


def _path(s, holes):
    table = {"pts": (s.array(s.point), True), "bulges": (s.array(s.float), False)}
    if holes:
        table["holes"] = (s.array(s.ring), False)
        # An area has parts from schema 5 on.
        if s.parts:
            table["parts"] = (s.array(s.part), False)
    elif s.line_parts:
        # A polyline from schema 17 on (docs/adr/0174).
        table["parts"] = (s.array(s.line_part), False)
    if s.elevations:
        table["zs"] = (s.array(s.elevation), False)
    # A polyline's measures from schema 39 on (docs/adr/0224): a route's; an area has none.
    if s.measures and not holes:
        table["ms"] = (s.array(s.elevation), False)
    # A polyline's `closed` and `widths` from schema 46 on (docs/adr/0258): a line's; an area has neither.
    if s.space_separators and not holes:
        table["spaceSeparator"] = (s.space_separator_role, False)
    if s.polyline_extras and not holes:
        table["closed"] = (s.only_true, False)
        table["widths"] = (s.array(s.width_pair), False)
    return table


ENTITY_KINDS = {
    "point": lambda s: {"p": (s.point, True), "z": (s.float, False), **({"parts": (s.array(s.point_part), False)} if s.line_parts else {})},
    # A line's end without an elevation has no key: `null` is not written there (§5.2).
    "line": lambda s: {
        "a": (s.point, True),
        "b": (s.point, True),
        **({"za": (s.float, False), "zb": (s.float, False)} if s.elevations else {}),
        # Its ends' measures from schema 39 on (docs/adr/0224), as its elevations.
        **({"ma": (s.float, False), "mb": (s.float, False)} if s.measures else {}),
    },
    "polyline": lambda s: _path(s, holes=False),
    "polygon": lambda s: _path(s, holes=True),
    "circle": lambda s: {"c": (s.point, True), "r": (s.float, True)},
    "arc": lambda s: {"c": (s.point, True), "r": (s.float, True), "a0": (s.float, True), "a1": (s.float, True)},
    "ellipse": lambda s: {"c": (s.point, True), "major": (s.point, True), "ratio": (s.float, True), "t0": (s.float, True), "t1": (s.float, True)},
    "spline": lambda s: {"pts": (s.array(s.point), True), "closed": (s.bool, True)},
    # Schema 47 (docs/adr/0259): a NURBS curve; `closed` and `periodic` only when true.
    "nurbs": lambda s: {
        "degree": (s.degree, True),
        "ctrl": (s.array(s.point), True),
        "knots": (s.array(s.float), True),
        "weights": (s.array(s.float), False),
        "closed": (s.only_true, False),
        "periodic": (s.only_true, False),
        "fit": (s.array(s.point), False),
        "tangents": (s.tangents, False),
        "fitTolerance": (s.float, False),
    },
    "xline": lambda s: {"p": (s.point, True), "dir": (s.point, True)},
    "ray": lambda s: {"p": (s.point, True), "dir": (s.point, True)},
    "text": lambda s: {
        "p": (s.point, True),
        "text": (s.text, True),
        "height": (s.float, True),
        "rotation": (s.float, True),
        **(text_extras(s, mask=True) if s.texts else {}),
        # Schema 18 (docs/adr/0175 §4): the object whose label it writes and the scale, only where objects have ids.
        **({"labelOf": (s.id16, False), "labelScale": (s.label_scale, False)} if s.linked_texts and s.inside is None else {}),
        # Schema 20 (docs/adr/0182 §1): a multi-line text's box, line spacing and letter formats.
        **({"boxWidth": (s.box_width, False), "lineSpacing": (s.line_spacing, False), "runs": (s.runs, False)} if s.paragraphs else {}),
        # Schema 21 (docs/adr/0183 §2): its style, typeface, bold, italic and slant.
        **(
            {
                "textStyle": (s.style_id, False),
                "font": (s.enum(DRAWING_FONTS), False),
                "bold": (s.face_flag, False),
                "italic": (s.face_flag, False),
                "oblique": (s.oblique, False),
            }
            if s.styles
            else {}
        ),
        # Schema 25 (docs/adr/0196 §1): the curve its letters stand on.
        **({"path": (s.text_path, False)} if s.text_paths else {}),
        # Schema 63 (docs/adr/0322 §5): the fields set apart from its style.
        **({"overrides": (s.overrides_of(TEXT_OVERRIDES), False)} if s.overrides else {}),
        # Schema 64 (docs/adr/0323): its scales.
        **({"annotative": (s.annotative_of("text"), False)} if s.annotative else {}),
        **({"layout": (s.layout_of, False)} if s.paragraph_layout else {}),
        **({"fields": (s.text_fields_of, False)} if s.text_fields else {}),
    },
    # Schema 9 (docs/adr/0147): five more kinds, `mask` only when true, a slope's two elevations.
    "dimension": lambda s: {
        "a": (s.point, True),
        "b": (s.point, True),
        "c": (s.point, False),
        "text": (s.text, False),
        "angle": (s.float, False),
        "style": (s.enum(DIMENSION_STYLES if s.dim_marks else DIMENSION_STYLES[:10] if s.dimensions else DIMENSION_STYLES[:5]), False),
        "height": (s.float, True),
        "offset": (s.float, True),
        **({"mask": (s.dimension_mask, False), "za": (s.float, False), "zb": (s.float, False)} if s.dimensions else {}),
        # Schema 21 (docs/adr/0183 §3): its style, arrowheads, sizes, value's place and writing, typeface.
        **(
            {
                "dimStyle": (s.style_id, False),
                "arrow": (s.enum(DIMENSION_ARROWS), False),
                "arrowSize": (s.size(True), False),
                "extOffset": (s.size(False), False),
                "extBeyond": (s.size(False), False),
                "textGap": (s.size(False), False),
                "textPlace": (s.enum(("centre",)), False),
                "decimals": (s.decimals, False),
                "unit": (s.enum(("mm", "cm", "m")), False),
                "prefix": (s.affix("prefix"), False),
                "suffix": (s.affix("suffix"), False),
                "font": (s.enum(DRAWING_FONTS), False),
            }
            if s.styles
            else {}
        ),
        # Schema 30 (docs/adr/0205 §6): its lines.
        **(
            {
                "dimLineColor": (s.dimension_colour("dimLineColor"), False),
                "dimLineWeight": (s.dimension_weight("dimLineWeight"), False),
                "dimLineType": (s.enum(LINE_TYPES), False),
                "extColor": (s.dimension_colour("extColor"), False),
                "extWeight": (s.dimension_weight("extWeight"), False),
                "extLineType": (s.enum(LINE_TYPES), False),
                "textColor": (s.dimension_colour("textColor"), False),
            }
            if s.annotation
            else {}
        ),
        # Schema 43 (docs/adr/0249 §4): what its points follow, only in the drawing (a block definition's objects have
        # no persistent ids to name).
        **({"refs": (s.dimension_refs, False)} if s.dim_refs and s.inside is None else {}),
        # Schema 61 (docs/adr/0321): its options, its moved value and its breaks.
        **({"options": (s.dimension_options, False), "textShift": (s.text_shift, False), "breaks": (s.dimension_breaks, False)}
           if s.dim_options else {}),
        # Schema 63 (docs/adr/0322 §5): the fields set apart from its style.
        **({"overrides": (s.overrides_of(DIMENSION_OVERRIDES), False)} if s.overrides else {}),
        # Schema 64 (docs/adr/0323): its scales.
        **({"annotative": (s.annotative_of("dimension"), False)} if s.annotative else {}),
    },
    # Schema 23 (docs/adr/0186): a pattern's name, scale and families, a gradient; the objects it follows, only in the
    # drawing (a block definition's objects have no persistent ids to name).
    "hatch": lambda s: {
        "ring": (s.array(s.point), True),
        "holes": (s.array(s.array(s.point)), False),
        "pattern": (s.hatch_pattern, True),
        **({"assoc": (s.hatch_assoc, False)} if s.hatches and s.inside is None else {}),
        # Schema 69 (docs/adr/0328 §1): its exact boundary.
        **({"boundary": (s.hatch_boundary, False)} if s.hatch_boundaries else {}),
        # Schema 71 (docs/adr/0331 §3): its scales; a pattern with lines only.
        **({"annotative": (s.annotative_of("hatch"), False)} if s.hatch_extras else {}),
    },
    # Schema 6 (docs/adr/0144): `mirror` only when true.
    "insert": lambda s: {"block": (s.id16, True), "p": (s.point, True), "scale": (s.scale, True), "rotation": (s.float, True), "mirror": (s.mirror, False),
                         **({"yScale": (s.y_scale, False)} if s.block_extras else {}),
                         **({"attributePlaces": (s.attribute_places, False)} if s.block_cards else {}),
                         **({"parameters": (s.param_values, False)} if s.dynamic_blocks else {}),
                         **({"clipping": (s.insert_clipping, False)} if s.clipping else {})},
    # Schema 22 (docs/adr/0184): rows and columns of one-line cells, merged ranges, alignments, a heading row only when
    # true, lines by name (all: no field), a frame's width, a text's face, a source; only in the drawing.
    "table": lambda s: {
        "p": (s.point, True),
        "rotation": (s.float, True),
        "height": (s.float, True),
        "rows": (s.array(s.float), True),
        "columns": (s.array(s.float), True),
        "cells": (s.array(s.array(s.text)), True),
        "merges": (s.merges, False),
        "aligns": (s.array(s.enum(TABLE_ALIGNS)), False),
        "header": (s.table_header, False),
        "grid": (s.enum(TABLE_GRIDS), False),
        "frame": (s.float, False),
        "textStyle": (s.style_id, False),
        "font": (s.enum(DRAWING_FONTS), False),
        "bold": (s.face_flag, False),
        "italic": (s.face_flag, False),
        "oblique": (s.oblique, False),
        "source": (s.table_source, False),
        # Schema 68 (docs/adr/0327): the cells' styles, formulas and fields, a linked file's data, a block schedule.
        **({"styles": (s.cell_styles, False), "formulas": (s.cell_formulas, False), "fields": (s.cell_fields, False),
            "link": (s.table_link, False), "schedule": (s.table_schedule, False)} if s.table_cells else {}),
    },
    # Schema 24 (docs/adr/0192): a picture's frame, its one source, its clip and opacity; `mirror` only when true; only in
    # the drawing.
    "image": lambda s: {
        "p": (s.point, True),
        "width": (s.float, True),
        "height": (s.float, True),
        "rotation": (s.float, True),
        "mirror": (s.mirror, False),
        "asset": (s.text, False),
        "file": (s.text, False),
        "clip": (s.array(s.point), False),
        "opacity": (s.float, False),
    },
    # Schema 29 (docs/adr/0204 §2): a raster's affine, size, bands and samples, its one source, its file's system, its
    # look and opacity; only in the drawing.
    "raster": lambda s: {
        "affine": (s.raster_affine, True),
        "width": (s.raster_side(MAX_RASTER_SIDE), True),
        "height": (s.raster_side(MAX_RASTER_SIDE), True),
        "bands": (s.raster_side(MAX_RASTER_BANDS), True),
        "sample": (s.enum(RASTER_SAMPLES), True),
        "asset": (s.text, False),
        "file": (s.text, False),
        # Schema 31 (docs/adr/0207 §1): an address read by ranges.
        **({"url": (s.text, False)} if s.point_clouds else {}),
        "srid": (s.uint(32), True),
        "style": (s.raster_style, True),
        "opacity": (s.float, False),
        # Schema 38 (docs/adr/0243 §6): a NetCDF variable and its slice.
        **({"dataset": (s.raster_dataset, False)} if s.multidim else {}),
        # Schema 88 (docs/adr/0518 §1): how it was georeferenced.
        **({"georef": (s.raster_georef, False)} if s.georef else {}),
        # Schema 103 (docs/adr/0627 §3): the GeoPackage tile table it shows from its file.
        **({"tiles": (s.raster_tiles, False)} if s.tile_tables else {}),
    },
    # Schema 31 (docs/adr/0207 §3): a point cloud's files, their bounds and points together, the files' system, its
    # look and opacity; only in the drawing.
    "pointcloud": lambda s: {
        "sources": (s.array(s.cloud_source), True),
        "bounds": (s.bounds6, True),
        "count": (s.uint(64), True),
        "srid": (s.uint(32), True),
        "style": (s.cloud_style, True),
        "opacity": (s.float, False),
    },
    # Schema 60 (docs/adr/0410 §2): a record without a geometry, no fields of its own.
    "record": lambda s: {},
    # Schema 75 (docs/adr/0339 §4): a wall; `closed` only when true, `caps` only when open.
    "wall": lambda s: {
        "pts": (s.array(s.point), True),
        "bulges": (s.array(s.float), False),
        "closed": (s.only_true, False),
        "width": (s.float, True),
        "justify": (s.enum(("left", "center", "right")), True),
        "offset": (s.float, False),
        "style": (s.text, False),
        "components": (s.array(s.wall_component), False),
        "caps": (s.wall_caps, False),
        "fill": (s.wall_colour("duvarın dolgu rengi"), False),
        "height": (s.float, False),
        "arch": (s.arch_info, False),
        # Schema 76 (docs/adr/0340 §1): its joins, its cleanup only when off.
        **({"joins": (s.wall_joins_, False), "cleanup": (s.only_false, False)} if s.wall_joins else {}),
    },
    # Schema 79 (docs/adr/0343 §1): a grid, a column, a stair, a ramp; its fields as one map under its kind's name.
    "grid": lambda s: {"grid": (s.arch_element("grid"), True)},
    "column": lambda s: {"column": (s.arch_element("column"), True)},
    "stair": lambda s: {"stair": (s.arch_element("stair"), True)},
    "ramp": lambda s: {"ramp": (s.arch_element("ramp"), True)},
    # Schema 80 (docs/adr/0344 §1): a tag; its fields as one map.
    "tag": lambda s: {"tag": (s.tag_fields, True)},
    # Schema 81 (docs/adr/0345 §1): a view mark; its fields as one map.
    "viewmark": lambda s: {"viewmark": (s.viewmark_fields, True)},
    # Schema 78 (docs/adr/0342 §1): a space; its parts and the rest of its fields.
    "space": lambda s: {
        "parts": (s.array(s.space_part), True),
        "space": (s.space_fields, True),
    },
    # Schema 77 (docs/adr/0341 §1): an opening; its jambs written from its wall.
    "opening": lambda s: {
        "host": (s.text, True),
        "at": (s.float, True),
        "width": (s.float, True),
        "place": (s.array(s.point), True),
        "opening": (s.opening_fields, True),
    },
    # Schema 8 (docs/adr/0146): two vertices or more, a positive height, a note that is not empty, `mask` only when true.
    # Schema 30 (docs/adr/0205 §7): AutoCAD's arrowheads, the arrowhead's size times the note's height.
    "leader": lambda s: {
        "pts": (s.leader_points, True),
        "text": (s.note, False),
        "height": (s.leader_height, True),
        "rotation": (s.float, True),
        "arrow": (s.enum(LEADER_ARROWS_30 if s.annotation else LEADER_ARROWS), False),
        "mask": (s.mask, False),
        **({"arrowSize": (s.leader_arrow_size, False)} if s.annotation else {}),
        # Schema 64 (docs/adr/0323): its scales.
        **({"annotative": (s.annotative_of("leader"), False)} if s.annotative else {}),
        # Schema 67 (docs/adr/0326): its arms, tip, content blocks and style.
        **({"arms": (s.leader_arms, False), "tip": (s.json_subref, False), "blocks": (s.leader_blocks, False),
            "leaderStyle": (s.text, False)} if s.multileader else {}),
    },
}


def text_extras(s, mask):
    """Schema 7's fields of a text (`mask` too) or an attribute definition (docs/adr/0145)."""
    table = {"align": (s.enum(TEXT_ALIGNS), False), "widthFactor": (s.width_factor, False)}
    if mask:
        table["mask"] = (s.mask, False)
    return table


def attribute_options_problem(a):
    """What is wrong with an attribute definition's options (kentos_contracts::blocks::options_problem), or None."""
    constant = a.get("constant", False)
    if constant and (a.get("verify") or a.get("preset")):
        return "sabit öznitelik sorulmaz; doğrula ve ön ayarlı seçilemez"
    if constant and a.get("movable"):
        return "sabit özniteliğin yeri yerleştirmede değişmez; taşınabilir seçilemez"
    if constant and "field" in a:
        return "sabit öznitelik alan olamaz"
    if "boxWidth" in a and not a.get("multiline"):
        return "kutu genişliği yalnız çok satırlı öznitelikte olur"
    if "field" in a and not a["field"].strip():
        return "alan boş olamaz"
    return None


def name_key(name):
    """A block's name as names are compared: each character lowercased, Turkish I (I → ı, İ → i)."""
    return "".join("ı" if c == "I" else "i" if c == "İ" else c.lower() for c in name)


def uuid_text(raw):
    h = raw.hex()
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


def read(data):
    """Reads a whole file: (header, document in the contract's JSON form); raises KcadError."""
    header, payload = read_header(data)
    return header, _Schema().root(decode_payload(payload))


# ── Command line ────────────────────────────────────────────────────────


def _summary(header, doc):
    kinds = Counter(e["kind"] for e in doc["entities"])
    versions = Counter(int(u[14], 16) for u in doc["uids"])
    layers = []

    def walk(nodes):
        for n in nodes:
            layers.append(n)
            walk(n["children"])

    walk(doc["layers"])
    lines = [
        f"KCAD {header['major']}.{header['minor']} (en az okuyucu {header['minReader']}), kodlama {header['encoding']} (CBOR profili 1), sıkıştırma yok",
        f"Yük: {header['payloadLength']} bayt, SHA-256 doğru ({header['sha256'][:16]}…)",
        f"Belge: {doc['format']} sürüm {doc['version']}",
        f"Ad: {doc['name']}",
        f"Koordinat sistemi: EPSG:{doc['settings']['srid']}, ölçek 1:{doc['settings']['plotScale']:g}",
        f"Katmanlar: {sum(1 for n in layers if n['type'] == 'layer')} katman, {sum(1 for n in layers if n['type'] == 'group')} grup; etkin: {doc['activeLayer']}",
        f"Nesneler: {len(doc['entities'])}" + (" — " + ", ".join(f"{k} {n}" for k, n in sorted(kinds.items())) if kinds else ""),
        f"Kalıcı kimlikler: {len(doc['uids'])}, benzersiz" + (" (" + ", ".join(f"v{v}: {n}" for v, n in sorted(versions.items())) + ")" if versions else ""),
        f"Proje kimliği: {doc.get('projectId', 'yok')}",
    ]
    source = doc.get("migratedFrom")
    lines.append(f"Göç kaynağı: {source['format']} sürüm {source['version']}, sha256 {source['sourceSha256']}" if source else "Göç kaynağı: yok")
    lines.append(f"Proje stilleri: {len(doc['styles']['items'])} öğe, {len(doc['styles']['categories'])} kategori")
    return "\n".join(lines)


def _json_default(v):
    raise TypeError(f"JSON'a yazılamıyor: {type(v).__name__}")


def main(argv):
    usage = "Kullanım: kcad.py inspect DOSYA | validate DOSYA… | dump DOSYA | sniff DOSYA…"
    if len(argv) < 2 or argv[0] not in ("inspect", "validate", "dump", "sniff"):
        print(usage, file=sys.stderr)
        return 2
    command, files = argv[0], argv[1:]
    if command in ("inspect", "dump") and len(files) != 1:
        print(usage, file=sys.stderr)
        return 2
    status = 0
    for name in files:
        try:
            with open(name, "rb") as f:
                data = f.read()
        except OSError as e:
            print(f"{name}: okunamadı ({e.strerror})", file=sys.stderr)
            status = 1
            continue
        if command == "sniff":
            print(f"{name}: {sniff(data)}")
            continue
        try:
            header, doc = read(data)
        except KcadError as e:
            print(f"{name}: {e.code}: {e.message}", file=sys.stderr if command != "validate" else sys.stdout)
            status = 1
            continue
        if command == "validate":
            print(f"{name}: geçerli ({len(doc['entities'])} nesne)")
        elif command == "inspect":
            print(_summary(header, doc))
        else:
            json.dump(doc, sys.stdout, ensure_ascii=False, indent=2, default=_json_default)
            print()
    return status




# Dynamic blocks (docs/adr/0318): the definition's rules and a value's, written from the ADR.

MAX_DYNAMIC_ITEMS = 256


def _dist(a, b):
    return math.hypot(b["x"] - a["x"], b["y"] - a["y"])


def _slack(x):
    return 1e-9 * max(abs(x), 1.0)


def number_problem(rule, v):
    """Why a number does not meet a rule: within min and max, on steps from min (or 0), one of the list."""
    if not math.isfinite(v):
        return "sonlu değil"
    if "min" in rule and v < rule["min"] - _slack(rule["min"]):
        return "en küçükten küçük"
    if "max" in rule and v > rule["max"] + _slack(rule["max"]):
        return "en büyükten büyük"
    if "step" in rule:
        k = (v - rule.get("min", 0.0)) / rule["step"]
        if abs(k - round(k)) * rule["step"] > _slack(v):
            return "adımda değil"
    if rule.get("list") and not any(abs(x - v) <= _slack(v) for x in rule["list"]):
        return "listede değil"
    return None


def default_number(p):
    if p["kind"] == "linear":
        return _dist(p["from"], p["to"])
    if p["kind"] == "polar":
        return _dist(p["from"], p["to"])
    return p["base"]


def value_problem(p, v):
    kind = p["kind"]
    if kind in ("linear", "angle"):
        return None if type(v) is float and number_problem(p.get("rule", {}), v) is None else "sayı kurala uymuyor"
    if kind == "polar":
        return None if type(v) is dict and number_problem(p.get("rule", {}), _dist(p["from"], v)) is None else "nokta kurala uymuyor"
    if kind == "point":
        return None if type(v) is dict else "nokta değil"
    if kind == "flip":
        return None if type(v) is bool else "açık ya da kapalı değil"
    if kind == "visibility":
        return None if type(v) is str and any(s["name"] == v for s in p["states"]) else "durum değil"
    return None if type(v) is str and any(r["label"] == v for r in p["rows"]) else "satır değil"


ACTION_FITS = {
    "move": ("linear", "polar", "point"), "stretch": ("linear", "polar", "point"), "scale": ("linear", "polar"),
    "array": ("linear",), "rotate": ("angle",), "flip": ("flip",),
}


def dynamic_problem(d, ids):
    """Why a dynamic part is not one (docs/adr/0318 §1), in the contract's order; None when it is."""
    params, actions = d["parameters"], d.get("actions", [])
    if not params:
        return "en az bir parametre olmalı"
    if len(params) > MAX_DYNAMIC_ITEMS or len(actions) > MAX_DYNAMIC_ITEMS:
        return "çok fazla parametre ya da eylem"
    names, visibility = set(), 0
    for p in params:
        name = p["name"]
        if not name.strip() or name.strip() != name:
            return "parametrenin adı boş ya da boşlukla çevrili"
        if name_key(name) in names:
            return f"“{name}” iki kez"
        names.add(name_key(name))
        pts = [p[k] for k in ("from", "to", "center", "at", "a", "b") if k in p]
        if any(not (math.isfinite(q["x"]) and math.isfinite(q["y"])) for q in pts):
            return "nokta sonlu değil"
        if p["kind"] in ("linear", "polar") and _dist(p["from"], p["to"]) == 0.0:
            return "iki nokta aynı"
        if p["kind"] == "flip" and _dist(p["a"], p["b"]) == 0.0:
            return "iki nokta aynı"
        if p["kind"] == "angle" and not math.isfinite(p["base"]):
            return "açı sonlu değil"
        if p["kind"] == "visibility":
            visibility += 1
            if visibility > 1:
                return "tek görünürlük parametresi olur"
            states = p["states"]
            if not states or len(states) > MAX_DYNAMIC_ITEMS:
                return "durum sayısı"
            seen = set()
            for s in states:
                if not s["name"].strip() or name_key(s["name"]) in seen:
                    return "durum adı"
                seen.add(name_key(s["name"]))
                if any(h not in ids for h in s.get("hidden", [])):
                    return "olmayan nesneyi gizliyor"
        if p["kind"] in ("linear", "polar", "angle"):
            r = p.get("rule", {})
            for k in ("min", "max", "step"):
                if k in r and not math.isfinite(r[k]):
                    return "kural sonlu değil"
            if "min" in r and "max" in r and r["min"] > r["max"]:
                return "en küçük en büyükten büyük"
            if "step" in r and r["step"] <= 0.0:
                return "adım artı değil"
            if any(not math.isfinite(x) for x in r.get("list", [])):
                return "liste sonlu değil"
            if "expression" in r and not r["expression"].strip():
                return "ifade boş"
            if "expression" in r and ("list" in r or "step" in r):
                return "ifadenin listesi ya da adımı olmaz"
            if "expression" not in r and number_problem(r, default_number(p)) is not None:
                return "varsayılan değer kurala uymuyor"
    by_name = {p["name"]: p for p in params}
    for p in params:
        if p["kind"] != "lookup":
            continue
        rows = p["rows"]
        if not rows or len(rows) > MAX_DYNAMIC_ITEMS:
            return "satır sayısı"
        labels = set()
        for row in rows:
            if not row["label"].strip() or name_key(row["label"]) in labels:
                return "satır adı"
            labels.add(name_key(row["label"]))
            if not row["values"]:
                return "satır değer vermiyor"
            for key, v in row["values"].items():
                t = by_name.get(key)
                if t is None or t["kind"] == "lookup":
                    return "satır olmayan parametreye değer veriyor"
                if "expression" in t.get("rule", {}):
                    return "satır hesaplanan parametreye değer veriyor"
                if value_problem(t, v):
                    return "satırın değeri uymuyor"
    for a in actions:
        p = by_name.get(a["parameter"])
        if p is None:
            return "eylemin parametresi yok"
        if p["kind"] not in ACTION_FITS[a["kind"]]:
            return "eylem parametreye uymuyor"
        pieces = a["pieces"]
        if not pieces or len(pieces) > MAX_DYNAMIC_ITEMS or len(set(pieces)) != len(pieces) or any(i not in ids for i in pieces):
            return "eylemin nesneleri"
        if a["kind"] == "stretch":
            lo, hi = a["min"], a["max"]
            if not all(math.isfinite(q) for q in (lo["x"], lo["y"], hi["x"], hi["y"])) or not (lo["x"] < hi["x"] and lo["y"] < hi["y"]):
                return "esnetmenin kutusu"
        if a["kind"] == "array" and not (math.isfinite(a["spacing"]) and a["spacing"] > 0.0):
            return "dizinin aralığı"
    return None


def values_problem(d, values):
    """Why an insert's kept values are not its block's (docs/adr/0318 §3)."""
    by_name = {p["name"]: p for p in d["parameters"]}
    for name, v in values.items():
        p = by_name.get(name)
        if p is None:
            return f"blokta “{name}” yok"
        if p["kind"] == "lookup":
            return f"“{name}” bir tablo"
        if value_problem(p, v):
            return f"“{name}” uymuyor"
    return None


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
