"use strict";
// Gitceps animation engine: holds every moving value, tweens them, and redraws the SVG each frame.
// scenes.js says what happens; this file knows how to draw it.
// Plain scripts rather than ES modules, so overlay.html also works when opened straight from disk.

// ---------- Layout, in SVG units (must match the viewBox in overlay.html) ----------
const VIEW = { x: 120, y: 0, w: 600, h: 440 };
const FLOOR = 380;                                // y of the floor
const SPOT = 400;                                 // x where he lifts
const OFFSCREEN = 400;                            // x offset that puts him or the forklift past the right edge
const FORKLIFT_PARK = 160;                        // forklift x offset that puts its forks under the bar
const SHOULDER = { x: 34, y: -106 }, GRIP = 48;   // his shoulders, and where his hands hold the bar
const BAR_HALF = 150, PLATE_EDGE = 138;           // half the bar's length, outer edge of the plates
const SKIN = [242, 185, 141], FLUSHED = [239, 96, 74];

const els = {};
document.querySelectorAll("[id]").forEach(e => els[e.id] = e);

// ---------- Animated values ----------
// Scenes animate these numbers; render() draws them every frame.
const anim = {
  opacity: 0,       // the whole picture fades in and out
  broX: OFFSCREEN,  // his x offset from SPOT
  hop: 0,           // height of his feet above the floor
  tilt: 0,          // degrees, positive leans right
  squat: 0,         // knee bend
  squash: 0,        // positive squashes him wide, negative stretches him tall
  running: 0,       // 0..1, swings his legs and arms
  hands: 58,        // height of his hands when they're not on the bar
  barX: 0,          // bar x offset from SPOT
  barHeight: 520,   // height of the bar above the floor (starts above the top edge)
  barSpin: 0,       // degrees
  barOpacity: 1,
  bend: 0,          // how far the bar ends droop
  wobble: 0,        // how much the bar ends bounce
  jitter: 0,        // shakes him
  quake: 0,         // shakes the whole picture
  red: 0,           // 0..1, how red his face is
  zoom: 0,          // 0..1, camera from the full picture to a close-up of his face
  cloudSquash: 0,   // the cloud flattens when code lands in it
  forkliftX: 520,   // forklift x offset from SPOT
  forkHeight: 0,
  rope: 0,          // 0..1, how much of the rope hangs down from the cloud
  cloudLean: 0,     // cloud tilt in degrees, positive leans it toward him (right)
  ropeSlack: 0,     // how much the rope bows sideways, for the tug
};
let holding = false;         // while true his hands stick to the bar
let sweating = false;
let puffedAt = -Infinity;    // when the last dust puff started
let forkliftShown = false;   // once the forklift is on screen, its driver does the talking
let ropeShown = false;
let bubble = { w: 0, h: 0 };
let bar = { radius: 0, plates: [] };   // radius of the biggest plate = height of the bar resting on the floor

// ---------- Tweens ----------
const easings = {
  inOut: t => t < .5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2,
  in:    t => t ** 3,
  out:   t => 1 - (1 - t) ** 3,
  back:  t => 1 + 2.7 * (t - 1) ** 3 + 1.7 * (t - 1) ** 2,   // overshoots a little, then settles
};

function tween(to, ms, ease = "inOut") {
  return new Promise(done => {
    const from = {};
    for (const k in to) from[k] = anim[k];
    const start = performance.now();
    (function frame(now) {
      const t = Math.min(1, (now - start) / ms), v = easings[ease](t);
      for (const k in to) anim[k] = from[k] + (to[k] - from[k]) * v;
      t < 1 ? requestAnimationFrame(frame) : done();
    })(start);
  });
}

const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
const lerp = (a, b, t) => a + (b - a) * t;
const jolt = amount => (Math.random() - .5) * 2 * amount;
const shorten = (text, max = 28) => text.length > max ? text.slice(0, max - 1) + "…" : text;

// ---------- Playing a scene ----------
// Entry point. The Mac app calls play({ mode, ins, del, msg, secret, remote }) once the page has loaded.
async function play(push) {
  push = { mode: "normal", msg: "", secret: "", from: "", ...push, ins: Number(push.ins) || 0, del: Number(push.del) || 0, commits: Number(push.commits) || 0 };
  if (typeof push.msgs === "string") push.msgs = push.msgs.split("\n").filter(Boolean);
  push.msg ||= "update";
  els.cloudName.textContent = shorten(push.remote || "origin", 18);
  buildBar(push);
  face("calm");
  requestAnimationFrame(render);
  tween({ opacity: 1 }, 350);   // fades in while the first steps play
  await run(stepsFor(push));
  await tween({ opacity: 0 }, 350);
  window.webkit?.messageHandlers?.done?.postMessage(true);   // tells the Mac app to quit
}

