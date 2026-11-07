// Beargarden in the browser: the sk8 simulation (wasm, beargarden/sk8) with the three input
// backends of the spec's C.2 (gamepad, keyboard and mouse, touch) filling one virtual
// controller, and a plain three.js view of the spot, the board and a crude rider.
//
// The lens is where Beargarden is meant to run (beargarden/feature_requests.md); this page
// is the harness that lets the feel be played and tuned on the web, iPhone Safari and a Mac
// before that lands. Build: ./build.sh, then serve this folder (python3 -m http.server).

import * as THREE from 'three';

const SPOTS = [
  ['ovningsplanen', 'Övningsplanen (test plaza)'],
  ['plattan', 'Plattan, Sergels torg'],
  ['ralis', 'Rålis, Rålambshovsparken'],
  ['fryshuset', 'Fryshuset'],
];
const STATES = ['Ground', 'Powerslide', 'Revert', 'Air', 'KnownAir', 'Grind', 'Wipeout', 'OffBoard', 'WallRide', 'Handplant'];
const BTN = { A: 0, B: 1, X: 2, Y: 3, LB: 4, RB: 5, L3: 6, R3: 7, DUP: 8, DDOWN: 9, DLEFT: 10, DRIGHT: 11 };

const $ = (id) => document.getElementById(id);
const store = {
  get(k, d) { try { const v = localStorage.getItem('beargarden.' + k); return v === null ? d : v; } catch { return d; } },
  set(k, v) { try { localStorage.setItem('beargarden.' + k, v); } catch { /* private mode */ } },
};

// ---- the simulation ------------------------------------------------------------------

let ex;
try {
  const r = await WebAssembly.instantiateStreaming(fetch('sk8.wasm'), {});
  ex = r.instance.exports;
} catch (e) {
  $('err').style.display = 'flex';
  $('err').textContent = 'sk8.wasm did not load (' + e + '). Run ./build.sh in beargarden/play first.';
  throw e;
}
const enc = new TextEncoder();
const dec = new TextDecoder();
const SAMPLE = 28;
const MAX_SAMPLES = 64;
const samplesPtr = ex.sk8_alloc(SAMPLE * MAX_SAMPLES);
let evCap = 4096;
let evPtr = ex.sk8_alloc(evCap);
let handle = 0;
let spotData = null;

function withBytes(bytes, fn) {
  const p = ex.sk8_alloc(bytes.length);
  new Uint8Array(ex.memory.buffer, p, bytes.length).set(bytes);
  try { return fn(p, bytes.length); } finally { ex.sk8_dealloc(p, bytes.length); }
}

async function loadSpot(name, difficulty, goofy) {
  const text = await (await fetch('spots/' + name + '.skate.json')).text();
  spotData = JSON.parse(text);
  if (handle) ex.sk8_free(handle);
  handle = withBytes(enc.encode(text), (p, n) => ex.sk8_new(p, n, difficulty));
  if (!handle) throw new Error('bad spot ' + name);
  ex.sk8_set_stance(handle, goofy ? 1 : 0);
  buildSpot(spotData);
}

function readFrame(ptr) {
  const dv = new DataView(ex.memory.buffer, ptr, 160);
  const f = (o) => dv.getFloat32(o, true);
  const v = (o) => new THREE.Vector3(f(o), f(o + 4), f(o + 8));
  return {
    pos: v(8), fwd: v(20), up: v(32), flip: f(44), shuv: f(48), pitch: f(52),
    riderPos: v(56), riderUp: v(68), crouch: f(80), lean: f(84), bodyFlip: f(88),
    clip: dv.getUint32(92, true), goofy: dv.getUint32(96, true), fakie: dv.getUint32(100, true),
    state: STATES[dv.getUint32(104, true)] || '?', speed: f(108),
    camPos: v(112), camTarget: v(124), camFov: f(136),
    score: f(140), line: f(144), lineTimer: f(148), multiplier: f(152), sequence: f(156),
  };
}

function drainEvents() {
  const n = ex.sk8_events(handle, 0, 0);
  if (n === 0) return [];
  if (n > evCap) { ex.sk8_dealloc(evPtr, evCap); evCap = n * 2; evPtr = ex.sk8_alloc(evCap); }
  ex.sk8_events(handle, evPtr, evCap);
  const text = dec.decode(new Uint8Array(ex.memory.buffer, evPtr, n).slice());
  return text.split('\n').filter(Boolean).map((l) => JSON.parse(l));
}

// ---- input (C.2): every backend fills the same controller ----------------------------------

const ctl = { lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 0, buttons: 0 };
let pending = [];
const trail = [];

