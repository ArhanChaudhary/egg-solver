// The cube pictures of the demo page: an unfolded net of the 54 stickers that
// can be painted (click or drag) or only shown, and a small 3D view of the
// U, F and R faces. A palette of the six colours, kept in localStorage, is
// the visitor's colour scheme: the colour of each face of their solved cube.
// Sticker order and moves: cube-model.mjs.

import { FACES, SOLVED, STICKERS, checkStickers } from './cube-model.mjs';

/** The usual scheme: white on U, green on F (red R, yellow D, orange L, blue B). */
const DEFAULTS = {
  U: ['#f6f6f2', 'white'],
  R: ['#d6262f', 'red'],
  F: ['#1f9a3b', 'green'],
  D: ['#f4d10b', 'yellow'],
  L: ['#f37a12', 'orange'],
  B: ['#2360cf', 'blue'],
};
const STORE_COLOURS = 'egg-solver-colours';
const STORE_CELLS = 'egg-solver-cube';
const SVG = 'http://www.w3.org/2000/svg';

function load(key) {
  try {
    return JSON.parse(localStorage.getItem(key) ?? 'null');
  } catch {
    return null;
  }
}

function save(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // private windows and blocked storage: the page works without it
  }
}

function el(tag, attrs = {}, ns = null) {
  const e = ns ? document.createElementNS(ns, tag) : document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

/** Black or white text, whichever reads better on `hex`. */
function ink(hex) {
  const n = parseInt(hex.slice(1), 16);
  const lum = 0.299 * (n >> 16) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255);
  return lum > 140 ? '#111' : '#fff';
}

const STYLE = `
.ci-palette { display: flex; flex-wrap: wrap; gap: 0.35rem; align-items: center; margin: 0.4rem 0; }
.ci-swatch { width: 2.6rem; height: 2.1rem; padding: 0; border: 2px solid var(--line); border-radius: 6px; font-size: 0.85rem; font-weight: 600; }
.ci-swatch[aria-pressed="true"] { outline: 3px solid var(--accent); outline-offset: 2px; }
.ci-edit { display: flex; flex-wrap: wrap; gap: 0.35rem 0.8rem; align-items: center; margin: 0.2rem 0 0.4rem; }
.ci-palette[hidden], .ci-edit[hidden], .ci-code[hidden] { display: none; }
.ci-about { flex-basis: 100%; margin: 0; font-size: 0.85rem; color: var(--muted); }
.ci-edit label { display: inline-flex; gap: 0.25rem; align-items: center; font-size: 0.85rem; }
.ci-edit input { width: 2.2rem; height: 1.6rem; padding: 0; border: 1px solid var(--line); background: none; }
.ci-views { display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; align-items: center; }
.ci-net { width: 100%; max-width: 30rem; display: block; touch-action: none; user-select: none; -webkit-user-select: none; }
.ci-net.ci-paint rect.ci-sticker { cursor: pointer; }
.ci-3d { width: 9.5rem; display: block; }
.ci-net rect.ci-sticker { stroke: var(--sticker-gap); stroke-width: 1.5; }
.ci-3d polygon { stroke: var(--sticker-gap); stroke-width: 0.06; stroke-linejoin: round; }
.ci-3d .ci-body { fill: var(--sticker-gap); }
.ci-net rect.ci-blank, .ci-3d polygon.ci-blank { fill: color-mix(in srgb, var(--line) 55%, var(--bg)); }
.ci-net rect.ci-bad { stroke: #e0119a; stroke-width: 3.5; }
.ci-net rect.ci-moved { stroke: var(--accent); stroke-width: 2.5; }
.ci-3d polygon.ci-moved { stroke: var(--accent); stroke-width: 0.1; }
.ci-turns { display: grid; grid-template-columns: repeat(6, auto); gap: 0.25rem; justify-content: start; margin: 0.3rem 0; }
.ci-turns button { min-width: 2.4rem; padding: 0.1rem 0.3rem; font-family: ui-monospace, monospace; border-color: var(--line); }
.ci-net text { font: 600 11px system-ui, sans-serif; pointer-events: none; }
.ci-code { width: 100%; box-sizing: border-box; font-family: ui-monospace, monospace; }
`;

// The net: each face's top-left cell (column, row) in a 12 x 9 grid of stickers.
const NET = { U: [3, 0], L: [0, 3], F: [3, 3], R: [6, 3], B: [9, 3], D: [3, 6] };
const CELL = 26;
const PAD = 2;