// Plays steps one after another. The step format is described at the top of scenes.js.
async function run(steps) {
  for (const step of steps) {
    if (step.face) face(step.face);
    if ("say" in step) say(step.say);
    if ("sweat" in step) sweating = step.sweat;
    if ("hold" in step) hold(step.hold);
    if (step.cloud) setCloud(...step.cloud);
    if (step.puff) puff();
    if (step.forklift) { els.forklift.removeAttribute("hidden"); forkliftShown = true; }
    if (step.rope) { buildRope(step.rope); els.rope.removeAttribute("hidden"); ropeShown = true; }
    if (step.to) await tween(step.to, step.ms, step.ease);
    else if (step.ms) await wait(step.ms);
  }
}

function hold(on) {
  if (holding && !on) anim.hands = anim.barHeight - anim.hop;   // let go where the bar was
  holding = on;
}

// ---------- Face, speech, effects ----------
const FACES = {   // [eyes, brows, mouth], matching the data-* groups in overlay.html
  calm:   ["dots", "calm", "smile"],
  bored:  ["lids", "low", "flat"],
  strain: ["squint", "angry", "teeth"],
  scream: ["squint", "angry", "scream"],
  ouch:   ["xx", "worried", "wavy"],
  sus:    ["sus", "skeptic", "side"],
  proud:  ["dots", "raised", "grin"],
  dead:   ["xx", null, "flat"],
  shock:  ["wide", "raised", "o"],
  cool:   ["shades", null, "smile"],
};

function face(name) {
  const [eyes, brows, mouth] = FACES[name];
  const show = (part, value) =>
    document.querySelectorAll(`[data-${part}]`).forEach(g => g.toggleAttribute("hidden", g.dataset[part] !== value));
  show("eyes", eyes);
  show("brows", brows);
  show("mouth", mouth);
}

// Measures the text now; render() places and shows the bubble on the next frame.
function say(text) {
  els.bubText.textContent = text;
  els.bubble.setAttribute("opacity", 0);
  if (!text) { bubble = { w: 0, h: 0 }; return; }
  const box = els.bubText.getBBox();
  bubble = { w: box.width + 20, h: box.height + 12 };
  els.bubRect.setAttribute("width", bubble.w);
  els.bubRect.setAttribute("height", bubble.h);
}

function setCloud(text, color) {
  els.cloudStatus.textContent = text;
  els.cloudStatus.setAttribute("fill", color);
}

function puff() {
  const xs = [-140, -120, 120, 140];
  [...els.dust.children].forEach((c, i) => c.setAttribute("cx", SPOT + anim.barX + xs[i]));
  puffedAt = performance.now();
}

