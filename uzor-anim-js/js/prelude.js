// uzor-anim-js prelude: a canvas-2d-shaped recorder writing the flat anim op
// stream decoded by `uzor::render::replay_anim_ops`. ES module; the web build
// ships it as a wasm-bindgen snippet (same-origin, no eval), native QuickJS
// evaluates it as a module. Op codes must match `uzor::render::anim_script::op`.
//
// Scripts register without depending on load order:
//   (globalThis.uzorAnims = globalThis.uzorAnims || {}).name = (ctx, t, p) => {...}
const NAMED = { transparent: 0, black: 0x000000ff, white: 0xffffffff };
function col(v) {
  if (typeof v === "number") return v >>> 0;
  const s = String(v).trim();
  if (s[0] === "#") {
    let h = s.slice(1);
    if (h.length === 3 || h.length === 4) h = h.replace(/./g, "$&$&");
    if (h.length === 6) h += "ff";
    return parseInt(h, 16) >>> 0;
  }
  const m = /^rgba?\(([^)]*)\)/.exec(s);
  if (m) {
    const p = m[1].split(",").map(Number);
    const a = p.length > 3 ? p[3] : 1;
    return (((p[0] & 255) << 24) | ((p[1] & 255) << 16) | ((p[2] & 255) << 8) | Math.round(a * 255)) >>> 0;
  }
  return NAMED[s] !== undefined ? NAMED[s] : 0x000000ff;
}
class Rec {
  constructor() { this.b = []; }
  save() { this.b.push(0); }
  restore() { this.b.push(1); }
  translate(x, y) { this.b.push(2, x, y); }
  rotate(a) { this.b.push(3, a); }
  scale(x, y) { this.b.push(4, x, y); }
  beginPath() { this.b.push(5); }
  moveTo(x, y) { this.b.push(6, x, y); }
  lineTo(x, y) { this.b.push(7, x, y); }
  quadraticCurveTo(a, b, x, y) { this.b.push(8, a, b, x, y); }
  bezierCurveTo(a, b, c, d, x, y) { this.b.push(9, a, b, c, d, x, y); }
  arc(x, y, r, s, e) { this.b.push(10, x, y, r, s, e); }
  ellipse(x, y, rx, ry, rot, s, e) { this.b.push(11, x, y, rx, ry, rot, s, e); }
  closePath() { this.b.push(12); }
  fill() { this.b.push(13); }
  stroke() { this.b.push(14); }
  fillRect(x, y, w, h) { this.b.push(19, x, y, w, h); }
  strokeRect(x, y, w, h) { this.b.push(20, x, y, w, h); }
  clip() { this.b.push(21); }
  set fillStyle(v) { this.b.push(15, col(v)); }
  set strokeStyle(v) { this.b.push(16, col(v)); }
  set lineWidth(v) { this.b.push(17, v); }
  set globalAlpha(v) { this.b.push(18, v); }
}
// mulberry32: deterministic per-instance randomness.
function rng(seed) {
  let a = seed >>> 0;
  return function () {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const G = globalThis;
G.uzorAnims = G.uzorAnims || {};
G.uzorAnim = (name, fn) => { G.uzorAnims[name] = fn; };
/** run(name, t, q) with q = [w, h, seed, mx, my, hover, a0, .., a7]. */
export function uzor_anim_run(name, t, q) {
  const f = G.uzorAnims[name];
  if (!f) throw new Error("uzor anim not registered: " + name);
  const r = new Rec();
  Math.random = rng(q[2]);
  f(r, t, {
    w: q[0], h: q[1], seed: q[2],
    mouse: q[5] ? { x: q[3], y: q[4] } : null,
    args: Array.from(q.slice(6, 14)),
    rand: rng(q[2]),
  });
  return new Float64Array(r.b);
}
export function uzor_anim_has(name) { return !!G.uzorAnims[name]; }
G.__uzorAnimRun = uzor_anim_run;