function snapshot() {
  const merged = mergeSources();
  if (pending.length < MAX_SAMPLES - 1) pending.push(merged);
  trail.push([merged.rx, merged.ry]);
  if (trail.length > 30) trail.shift();
}

// Keyboard: an analog ramp on digital keys (0 to 1 in 0.12 s, back in 0.08 s).
const keys = new Set();
const kb = { lx: 0, ly: 0 };
const KEYMAP = { Space: BTN.A, KeyC: BTN.B, KeyE: BTN.X, KeyF: BTN.Y, KeyQ: BTN.LB, KeyR: BTN.RB, ShiftLeft: BTN.L3, ShiftRight: BTN.L3 };
addEventListener('keydown', (e) => {
  if (['Space', 'Tab', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.code)) e.preventDefault();
  if (e.code === 'Tab') { toggleMenu(); return; }
  keys.add(e.code);
});
addEventListener('keyup', (e) => keys.delete(e.code));
function rampKeys(dt) {
  const tx = (keys.has('KeyD') || keys.has('ArrowRight') ? 1 : 0) - (keys.has('KeyA') || keys.has('ArrowLeft') ? 1 : 0);
  const ty = (keys.has('KeyW') || keys.has('ArrowUp') ? 1 : 0) - (keys.has('KeyS') || keys.has('ArrowDown') ? 1 : 0);
  const ramp = (v, t) => {
    if (t !== 0 && Math.sign(t) === Math.sign(v || t)) return Math.max(-1, Math.min(1, v + Math.sign(t) * dt / 0.12));
    const d = dt / 0.08;
    return Math.abs(v) <= d ? (t === 0 ? 0 : Math.sign(t) * d) : v - Math.sign(v) * d;
  };
  kb.lx = ramp(kb.lx, tx);
  kb.ly = ramp(kb.ly, ty);
}

// Mouse as the right stick: integrate deltas, spring back when it rests.
const mouse = { rx: 0, ry: 0, lastMove: 0, lt: 0, rt: 0, r3: false, sens: Number(store.get('mouseSens', '1')) };
const canvas = $('view');
canvas.addEventListener('pointermove', (e) => {
  if (e.pointerType !== 'mouse' || document.pointerLockElement !== canvas) return;
  const evs = e.getCoalescedEvents ? e.getCoalescedEvents() : [e];
  for (const c of (evs.length ? evs : [e])) {
    mouse.rx += (c.movementX || 0) / 120 * mouse.sens;
    mouse.ry -= (c.movementY || 0) / 120 * mouse.sens;
    const l = Math.hypot(mouse.rx, mouse.ry);
    if (l > 1) { mouse.rx /= l; mouse.ry /= l; }
    mouse.lastMove = performance.now();
    snapshot();
  }
});
canvas.addEventListener('mousedown', (e) => {
  if (document.pointerLockElement !== canvas) { if (!menuOpen()) lockPointer(); return; }
  if (e.button === 0) mouse.rt = 1;
  if (e.button === 2) mouse.lt = 1;
  if (e.button === 1) mouse.r3 = true;
});
addEventListener('mouseup', (e) => {
  if (e.button === 0) mouse.rt = 0;
  if (e.button === 2) mouse.lt = 0;
  if (e.button === 1) mouse.r3 = false;
});
canvas.addEventListener('contextmenu', (e) => e.preventDefault());
function relaxMouse(dt) {
  if (performance.now() - mouse.lastMove > 30) {
    const k = Math.max(0, 1 - 12 * dt);
    mouse.rx *= k; mouse.ry *= k;
  }
}

