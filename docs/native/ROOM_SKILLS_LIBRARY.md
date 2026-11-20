# Native Skills library

The Studio Packs window includes a local Skills library for importing one
`SKILL.md` file, reviewing immutable versions, enabling one exact resource,
and capturing its text into AI Context. This extends the existing declarative
Pack store and activation review; Skills do not gain a separate execution path.

## Import and provenance

Import runs on the background worker and reads only the selected file. The
local source-path policy and a 32 KiB byte limit apply before parsing. Input
must be UTF-8, begin with a YAML-style `---` frontmatter delimiter, and have
nonempty root-level `name` and `description` values. The bounded metadata
reader accepts plain quoted values and indented literal/folded block scalars;
it is not a general YAML parser. Nested keys cannot supply the displayed
identity. The original text, including its metadata and body, is retained
exactly as the Skill resource. No sibling file, link, script, or asset is
opened, copied, or run.

The import review shows the observed local path and text SHA-256. Pack ID,
version, publisher, license, and source are user-editable pack metadata.
Publisher, license, and source remain claims; a digest proves byte identity,
not publisher identity, license rights, or safety. The user submits the pack
to the existing exact disabled-install review. Existing IDs accept a new
immutable version; an existing version cannot be overwritten.

## Search, enable, history, and AI context

Installed Skills can be searched and inspected as full text with their pack
identity, claims, version, and digest. Enabling/disabling uses the existing
frozen activation review. A version transition selects only the chosen Skill
resource and does not silently carry over other resource selections. Older
versions remain in the catalog; the existing history/rollback page creates a
new reviewed activation when restoring them.

Only an active resource present in the prepared runtime can be captured.
Capture creates a local historical AI Context snapshot in the open trusted
project, initially excluded. Include and provider Send remain separate
choices. Disabling or updating a Skill does not delete earlier snapshots or
retract context already sent to a provider; users can remove snapshots in AI
Context. These instructions are context text, not tool grants, app
permissions, provider capabilities, or sandbox guarantees.

## Acceptance boundaries

Core parser and storage tests plus Windows CI/package proof establish source,
policy, and build behavior. They do not establish a visible desktop import and
activation trial, a real-provider request review, or a physical two-client
Room trial. Those manual gates remain separately open and are not implied by
the Skills library.