// ---------- The barbell ----------
// Plates grow with the number of changed lines; past 1,000 lines there are more of them.
function buildBar(push) {
  const lines = push.ins + push.del;
  const radius = Math.round(clamp(10 + 9.5 * Math.log10(lines + 1), 11, 64));
  const count = lines >= 10000 ? 3 : lines >= 1000 ? 2 : 1;
  const width = Math.round(radius * (count > 1 ? .34 : .62)), gap = 3;
  const svg = (tag, attrs, parent) => {
    const e = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const k in attrs) e.setAttribute(k, attrs[k]);
    return parent.appendChild(e);
  };

  els.barbell.textContent = "";
  bar = { radius, plates: [], shaft: svg("path", { fill: "#8a8f98" }, els.barbell) };
  for (const side of [-1, 1]) {
    for (let i = 0; i < count; i++) {
      const r = radius * (1 - i * .12), g = svg("g", {}, els.barbell);
      svg("rect", { x: -width / 2, y: -r, width, height: r * 2, rx: 4, fill: i % 2 ? "#3a3d42" : "#26282c" }, g);
      bar.plates.push({ g, x: (PLATE_EDGE - width / 2 - i * (width + gap)) * side });
    }
    const collar = svg("g", {}, els.barbell);
    svg("rect", { x: -2.5, y: -8, width: 5, height: 16, rx: 1, fill: "#5d626b" }, collar);
    bar.plates.push({ g: collar, x: (PLATE_EDGE - count * width - (count - 1) * gap - 2.5) * side });
  }

  // A name tag tied to the bar: commit message, then size. It hangs right of his head so it never hides his face.
  const n = v => v.toLocaleString("en-US");
  const detail = push.mode === "secret" ? shorten(push.secret || "secret")
    : push.mode === "pull" ? `${n(push.commits || 1)} commit${push.commits == 1 ? "" : "s"} from ${shorten(push.from || "a teammate", 16)}`
    : `+${n(push.ins)} −${n(push.del)}`;
  els.barTag.textContent = "";
  svg("path", { d: `M60 -3 V${-radius - 12}`, stroke: "#d8c98d", "stroke-width": 1.5 }, els.barTag);
  const tag = svg("g", { transform: `translate(30,${-radius - 48})`, "font-size": 12, "font-family": "ui-monospace,Menlo,monospace" }, els.barTag);
  const box = svg("rect", { height: 36, rx: 5, fill: "#fff7d6", stroke: "#d8c98d" }, tag);
  svg("text", { x: 9, y: 15, fill: "#5a4e1f" }, tag).textContent = shorten(push.msg);
  svg("text", { x: 9, y: 29, fill: "#8a7a3a" }, tag).textContent = detail;
  box.setAttribute("width", tag.getBBox().width + 18);
}

// Ties the incoming commit messages along the rope, newest nearest the bar.
function buildRope(messages) {
  const svg = (tag, attrs, parent) => {
    const e = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const k in attrs) e.setAttribute(k, attrs[k]);
    return parent.appendChild(e);
  };
  els.ropeTags.textContent = "";
  messages.slice(0, 3).forEach((m, i) => {
    const g = svg("g", { "data-at": .22 + i * .2 }, els.ropeTags);
    const box = svg("rect", { y: -9, height: 18, rx: 4, fill: "#fff7d6", stroke: "#d8c98d" }, g);
    const t = svg("text", { y: 4, fill: "#5a4e1f", "text-anchor": "end" }, g);
    t.textContent = shorten(m, 20);
    const w = t.getBBox().width + 12;
    t.setAttribute("x", -14);
    box.setAttribute("x", -8 - w);
    box.setAttribute("width", w);
  });
}

// The rope hangs from the cloud. Its lower end is the bar; the middle bows by ropeSlack.
function drawRope() {
  const topX = SPOT, topY = 108;
  const endX = SPOT + anim.barX, endY = FLOOR - anim.barHeight;
  const x = lerp(topX, endX, anim.rope), y = lerp(topY, endY, anim.rope);
  const mx = (topX + x) / 2 + anim.ropeSlack, my = (topY + y) / 2;
  els.ropeLine.setAttribute("d", `M${topX} ${topY} Q${mx} ${my} ${x} ${y}`);
  for (const g of els.ropeTags.children) {
    const t = Number(g.dataset.at) * anim.rope;
    const px = (1 - t) ** 2 * topX + 2 * (1 - t) * t * mx + t * t * x;
    const py = (1 - t) ** 2 * topY + 2 * (1 - t) * t * my + t * t * y;
    g.setAttribute("transform", `translate(${px},${py})`);
  }
}

// ---------- Drawing ----------
// His body: position, tilt and squash, plus conversions between his own coordinates and the picture's.
function broPose() {
  return { x: SPOT + anim.broX + jolt(anim.jitter), y: FLOOR - anim.hop + jolt(anim.jitter / 2),
           a: anim.tilt * Math.PI / 180, sx: 1 + anim.squash, sy: 1 - anim.squash };
}
function toPicture(pose, x, y) {
  x *= pose.sx; y *= pose.sy;
  return [pose.x + x * Math.cos(pose.a) - y * Math.sin(pose.a), pose.y + x * Math.sin(pose.a) + y * Math.cos(pose.a)];
}
function toBro(pose, x, y) {
  const dx = x - pose.x, dy = y - pose.y;
  return [(dx * Math.cos(pose.a) + dy * Math.sin(pose.a)) / pose.sx, (-dx * Math.sin(pose.a) + dy * Math.cos(pose.a)) / pose.sy];
}

