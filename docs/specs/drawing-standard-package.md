# Drawing standard document, version 1

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
The file is limited to 1 MiB by the desktop file picker.

Version 1 contains drawing-standard rules only. It does not package styles,
blocks, hatch definitions, templates, palette items or viewport overrides.
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
package. Future resource packaging and the complete association workflow
require explicit format and project-schema evolution, resource conflict handling,
migration tests, and the same command access for desktop, Python and MCP.

For the association workflow Autodesk documents both
[drawing-to-standard associations](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-3430632D-3294-44EA-ACDE-429CBE1F1BDD.htm)
and [ordered precedence for conflicting standards](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-22AC35EC-47D1-4FA4-96AF-671663396F55.htm).
The current resolution covers the named definitions and global rules described
above; block, hatch, line-type definition and template conflicts remain open.
