// Runs a tour in the Mac app's panel (app/TourViewer.swift), which calls start(json) with the tour
// app/TourCheck.swift checked, and opens clicked links in the default browser. Push Bro lives in a window of
// his own beside the panel; the app passes him the reactions asked for here. To try the page in a browser,
// call start() with what `GITCEPS_DRY=1 app/gitceps-app --tour FILE` prints; it works there without him.
"use strict";
const $ = id => document.getElementById(id);
const make = (tag, className, text) => {
  const e = document.createElement(tag);
  if (className) e.className = className;
  if (text != null) e.textContent = text;
  return e;
};
const app = window.webkit?.messageHandlers;   // absent in a browser
const plural = (n, word) => `${n.toLocaleString("en-US")} ${word}${n === 1 ? "" : "s"}`;
const FOLD = 30;   // longer code cards show this many lines and a button for the rest

let tour, current = -1, started = false, finished = false;
let read;   // per step: has the reviewer clicked Got It on it

function react(name, bubbles) {
  if ($("show-bro").checked) app?.react.postMessage({ react: name, bubbles });
}

// ---------- Drawing ----------
function link(href, text) {
  const a = make("a", null, text);
  a.href = href;
  return a;
}

function card({ path, from, to, link: href, lines, more }) {
  const box = make("div", "card");
  const head = make("div", "card-head");
  head.append(make("span", null, from === to ? `${path}  ${from}` : `${path}  ${from}–${to}`));
  if (href) head.append(link(href, "Open ↗"));
  const body = make("div", "lines");
  const fold = lines.length > FOLD + 8;   // folding away only a few lines would hide more than it saves
  lines.forEach(([number, kind, text], i) => {
    const row = make("div", "line " + ({ "+": "added", "-": "removed", gap: "gap" }[kind] || ""));
    row.append(make("span", "n", kind === "gap" ? "⋯" : number ?? ""), make("span", "m", kind === "gap" || kind === " " ? "" : kind));
    if (kind !== "gap") row.append(text);
    row.hidden = fold && i >= FOLD;
    body.append(row);
  });
  if (more) {
    const rest = make("div", "line gap");
    rest.append(make("span", "n", "⋯"), make("span", "m", ""), `${plural(more, "more line")}${href ? ", open the file to see them" : ""}`);
    rest.hidden = fold;
    body.append(rest);
  }
  box.append(head, body);
  if (fold) {
    const expand = make("button", "more", `Show ${plural(lines.length - FOLD, "More Line")}`);
    expand.type = "button";
    expand.addEventListener("click", () => { body.querySelectorAll(".line[hidden]").forEach(row => row.hidden = false); expand.remove(); });
    box.append(expand);
  }
  return box;
}

function overview() {
  const box = make("div", "card");
  box.append(make("div", "card-head", "What changed"));
  const body = make("div", "summary");
  for (const file of tour.overview) {
    const row = make("div", "row");
    const facts = file.facts.length ? `  (${file.facts.join(", ")})` : "";
    row.append(make("span", null, file.path + facts), make("span", null, `+${file.ins} −${file.del}`));
    body.append(row);
  }
  box.append(body);
  return box;
}

// Every routine change the tour left out, grouped by reason, each one click away.
function skippedList() {
  const box = make("div", "card");
  box.append(make("div", "card-head", `Skipped as routine: ${plural(tour.skippedLines, "line")}`));
  const body = make("div", "summary");
  for (const group of tour.skipped) {
    const item = make("div", "group"), head = make("div", "group-head");
    const count = make("span", "count", `${plural(group.count, "line")} · `);
    const show = make("button", "link", "Show"), cards = make("div", "group");
    show.type = "button";
    cards.hidden = true;
    show.addEventListener("click", () => {
      if (!cards.childElementCount) cards.append(...group.cards.map(card));
      cards.hidden = !cards.hidden;
      show.textContent = cards.hidden ? "Show" : "Hide";
    });
    count.append(show);
    head.append(make("span", null, group.reason), count);
    item.append(head, cards);
    body.append(item);
  }
  box.append(body);
  return box;
}

// The top of the main area: a small line above the title, the title, then whatever explains it.
function heading(eyebrow, title, ...rest) {
  $("why").replaceChildren(make("div", "eyebrow", eyebrow), make("h1", null, title), ...rest);
}

function progress() {
  const n = tour.steps.reduce((sum, step, i) => sum + (read[i] ? step.covers : 0), 0);
  $("progress").textContent = `${n} of ${tour.shownLines} important lines read`;
  $("meter").style.width = tour.shownLines ? `${100 * n / tour.shownLines}%` : "100%";
}

