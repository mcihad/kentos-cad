# Drawing standard document, versions 1 and 2

A versioned drawing standard document is UTF-8 JSON. Its root is an object
with exactly these fields:

```json
{
  "format": "kentos.drawing-standard",
  "version": 1,
  "id": "office-architecture",
  "revision": "2026.1",
  "standard": { "name": "Office", "layers": [] }
}
```

`id` identifies the office definition and `revision` identifies the office's
revision of that definition. Both are nonblank strings of at most 128 UTF-8
bytes. The integer `version` identifies this document format, not the office
revision. A changed standard should receive a new office revision. Names are
not trusted as file paths and carry no access permissions.

The `standard` object uses the existing `DrawingStandard` command contract.
The desktop also accepts the previous unwrapped standard JSON. Unknown fields
and unsupported versions are rejected instead of silently dropping rules.
Duplicate fields in the original JSON are rejected at the package, standard
and supported definition levels, rather than silently taking the last value.
The file is limited to 1 MiB by the desktop file picker.

Version 1 contains drawing-standard rules only. It does not package styles,
blocks, hatch definitions, templates, palette items or viewport overrides.

## Version 2: named resources and block library references

Version 2 keeps the same `format`, `id`, `revision` and `standard` fields and
requires a `resources` object. Its supported fields are `lineTypes`,
`hatchPatterns`, `blocks`, `sheetTemplates`, `leaderStyles`,
`textStyleDefinitions` and `dimensionStyleDefinitions`. The first two
hold the corresponding KentOS project definitions. `blocks` holds a block name and its existing library
`publication` metadata (`familyId`, `typeId`, `typeName`, `release`), without
embedding geometry. `sheetTemplates` holds approved template IDs and revision
numbers; it does not embed a template. Association alone installs nothing:
`cad.standards.install` (docs/adr/1002, docs/adr/1100) resolves a block reference against the block libraries
KentOS reads by its publication and installs it by the user's choice, and records an approved template revision
in the drawing (`approvedSheetTemplates`) without changing a sheet; a reference no library holds is reported as
`unavailable`. An empty resources object is
valid. For example:

```json
{
  "format": "kentos.drawing-standard",
  "version": 2,
  "id": "office-architecture",
  "revision": "2026.2",
  "standard": { "name": "Office", "layers": [] },
  "resources": {
    "lineTypes": [{ "name": "OFFICE-DASH", "elements": [{ "length": 2 }, { "length": -1 }] }],
    "hatchPatterns": [{ "name": "OFFICE-HATCH", "lines": [{ "angle": 45, "origin": [0, 0], "offset": [0, 2] }] }],
    "blocks": [{ "name": "OFFICE-DOOR", "publication": { "familyId": "22222222-3333-4444-8555-666666666666", "typeId": "33333333-4444-4555-8666-777777777777", "typeName": "Standard door", "release": 2 }, "units": "mm" }],
    "sheetTemplates": [{ "templateId": "office-a3", "revision": 2 }],
    "leaderStyles": [{ "id": "office-note", "name": "A-NOTE", "height": 2.5, "arrow": "open", "arrowSize": 1.5 }],
    "textStyleDefinitions": [{ "id": "office-text", "name": "A-TEXT", "font": "arimo", "height": 2.5, "widthFactor": 0.9 }],
    "dimensionStyleDefinitions": [{ "id": "office-dim", "name": "A-DIM", "height": 2.5, "dimLineColor": "#ff0000", "options": { "round": 0.001 } }]
  }
}
```

Package loading rejects unknown resource fields, including nested definition
and pattern fields. Definition validity and duplicate names are checked using the same
rules as project line types, hatch patterns and published block cards. Block
names use the drawing's Turkish case folding, must be unique, and the
publication identifiers and release must be valid. Optional `units` records
the authored block unit (`mm`, `cm`, `m`, `in`, `ft` or `unitless`). Omitting it
leaves the unit check disabled for existing packages. Template IDs must be unique
without regard to case, must contain 1–128 bytes, and revisions must be
positive. Named leader styles use the project's validated leader-style shape;
their IDs are transported but a local style with the same name and appearance
can satisfy the standard even if its local ID differs. Complete text and dimension style definitions use the project's
validated style shapes; the short `standard.textStyles` and
`standard.dimensionStyles` rules remain separate. Source IDs are not compared
with local IDs. Version 1 rejects `resources`; version 2 without `resources`
and unsupported later versions are rejected.

