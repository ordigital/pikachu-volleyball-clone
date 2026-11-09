# Local patch to miniquad 0.4.11

Source: crates.io miniquad 0.4.11, https://github.com/not-fl3/miniquad.
Original MIT/Apache-2.0 licenses are preserved. Only build/runtime sources and
package documentation are retained; examples and unrelated authoring files are omitted.

`src/graphics/gl.rs::delete_texture` calls the existing
`self.cache.clear_texture_bindings()` before deleting a texture. OpenGL unbinds
a deleted texture, but the old cache still recorded its name. On Wine/Wayland,
font-atlas growth then reused that GL name, skipped the required binding, and
all subsequent glyph masks were empty. This hid menu choices and score digits.

The local Cargo patch makes this fix reproducible on all target platforms.
Regression: `cargo run --example text_render_check` (or build the Windows example
and run it using Wine). It uses the actual game text renderer and checks GPU
pixels after repeated atlas/cache growth. No runtime fallback or Wine detection.

Two upstream warnings exposed by using a local dependency are also cleaned up:
`set_or_replace_display` is compiled only on Android (its only caller), and
unused rectangle assignments are removed from Windows `set_window_position`.
These do not change window placement or Android display initialization.

The X11 clipboard message uses a Mutex instead of a mutable static reference,
removing the native compiler safety warning without changing clipboard contents.

IME display locks are compiled only where those requests are implemented;
Android IME-position/enabled no-ops no longer create unused display guards.
