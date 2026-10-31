# Changes 1.5.14 – 1.5.20

Written from the git history (tags v1.5.14 … v1.5.20, 2ffed38..b70d4e6). Every crate in the workspace shares one version; crates without changes were re-released with the family.

| version | commit | change |
|---|---|---|
| 1.5.14 | 4af8a74 | urx: thin strokes use a 1px coverage band |
| 1.5.15 | ae5f6af | render: default family is URX |
| 1.5.16 | a42aafa | host: set the taskbar icon before the window is shown |
| 1.5.17 | 9224fc7 | field: paint selection and blink the caret |
| 1.5.18 | f3992a3 | field: extend the selection while the drag is held |
| 1.5.19 | 862afef | field: drop the caret on a press outside the field |
| 1.5.20 | e3221f0 | render: antialias UrxWgpu paths at the pipeline's 4x default |

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