Direct `cad.standards.check` input and command normalization enforce the same
resource-field allowlist before deserialization, so Python and MCP calls cannot
silently discard an unrecognized text or dimension style field. Legacy KCAD
style records retain their separate, permissive reading contract.
The project keeps the original JSON text, including the order and content of
both versions. Attaching it through `cad.settings.set` remains revision checked
and does not replace, install, purge or rename a drawing resource.

`cad.standards.resolve` returns the first definition of each resource name in
binding order under `resources`, with its source document and revision in
`provenance.lineTypes`, `provenance.hatchPatterns`, `provenance.blocks`,
`provenance.sheetTemplates`, `provenance.leaderStyles`,
`provenance.textStyleDefinitions` or `provenance.dimensionStyleDefinitions`.
Line and hatch names use the project resource tables' case insensitive
comparison; block names use the block table's Turkish case folding. The
standard's drawing rules still resolve as before. `cad.standards.conflicts` reports differing
fields and both values when two bound packages define the same resource name
with different content; a case difference in the name alone is not a content
conflict. Numeric line-type and hatch-pattern fields use the same absolute
1e-9 tolerance as named annotation styles, recursively within arrays and
objects. `cad.standards.check` uses that tolerance too, so a near-equal
definition cannot pass one query and fail the other. Both queries are read
only and available through the command
registry, Python SDK and MCP contract.
Conflicting template revisions of the same ID are reported as
`sheetTemplateReference` with both sources and values; binding order selects
the first revision for checking.
Leader-style conflicts compare appearance fields, excluding the local style ID
and the case of its name. Numeric height and arrow-size differences use the
standard check's 1e-9 tolerance. The first named definition wins for checking.
Complete text and dimension style conflicts use the same identity rule and
compare every supplied appearance field, including nested dimension options.
Numeric differences within 1e-9 are treated as equal. A missing field is not
silently equated to an explicit value.

`cad.standards.check` may receive the resolved or a single package's
`resources` alongside its `standard`. It reports missing named line/hatch
definitions and different geometric content as separate, non-fixable issues.
For each missing or different named line type, the finding states how many
layers, saved layer-state nodes, direct objects and block-definition parts
name it. For a hatch
pattern, it reports direct hatches and block-definition parts with that name.
`objects` holds only the direct object count; zero is omitted from JSON.
The scan visits each drawing entity once for all package resources. Hatch
entities keep their own pattern geometry: a matching name is an impact hint,
not proof that replacing the project definition updates those hatches.
Published block references are checked against the drawing's block table:
missing block names and mismatched publication metadata are separate,
non-fixable issues. The block check compares the publication family, type,
display name and release, without treating equal metadata as proof of equal
geometry. An unpublished local block does not satisfy a published reference.
For a publication mismatch, the finding reports direct placements in its
`objects` count and states separately how many references occur inside block
definitions. These counts are an impact hint; they do not authorize replacing
the definition or prove that equal publication metadata means equal geometry.
The check builds direct and nested insert counts once for the drawing, then
looks up every required block by its normalized name. Office packages with
many block references do not trigger a new full drawing scan per definition.
If a block reference specifies `units`, a different or unspecified local unit
gets a separate, non-fixable `blockUnit` finding even when publication metadata
matches. This helps identify scale risks without rescaling inserts or replacing
the definition. Conflicts between office packages report `units` separately.
Names are compared case insensitively; source path and description do not
change geometric equality. The command and the desktop standard window
perform this check without installing or replacing a definition. The full
expected and actual geometry stays in the structured command result; the
window uses short row labels and shows the values after selecting a finding.
When `sheetTemplates` is nonempty, the check also compares each sheet's
recorded template origin against the approved ID and revision. A sheet made
from an unlisted template or an older/different revision gets a non-fixable
`sheetTemplateOrigin` finding. Manually created sheets without an origin are
not subject to this check. The finding neither rebuilds a sheet nor proves
that its actual page content equals the template: `origin` is provenance
metadata, not a content digest.
Named leader styles are checked by name and their paper height, arrow type,
arrow size and mask. A missing or different style is a non-fixable finding;
the check never replaces an existing style or rewrites its leaders.
Complete text and dimension styles are likewise checked by name and all
serialized appearance fields. These findings are non-fixable and do not rewrite
existing annotations or style tables. The short rules may also report their
own, narrower name/font/height findings.
For a differing local text, dimension or leader style, the finding reports
direct entity uses and uses inside block definitions by the local style ID.
Text styles also report how many named line-type definitions contain text
elements using that style name; multiple elements in one line type count once.
The `objects` field contains direct entity uses only. These counts explain
potential impact and do not authorize replacing a style or changing its users.

