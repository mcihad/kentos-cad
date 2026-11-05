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
blocks, hatch definitions, templates, palette items or viewport overrides;
nor does choosing a document associate it with a project. The selection is
temporary and the original file is not modified. In particular, loading a
document does not grant permission to change a drawing: checking and applying
are separate actions, and applying is revision-checked.

This format is a starting point for CAD-ADV-12, not the complete office
package. Future resource packaging and project association require explicit
format and project-schema evolution, conflict handling, migration tests, and
the same command access for desktop, Python and MCP.
