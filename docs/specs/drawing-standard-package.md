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
Use `expectedRevision` to protect the edit from intervening changes. The same
field is available through the generated Python SDK and MCP command contract.
Association does not apply a standard. The standard-check window selects one
project binding at a time; it does not yet combine multiple standards.

This format is a starting point for CAD-ADV-12, not the complete office
package. Future resource packaging and the complete association workflow
require explicit format and project-schema evolution, conflict handling,
migration tests, and the same command access for desktop, Python and MCP.

For the remaining association workflow, support more than one source with an
explicit, visible priority and conflict report. Autodesk documents both
[drawing-to-standard associations](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Core/files/GUID-3430632D-3294-44EA-ACDE-429CBE1F1BDD.htm)
and [ordered precedence for conflicting standards](https://help.autodesk.com/cloudhelp/2022/ENU/AutoCAD-Architecture/files/GUID-22AC35EC-47D1-4FA4-96AF-671663396F55.htm).
These are design inputs, not functionality implemented by version 1.
