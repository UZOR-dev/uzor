# Changelog

All crates in the workspace share one version.

## 1.5.30

### Mobile shell kit (`uzor::ui::mobile`)

- New DOM kit for phone layouts with one stylesheet, `MOBILE_CSS` (monochrome; only the small
  status dots are coloured; only overlays use glass). `sheet_with(app_css)` stacks it on the
  kit and touch sheets.
- Builders: `tab_bar` (badges and dots), `large_title`, `top_bar` (back, title, subtitle,
  actions), `drawer` (edge-swipe, with progress), `side_rail`, `fab` and `action_modal`,
  `bottom_sheet` (peek, half and full detents), `swipe_row` (left and right actions),
  `toast`, `segmented`, `composer`, `hero`, `quick_actions` and `kv_group` for agent cards.
- Motion state with no clock of its own: `SheetState` (drag, fling and snap to a detent),
  `DrawerState`, `SwipeState`, and `approach` for frame-rate independent easing.
- `composer_bottom` and `reveal_above` place a composer over the soft keyboard and lift a
  focused field above it.

### CSS units (`uzor`)

- `vh`, `vw`, `vmin`, `vmax` and `rem` lengths resolve against the media environment.

### DOM paint order (`uzor`)

- A positioned box with a `z-index` is a stacking context: its positioned descendants paint
  inside it, ordered by their own `z-index`, instead of among the page's layers. Before this,
  a `position: relative` child of an overlay was painted under the overlay (the drawer's
  avatar was hidden). Mobile avatars also centre their letters.

### Mobile drivers (`uzor-framework::widgets::mobile`)

- `DragTracker` turns touch pans and mouse drags into one `Drag` stream for a target rect or a
  screen edge, and swallows the click after a mouse drag. `drive_sheet`, `drive_drawer` and
  `drive_swipe` feed it into the kit states; `composer_rect` and `reveal_field` follow the
  keyboard inset.
- A touch drag that starts on a text field scrolls the view instead of selecting text.
- Web host: keyboard height comes from `visualViewport`, and safe-area insets from `env()`;
  both reach the app as `Insets` on resize and viewport scroll.

### Touch and mobile platform (`uzor`, `uzor-framework`)

- Suspend and resume survive on Android and iOS.
- Touch gesture recogniser with release velocity and fling; one-finger scroll, fling,
  long-press and gestures on touch.
- CSS `@media` and `env()` evaluated against a media environment; safe-area and soft-keyboard
  insets and coarse pointer in the view; touch-sized kit targets under `pointer: coarse`.
- `native-lean` feature with only the backends a phone needs.

### New crates

- `uzor-window-mobile`: the thin mobile layer over `uzor-framework`.
- `uzor-secure`: platform secure-store trait with a fail-closed unlock gate.

## 1.5.29

### Android native host (`uzor-framework`)

- The native host builds for Android. `arboard` and `tray-icon` have no Android backend, and
  the X11 flush and pointer query only run on desktop unix; Android takes the same no-op
  clipboard and tray path as iOS.
- Features `android-native-activity` and `android-game-activity` pick winit's activity glue.
  The app's `android_main` passes its `AndroidApp` in `NativeOptions::android_app`; without
  it `run_native` returns an error, because winit cannot build an event loop on Android
  otherwise.

### QuickJS bindings on Android (`uzor-anim-js`)

- `quickjs-jit-sys` ships no `aarch64-linux-android` bindings, so the native build failed
  there the same way it did on iOS. Android now enables the `bindgen` feature too (with the
  NDK sysroot in `BINDGEN_EXTRA_CLANG_ARGS_<target>`).

### iOS surface size (`uzor-framework`)

- On iOS winit's `inner_size` is the safe area, while the view, its Metal layer and every
  `Resized` event cover the whole window. The first swapchain and the `Created` echo used
  the safe area, so the opening frames were stretched and touches were off by the notch
  until a resize came in. `surface_px` sizes the surface and the echo from the whole view
  (`outer_size` on iOS).