// Touch: two floating sticks, a tap on the left pushes, a second left finger brakes.
const touch = { left: null, right: null, extraLeft: 0, lx: 0, ly: 0, rx: 0, ry: 0, lt: 0, rt: 0, pushUntil: 0, yUntil: 0, bailStep: 0 };
const RADIUS = 60;
canvas.addEventListener('pointerdown', (e) => {
  if (e.pointerType === 'mouse') return;
  document.body.classList.add('touch');
  canvas.setPointerCapture(e.pointerId);
  const left = e.clientX < innerWidth / 2;
  const t = { id: e.pointerId, x0: e.clientX, y0: e.clientY, t0: performance.now(), moved: 0 };
  if (left) { if (touch.left) touch.extraLeft++; else touch.left = t; }
  else if (!touch.right) touch.right = t;
  else if (left === false && STATES.indexOf(lastFrame?.state) >= 3) touch.rt = 1;
});
canvas.addEventListener('pointermove', (e) => {
  if (e.pointerType === 'mouse') return;
  const evs = e.getCoalescedEvents ? e.getCoalescedEvents() : [e];
  for (const c of (evs.length ? evs : [e])) {
    for (const side of ['left', 'right']) {
      const t = touch[side];
      if (!t || t.id !== e.pointerId) continue;
      let x = (c.clientX - t.x0) / RADIUS, y = -(c.clientY - t.y0) / RADIUS;
      const l = Math.hypot(x, y);
      if (l > 1) { x /= l; y /= l; }
      t.moved = Math.max(t.moved, Math.hypot(c.clientX - t.x0, c.clientY - t.y0));
      if (side === 'left') { touch.lx = x; touch.ly = y; } else { touch.rx = x; touch.ry = y; }
      snapshot();
    }
  }
}, { passive: true });
function touchEnd(e) {
  if (e.pointerType === 'mouse') return;
  if (touch.left && touch.left.id === e.pointerId) {
    if (performance.now() - touch.left.t0 < 150 && touch.left.moved < 8) touch.pushUntil = performance.now() + 60;
    touch.left = null; touch.lx = 0; touch.ly = 0;
  } else if (touch.right && touch.right.id === e.pointerId) {
    touch.right = null; touch.rx = 0; touch.ry = 0;
  } else if (touch.extraLeft > 0) {
    touch.extraLeft--;
  }
  touch.rt = touch.rtButton ? 1 : 0;
}
canvas.addEventListener('pointerup', touchEnd);
canvas.addEventListener('pointercancel', touchEnd);
function holdButton(id, on, off) {
  const el = $(id);
  el.addEventListener('pointerdown', (e) => { e.preventDefault(); on(); });
  el.addEventListener('pointerup', off);
  el.addEventListener('pointercancel', off);
}
holdButton('tLT', () => { touch.lt = 1; }, () => { touch.lt = 0; });
holdButton('tRT', () => { touch.rtButton = true; touch.rt = 1; }, () => { touch.rtButton = false; touch.rt = 0; });
holdButton('tOff', () => { touch.yUntil = performance.now() + 60; }, () => {});
holdButton('tRespawn', () => { touch.bailStep = 1; }, () => {});

// Gamepad (standard mapping).
function readPad() {
  const pads = navigator.getGamepads ? navigator.getGamepads() : [];
  for (const p of pads) {
    if (!p || p.mapping !== 'standard') continue;
    const b = (i) => p.buttons[i] && p.buttons[i].value > 0.5;
    let bits = 0;
    const map = [[0, BTN.A], [1, BTN.B], [2, BTN.X], [3, BTN.Y], [4, BTN.LB], [5, BTN.RB], [10, BTN.L3], [11, BTN.R3], [12, BTN.DUP], [13, BTN.DDOWN], [14, BTN.DLEFT], [15, BTN.DRIGHT]];
    for (const [i, bit] of map) if (b(i)) bits |= 1 << bit;
    return { pad: p, lx: p.axes[0] || 0, ly: -(p.axes[1] || 0), rx: p.axes[2] || 0, ry: -(p.axes[3] || 0), lt: p.buttons[6]?.value || 0, rt: p.buttons[7]?.value || 0, buttons: bits };
  }
  return null;
}
let gp = null;

// ?demo=1: a scripted ride through the same controller (push for a second and a half, then an
// ollie flick, again every four seconds), for headless test runs and for showing the game.
const demo = new URLSearchParams(location.search).get('demo') ? { t0: 0 } : null;
function demoInput(now) {
  if (!demo) return null;
  if (!demo.t0) demo.t0 = now;
  const t = ((now - demo.t0) / 1000) % 4;
  if (t < 1.5) return { buttons: 1 << BTN.A, ry: 0 };
  if (t > 2.5 && t < 2.6) return { buttons: 0, ry: -1 };
  if (t >= 2.6 && t < 2.63) return { buttons: 0, ry: 0 };
  if (t >= 2.63 && t < 2.7) return { buttons: 0, ry: 1 };
  return { buttons: 0, ry: 0 };
}