Version 2 currently transports and checks definitions for explicit
mapping work. It does not apply them to the drawing. In particular, named
patterns in layers and objects, SHX shape files referenced by a line type,
block geometry, table style content, sheet-template
contents, palette recipes and viewport overrides are not yet packaged or
reconciled. An office standard
must not be marked fully applied on the basis of these resource queries.

The block reference is a KentOS extension: AutoCAD's [CHECKSTANDARDS](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-Core/files/GUID-F47E5F56-88BE-4FC2-A0D6-D35E8DFAC7A6.htm)
describes layer, dimension, line type and text style checks, while its
[block library guidance](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Core/files/GUID-9C9FF3BC-7275-44E8-93D4-00A2F50BC96D.htm)
uses drawing files as reusable sources. Automatic block replacement would
change every insertion, as the [block editor documentation](https://help.autodesk.com/cloudhelp/2026/ENU/AutoCAD-MAC-Core/files/GUID-BBD754F5-D51F-4692-8194-49E5B130CE68.htm)
explains; the standards check therefore leaves geometry untouched.
The desktop file selection is temporary until the user explicitly chooses
“Projeye bağla”. Loading or associating a document does not apply its rules:
checking and applying are separate actions, and applying is revision-checked.

`cad.settings.set` accepts `standardDocs`, an ordered list of objects with a
`text` field containing the complete versioned JSON document. It validates
all documents before replacing the list; an empty list removes the bindings.
For recovery, it also accepts a shorter ordered subset of the existing exact
document texts. This permits removing unsupported embedded documents one at a
time without admitting new, modified or reordered unsupported documents.
Use `expectedRevision` to protect the edit from intervening changes. The same
field is available through the generated Python SDK and MCP command contract.
Association does not apply a standard. The standard-check window can inspect a
single binding or explicitly resolve the ordered bindings together.
`cad.standards.resolve` returns a prospective `DrawingStandard` and document
revision: global rules come from the first binding, and each named layer, text
style and dimension style comes from its first occurrence. Remaining named
definitions are appended in order. The query itself does not change the
drawing. Pass its standard and revision explicitly to check or apply; a stale
revision cannot be applied. The window refuses a combined preview when its
source documents have changed, and requires it to be prepared again.
The resolved result also identifies the source document and office revision
for the global rules and each effective layer, text style and dimension style.
These origins describe the resolved standard definitions, not an object's
effective appearance or a viewport override.
`cad.standards.conflicts` reports differences in global rules and identically named layer,
text-style and dimension-style definitions in the ordered documents. It
identifies the first and later document and the differing fields. It does not
merge the documents or decide which definition to
apply. An unreadable document makes the query fail instead of giving a partial
report. The desktop uses the same query; it is also available through Python
and the local command registry.
Each difference includes the first and later values of the differing fields;
an absent optional value is represented as null. The desktop exposes these
values when the user selects a difference, and clears the selection whenever
the comparison changes.

This format is a starting point for CAD-ADV-12, not the complete office
package. Remaining resource packaging and the complete association workflow
require explicit format evolution, conflict mappings, migration tests, and
the same command access for desktop, Python and MCP.

For the association workflow Autodesk documents both
[drawing-to-standard associations](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-3430632D-3294-44EA-ACDE-429CBE1F1BDD.htm)
and [ordered precedence for conflicting standards](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-22AC35EC-47D1-4FA4-96AF-671663396F55.htm).
The current resolution covers the named definitions and global rules described
above. Installation by the user's mapping is `cad.standards.install` (docs/adr/1002, 1100); automatic
mapping, table styles and published tool palettes remain open.
