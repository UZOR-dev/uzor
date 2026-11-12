# uzor-plot

Plot math for uzor apps: the numbers behind axes and charts. No rendering,
no DOM, no dependencies (optional `serde`). Widgets and renderers on top
decide how things look.

| Module        | What it does |
|---------------|--------------|
| `ticks`       | 1-2-5 nice steps, pixel-based tick density, bounded ticks, label culling, label precision |
| `value_scale` | linear / log / percent / indexed value axis, auto-fit, zoom, pan, ticks and labels |
| `viewport`    | horizontal slot viewport (slot ↔ x, visible window, zoom limits, fits) and kinetic panning |
| `time_scale`  | calendar-aware time ticks over a slot viewport, calendar math, time labels |
| `crosshair`   | crosshair state, magnet snapping, tooltip placement |

```rust
use uzor_plot::value_scale::ValueScale;
use uzor_plot::viewport::SlotViewport;

let scale = ValueScale::new(0.0, 100.0);
let ticks = scale.ticks(300.0); // grid values for a 300 px axis
let y = scale.value_to_coord(75.0, 300.0);

let mut vp = SlotViewport::new(800.0).with_spacing(10.0).with_count(500);
vp.scroll_to_end();
let range = vp.visible_range(); // data indices to draw
```

Finite, valid input gives the same numbers as the chart code the crate was
extracted from. Degenerate input (zero lengths or spacings, `NaN`, infinite
ranges, swapped limits) never panics: functions return a neutral value, no
ticks, `None`, or leave the model unchanged, as each one documents.

Every module is plain functions and small `Copy` models, so whatever an app
does not call is dropped by the linker. Runnable examples live in
`examples/`. The crate builds for `wasm32-unknown-unknown`.
