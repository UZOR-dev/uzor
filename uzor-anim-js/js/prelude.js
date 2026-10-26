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
  transform(a, b, c, d, e, f) { this.b.push(22, a, b, c, d, e, f); }
  drawImage(img, a, b, c, d, e, f, g, h) {
    var slot = publish(img);
    if (slot < 0) return;
    var iw = img.naturalWidth || img.width || img._w || 0;
    var ih = img.naturalHeight || img.height || img._h || 0;
    if (e !== undefined) this.b.push(23, slot, a, b, c, d, e, f, g, h);
    else if (c !== undefined) this.b.push(23, slot, 0, 0, iw, ih, a, b, c, d);
    else this.b.push(23, slot, 0, 0, iw, ih, a, b, iw, ih);
  }
  drawImageTri(img, x0, y0, x1, y1, x2, y2, u0, v0, u1, v1, u2, v2) {
    var slot = publish(img);
    if (slot < 0) return;
    this.b.push(24, slot, x0, y0, x1, y1, x2, y2, u0, v0, u1, v1, u2, v2);
  }
  set fillStyle(v) { this.b.push(15, col(v)); }
  set strokeStyle(v) { this.b.push(16, col(v)); }
  set lineWidth(v) { this.b.push(17, v); }
  set globalAlpha(v) { this.b.push(18, v); }
  set filter(v) {
    var m = /blur\(\s*([0-9.]+)\s*px\s*\)/i.exec(String(v));
    this.b.push(25, m ? +m[1] : 0);
  }
  set globalCompositeOperation(_v) {}
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
function publish(img) {
  if (!img) return -1;
  if (typeof img._slot === "number" && img._slot >= 0 && !img._dirty) return img._slot;
  var px = img._px || img._rgba;
  if (!px) return -1;
  var id = (typeof img._slot === "number" && img._slot >= 0) ? img._slot : (G.__uzorSlotSeq++);
  img._slot = id;
  var w = img._w || img.naturalWidth || img.width || 0;
  var h = img._h || img.naturalHeight || img.height || 0;
  var copy = new Uint8Array(px.length);
  copy.set(px);
  G.__uzorPublish.push({ id: id, w: w, h: h, px: copy });
  img._dirty = false;
  return id;
}
function lerpCol(a, b, u) {
  function ch(shift) {
    var ca = (a >>> shift) & 255, cb = (b >>> shift) & 255;
    return (ca + (cb - ca) * u) | 0;
  }
  return ((ch(24) << 24) | (ch(16) << 16) | (ch(8) << 8) | ch(0)) >>> 0;
}
function sampleGrad(gr, x, y) {
  var dx = x - gr.x1, dy = y - gr.y1;
  var dist = Math.sqrt(dx * dx + dy * dy);
  var t = (dist - gr.r0) / ((gr.r1 - gr.r0) || 1);
  if (t < 0) t = 0;
  if (t > 1) t = 1;
  var stops = gr.stops;
  if (!stops.length) return 0xffffffff;
  var i;
  for (i = 1; i < stops.length; i++) {
    if (t <= stops[i][0]) {
      var span = (stops[i][0] - stops[i - 1][0]) || 1;
      return lerpCol(stops[i - 1][1], stops[i][1], (t - stops[i - 1][0]) / span);
    }
  }
  return stops[stops.length - 1][1];
}
function fillTri(ctx, ax, ay, bx, by, cx, cy) {
  var cv = ctx.cv;
  var minx = Math.max(0, Math.floor(Math.min(ax, bx, cx)));
  var maxx = Math.min(cv._w - 1, Math.ceil(Math.max(ax, bx, cx)));
  var miny = Math.max(0, Math.floor(Math.min(ay, by, cy)));
  var maxy = Math.min(cv._h - 1, Math.ceil(Math.max(ay, by, cy)));
  var den = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
  if (den > -1e-6 && den < 1e-6) return;
  var px = cv._px, w = cv._w;
  for (var y = miny; y <= maxy; y++) {
    for (var x = minx; x <= maxx; x++) {
      var pxc = x + 0.5, pyc = y + 0.5;
      var w0 = ((by - cy) * (pxc - cx) + (cx - bx) * (pyc - cy)) / den;
      var w1 = ((cy - ay) * (pxc - cx) + (ax - cx) * (pyc - cy)) / den;
      var w2 = 1 - w0 - w1;
      if (w0 < -0.01 || w1 < -0.01 || w2 < -0.01) continue;
      px[(y * w + x) * 4 + 3] = 0;
    }
  }
  cv._dirty = true;
}
function paintRect(ctx, x, y, rw, rh) {
  var cv = ctx.cv;
  var x0 = Math.max(0, Math.floor(x));
  var y0 = Math.max(0, Math.floor(y));
  var x1 = Math.min(cv._w, Math.ceil(x + rw));
  var y1 = Math.min(cv._h, Math.ceil(y + rh));
  var px = cv._px, grad = ctx.fillColor && ctx.fillColor.stops ? ctx.fillColor : null;
  var rgba = grad ? 0 : col(ctx.fillColor || "#000");
  var sr = (rgba >>> 24) & 255, sg = (rgba >>> 16) & 255, sb = (rgba >>> 8) & 255, sa = rgba & 255;
  var clip = ctx.clipE;
  for (var yy = y0; yy < y1; yy++) {
    for (var xx = x0; xx < x1; xx++) {
      if (clip) {
        var dx = (xx + 0.5 - clip.x) / (clip.rx || 1);
        var dy = (yy + 0.5 - clip.y) / (clip.ry || 1);
        if (dx * dx + dy * dy > 1) continue;
      }
      var i = (yy * cv._w + xx) * 4;
      var r = sr, g = sg, b = sb, a = sa;
      if (grad) {
        var c = sampleGrad(grad, xx + 0.5, yy + 0.5);
        r = (c >>> 24) & 255; g = (c >>> 16) & 255; b = (c >>> 8) & 255; a = c & 255;
      }
      if (ctx.comp === "source-in") {
        px[i] = r; px[i + 1] = g; px[i + 2] = b;
        px[i + 3] = (px[i + 3] * a / 255) | 0;
      } else if (ctx.comp === "destination-out") {
        px[i + 3] = (px[i + 3] * (255 - a) / 255) | 0;
      } else {
        var inv = 255 - a, da = px[i + 3], outA = (a + da * inv / 255) | 0;
        if (outA > 0) {
          px[i] = ((r * a + px[i] * da * inv / 255) / outA) | 0;
          px[i + 1] = ((g * a + px[i + 1] * da * inv / 255) / outA) | 0;
          px[i + 2] = ((b * a + px[i + 2] * da * inv / 255) / outA) | 0;
          px[i + 3] = outA;
        }
      }
    }
  }
  cv._dirty = true;
}
function makeSoft(cv) {
  return {
    cv: cv, comp: "source-over", fillColor: "#000", clipE: null, pts: [], ell: null, stack: [],
    set fillStyle(v) { this.fillColor = v; },
    set globalCompositeOperation(v) { this.comp = String(v); },
    save() { this.stack.push({ comp: this.comp, fillColor: this.fillColor, clipE: this.clipE }); },
    restore() {
      var s = this.stack.pop();
      if (!s) return;
      this.comp = s.comp; this.fillColor = s.fillColor; this.clipE = s.clipE;
    },
    beginPath() { this.pts = []; this.ell = null; },
    moveTo(x, y) { this.pts = [x, y]; },
    lineTo(x, y) { this.pts.push(x, y); },
    closePath() {},
    ellipse(x, y, rx, ry) { this.ell = { x: x, y: y, rx: rx, ry: ry }; },
    clip() { this.clipE = this.ell; },
    fill() {
      var p = this.pts;
      if (p.length >= 6) fillTri(this, p[0], p[1], p[2], p[3], p[4], p[5]);
    },
    fillRect(x, y, w, h) { paintRect(this, x, y, w, h); },
    drawImage(img) {
      var src = img && (img._px || img._rgba);
      var sw = img ? (img._w || img.naturalWidth || img.width || 0) : 0;
      var sh = img ? (img._h || img.naturalHeight || img.height || 0) : 0;
      if (!src || sw !== cv._w || sh !== cv._h || src.length < cv._px.length) return;
      cv._px.set(src.subarray(0, cv._px.length));
      cv._dirty = true;
    },
    getImageData() { return { data: new Uint8Array(cv._px), width: cv._w, height: cv._h }; },
    putImageData(id) {
      if (id && id.data && id.data.length === cv._px.length) {
        cv._px.set(id.data);
        cv._dirty = true;
      }
    },
    createRadialGradient(x0, y0, r0, x1, y1, r1) {
      return {
        x0: x0, y0: y0, r0: r0, x1: x1, y1: y1, r1: r1, stops: [],
        addColorStop(t, c) { this.stops.push([+t, col(c)]); }
      };
    }
  };
}
function makeCanvas() {
  var cv = {
    _w: 0, _h: 0, _px: new Uint8Array(0), _ctx: null, _slot: -1, _dirty: true,
    getContext() {
      if (!cv._ctx) cv._ctx = makeSoft(cv);
      return cv._ctx;
    }
  };
  Object.defineProperty(cv, "width", {
    get() { return cv._w; },
    set(v) { cv._w = v | 0; cv._px = new Uint8Array(cv._w * cv._h * 4); cv._dirty = true; }
  });
  Object.defineProperty(cv, "height", {
    get() { return cv._h; },
    set(v) { cv._h = v | 0; cv._px = new Uint8Array(cv._w * cv._h * 4); cv._dirty = true; }
  });
  Object.defineProperty(cv, "naturalWidth", { get() { return cv._w; } });
  Object.defineProperty(cv, "naturalHeight", { get() { return cv._h; } });
  return cv;
}
const G = globalThis;
if (typeof G.window === "undefined") G.window = G;
G.__uzorSlotSeq = 2;
G.__uzorPublish = [];
if (typeof G.document === "undefined") {
  G.document = {
    createElement(tag) { return tag === "canvas" ? makeCanvas() : {}; }
  };
}
G.uzorAnims = G.uzorAnims || {};
G.uzorAnim = (name, fn) => { G.uzorAnims[name] = fn; };
/** run(name, t, q) with q = [w, h, seed, mx, my, hover, a0, .., a7]. */
export function uzor_anim_run(name, t, q) {
  const f = G.uzorAnims[name];
  if (!f) throw new Error("uzor anim not registered: " + name);
  G.__uzorPublish = [];
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