/**
 * A cube picture inside `root`. `paint`: the stickers can be painted (the
 * palette, colour editing and the sticker code are shown too).
 * `onChange()` is called after every edit.
 */
export class CubeInput {
  constructor(root, { paint = false, onChange = () => {}, remember = false, onTurn = null } = {}) {
    this.onChange = onChange;
    this.remember = remember;
    const stored = load(STORE_COLOURS);
    this.colours = Object.fromEntries(
      FACES.map((l) => [l, typeof stored?.[l] === 'string' ? stored[l] : DEFAULTS[l][0]]),
    );
    this.cells = [...SOLVED];
    if (remember) this.restore(false);
    this.brush = 'U';
    this.listeners = [];
    if (!document.getElementById('ci-style')) {
      const style = el('style', { id: 'ci-style' });
      style.textContent = STYLE;
      document.head.append(style);
    }
    const parts = [];
    if (paint) parts.push(...this.buildPalette());
    // the pictures
    const views = el('div', { class: 'ci-views' });
    this.net = el('svg', {
      class: `ci-net${paint ? ' ci-paint' : ''}`,
      viewBox: `0 0 ${12 * CELL + 2 * PAD} ${9 * CELL + 2 * PAD}`,
      role: paint ? 'group' : 'img',
      'aria-label': paint ? 'The cube unfolded: click or drag over stickers to paint them' : 'The cube unfolded',
    }, SVG);
    this.rects = [];
    for (let i = 0; i < 54; i++) {
      const face = FACES[Math.floor(i / 9)];
      const [c0, r0] = NET[face];
      const x = PAD + (c0 + (i % 3)) * CELL;
      const y = PAD + (r0 + Math.floor((i % 9) / 3)) * CELL;
      const rect = el('rect', { x, y, width: CELL, height: CELL, rx: 3, class: 'ci-sticker', 'data-i': i }, SVG);
      this.net.append(rect);
      this.rects.push(rect);
    }
    this.labels = FACES.map((face, f) => {
      const [c0, r0] = NET[face];
      const t = el('text', { x: PAD + (c0 + 1.5) * CELL, y: PAD + (r0 + 1.5) * CELL + 4, 'text-anchor': 'middle' }, SVG);
      t.textContent = face;
      this.net.append(t);
      return { t, f };
    });
    views.append(this.net);
    this.iso = this.build3d();
    views.append(this.iso);
    parts.push(views);
    if (onTurn) {
      this.turns = el('div', { class: 'ci-turns', role: 'group', 'aria-label': 'Turn a face of the cube' });
      for (const suffix of ['', "'", '2']) {
        for (const face of FACES) {
          const b = el('button', { type: 'button', title: `Turn ${face}${suffix}` });
          b.textContent = `${face}${suffix}`;
          b.addEventListener('click', () => onTurn(`${face}${suffix}`));
          this.turns.append(b);
        }
      }
      parts.push(this.turns);
    }
    if (paint) {
      this.code = el('input', {
        class: 'ci-code', type: 'text', spellcheck: 'false', autocomplete: 'off', autocapitalize: 'characters',
        'aria-label': 'Sticker code: 54 letters, faces U R F D L B, each read row by row',
      });
      this.code.addEventListener('change', () => this.setCode(this.code.value));
      parts.push(this.code);
      this.enablePainting();
    }
    root.replaceChildren(...parts);
    this.setPaintable(paint);
    this.render();
  }

  /** Paintable (with the palette, colour editing and sticker code) or only shown. */
  setPaintable(on) {
    this.paintable = on && Boolean(this.palette);
    this.net.classList.toggle('ci-paint', this.paintable);
    for (const e of [this.palette, this.code]) if (e) e.hidden = !this.paintable;
    if (this.edit && !this.paintable) this.edit.hidden = true;
  }

  /** The stickers last painted (kept in localStorage), else the solved cube. */
  restore(draw = true) {
    const cells = load(STORE_CELLS);
    this.cells = Array.isArray(cells) && cells.length === 54 && cells.every((c) => c === null || FACES.includes(c))
      ? cells
      : [...SOLVED];
    if (draw) {
      this.highlight([]);
      this.render();
    }
  }

