# Changelog

All crates in the workspace share one version.

## 1.5.10

### Animation nodes in the DOM (uzor)

- `El::anim(AnimNode)` is a box whose content a named `AnimScript` paints. The
  box is laid out by flex. Its `id` makes it a hit, and document order is its
  z-order. Its frame is clipped to the content box unless `overflow_visible`,
  and it gets the element's opacity.
- `dom::paint_with(ctx, &dom, &mut registry, &AnimFrame)` paints boxes and
  animations together:
  - a node outside the visible rect does not run;
  - a paused frame (hidden page, reduced motion) paints the pose for `t` without
    asking for another frame;
  - the script gets its size, its seed, up to eight args (`ANIM_ARGS`) and the
    pointer in local coordinates;
  - the pointer comes through `pointer_over(id)`, so a drawer above the node
    hides it, unless the node is `track_pointer`.
- `AnimReport` says when the next frame is due and whether any node follows the
  pointer.
- `AnimRegistry` maps names to a primary script and a fallback painter (Rust,
  `BakedAnim` or `FnAnim`):
  - when an engine is missing, the fallback paints for good;
  - when a script fails, the fallback paints and the script is retried a second
    later.

### JS animations (uzor-anim-js, new crate)

- `JsAnimHost` runs `globalThis.uzorAnims[name] = (ctx, t, p) => {}` scripts as
  `AnimScript`s:
  - on web, through the page's JS engine (scripts load with `<script src>`; no
    eval, so it works under a CSP without `unsafe-eval`);
  - on native, through QuickJS behind the `quickjs` feature (off by default).
- `JsAnimHost::install(&mut registry, names)` makes the scripts the registry's
  primary; `has(name)` says whether a script is registered.
- The scripts belong to the app; the crate ships only `hexstar.js` as an
  example.

### Op stream (uzor)

- `AnimScript`, `AnimParams`, `AnimOpWriter` and `replay_anim_ops` make a flat
  f64 stream of Canvas2D-style ops that any `RenderContext` replays.
  `BakedAnim` plays recorded frames.

### Motion math (uzor)

- Envelope primitives: `smoothstep`, `back_out`, `bell`, `arc`, `hold`, `ring`,
  `squash_stretch`, `pivot_shift` and `ellipse_lift`.
- `Rot3` turns a shape around an axis in fake 3D (projection, facing).
  `TrickCycle`, `slot_hash`, `back_in_out`, `breathe` and `turn_delta` sequence
  tricks.

### Widgets and effects (uzor)

- `ring_mark` is a polygon ring tumbling around a counter-spinning star, with
  idle, loading and busy tricks.
- `effects::text::MatrixText` is a stateless line decode, with an idle flicker
  and a scramble under the pointer.
- Composer: `voice_off` draws an unavailable voice-chat button as disabled.
- Critter: `draw_critter` and a session pill without motion paint the rest pose
  directly, without going through `critter_pose`. The critter API is unchanged.

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
