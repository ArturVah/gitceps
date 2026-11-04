"use strict";
// What the bro does for each kind of push. To add a joke, this is the file to edit.
//
// A scene is a list of steps, played one after another. A step can set:
//   face      his expression: calm, bored, strain, scream, ouch, sus, proud, dead, shock, cool
//   say       speech bubble text ("" hides it)
//   sweat     true or false
//   hold      true: his hands stick to the bar; false: he lets go
//   cloud     [text, color] shown under the cloud
//   puff      true: dust where the bar meets the floor
//   forklift  true: the forklift appears, and its driver does the talking from then on
//   to, ms    animate values from `anim` (engine.js) over ms milliseconds;
//             optional ease: "inOut" (default), "in", "out", "back"
//   ms        on its own: wait

const GREEN = "#1a7f37", AMBER = "#bf8700", CRIMSON = "#cf222e";
const SMALL_WORDS = /\b(small|tiny|minor|quick|little|typo|tweak|nit|one[- ]?liner?)\b/i;

// ---------- Pieces shared by several scenes ----------

// The code falls in from the top and lands with a thud. weight: 0 (feather) to 1 (enormous).
const dropBar = weight => [
  { to: { barHeight: bar.radius }, ms: 520, ease: "in" },
  { puff: true, to: { quake: weight * 6, bend: weight * 10 }, ms: 70 },
  { to: { quake: 0, bend: 0 }, ms: 260, ease: "out" },
];

// He sprints in from the right, skids and stops.
const runIn = () => [
  { to: { broX: -16, running: 1, tilt: -10 }, ms: 520, ease: "out" },
  { to: { broX: 0, running: 0, tilt: 5, squash: .12 }, ms: 140 },
  { to: { tilt: 0, squash: 0 }, ms: 260, ease: "back" },
];

const runOff = () => [
  { say: "", to: { squash: .1 }, ms: 120 },
  { to: { broX: OFFSCREEN + 60, running: 1, tilt: 10, squash: 0 }, ms: 600, ease: "in" },
];

const grab = (squat, ms = 300) => [
  { to: { hands: bar.radius, squat, squash: .06 }, ms },
  { hold: true, to: { squash: 0 }, ms: 120 },
];

// He lets go and the code flies up into the cloud, which flattens a little as it lands.
const toCloud = ms => [
  { hold: false, say: "", to: { squash: -.08 }, ms: 90 },
  { to: { barHeight: 330, barOpacity: 0, hands: 215, jitter: 0, quake: 0, bend: 0, wobble: 0, squash: 0 }, ms, ease: "out" },
  { to: { cloudSquash: .12 }, ms: 90 },
  { to: { cloudSquash: 0 }, ms: 260, ease: "back" },
];

// A quick zoom onto his face.
const closeUp = (line, ms) => [
  { face: "shock", say: line, to: { zoom: 1, squash: -.1 }, ms: 130, ease: "out" },
  { to: { squash: 0 }, ms },
  { say: "", to: { zoom: 0 }, ms: 300 },
];

// First reaction to a heavy push. If the commit message calls it small, he reads the tag
// out loud and the camera punches in on "SMALL?!".
function sizeUp(push) {
  const word = SMALL_WORDS.exec(push.msg)?.[1];
  if (!word) return [{ face: "shock", say: `${(push.ins + push.del).toLocaleString("en-US")} lines?!`, ms: 1000 }];
  return [
    { face: "sus", to: { squat: 10, tilt: -6 }, ms: 350 },
    { say: `"${shorten(push.msg)}"?`, ms: 1100 },
    { say: "", to: { squat: 0, tilt: 0 }, ms: 250 },
    ...closeUp(`${word.toUpperCase()}?!`, 1000),
  ];
}

// ---------- Scenes ----------