### Soft shadows (`uzor-render-urx`)

- Blurred shadows of rects and rounded rects are soft: a stack of nested rounded rects
  builds the Canvas2D `shadowBlur` Gaussian edge (sigma = blur / 2).

### Glyph run face (`uzor-urx-glyph`, `uzor-urx-wgpu`)

- `font_css(id)` returns a registered face as CSS font words (`"400 'JetBrains Mono'"`).
  `uzor-urx-wgpu` draws a `GlyphRun` through that face, so a monospace run stays monospace.

## 1.5.28

### Multiline text area (`uzor_framework::widgets::kit::area`)

- `kit::area(id, rect)`: a multiline text field (a chat composer) on the same text store as
  `kit::field`: hard `\n` breaks, soft wrap after spaces, long words broken by character
  (`kit::area_lines` gives the laid lines), the selection painted per line, the caret, the IME
  preedit underline and the IME area at the caret. Clicks, double / triple clicks and drags
  hit by line and x; ArrowUp / ArrowDown move by laid line at the same x, Home / End (with
  Shift) to the laid line's ends. `.text_rect`, `.clip_to`, `.scroll`, `.font`,
  `.line_height`, `.colors`, `.caret_color`, `.set`, `.parent` as on `kit::field`;
  `AreaResponse` reports the line count, the content height and the caret line's top.
- Text store: `TextFieldStore::update_field_lines` (per-char x and line top),
  `has_lines`, `begin_drag_at_xy`, `on_drag_move_xy`, `select_drag_xy`,
  `select_at_click_count_xy`, `TextFieldState::boundary_at(x, y)`; the x-only methods keep
  working (single-line fields).

### Virtual list (`uzor::ui::domkit::VirtualList`)

- `set_height(i, h)` (a height known without laying the row out, e.g. from a cache),
  `height(i)` (a measured height, if any) and `tops()` (the row tops and the total height).

### Headless frames (`uzor_framework::handle::VisualView`)

- `VisualView::with_focus(id)`: a view with `id` focused, for headless renders and app tests
  of a frame (a focused field paints its caret and selection).

## 1.5.27

### DOM: scroll containers clip everything they scroll (`uzor::ui::dom`)

- `paint_box` paints inside the element's overflow clip (`Placed::clip`) itself, so a host
  that paints a laid-out tree box by box (doing its own per-box work) no longer lets text,
  placeholders, inputs or frames of an `overflow: hidden | auto | scroll` descendant leak
  out of it (e.g. under a dialog footer). `paint`, `paint_slots` and `paint_with` are
  unchanged.
- `El::scroll_top(y)`: CSS `scrollTop` for an overflow container: its descendants (in flow
  and positioned, not `fixed`) are placed `y` higher, the container stays, its padding box
  clips them; hits are cut by the same clip.
- `Placed::scroll_height`: an overflow container's content height (CSS `scrollHeight`).
- `with_clip(ctx, placed, f)` is public: paint host content (an embed, an animation) inside
  a box's overflow clip.

### Text fields clip and scroll (`uzor_framework::widgets::kit::field`)

- A kit field clips its text, placeholder, selection and caret to its rect by default
  (`.clip(false)` lets it overflow) and scrolls sideways with the caret while focused, as an
  `<input>` does: last frame's scroll is kept while the caret is in view, moved just enough
  when it is not, never past the text's end; back to the start when not focused. A long
  pasted string no longer runs past the field.