function drawSteps() {
  $("position").textContent = finished ? "Done" : started ? `${current + 1} of ${tour.steps.length}` : "";
  $("steps").querySelectorAll(".step").forEach((button, i) => {
    const here = i === current && !finished;
    button.className = "step " + (here ? "current" : read[i] ? "done" : "");
    button.querySelector(".mark").textContent = here ? "▸" : read[i] ? "✓" : "○";
  });
}

function setButtons() {
  const last = current === tour.steps.length - 1;
  $("go").textContent = !started ? "Start Tour" : finished ? "Close" : last ? "Finish" : "Got It";
  $("back").disabled = !started || (current === 0 && !finished);
}

// ---------- Moving through the tour ----------
// Each step explains itself once, then waits. Got It is what counts a step as read.
function enter(i) {
  current = i;
  finished = false;
  const step = tour.steps[i];
  const rest = [make("p", null, step.why)];
  if (step.note) {
    const note = make("div", "note " + step.note.kind, (step.note.kind === "warn" ? "⚠ " : "✓ ") + step.note.text);
    if (step.note.source) note.append(" ", step.note.link ? link(step.note.link, step.note.source) : make("span", "source", step.note.source));
    rest.push(note);
  }
  const said = make("p", "said", `Push Bro: “${step.bro}”`);
  said.hidden = !$("show-bro").checked;
  rest.push(said);
  heading(`Step ${i + 1} of ${tour.steps.length}`, step.title, ...rest);
  $("code").replaceChildren(...(step.cards.length ? step.cards.map(card) : [overview()]));
  $("scroll").scrollTop = 0;
  drawSteps();
  setButtons();
  react(step.react, step.bubbles);
}

function finish() {
  finished = true;
  const unread = read.filter(done => !done).length;   // steps skipped by jumping ahead in the list
  const skipped = tour.skippedLines ? [make("p", null, `${plural(tour.skippedLines, "routine line")} skipped, listed below the summary.`)] : [];
  heading("Done", unread ? `${plural(unread, "step")} not read yet` : `All ${tour.shownLines} important lines read`, ...skipped);
  const summary = make("div", "card"), body = make("div", "summary");
  summary.append(make("div", "card-head", "Summary"));
  body.append(make("span", null, tour.summary));
  if (tour.flag) body.append(make("div", "note warn", tour.flag));
  summary.append(body);
  $("code").replaceChildren(summary, ...(tour.skippedLines ? [skippedList()] : []));
  $("scroll").scrollTop = 0;
  progress();
  drawSteps();
  setButtons();
  react("finale", [tour.number ? "PR spotted. ✓" : "spotted. ✓"]);
}

function go() {
  if (finished) {
    app?.done.postMessage(true);   // Close
  } else if (started) {
    read[current] = true;
    progress();
    current < tour.steps.length - 1 ? enter(current + 1) : finish();
  } else {
    started = true;
    enter(0);
  }
}

function back() {
  if (finished) enter(tour.steps.length - 1);
  else if (current > 0) enter(current - 1);
}

// ---------- Start ----------
// Called by the Mac app with the checked tour as JSON.
function start(json) {
  tour = JSON.parse(json);
  read = tour.steps.map(() => false);
  tour.steps.forEach((step, i) => {
    const button = make("button", "step");
    button.type = "button";
    button.append(make("span", "mark", "○"), make("span", null, step.title + (step.note?.kind === "warn" ? " ⚠" : "")));
    button.addEventListener("click", () => {
      started = true;
      enter(i);
    });
    $("steps").append(button);
  });
  const skipped = tour.skippedLines ? ` The other ${tour.skippedLines} are routine and listed at the end, each with its reason.` : "";
  heading("Tour", tour.title, make("p", null,
    `You'll read ${tour.shownLines} of ${plural(tour.shownLines + tour.skippedLines, "changed line")}, in the order the code runs.` +
    `${skipped} Each step waits for you to click Got It.`));
  $("code").replaceChildren(overview());
  progress();
  drawSteps();
  setButtons();
}

$("go").addEventListener("click", go);
$("back").addEventListener("click", back);
$("show-bro").addEventListener("change", () => {
  const shown = $("show-bro").checked;
  app?.bro.postMessage(shown);
  $("why").querySelector(".said")?.toggleAttribute("hidden", !shown);
  if (shown && started && !finished) react(tour.steps[current].react, tour.steps[current].bubbles);   // he missed this step while he was away
});
// Return is the default button, unless a control has focus, where it presses that. Esc closes the tour.
addEventListener("keydown", event => {
  if (event.key === "Escape") app?.done.postMessage(true);
  else if (event.key === "Enter" && !event.target.closest("button, a, input")) {
    event.preventDefault();
    go();
  }
});