function mergeSources() {
  const pick = (...v) => v.reduce((a, b) => (Math.abs(b) > Math.abs(a) ? b : a), 0);
  let bits = 0;
  for (const [code, bit] of Object.entries(KEYMAP)) if (keys.has(code)) bits |= 1 << bit;
  if (mouse.r3) bits |= 1 << BTN.R3;
  const now = performance.now();
  if (now < touch.pushUntil) bits |= 1 << BTN.A;
  if (touch.extraLeft > 0 && touch.left) bits |= 1 << BTN.B;
  if (now < touch.yUntil) bits |= 1 << BTN.Y;
  let lt = Math.max(mouse.lt, touch.lt), rt = Math.max(mouse.rt, touch.rt);
  if (touch.bailStep > 0) {
    // The bail chord: both triggers, L3, then R3 pressed fresh.
    lt = rt = 1; bits |= 1 << BTN.L3; if (touch.bailStep > 1) bits |= 1 << BTN.R3;
  }
  const out = {
    lx: pick(kb.lx, touch.lx, gp?.lx || 0), ly: pick(kb.ly, touch.ly, gp?.ly || 0),
    rx: pick(mouse.rx, touch.rx, gp?.rx || 0), ry: pick(mouse.ry, touch.ry, gp?.ry || 0),
    lt: Math.max(lt, gp?.lt || 0), rt: Math.max(rt, gp?.rt || 0), buttons: bits | (gp?.buttons || 0),
  };
  const d = demoInput(now);
  if (d) { out.buttons |= d.buttons; if (d.ry) out.ry = d.ry; }
  // A recovering rider: A after a bail respawns.
  if (lastFrame && lastFrame.state === 'Wipeout' && (touch.pushUntil > now)) out.buttons |= 1 << BTN.A;
  return out;
}

function writeSamples(list) {
  const dv = new DataView(ex.memory.buffer, samplesPtr, SAMPLE * MAX_SAMPLES);
  list.forEach((s, i) => {
    const o = i * SAMPLE;
    dv.setFloat32(o, s.lx, true); dv.setFloat32(o + 4, s.ly, true);
    dv.setFloat32(o + 8, s.rx, true); dv.setFloat32(o + 12, s.ry, true);
    dv.setFloat32(o + 16, s.lt, true); dv.setFloat32(o + 20, s.rt, true);
    dv.setUint32(o + 24, s.buttons >>> 0, true);
  });
}

// ---- the view -----------------------------------------------------------------------------

const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x9fb4c4);
scene.fog = new THREE.Fog(0x9fb4c4, 60, 220);
const camera = new THREE.PerspectiveCamera(70, 1, 0.05, 500);
scene.add(new THREE.HemisphereLight(0xdfe8f0, 0x5a5348, 1.1));
const sun = new THREE.DirectionalLight(0xfff4e0, 1.6);
sun.position.set(30, 50, 20);
scene.add(sun);

const SURFACE_COLOR = { smooth: 0xb9b6ae, rough: 0x8d8a84, slow: 0x6f8f4e, slippery: 0xa9c3d0, veryslow: 0x6b5a40, stairs: 0xcfcac0, unrideable: 0x4a4f63 };
let spotGroup = null;
function buildSpot(spot) {
  if (spotGroup) scene.remove(spotGroup);
  spotGroup = new THREE.Group();
  for (const m of spot.meshes) {
    const pos = new Float32Array(m.triangles.length * 9);
    m.triangles.forEach((t, i) => t.forEach((vi, k) => pos.set(m.vertices[vi], i * 9 + k * 3)));
    const g = new THREE.BufferGeometry();
    g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    g.computeVertexNormals();
    const mat = new THREE.MeshLambertMaterial({ color: SURFACE_COLOR[m.surface] ?? 0xaaaaaa, flatShading: true, side: THREE.FrontSide });
    spotGroup.add(new THREE.Mesh(g, mat));
  }
  const railMat = { metal: new THREE.MeshLambertMaterial({ color: 0xd9822b }), other: new THREE.MeshLambertMaterial({ color: 0xf0ede6 }) };
  for (const r of spot.rails) {
    const pts = r.points.map((p) => new THREE.Vector3(...p));
    if (r.closed) pts.push(pts[0]);
    for (let i = 0; i + 1 < pts.length; i++) {
      const a = pts[i], b = pts[i + 1];
      const len = a.distanceTo(b);
      const rad = r.kind === 'thinrail' || r.kind === 'coping' ? 0.03 : 0.015;
      const cyl = new THREE.Mesh(new THREE.CylinderGeometry(rad, rad, len, 8), r.material === 'metal' ? railMat.metal : railMat.other);
      cyl.position.copy(a).add(b).multiplyScalar(0.5);
      cyl.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), b.clone().sub(a).normalize());
      spotGroup.add(cyl);
      // Posts under a free-standing rail.
      if (r.kind === 'thinrail' || r.kind === 'fatrail') {
        for (const p of [a, b]) {
          const post = new THREE.Mesh(new THREE.CylinderGeometry(0.02, 0.02, p.y, 6), railMat.metal);
          post.position.set(p.x, p.y / 2, p.z);
          if (p.y > 0.05 && i === 0 || p === b) spotGroup.add(post);
        }
      }
    }
  }
  scene.add(spotGroup);
}

