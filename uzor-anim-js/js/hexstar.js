// Prototype: hexagon ring orbiting a counter-spinning star in fake 3D.
(globalThis.uzorAnims = globalThis.uzorAnims || {}).hexstar = function (ctx, t, p) {
  let cx = p.w / 2, cy = p.h / 2;
  const R = Math.min(p.w, p.h) * 0.36;
  const tilt = 0.38 + 0.08 * Math.sin(t * 0.7);
  const pulse = 1 + 0.06 * Math.sin(t * 2.1);
  if (p.mouse) { cx += (p.mouse.x - cx) * 0.05; cy += (p.mouse.y - cy) * 0.05; }
  function hex(x, y, r, rot) {
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      const a = rot + i * Math.PI / 3, px = x + r * Math.cos(a), py = y + r * Math.sin(a);
      if (i === 0) ctx.moveTo(px, py); else ctx.lineTo(px, py);
    }
    ctx.closePath();
  }
  const items = [];
  for (let i = 0; i < 6; i++) {
    const a = t * 0.9 + i * Math.PI / 3, z = Math.sin(a);
    items.push({ x: cx + R * pulse * Math.cos(a), y: cy + R * pulse * tilt * z, z, rot: -t * 1.3 + i });
  }
  items.sort((u, v) => u.z - v.z);
  function drawHex(it) {
    const s = 0.75 + 0.25 * (it.z + 1) / 2;
    ctx.globalAlpha = 0.45 + 0.55 * (it.z + 1) / 2;
    hex(it.x, it.y, R * 0.22 * s, it.rot);
    ctx.fillStyle = "#1d2a3a"; ctx.fill();
    ctx.strokeStyle = "#f2a93b"; ctx.lineWidth = 2 * s; ctx.stroke();
  }
  let k = 0;
  for (; k < items.length && items[k].z < 0; k++) drawHex(items[k]);
  ctx.globalAlpha = 1;
  ctx.save();
  ctx.translate(cx, cy); ctx.rotate(-t * 1.6); ctx.scale(pulse, pulse);
  ctx.beginPath();
  const n = 5, ro = R * 0.42, ri = ro * 0.45;
  for (let j = 0; j < n * 2; j++) {
    const rr = j % 2 ? ri : ro, aa = j * Math.PI / n - Math.PI / 2;
    if (j === 0) ctx.moveTo(rr * Math.cos(aa), rr * Math.sin(aa)); else ctx.lineTo(rr * Math.cos(aa), rr * Math.sin(aa));
  }
  ctx.closePath();
  ctx.fillStyle = "#ffcf5a"; ctx.fill();
  ctx.restore();
  for (; k < items.length; k++) drawHex(items[k]);
  ctx.globalAlpha = 1;
};
