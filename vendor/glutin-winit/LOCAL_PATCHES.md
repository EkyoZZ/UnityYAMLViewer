# Local patch for glutin-winit 0.5.0

Source: the registry package glutin-winit 0.5.0, with its MIT license retained.
Cargo resolves this copy through the root `[patch.crates-io]` table. No dependency
versions are upgraded.

`finalize_window` preserves an explicitly requested transparent window on Windows
instead of clearing that attribute when the GL config reports no transparency.
This lets winit perform its normal DWM initialization when it creates the window.
Other platforms keep the original config check, and opaque windows stay opaque.

This is the small integration-level equivalent of the Windows child-viewport fix
in [eframe PR 8423](https://github.com/emilk/egui/pull/8423). The installed eframe
0.31.1 still logs its unsupported-transparency diagnostic before calling this
function; that diagnostic does not disable the window after this patch.

Only `src/lib.rs` differs from the original source. Preserve this change until the
project adopts an upstream window integration that handles Windows transparency.
Manual verification is still needed for the recording toolbar, standard popup,
window hide/show, display/DPI changes and capture exclusion.