// The board: a deck with kicks, trucks and wheels, nose marked.
const board = new THREE.Group();
const flipper = new THREE.Group();
board.add(flipper);
{
  const deckMat = new THREE.MeshLambertMaterial({ color: 0x2f3a46 });
  const gripMat = new THREE.MeshLambertMaterial({ color: 0x1b1d20 });
  const deck = new THREE.Mesh(new THREE.BoxGeometry(0.21, 0.02, 0.6), deckMat);
  flipper.add(deck);
  const grip = new THREE.Mesh(new THREE.BoxGeometry(0.205, 0.003, 0.58), gripMat);
  grip.position.y = 0.0115;
  flipper.add(grip);
  for (const s of [1, -1]) {
    const kick = new THREE.Mesh(new THREE.BoxGeometry(0.2, 0.02, 0.12), s > 0 ? new THREE.MeshLambertMaterial({ color: 0xe8c547 }) : deckMat);
    kick.position.set(0, 0.025, s * 0.35);
    kick.rotation.x = -s * 0.35;
    flipper.add(kick);
    const truck = new THREE.Mesh(new THREE.BoxGeometry(0.19, 0.03, 0.04), new THREE.MeshLambertMaterial({ color: 0xa9adb3 }));
    truck.position.set(0, -0.03, s * 0.243);
    flipper.add(truck);
    for (const x of [0.095, -0.095]) {
      const w = new THREE.Mesh(new THREE.CylinderGeometry(0.031, 0.031, 0.03, 12), new THREE.MeshLambertMaterial({ color: 0xf2efe6 }));
      w.rotation.z = Math.PI / 2;
      w.position.set(x, -0.044, s * 0.243);
      flipper.add(w);
    }
  }
}
scene.add(board);

// The rider: a crude figure (B.17 allows it) that crouches, leans and tumbles.
const rider = new THREE.Group();
const riderBody = new THREE.Group();
rider.add(riderBody);
const skin = new THREE.MeshLambertMaterial({ color: 0xd8b79a });
const shirt = new THREE.MeshLambertMaterial({ color: 0x3f7cac });
const pants = new THREE.MeshLambertMaterial({ color: 0x2b2b30 });
const legs = [];
for (const s of [1, -1]) {
  const leg = new THREE.Mesh(new THREE.BoxGeometry(0.12, 1, 0.12), pants);
  leg.position.z = s * 0.2;
  riderBody.add(leg);
  legs.push(leg);
}
const torso = new THREE.Mesh(new THREE.BoxGeometry(0.22, 0.6, 0.34), shirt);
riderBody.add(torso);
const head = new THREE.Mesh(new THREE.SphereGeometry(0.11, 16, 12), skin);
riderBody.add(head);
const arms = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.08, 1.1), shirt);
riderBody.add(arms);
scene.add(rider);
const shadow = new THREE.Mesh(new THREE.CircleGeometry(0.45, 24), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.22, depthWrite: false }));
shadow.rotation.x = -Math.PI / 2;
scene.add(shadow);
const raycaster = new THREE.Raycaster();

function basis(fwd, up) {
  const z = fwd.clone().normalize();
  const y = up.clone().normalize();
  const x = new THREE.Vector3().crossVectors(y, z).normalize();
  const y2 = new THREE.Vector3().crossVectors(z, x);
  return new THREE.Matrix4().makeBasis(x, y2, z);
}