  buildPalette() {
    this.palette = el('div', { class: 'ci-palette', role: 'group', 'aria-label': 'Colours' });
    this.swatches = {};
    this.edit = el('div', { class: 'ci-edit', role: 'group', 'aria-label': 'Colour of each face' });
    this.edit.hidden = true;
    const about = el('p', { class: 'ci-about' });
    about.textContent = 'The colour of each face of your solved cube, held as you scramble it (usually '
      + 'white on U and green on F). Colours change only how the cube is drawn, never the solution.';
    this.edit.append(about);
    for (const letter of FACES) {
      const b = el('button', { type: 'button', class: 'ci-swatch' });
      b.addEventListener('click', () => this.setBrush(letter));
      this.swatches[letter] = b;
      this.palette.append(b);
      const label = el('label');
      const picker = el('input', { type: 'color', 'aria-label': `Colour of face ${letter}` });
      picker.value = this.colours[letter];
      picker.addEventListener('input', () => {
        this.colours[letter] = picker.value;
        save(STORE_COLOURS, this.colours);
        this.render();
      });
      label.append(`${letter}`, picker);
      this.edit.append(label);
    }
    const eraser = el('button', { type: 'button', class: 'ci-swatch', title: 'Blank: unpaint stickers' });
    eraser.textContent = '×';
    eraser.addEventListener('click', () => this.setBrush(null));
    this.swatches.blank = eraser;
    this.palette.append(eraser);
    const editButton = el('button', { type: 'button' });
    editButton.textContent = 'Change colours';
    editButton.addEventListener('click', () => {
      this.edit.hidden = !this.edit.hidden;
      editButton.setAttribute('aria-expanded', String(!this.edit.hidden));
    });
    const reset = el('button', { type: 'button' });
    reset.textContent = 'Usual colours';
    reset.addEventListener('click', () => {
      for (const l of FACES) this.colours[l] = DEFAULTS[l][0];
      for (const p of this.edit.querySelectorAll('input[type=color]')) p.value = this.colours[p.getAttribute('aria-label').slice(-1)];
      save(STORE_COLOURS, this.colours);
      this.render();
    });
    this.edit.append(reset);
    this.palette.append(editButton);
    this.setBrush('U', false);
    return [this.palette, this.edit];
  }

  /** The 3D view: U, F and R seen from above the URF corner. */
  build3d() {
    const svg = el('svg', { class: 'ci-3d', viewBox: '-2.9 -3.15 5.8 6.3', role: 'img', 'aria-label': 'The cube in 3D: faces U, F and R' }, SVG);
    const project = ([x, y, z]) => [(x - z) * 0.866, (x + z) * 0.5 - y];
    const hull = [[-1.5, 1.5, -1.5], [1.5, 1.5, -1.5], [1.5, -1.5, -1.5], [1.5, -1.5, 1.5], [-1.5, -1.5, 1.5], [-1.5, 1.5, 1.5]]
      .map(project).map((p) => p.map((v) => v.toFixed(3)).join(',')).join(' ');
    svg.append(el('polygon', { class: 'ci-body', points: hull }, SVG));
    this.polys = [];
    STICKERS.forEach(({ position, normal }, i) => {
      if (!(normal[0] === 1 || normal[1] === 1 || normal[2] === 1)) return;
      // the sticker's square: centre on the face plane, two directions in it
      const centre = position.map((p, a) => (normal[a] ? 1.5 * normal[a] : p));
      const dirs = [0, 1, 2].filter((a) => !normal[a]);
      const corner = (s, t) => centre.map((v, a) => v + (a === dirs[0] ? s : a === dirs[1] ? t : 0) * 0.44);
      const pts = [corner(-1, -1), corner(1, -1), corner(1, 1), corner(-1, 1)].map(project);
      const poly = el('polygon', { points: pts.map((p) => p.map((v) => v.toFixed(3)).join(',')).join(' ') }, SVG);
      svg.append(poly);
      this.polys.push([i, poly]);
    });
    return svg;
  }

  setBrush(letter, focus = true) {
    this.brush = letter;
    for (const [l, b] of Object.entries(this.swatches)) b.setAttribute('aria-pressed', String(l === (letter ?? 'blank')));
    if (focus) this.swatches[letter ?? 'blank'].focus();
  }