const SCENES = {
  tiny: () => [
    ...dropBar(0), ...runIn(),
    { face: "bored", say: "this? seriously?", ms: 1000 },
    ...grab(4),
    { to: { barHeight: 205, squat: 0 }, ms: 450, ease: "out" },
    ...toCloud(450),
    { cloud: ["✓ pushed", GREEN], to: { hands: 58 }, ms: 300 },
    { say: "next.", ms: 1000 },
    ...runOff(),
  ],

  normal: () => [
    ...dropBar(.3), ...runIn(),
    { face: "proud", say: "light weight, baby!", ms: 900 },
    ...grab(10),
    { face: "strain", to: { barHeight: 115, squat: 4, bend: 4, red: .3 }, ms: 600 },
    { to: { barHeight: 208, squat: 0, red: 0 }, ms: 550 },
    { face: "proud", to: { bend: 0 }, ms: 100 },
    ...toCloud(500),
    { cloud: ["✓ pushed", GREEN], to: { hands: 58 }, ms: 300 },
    { say: "yeah buddy!", ms: 1200 },
    ...runOff(),
  ],

  huge: push => [
    ...dropBar(.8), ...runIn(),
    ...sizeUp(push),
    ...grab(24, 450),
    { face: "strain", sweat: true, say: "NNNGGHH", to: { bend: 12, wobble: 2, jitter: 2 }, ms: 250 },
    { to: { barHeight: 72, squat: 16, jitter: 3, red: .6 }, ms: 1300 },
    { to: { barHeight: 112, squat: 8, red: .8 }, ms: 700 },
    { face: "scream", say: "WHO WROTE ALL THIS?!", to: { jitter: 4.5, red: 1, wobble: 4 }, ms: 800 },
    { face: "strain", to: { barHeight: 208, squat: 0, jitter: 5 }, ms: 1100 },
    ...toCloud(600),
    { cloud: ["✓ pushed", GREEN], face: "dead", sweat: false, to: { red: 0, hands: 40 }, ms: 250 },
    { to: { tilt: 84, hop: 6, squash: .1 }, ms: 420, ease: "in" },   // timber
    { to: { hop: 0, squash: 0 }, ms: 200, ease: "back" },
    { say: "never. again.", ms: 1500 },
  ],

  forklift: push => [
    ...dropBar(1), ...runIn(),
    ...sizeUp(push),
    ...grab(26, 450),
    { face: "strain", sweat: true, say: "NNNGGHH", to: { jitter: 3, red: .9, bend: 3 }, ms: 1300 },
    { hold: false, say: "", sweat: false, to: { squat: 0, hands: 58, red: 0, jitter: 0, bend: 0 }, ms: 350 },
    { face: "bored", say: "hold on.", ms: 800 },
    ...runOff(),
    { forklift: true, face: "cool", say: "beep. beep. beep.", to: { forkliftX: FORKLIFT_PARK, forkHeight: bar.radius - 4 }, ms: 1300, ease: "out" },
    { say: "work smarter.", ms: 900 },
    { say: "", to: { forkHeight: 300, barHeight: 304 }, ms: 1500 },
    ...toCloud(400),
    { cloud: ["✓ pushed", GREEN], to: { forkHeight: bar.radius - 4 }, ms: 500 },
    { say: "never skip forklift day.", ms: 1500 },
  ],

  force: () => [
    ...dropBar(.4), ...runIn(),
    ...closeUp("--FORCE?!", 700),
    ...grab(14, 250),
    { face: "strain", to: { barHeight: 115, squat: 6, red: .7 }, ms: 350 },
    { face: "scream", say: "AAAAAAARGH!!!", to: { quake: 7, jitter: 4, barHeight: 212, squat: 0, red: 1 }, ms: 500 },
    ...toCloud(300),
    { cloud: ["✓ forced update", AMBER], face: "proud", to: { hands: 58, red: 0 }, ms: 300 },
    { say: "(somewhere, a teammate cries)", ms: 1800 },
    ...runOff(),
  ],

  // A push git will refuse because the remote has commits you don't (someone pushed first).
  rejected: () => [
    ...dropBar(.3), ...runIn(),
    { face: "proud", say: "easy.", ms: 700 },
    ...grab(10),
    { to: { barHeight: 208, squat: 0 }, ms: 650 },
    { hold: false, say: "" },
    { to: { barHeight: 292, hands: 215 }, ms: 380, ease: "out" },
    { cloud: ["✗ rejected", CRIMSON], to: { cloudSquash: -.15 }, ms: 120 },
    { to: { cloudSquash: 0 }, ms: 200, ease: "back" },
    { to: { barX: 22, barHeight: bar.radius, barSpin: 8 }, ms: 480, ease: "in" },   // straight onto his foot
    { puff: true, face: "ouch", say: "MY FOOT!!", to: { hop: 28, tilt: -6, hands: 120, quake: 3 }, ms: 180, ease: "out" },
    { to: { hop: 0, tilt: 0, quake: 0 }, ms: 180, ease: "in" },
    { to: { hop: 28, tilt: 6 }, ms: 180, ease: "out" },
    { to: { hop: 0, tilt: 0 }, ms: 180, ease: "in" },
    { to: { hop: 20, tilt: -4 }, ms: 160, ease: "out" },
    { to: { hop: 0, tilt: 0, hands: 58 }, ms: 160, ease: "in" },
    { face: "sus", say: "should've pulled first", ms: 1800 },
  ],

  secret: () => [
    ...dropBar(.2), ...runIn(),
    ...grab(10),
    { to: { barHeight: 70, squat: 4 }, ms: 450 },
    { face: "sus", say: "wait... is that a password?", to: { jitter: 1 }, ms: 1600 },
    { face: "strain", say: "NOPE.", to: { jitter: 0, barHeight: 120, barX: -20 }, ms: 220 },
    { hold: false },
    { to: { barX: -330, barHeight: 230, barSpin: -90, hands: 150 }, ms: 380, ease: "out" },
    { to: { barX: -700, barHeight: 60, barSpin: -200 }, ms: 380, ease: "in" },
    { cloud: ["✗ push blocked", CRIMSON], face: "proud", say: "no secrets in my gym.", to: { hands: 58 }, ms: 2000 },
  ],
};

function stepsFor(push) {
  return (SCENES[push.mode] || SCENES.normal)(push);
}