let lastFrame = null;
function draw(prev, cur, alpha) {
  const lerpV = (a, b) => a.clone().lerp(b, alpha);
  const pos = lerpV(prev.pos, cur.pos);
  const fwd = lerpV(prev.fwd, cur.fwd);
  const up = lerpV(prev.up, cur.up);
  board.position.copy(pos);
  board.quaternion.setFromRotationMatrix(basis(fwd, up));
  flipper.rotation.set(0, 0, 0);
  flipper.rotateX(cur.pitch);
  flipper.rotateY(cur.shuv);
  flipper.rotateZ(cur.flip);

  const rpos = lerpV(prev.riderPos, cur.riderPos);
  const rup = lerpV(prev.riderUp, cur.riderUp);
  const onBoard = cur.state !== 'Wipeout' && cur.state !== 'OffBoard';
  // The rider stands side-on: its local z along the board, chest to the board's right
  // for a regular rider (A.2).
  rider.position.copy(onBoard ? pos.clone().add(up.clone().multiplyScalar(0.02)) : rpos);
  const rfwd = onBoard ? fwd : (cur.fwd);
  rider.quaternion.setFromRotationMatrix(basis(rfwd.clone().projectOnPlane(rup).normalize(), onBoard ? rup : new THREE.Vector3(0, 1, 0)));
  const com = 1 - 0.38 * cur.crouch;
  const legH = 0.85 * com;
  legs.forEach((l) => { l.scale.y = legH; l.position.y = legH / 2; });
  torso.position.y = legH + 0.3;
  head.position.y = legH + 0.72;
  arms.position.y = legH + 0.5;
  riderBody.rotation.set(0, 0, 0);
  if (cur.state === 'Wipeout') {
    // The tumble and its pose are the sim's (Hall of Meat, B.13.1).
    riderBody.rotation.set(cur.bodyFlip, 0, cur.bodyFlip * 0.7);
    rider.position.y -= 0.6;
  } else {
    riderBody.rotateZ(-cur.lean * (cur.goofy ? -1 : 1));
    riderBody.rotateX(cur.bodyFlip);
  }
  // Blob shadow under the board.
  raycaster.set(pos.clone().add(new THREE.Vector3(0, 0.5, 0)), new THREE.Vector3(0, -1, 0));
  const hit = spotGroup ? raycaster.intersectObject(spotGroup, true)[0] : null;
  shadow.visible = !!hit;
  if (hit) { shadow.position.copy(hit.point).add(new THREE.Vector3(0, 0.01, 0)); }

  camera.position.copy(lerpV(prev.camPos, cur.camPos));
  camera.lookAt(lerpV(prev.camTarget, cur.camTarget));
  if (Math.abs(camera.fov - cur.camFov) > 0.1) { camera.fov = cur.camFov; camera.updateProjectionMatrix(); }
}

// ---- HUD ----------------------------------------------------------------------------------

let namesShown = [];
let namesTimer = 0;
function flash(text, bad) {
  const el = $('flash');
  el.textContent = text;
  el.className = 'hud' + (bad ? ' bad' : '');
  el.style.opacity = 1;
  clearTimeout(flash.t);
  flash.t = setTimeout(() => { el.style.opacity = 0; }, 900);
}
function rumble(ms, strength) {
  try { navigator.vibrate?.(ms); } catch { /* not allowed */ }
  try { gp?.pad?.vibrationActuator?.playEffect?.('dual-rumble', { duration: ms, strongMagnitude: strength, weakMagnitude: strength }); } catch { /* none */ }
}
function onEvent(e) {
  switch (e.type) {
    case 'pop': rumble(20, 0.4); break;
    case 'trick': namesShown = [e.name]; namesTimer = 2.5; break;
    case 'grind_start': namesShown = [e.name]; namesTimer = 2.5; rumble(15, 0.2); break;
    case 'land':
      rumble(30, 0.6);
      if (e.quality !== 'Clean') flash(e.quality.toUpperCase());
      break;
    case 'banked': namesShown = e.names; namesTimer = 2.5; $('pts').textContent = '+' + Math.round(e.points) + (e.multiplier > 1 ? '  x' + e.multiplier : ''); break;
    case 'line_ended': if (e.points > 0) flash('LINE +' + Math.round(e.points)); break;
    case 'bail': flash('BAIL: ' + e.reason, true); rumble(120, 1); break;
    case 'run_out': flash('RUN OUT', true); break;
    case 'respawn': break;
  }
}
function drawHud(f, dt) {
  $('scoreN').textContent = Math.round(f.score + f.line).toLocaleString();
  $('speedN').textContent = (f.speed * 3.6).toFixed(0) + ' km/h' + (f.fakie ? '  fakie' : '');
  namesTimer -= dt;
  if (namesTimer <= 0 && f.sequence === 0) { namesShown = []; if (f.line === 0) $('pts').textContent = ''; }
  $('names').textContent = namesShown.join(' + ');
  if (f.sequence > 0) $('pts').textContent = Math.round(f.sequence) + (f.multiplier > 1 ? '  x' + f.multiplier : '');
  $('bar').style.width = Math.max(0, Math.min(1, f.lineTimer / 5)) * 100 + '%';
  $('status').textContent = f.state + (f.state === 'Wipeout' ? '  (A or tap to get up)' : '') + '  ' + samplesPerSecond + ' samples/s';
}

