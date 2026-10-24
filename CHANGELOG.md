# Changelog

All crates in the workspace share one version.

## 1.5.9

### Input: one topmost hit target (uzor, uzor-framework)

- Pointer hit-testing is a single z-ordered pass over the whole layer stack:
  exactly one topmost widget gets hover, press, move, release, wheel and the
  cursor. Hover on widgets hidden under a higher layer is cleared.
- Opaque layers (`push_layer_opaque`, `set_layer_opaque`) and occluder rects
  (`occlude`, `occlude_default`) hide everything below them; modal layers still
  block everything. Drag capture is unchanged.
- Widgets ask the framework instead of reading the raw position:
  `pointer_over(id)`, `scroll_over(id)` and `is_topmost_at(id, x, y)`.
  `raw_pointer()` is the explicit opt-in to the unfiltered position.
- `pointer()` is deprecated: it ignores z-order.

### Clipboard: one contract on web and native (uzor-framework)

- Platform paste arrives as `InputEvent::Paste(text)`. A focused text field takes
  it; otherwise the app gets `UnhandledInput::Paste { win, text }`.
- On the web, the DOM `paste` / `copy` / `cut` events drive the clipboard:
  Ctrl/Cmd+V is no longer swallowed, and copy / cut are answered from the focused
  widget's offer (`ClipboardCmd::Offer`) through `clipboardData.setData`.
  `navigator.clipboard` remains the fallback. Native uses arboard.
- Kit text inputs use the same contract on both platforms.

### Critter motion (uzor)

- `CritterPose` (tilt, yaw, roll, squash/stretch, hop, eye shape, body morph) and
  a trick animator (`CritterTrick`, `WORKING_TRICKS`, `ATTENTION_TRICKS`,
  `trick_pose`, `trick_for`, `critter_pose`): eased tricks with slight overshoot,
  scheduled from a stateless hash of (seed, time slot), shuffled per cycle so a
  trick never repeats back to back. `draw_critter_pose` paints a pose;
  `draw_critter_motion` uses it.
- State is shown by motion only: working runs tricks continuously, attention uses
  a calmer set, idle is still (`critter_next_frame` returns `None`), reduced
  motion gets a gentle tilt only.
- Removed: the `THOUGHT` and `ATTENTION` critter colour tokens (no rings, glows,
  pulses or thought dots).
- Example: `uzor-render-tiny-skia/examples/critter_strip.rs` renders a frame strip.

### DOM layout (uzor)

- `line-clamp` / `-webkit-line-clamp` keeps N wrapped lines and ellipsizes the
  last one; the value is parsed with the float parser that is already linked.

### Size (wasm)

- Overlay ordering and style custom properties sort through `uzor::sort` and
  build maps by insert, so no extra copies of std's stable sort are linked.