- `FieldBuilder::clip_to(rect)`: the scroll viewport the field sits in (a dialog body). Paint
  and the hit rect are cut to it; a field scrolled out of it paints nothing (no placeholder
  under a dialog's footer) and cannot be clicked there.

### iOS GPU surface (`uzor-render-hub`, `uzor-framework`)

- iOS builds the wgpu instance with `Backends::METAL` (Windows keeps DX12 through
  DirectComposition, other platforms the default backends / `WGPU_BACKEND`).
- `init_gpu_surface` requests the device without binding it to the surface (on iOS a
  surface-bound request found no device, the window spawn was cancelled and the frame stayed
  black), then configures the surface with it; only when that adapter cannot present to the
  surface is a surface-compatible device requested.
- A failed window spawn is also printed to stderr, so it shows without a logger.

## 1.5.26

### Dashboard widgets (`uzor::ui::dash`)

- Generic DOM widgets fed by plain data, monochrome (`--d-*` variables, one green live dot):
  space switcher and its menu, summary tiles with mini histograms, live agents by space and
  node, task rows with trees (`TaskNode`, `build_tree` from parent links, `flatten_tree`,
  `toggle_fold`, progress over all descendants), limit rings, task card, mini board, 24h
  activity, titled `section`, mini panels (`MiniPanel` in `PanelMode::Widget` / `Collapsed` /
  `Rail`, `PanelMode::after` for the collapse / expand hits) and monochrome provider glyphs
  (`provider_glyph`, `glyph`: claude, codex, grok, kimi, opencode, gemini, cursor, qwen,
  deepseek, human; an unknown provider shows its letter).
- Overlay glass: `glass(el)` wraps popovers, menus, select popovers and dialogs in
  `--k-glass-bg`, `--k-glass-line`, `--k-glass-blur` (`glass_vars` retunes them); embedded
  widgets stay opaque.
- CSS `backdrop-filter: blur(N)` (`Style::backdrop_blur`), painted through the new
  `UiEffectHelpers::blur_backdrop` (tiny-skia: box blur under rounded corners, through the
  current scale; canvas2d: `filter: blur()` over the canvas region, root canvas only; other
  backends: no-op, the translucent fill alone).
- `Control::Ring { frac }` (`role="meter"`): a track and a clockwise arc, `--ring-color`,
  `--ring-track`, `--ring-width`.
- Examples: `dom_widgets` page (EN/RU through `uzor::ui::i18n`), bins `dom-widgets` (native)
  and `dom-widgets-frames` (PNG shots).

## 1.5.25

### PDF text and images keep the full transform

- `PdfRenderContext` text `Tm` and image `cm` carry the current transform: rotation, shear, and independent horizontal and vertical scale. Font size stays the em size. Stroke width, dash, and radial-gradient radius still use the uniform scale factor. Paragraph `PdfTextRun`s stay axis-aligned.
- `PdfPageSpec` takes optional `trim` and `bleed` boxes in top-left, y-down points. Unset boxes are omitted from the page dict.

## 1.5.24

### Dev control, stage 2 (uzor-framework feature `dev-control`)

- `--headless` (implies `--dev`; or `<APP>_DEV=headless`): the app runs on the headless host,
  ticked in real time, with no window and no display server; screenshots are offscreen and the
  RPC is the same. `DevControlConfig::headless`; a build without the feature refuses it.
  `host::run_headless` is the loop (feature `automation`); its frames paint into a 1x1 tiny-skia
  context, so text measures with the real fonts (the same layout as the offscreen screenshots).
- `wait-for` (`target` selector with `state` present/gone/focused/enabled and `value`, or `cond`,
  a JSON subset of the state report) and `settle` (`idle_ms`: the tree, widget rects, focus,
  hover and state report unchanged), both checked once per painted frame; `timeout` on failure.
- `record` / `replay`: input is recorded as JSONL of RPC calls (`{"t_ms","method","params"}`),
  selectors preferred over coordinates, masked text written as `"<masked>"`, typing lines hold
  the field's final text and replay with `type`'s new `clear` (select all, delete); replay needs
  `params.secrets` for such lines, waits for each selector, honours `speed`. Progress goes to
  `GET /events`.
- Browser bridge: `host::run_web_with` + `WebOptions::dev_control(WebDevControl)` installs
  `window.__uzorDev` (`call(req) -> Promise`, `events(since)`, optional aliases) only when the page
  URL has `?dev=1`; served once per RAF frame like the native host. No network listener.
- The dev-control core (`Dispatcher`, `EventLog`) now builds for wasm too; the automation
  client is async (`AutomationClient::call_async`, `ReplyFuture`); hosts count painted frames
  (`AutoCmd::Frames` / `Frame`) and tap input for recording.
- `uzor-dev`: the four methods with bare arguments, `@file` values, `record off --out FILE`.

## 1.5.23

### Dev control (uzor-framework feature `dev-control`, new crate uzor-devtools)

- One switch: `uzor-framework = { version = "=1.5.23", features = ["dev-control"] }`. At run
  time an app sets `NativeOptions::dev_control` (`DevControlConfig::from_args_env(app_dir, app,
  "MYAPP_DEV")` is `Some` for `--dev` or `MYAPP_DEV=1`); the native host then serves a loopback
  JSON RPC for the whole run. A build without the feature compiles none of it, and a config set
  there is ignored with one line on stderr, so apps need no `cfg`.
- Methods: `ping`, `windows`, `dump-tree`, `click`, `hover`, `focus`, `type`, `key`, `scroll`,
  `screenshot`, `state`; `GET /events` streams the calls. Stable English error codes.
  `focus` and `type` with a selector wait until the clicked widget holds the focus.
- Access: `127.0.0.1` on a random port; exact loopback `Host` (421), no `Origin` (403), a
  32-byte bearer token compared in constant time (401). The token is only in the discovery file
  `$XDG_RUNTIME_DIR/<app>/dev-<pid>.json` (else `~/.config/<app>/`), directory 0700, file 0600,
  removed when the app exits.
- `uzor-devtools`: the transport (server, token, discovery file, event log, the `Rpc` trait) and
  the `uzor-dev` CLI. It does not depend on the framework; the RPC core is
  `uzor_framework::dev_control::Dispatcher`, the transport is re-exported as
  `dev_control::devtools`.

### Automation port (uzor-framework feature `automation`)

- `automation::channel()` gives a tool thread an `AutomationClient` (windows, tree, injected
  pointer / key / wheel / IME input, screenshots, app state); the native host answers on its loop
  thread after the frame paints, `HeadlessHost::serve_automation` does the same for tests.
- `App::automation_state` lets an app report a whitelisted state (never secrets).
- Screenshots: the presented window read back (GPU, or the software surface through
  `WindowRenderState::software_readback`), or an offscreen tiny-skia render
  (`ShotSource::{Auto, Window, Offscreen}`).
- `dev-control` turns this feature on; it is internal plumbing for most apps.

### Accessibility (uzor)

- Nodes carry `value` (a text field's text, bullets when masked), `masked`, `disabled` and the
  widget `kind`. `set_a11y_masked` / `set_a11y_disabled` on the coordinator; a kit field drawn
  masked marks itself secret.
- `a11y::query`: CSS-like selectors (`#id`, role, `[attr=|~=|^=value]` on id, name, value, kind,
  role, focused, disabled, masked, `:nth(n)`) resolving to exactly one node, with not-found and
  ambiguous errors.

## 1.5.22

### Examples (uzor-examples, uzor-framework/examples)

- Every example now builds its UI on the DOM engine and `ui::domkit`: framework `demo`
  (native and web), `image_viewer`, `monitor`, `native_two_windows`, `dashboard`,
  `kit_gallery`, plus a live `dom-gallery` page.
- `uzor-launcher`: a DOM index of all examples (filter by Native / Web / Offscreen,
  search, dark and light theme, details panel, Launch). A new example is one line in
  `EXAMPLES` (`catalog.rs`). `PageApp<P: Page>` wraps single-page examples.

### DOM engine and widget catalog (uzor)

- domkit colours are `--k-*` custom properties; `LIGHT_VARS` gives a light theme.
- Inline custom properties take part in the element's own cascade.
- Overlays know the window viewport; `kit::dom` supports `:focus` and `:focus-visible`;
  `HeadlessHost::paint_window`.
- Without a GPU the window falls back to software rendering instead of failing to open.
- A change made by an intent repaints in the same tick under on-demand redraw.

### Notes

- A kit widget registered over a DOM tree sits above DOM overlays; don't register it while
  an overlay is open.
- Apps need the `flow-flex` / `flow-grid` features; `TickRate::Dirty` is recommended for
  static apps.

## 1.5.21

### DOM engine (uzor)

- Selectors: descendant (`.a .b`) and child (`.a > .b`) combinators
  (`Sheet::resolve_in`, `Sheet::has_complex`).
- Layout: `%` widths and heights, `aspect-ratio`, `position: absolute` and
  `position: fixed` (a fixed layer placed against the viewport given to
  `dom::lay_out_in`), `border-style` (solid / dashed / dotted / none), overflow clipping.
- Built-in controls on any element (`El::control`): `Control::Switch`, `Checkbox`,
  `Spinner` and `Input` (caret, selection, placeholder, IME area). They are styled by the
  sheet (`:checked`, `:focus`, custom properties such as `--knob`).
- `El::name` is the accessible name (`Placed` exposes it with the role and the checked
  state); `El::slot` reserves a box that a kit widget paints into (`dom::paint_slots`).
- Inline `El::css` custom properties (`--name: value`) cascade to the subtree, and inline
  values resolve `var()`.
- Markdown inline runs support italic (`*x*`, `_x_`) next to bold, code and links.

### Widget catalog (uzor `ui::domkit`)

- `KIT_CSS`, `sheet()`, `sheet_with(app_css)`: the default dark theme, themed through
  `:root` variables (`--k-accent`, `--k-bg`, `--k-panel`, …).
- Controls: `button`, `switch` / `switch_at`, `checkbox`, `spinner`, `input`.
- Select: `select_trigger`, `select_popover`, `popover_at`, `picked_opt`,
  `SelectOpt::new(..).sub(..).disabled(..)`, and `select_list` with `ListView` (a
  windowed, searchable list with highlight, tick, "No matches" and a scroll thumb). The
  popover is `position: fixed`, so it does not scroll with the content under it.
- Chat blocks: `bubble`, `action_row` (hover actions), `card` (collapsible), `chip`,
  `banner`, `side_panel`, and `inline_md`.
- `VirtualList` / `Window`: only the rows in view are built.
- `icons`: `CLOSE`, `RIGHT`, `DOWN`, `REPLY`, `COPY`, `SMILE`, `MORE`, `CHECK`, `SEARCH` (SVG bodies for `icon()`).

### Framework (uzor-framework)

- `kit::dom(rect, sheet, root)`: a DOM tree inside a kit panel. It gives hit targets,
  a11y nodes, text-store-backed inputs and the IME area, and kit widgets can go in its slots.
  `kit` re-exports `domkit`.

### Examples (uzor-examples)

- `dom_gallery`: every catalog widget on one page. The `dom_gallery_frames` bin renders it
  to PNG frames.

### Size

- Float clamps with runtime bounds are written as `max(..).min(..)`. `f32::clamp` and
  `f64::clamp` kept their `min > max` / NaN assert, and with it the float `Debug`
  formatter, in every wasm build (about 3.5K raw).

### Compatibility notes

- New public fields on `ui::style::Style`, `ui::dom::El` and `ui::dom::Placed`: struct
  literals without `..Default::default()` stop compiling.
- The rewritten clamps (dock, scroll, modal, popup, state, frame view, canvas2d) no
  longer panic when a bound is NaN or `min > max`; they return the lower bound.
- Flex items: when CSS `min-*` exceeds `max-*`, the minimum wins, as in CSS. Before,
  the maximum won.
- `position: fixed` is relative to the viewport passed to `lay_out_in`, not to the
  layout origin. Pass the window rect (origin 0,0) when placing popovers in window
  coordinates.
- Custom properties set inline now reach descendants. A child that used to fall back
  to the `:root` value now sees the inline one.

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