// The input visualiser (D.2 rule 3): both sticks and the right stick's trail.
const vis = $('sticks').getContext('2d');
function drawSticks(c) {
  const w = 336, h = 160, r = 64;
  vis.clearRect(0, 0, w, h);
  [[84, c.lx, c.ly], [252, c.rx, c.ry]].forEach(([cx, x, y], i) => {
    vis.strokeStyle = 'rgba(255,255,255,0.5)'; vis.lineWidth = 2;
    vis.beginPath(); vis.arc(cx, 80, r, 0, Math.PI * 2); vis.stroke();
    vis.strokeStyle = 'rgba(255,255,255,0.2)';
    vis.beginPath(); vis.arc(cx, 80, r * 0.25, 0, Math.PI * 2); vis.stroke();
    if (i === 1) {
      vis.strokeStyle = 'rgba(232,197,71,0.9)'; vis.beginPath();
      trail.forEach(([tx, ty], k) => { const px = cx + tx * r, py = 80 - ty * r; k ? vis.lineTo(px, py) : vis.moveTo(px, py); });
      vis.stroke();
    }
    vis.fillStyle = '#fff'; vis.beginPath(); vis.arc(cx + x * r, 80 - y * r, 6, 0, Math.PI * 2); vis.fill();
  });
}

// ---- the world (mull) -------------------------------------------------------------------------
//
// Beargarden runs on mull: the world keeps the player, the spot, the board and the score. Its web
// port bridges a WebSocket to the line protocol (each frame one command line, each line the driver
// writes one frame). The page logs in, goes to the spot, steps on a board, says where the board is
// as the sim moves it, and tells the world what happened in `skate ...` lines
// (world/support/skate_rules.c), the same lines a lens with sk8 sends (GF-REQ-LENS-2).

const world = { ws: null, ready: false, lastWalk: 0, name: '', lines: [] };
function worldStatus(t) { $('worldStatus').textContent = t; }
function worldSend(line) { if (world.ws && world.ws.readyState === 1) world.ws.send(line); }
function worldConnect(url, spot, mark) {
  if (world.ws) { try { world.ws.close(); } catch { /* closed */ } }
  world.ready = false;
  if (!url) { worldStatus('Skating alone: nothing is kept.'); return; }
  world.name = store.get('worldName', '') || ('skater' + Math.floor(Math.random() * 1e6));
  const key = store.get('worldKey.' + world.name, '');
  const ws = new WebSocket(url);
  world.ws = ws;
  worldStatus('Connecting to ' + url + '...');
  ws.onopen = () => { ws.send(key ? world.name + ' ' + key : world.name); };
  ws.onmessage = (m) => {
    for (const line of String(m.data).split('\n')) {
      const t = line.trim();
      if (!t) continue;
      const k = t.match(/^Your key for '([^']+)' is (\S+)/);
      if (k) { store.set('worldName', k[1]); store.set('worldKey.' + k[1], k[2]); }
      if (/^Welcome, /.test(t) && !world.ready) {
        world.ready = true;
        worldStatus('In the world as ' + world.name + '.');
        // To the spot, onto the board that stands nearest the start.
        // The spot by its name (a ride, the soul's), then to the start, beside the first board.
        worldSend(spot);
        worldSend('jump ' + mark[0].toFixed(2) + ' ' + mark[2].toFixed(2));
        setTimeout(() => worldSend('board'), 400);
      }
      if (/^You step onto a board|^You get into /.test(t)) worldStatus('In the world as ' + world.name + ', on a board.');
      if (/^Every board here is taken/.test(t)) worldStatus('In the world as ' + world.name + ': every board here is taken.');
      const room = t.match(/^(Plattan|Rålis|Fryshuset|Övningsplanen)/);
      if (room) world.room = { Plattan: 'plattan', Rålis: 'ralis', Fryshuset: 'fryshuset', Övningsplanen: 'ovningsplanen' }[room[1]];
      if (/^(Time!|Two minutes|A session|Step on|You left|Sessions are)/.test(t)) flash(t);
      console.log('[world] ' + t);
      world.lines.push(t);
      if (world.lines.length > 50) world.lines.shift();
      window.beargardenWorldLines = world.lines;
    }
  };
  ws.onclose = () => { if (world.ws === ws) { world.ready = false; worldStatus('The world closed the connection.'); } };
  ws.onerror = () => worldStatus('Could not reach ' + url + '.');
}
// Each sk8 event the world should hear, as the lens would say it.
function worldEvent(e) {
  if (!world.ready) return;
  switch (e.type) {
    case 'banked': worldSend('skate banked ' + Math.round(e.points) + ' ' + e.multiplier.toFixed(1) + ' ' + e.names.join('+')); break;
    case 'line_ended': worldSend('skate line ' + Math.round(e.points)); break;
    case 'bail': worldSend('skate bail ' + e.reason); break;
    case 'land': worldSend('skate land ' + e.quality.toLowerCase()); break;
    case 'grind_start': worldSend('skate grind ' + e.name); break;
  }
}
// Where the board is, four times a second.
function worldWalk(f, now) {
  if (!world.ready || now - world.lastWalk < 250) return;
  world.lastWalk = now;
  const yaw = Math.atan2(f.fwd.x, f.fwd.z) * 180 / Math.PI;
  worldSend('walk ' + f.pos.x.toFixed(2) + ' ' + f.pos.z.toFixed(2) + ' ' + yaw.toFixed(1));
}

