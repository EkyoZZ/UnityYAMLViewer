# Local changes

Based on egui_commonmark 0.20.0, retaining upstream licenses.

- Route InlineHtml events through the configured render_html_fn callback, matching HTML block handling. Without a callback, preserve upstream literal text behavior. Fixes application link/image markers shown as text inside paragraphs and list items.

- Re-export the upstream code_block widget for the application code-color layouter, retaining its original copy icon, placement and selection-aware copying.

- Adapt the upstream code_block widget locally: disable TextEdit frame painting to avoid two outlines, retain one background frame and all copy/selection interactions, set text margins to 4 px on left/top/bottom and 28 px on the right to reserve the copy icon.

- egui 0.31 TextEditOutput.response.rect excludes text margins. Expand it by the explicit margin before painting the frame, so the padding is inside the border rather than outside it.
