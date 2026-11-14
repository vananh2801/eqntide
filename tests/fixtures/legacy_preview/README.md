Minimal object/preview pairs extracted from `bug_upgrade_before.docx` supplied
for the November 2026 Upgrade regression. Objects 0, 1 and 93 encode `(Ou,Ov)`,
`pi/3` and a two-row recurrence respectively. The OLE streams are valid; their
EMF comments contain bare MTEF v5 records beginning with template tag 3, mistaken
by an older writer for a complete v3 stream. Blind v3 decoding produces unrelated
Unicode characters. Repair is allowed only when wrapping the preview records
exactly reproduces the paired OLE's complete MTEF.