  enablePainting() {
    let painting = false;
    const paintAt = (e) => {
      const target = document.elementFromPoint(e.clientX, e.clientY);
      const i = target?.dataset?.i;
      if (!this.paintable || i === undefined || this.cells[i] === this.brush) return;
      this.cells[i] = this.brush;
      this.changed();
    };
    this.net.addEventListener('pointerdown', (e) => {
      painting = true;
      try {
        this.net.setPointerCapture(e.pointerId);
      } catch {
        // (dragging still paints while the pointer stays over the picture)
      }
      paintAt(e);
    });
    this.net.addEventListener('pointermove', (e) => { if (painting) paintAt(e); });
    const stop = () => { painting = false; };
    this.net.addEventListener('pointerup', stop);
    this.net.addEventListener('pointercancel', stop);
  }

  changed() {
    this.highlight([]);
    if (this.remember) save(STORE_CELLS, this.cells);
    this.render();
    this.onChange();
  }

  /** Redraw with the current cells and colours; `moved`: stickers to mark. */
  render(moved = []) {
    const fill = (c) => (c ? this.colours[c] : null);
    const mark = new Set(moved);
    this.rects.forEach((r, i) => {
      const colour = fill(this.cells[i]);
      if (colour) r.setAttribute('fill', colour);
      else r.removeAttribute('fill');
      r.classList.toggle('ci-blank', !colour);
      r.classList.toggle('ci-moved', mark.has(i));
    });
    for (const { t, f } of this.labels) {
      const c = this.cells[9 * f + 4];
      t.setAttribute('fill', c ? ink(this.colours[c]) : 'currentColor');
    }
    for (const [i, p] of this.polys) {
      const colour = fill(this.cells[i]);
      if (colour) p.setAttribute('fill', colour);
      else p.removeAttribute('fill');
      p.classList.toggle('ci-blank', !colour);
      p.classList.toggle('ci-moved', mark.has(i));
    }
    if (this.swatches) {
      for (const l of FACES) {
        const b = this.swatches[l];
        b.style.background = this.colours[l];
        b.style.color = ink(this.colours[l]);
        b.textContent = l;
        b.title = `Paint with the colour of face ${l}`;
      }
    }
    if (this.code && document.activeElement !== this.code) this.code.value = this.codeText();
  }

  /** The stickers as text: a facelet string, '.' for blanks. */
  codeText() {
    return this.cells.map((c) => c ?? '.').join('');
  }

  /** Show a cube: a 54-character string of face letters (or '.' for blank). Not remembered. */
  setStickers(text, moved = []) {
    this.cells = [...text].map((c) => (FACES.includes(c) ? c : null));
    this.highlight([]);
    this.render(moved);
  }

  /** Paint the whole cube at once (as an edit: remembered). */
  paintAll(text) {
    this.cells = [...text].map((c) => (FACES.includes(c) ? c : null));
    this.changed();
  }

  /** From the sticker code field: letters U R F D L B (other symbols: by order of appearance). */
  setCode(text) {
    const chars = [...text].filter((c) => !/\s/.test(c));
    if (chars.length !== 54) {
      this.code.setCustomValidity(`${chars.length} stickers: a cube has 54`);
      this.code.reportValidity();
      return;
    }
    this.code.setCustomValidity('');
    // Face letters as they are; any other six symbols stand for the face
    // letter of the centre that has them.
    const direct = chars.every((c) => c === '.' || FACES.includes(c));
    const map = new Map(FACES.map((letter, f) => [chars[9 * f + 4], letter]));
    this.cells = chars.map((c) => (c === '.' ? null : direct ? c : map.get(c) ?? null));
    this.changed();
  }

  fillSolved() {
    this.cells = [...SOLVED];
    this.changed();
  }

  clear() {
    this.cells = [...SOLVED].map((c, i) => (i % 9 === 4 ? c : null));
    this.changed();
  }

  get blanks() {
    return this.cells.filter((c) => !c).length;
  }

  /** The facelet string for the solver, or null with the reason (`checkStickers`). */
  facelets() {
    const problem = checkStickers(this.cells);
    return problem ? { problem } : { facelets: this.cells.join('') };
  }

  colourName(letter) {
    return this.colours[letter] === DEFAULTS[letter][0] ? DEFAULTS[letter][1] : `the colour of ${letter}`;
  }

  /** Outline stickers (e.g. those of a mistake). */
  highlight(stickers) {
    const bad = new Set(stickers);
    this.rects.forEach((r, i) => r.classList.toggle('ci-bad', bad.has(i)));
  }
}