function drawArm(side, hx, hy, arm, bicep, hand) {
  const sx = SHOULDER.x * side, sy = SHOULDER.y + anim.squat;
  const dx = hx - sx, dy = hy - sy, len = Math.hypot(dx, dy) || 1;
  let nx = -dy / len, ny = dx / len;
  if (nx * side < 0) { nx = -nx; ny = -ny; }   // elbows always point outwards
  const elbow = Math.min(26, len * .28);
  const ex = (sx + hx) / 2 + nx * elbow, ey = (sy + hy) / 2 + ny * elbow;
  arm.setAttribute("d", `M${sx} ${sy} Q${ex} ${ey} ${hx} ${hy}`);
  const t = .32, a = (1 - t) ** 2, b = 2 * (1 - t) * t, c = t * t;   // bicep sits a third of the way along the curve
  bicep.setAttribute("cx", a * sx + b * ex + c * hx);
  bicep.setAttribute("cy", a * sy + b * ey + c * hy);
  hand.setAttribute("cx", hx);
  hand.setAttribute("cy", hy);
}

function drawLegs(now) {
  const s = anim.squat, run = anim.running;
  for (const [side, leg, shoe] of [[-1, els.legL, els.shoeL], [1, els.legR, els.shoeR]]) {
    const stride = now * .025 + (side < 0 ? Math.PI : 0);
    const fx = 18 * side - Math.sin(stride) * 16 * run, fy = -4 - Math.max(0, Math.cos(stride)) * 12 * run;
    const kx = (16 + s * 1.1) * side - Math.sin(stride) * 6 * run, ky = -20 + s / 2 + (fy + 4) / 3;
    leg.setAttribute("d", `M${14 * side} ${-40 + s} L${kx} ${ky} L${fx} ${fy}`);
    shoe.setAttribute("cx", fx + 4 * side);
    shoe.setAttribute("cy", fy + 1);
  }
}

// The bar bends like a parabola through its middle, so the ends droop under weight.
const sagAt = (now, x) => (anim.bend + anim.wobble * Math.sin(now * .03)) * (x / BAR_HALF) ** 2;

function drawBar(now) {
  const x = SPOT + anim.barX, y = FLOOR - anim.barHeight, end = sagAt(now, BAR_HALF);
  for (const g of [els.barbell, els.barTag]) {
    g.setAttribute("transform", `translate(${x},${y}) rotate(${anim.barSpin})`);
    g.setAttribute("opacity", anim.barOpacity);
  }
  bar.shaft.setAttribute("d", `M${-BAR_HALF} ${end - 3} Q0 ${-end - 3} ${BAR_HALF} ${end - 3} V${end + 3} Q0 ${-end + 3} ${-BAR_HALF} ${end + 3} Z`);
  for (const p of bar.plates) {
    const slope = Math.atan(2 * end * p.x / BAR_HALF ** 2) * 180 / Math.PI;
    p.g.setAttribute("transform", `translate(${p.x},${sagAt(now, p.x)}) rotate(${slope})`);
  }
  return [x, y];
}

function drawForklift(now) {
  const f = anim.forkHeight, top = Math.max(150, f + 46);   // the inner mast telescopes up with the forks
  els.forklift.setAttribute("transform", `translate(${SPOT + anim.forkliftX},${FLOOR})`);
  els.carriage.setAttribute("transform", `translate(0,${-f})`);
  els.mastInner.setAttribute("y", -top);
  els.mastInner.setAttribute("height", top - 60);
  els.wheelF.setAttribute("transform", `translate(40,-20) rotate(${anim.forkliftX * 2.9})`);   // rolls with the distance driven
  els.wheelR.setAttribute("transform", `translate(130,-17) rotate(${anim.forkliftX * 3.4})`);
  els.beacon.setAttribute("opacity", Math.sin(now * .012) > 0 ? 1 : .35);
}

// Three drops run down his face, a third of a cycle apart.
function drawSweat(now) {
  els.sweat.setAttribute("opacity", sweating ? 1 : 0);
  [...els.sweat.children].forEach((drop, i) => {
    const t = (now / 900 + i / 3) % 1;
    drop.setAttribute("opacity", t < .2 ? t / .2 : (1 - t) / .8);
    drop.setAttribute("transform", `translate(0,${26 * t})`);
  });
}

// Dust clouds grow and fade for 0.6 s after the bar hits the floor.
function drawDust(now) {
  const t = clamp((now - puffedAt) / 600, 0, 1), grow = easings.out(t);
  els.dust.setAttribute("opacity", .7 * (1 - t));
  [...els.dust.children].forEach((c, i) => {
    c.setAttribute("r", (i % 2 ? 7 : 9) * (.3 + 1.3 * grow));
    c.setAttribute("cy", FLOOR - 4 - 6 * grow);
  });
}

