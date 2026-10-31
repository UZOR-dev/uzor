# Changelog

All crates in the workspace share one version.

## 1.5.20

### Render (uzor-render-hub)

- Native URX (`UrxWgpu`) antialiases paths with the pipeline's 4x MSAA unless AA is
  explicitly off (`msaa_samples = 1`). The autodetect default `0` used to map to one
  sample, so every autodetected `UrxWgpu` path drew hard-edged.

## 1.5.19

### Text fields (uzor, uzor-framework)

- A press outside every text field drops the caret: the store and the engine focus
  record are cleared on the press itself, so the same frame paints without a caret.
- Blurring a field clears its engagement flag (`TextFieldStore`), so the caret does not
  survive focus loss. A click on an already focused field keeps its engagement.

## 1.5.18

### Text fields (uzor-framework)

- Dragging in a text field extends the selection while the button is held: the press
  arms the drag (`begin_drag_at`), each move writes the store range, and paint reads it
  before `end_frame`. Double / triple click still selects the word / line.

## 1.5.17

### Kit fields (uzor-framework)

- `kit::input` and `kit::field` blink the caret, and only while the field is engaged
  (`TextFieldStore::cursor_visible`); borderless `field`s paint the selection fill.
- `ContentCx::time_ms()` gives widget helpers the host clock in milliseconds. It is a
  required trait method: an outside `ContentCx` implementation has to add it.

## 1.5.16

### Native host (uzor-framework)

- A visible window is created hidden and shown on the tick after `Opened`, once the
  app's icon command ran, so the taskbar button reads the app icon. On Windows the icon
  is also set as `ICON_BIG` (taskbar, Alt-Tab).

## 1.5.15

### Render family (uzor, uzor-render-hub, uzor-framework)

- The unset default `RenderFamily` is `Urx`. `UZOR_RENDER_FAMILY=vello`, a builder family
  or an explicit backend still select Vello. The native host's `build_hub` resolves the
  family from the environment (`resolve_render_family_from_process_env`).

## 1.5.14

### URX GPU strokes (uzor-urx-wgpu)

- The quad and line shaders use a 1px coverage band (`smoothstep(-0.5, 0.5, d / fwidth)`,
  the `uzor-urx-cpu` contract); the old ±fwidth ramp kept 2px strokes from reaching
  full alpha.
- Connected `MoveTo`/`LineTo`/`ClosePath` polylines with round caps and round joins go to
  the capsule line shader instead of the tessellator.
- A test parses the line and quad WGSL.

## 1.5.13

- urx: canvas images draw as GPU quads and triangles; a covering fill under one
  axis-aligned ellipse clip draws as that ellipse.
- anim-js: native scripts run on quickjs-jit 0.14.1; replay covers `drawImage`, clip,
  transform and skinned triangles.
- window: choose caption buttons and set the Windows taskbar icon.
- vello-gpu: one blob id for identical image bytes; render-hub waits on wgpu
  submissions so frame copies are reclaimed.
- ios: stub tray and clipboard, generated QuickJS bindings.

## 1.5.12

### Terminal (uzor)

- Block elements (U+2580-259F) and box drawing (U+2500-257F) are drawn as
  cell-filling rectangles snapped to device pixels instead of font glyphs, so
  adjacent cells join without gaps or dashed lines.
- The cell grid, backgrounds, cursor and underline are snapped to device
  pixels; `TermStyle::measured` sizes cells from the font actually resolved.
- The web canvas is opaque, so terminal text renders sharply.

### Widgets

- `ring_mark` colours come from tokens (`RING_MARK_INK`, `RING_MARK_STAR`,
  `RING_MARK_PAPER`); the `no_literal_colors_in_widgets` check passes.

## 1.5.11

### Inline rich text in the DOM (uzor)

- `El::runs([Run, ...])` gives an element mixed-style inline text in place of
  `text`. Each `Run` has a selector (`b`, `i`, `code`, `a.link`, ...) resolved
  against the sheet with the element as parent, plus optional inline `css` and
  an `id`.
- Runs wrap together like one text (`white-space: normal`/`pre-wrap`, forced
  newlines, and break-anywhere with `overflow-wrap: anywhere`/`break-word`).
  They size their box like text does.
- A run with a background and padding paints as a rounded chip (inline code).
  `font-style: italic` is supported.
- Each piece of a run with an `id` is a hit of its own (`Dom::hits`),
  `Dom::run_at(x, y)` finds the run under a point, and `Placed::char_at(x, y)`
  gives a char index for carets and selection. `Placed::inline` holds the laid
  lines, pieces and their bounds.

### Positioned boxes (uzor)

- `position: absolute` with `top`/`right`/`bottom`/`left`/`inset`, sized by
  `width`/`height`, both insets, or shrink-to-fit. It is placed against the
  padding box of the nearest positioned ancestor (the root otherwise) and takes
  no space in the flow.
- `position: relative` shifts a box and its subtree, leaving its slot in place.
- Positioned boxes paint and hit-test above the flow, ordered by `z-index`.
  Negative `z-index` paints under the flow.

### Corners and sides (uzor)

- Per-corner and elliptical `border-radius` (`12px 12px 0 0`,
  `20px / 10px`), plus the `border-*-*-radius` longhands.
- Per-side `border-width` and `border-color`, and the `border-top`/
  `border-right`/`border-bottom`/`border-left` shorthands and longhands. The
  content box follows each side's width.
- Uniform borders and radii paint exactly as before.
- `path_border_box` traces a box's rounded outline (for a shadow or a custom
  fill), and `clip_padding_box` clips to its padding box with the inner radii,
  so children stay inside rounded corners.

### Exact icons (uzor)

- `draw_svg_icon_exact` draws an icon without rounding its origin or
  quantising stroke widths. `shape-rendering: geometricPrecision` on a DOM
  element does the same for its `svg`.

### Press origin (uzor-framework)

- `VisualView::press_origin()` / `pressed_at(id)` give the point where the held
  press went down. `InputEngineView::press_origin(win)` gives the same point
  at the engine level.
- A drag anchored there keeps the grabbed point under the pointer, even when
  the first frame sees the pointer already moved.

### Compatibility

- New public fields: `El::runs`, `Placed::inline`, and on `Style`:
  `position`, `top`, `right`, `bottom`, `left`, `z_index`, `border_radii`,
  `border_widths`, `border_colors`, `font_style`, `overflow_wrap`,
  `shape_rendering`.
- Code that builds `Placed` with a struct literal adds `inline: None` or
  `..Default::default()`. `Placed` now implements `Default`.

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