// ---- the loop -------------------------------------------------------------------------------

let samplesPerSecond = 60;
let sampleCount = 0;
let rateClock = 0;
let last = performance.now();
let prevF = null;
function frame(now) {
  requestAnimationFrame(frame);
  const dt = Math.min(0.1, (now - last) / 1000);
  last = now;
  if (!handle || document.hidden) return;
  gp = readPad();
  rampKeys(dt);
  relaxMouse(dt);
  snapshot();
  if (touch.bailStep > 0) touch.bailStep = touch.bailStep >= 3 ? 0 : touch.bailStep + 1;
  const list = pending;
  pending = [];
  sampleCount += list.length;
  rateClock += dt;
  if (rateClock >= 1) {
    const rate = sampleCount / rateClock > 90 ? 120 : 60;
    if (rate !== samplesPerSecond) { samplesPerSecond = rate; ex.sk8_set_sample_rate(handle, rate); }
    sampleCount = 0; rateClock = 0;
  }
  writeSamples(list);
  ex.sk8_advance(handle, dt, samplesPtr, list.length);
  const cur = readFrame(ex.sk8_frame(handle));
  const prev = readFrame(ex.sk8_prev_frame(handle));
  lastFrame = cur;
  for (const e of drainEvents()) { onEvent(e); worldEvent(e); }
  worldWalk(cur, now);
  draw(prev, cur, Math.max(0, Math.min(1, ex.sk8_alpha(handle))));
  drawHud(cur, dt);
  drawSticks(list[list.length - 1]);
  renderer.render(scene, camera);
  prevF = cur;
}

function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
resize();

// ---- menu -----------------------------------------------------------------------------------

const spotSel = $('spot');
for (const [id, title] of SPOTS) spotSel.add(new Option(title, id));
spotSel.value = new URLSearchParams(location.search).get('spot') || store.get('spot', 'plattan');
$('difficulty').value = store.get('difficulty', '1');
$('world').value = new URLSearchParams(location.search).get('world') || store.get('world', '');
$('session').addEventListener('click', () => {
  if (!world.ready) { flash('Sessions are kept by the world: connect to one first.'); return; }
  worldSend('session');
  $('menu').classList.add('hidden');
});
$('stance').value = store.get('stance', '0');
// Pointer lock needs a user's gesture; without one (an autostart) it is simply not taken.
function lockPointer() {
  try { const r = canvas.requestPointerLock?.(); if (r && r.catch) r.catch(() => {}); } catch { /* no gesture */ }
}
function menuOpen() { return !$('menu').classList.contains('hidden'); }
function toggleMenu() {
  $('menu').classList.toggle('hidden');
  if (menuOpen()) document.exitPointerLock?.();
}
$('go').addEventListener('click', async () => {
  store.set('spot', spotSel.value);
  store.set('difficulty', $('difficulty').value);
  store.set('stance', $('stance').value);
  store.set('world', $('world').value.trim());
  await loadSpot(spotSel.value, Number($('difficulty').value), $('stance').value === '1');
  const mark = (spotData.marks.find((m) => m.name === 'start') || { pos: spotData.spawn }).pos;
  worldConnect($('world').value.trim(), spotSel.value, mark);
  const diff = ['easy', 'normal', 'hardcore'][Number($('difficulty').value)];
  setTimeout(() => { worldSend('stance ' + ($('stance').value === '1' ? 'goofy' : 'regular')); worldSend('difficulty ' + diff); }, 600);
  $('menu').classList.add('hidden');
  if (matchMedia('(pointer: fine)').matches) lockPointer();
});
await loadSpot(spotSel.value, Number($('difficulty').value), $('stance').value === '1');
requestAnimationFrame(frame);
// For tests and the console: the current frame and a way to step without a display.
window.beargarden = { get frame() { return lastFrame; }, loadSpot, world, worldSend };
// ?autostart=1 skates at once (with ?world=... it joins that world): for a test run headless.
if (new URLSearchParams(location.search).get('autostart')) $('go').click();