function drawBubble(pose) {
  els.bubble.setAttribute("opacity", bubble.w ? 1 : 0);
  if (!bubble.w) return;
  const [tipX, tipY] = forkliftShown
    ? [SPOT + anim.forkliftX + 80, FLOOR - 172]   // the driver's head
    : toPicture(pose, 14, -178 + anim.squat);
  const left = clamp(tipX - 24, VIEW.x + 8, VIEW.x + VIEW.w - 8 - bubble.w);
  const top = Math.max(VIEW.y + 6, tipY - 34 - bubble.h), bottom = top + bubble.h;
  const foot = clamp(tipX + 6, left + 14, left + bubble.w - 30);
  els.bubRect.setAttribute("x", left);
  els.bubRect.setAttribute("y", top);
  els.bubTail.setAttribute("d", `M${foot} ${bottom - 1} L${tipX} ${tipY} L${foot + 22} ${bottom - 1}`);
  els.bubText.setAttribute("x", left + 10);
  els.bubText.setAttribute("y", bottom - 10);
}

// The camera eases the viewBox from the full picture to a close-up of his face.
let lastZoom = 0;
function drawCamera(pose) {
  const [hx, hy] = toPicture(pose, 0, -150 + anim.squat);
  const w = lerp(VIEW.w, 230, anim.zoom), h = w * VIEW.h / VIEW.w;
  const cx = lerp(VIEW.x + VIEW.w / 2, hx + 20, anim.zoom), cy = lerp(VIEW.y + VIEW.h / 2, hy - 20, anim.zoom);
  els.svg.setAttribute("viewBox", `${cx - w / 2} ${cy - h / 2} ${w} ${h}`);
  if (anim.zoom !== lastZoom) {   // keep the hand-drawn wobble the same size on screen at any zoom
    els.boilMap.setAttribute("scale", 2.5 * w / VIEW.w);
    els.boilNoise.setAttribute("baseFrequency", .035 * VIEW.w / w);
    lastZoom = anim.zoom;
  }
}

let lastBoil = 0;
function render(now) {
  const pose = broPose();
  const transform = `translate(${pose.x},${pose.y}) rotate(${anim.tilt}) scale(${pose.sx},${pose.sy})`;
  els.broBody.setAttribute("transform", transform);
  els.broArms.setAttribute("transform", transform);
  els.upper.setAttribute("transform", `translate(0,${anim.squat})`);
  els.shadow.setAttribute("cx", pose.x);
  drawLegs(now);

  const [bx, by] = drawBar(now);
  const hands = [-1, 1].map(side => holding
    ? toBro(pose, bx + GRIP * side, by + sagAt(now, GRIP))
    : [GRIP * side, -anim.hands + Math.sin(now * .025 + (side < 0 ? 0 : Math.PI)) * 12 * anim.running]);
  drawArm(-1, ...hands[0], els.armL, els.bicepL, els.handL);
  drawArm(1, ...hands[1], els.armR, els.bicepR, els.handR);

  const red = clamp(anim.red, 0, 1);
  els.face.setAttribute("fill", `rgb(${SKIN.map((c, i) => Math.round(lerp(c, FLUSHED[i], red)))})`);
  els.vein.setAttribute("opacity", clamp(red * 2.5 - 1.25, 0, 1));
  els.vein.setAttribute("transform", `translate(-19,-168) scale(${1 + .15 * Math.sin(now * .03)})`);

  drawSweat(now);
  drawDust(now);
  els.cloud.setAttribute("transform", `rotate(${anim.cloudLean}) scale(${1 + anim.cloudSquash},${1 - anim.cloudSquash})`);
  if (forkliftShown) drawForklift(now);
  if (ropeShown) drawRope();
  drawBubble(pose);
  els.stage.setAttribute("opacity", anim.opacity);
  els.stage.setAttribute("transform", `translate(${jolt(anim.quake)},${jolt(anim.quake)})`);
  drawCamera(pose);

  if (now - lastBoil > 120) {   // new wobble about 8 times a second, like hand-drawn frames
    els.boilNoise.setAttribute("seed", Math.floor(now / 120) % 50);
    lastBoil = now;
  }
  requestAnimationFrame(render);
}
