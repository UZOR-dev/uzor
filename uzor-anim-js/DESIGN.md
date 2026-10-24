# uzor-anim-js — running JS-authored animations in uzor

Contract: a frame is `f(ctx, t, p)` written against a canvas-2d subset. The
prelude records calls into a flat `f64` op stream; uzor replays it with
`uzor::render::replay_anim_ops` on any `RenderContext`. The stream is the
only interface, so producers are interchangeable:

| producer | where | cost |
|---|---|---|
| page JS engine (V8/SpiderMonkey/JSC) | web | no interpreter in the wasm; prelude is a same-origin wasm-bindgen snippet, so no `unsafe-eval` is needed |
| embedded QuickJS (`quickjs` feature) | native | about +0.7 MB stripped, about 1 ms startup |
| `BakedAnim` (frames baked offline in node) | both | no engine; data only; loses mouse and live params |
| Rust `AnimOpWriter` (hand port) | both | none; the final form for hot paths |

Params: `AnimParams { w, h, seed, mouse, args[4] }` go over the wire as
`[w, h, seed, mx, my, hover, a0..a7]`. `Math.random` is reseeded from `seed`
every frame, and `p.rand` is a seeded PRNG, so a frame is a pure function of
`(t, params)`. `tests/native_quickjs.rs` checks that QuickJS and V8 (node)
emit the same stream.

Op set (codes in `uzor::render::anim_script::op`): save/restore,
translate/rotate/scale, beginPath/moveTo/lineTo/quadraticCurveTo/
bezierCurveTo/arc/ellipse/closePath, fill/stroke/clip, fillRect/strokeRect,
fillStyle/strokeStyle (packed 0xRRGGBBAA), lineWidth, globalAlpha.
Not yet supported: gradients, text, setTransform, lineCap/Join, shadows.

Options that were measured or considered:
- boa_engine: +4.1 MB stripped, about 330 µs per frame (QuickJS about 75 µs,
  V8 about 7 µs), and its op stream differed slightly from V8's. Rejected.
- Build-time JS-subset → Rust or bytecode translation: no runtime engine,
  but it means owning a compiler, and every JS feature a prototype uses
  turns into translator work. Revisit only if one is needed for no-std or
  embedded targets.

## In the DOM (`uzor::ui::dom`)

`El::anim(AnimNode::new("spinner").seed(s).args([..]))` makes a box whose content an
animation paints. The box is laid out by flex like any element, an `id` makes it a hit,
and document order is its z-order. `dom::paint_with(ctx, &dom, &mut registry, &frame)`
paints boxes and animations in that order. Per frame, `AnimFrame` gives:

- `t`;
- the `visible` rect (scroll viewport): a node outside it does not run;
- the window `pointer`, which `track_pointer` nodes get anywhere;
- `pointer_over(id)`, which every other node gets, so it is z-order safe and a drawer
  above a node hides the pointer from it;
- `paused`, set for a hidden page or reduced motion.

The script sees its content box: the origin is at the top-left, the size is `p.w × p.h`,
and the pointer is in local coordinates. The frame is clipped to the content box unless
`overflow_visible`, and it gets the element's opacity. `AnimReport` gives the next frame
due (the smallest `1/fps` among running nodes), so a host can sleep when every node is
still.

`AnimRegistry` maps names to a primary script and a fallback painter:

```rust
let mut reg = AnimRegistry::new();
reg.fallback("spinner", Box::new(FnAnim(|ctx, t, p| draw_spinner_in_rust(ctx, t, p))));
let js = uzor_anim_js::JsAnimHost::install(&mut reg, &["spinner"]);
// native + quickjs: js.unwrap().load(include_str!("spinner.js"))
// web: <script src="pkg/anims/spinner.js"> registers it
```

When an engine is missing (`Unavailable`), the registry stays on the fallback for good.
When a script fails, for example because it is not loaded yet, the fallback paints and the
script is tried again a second later. `FnAnim(|ctx, t, p| ..)` turns any Rust painter into
a fallback.

The scripts belong to the app. uzor ships only the runtime and `hexstar.js`, an example.
On web the app serves its scripts same-origin as `<script src>`, so no eval is needed. On
native it embeds the same files with `include_str!` and loads them into QuickJS.
