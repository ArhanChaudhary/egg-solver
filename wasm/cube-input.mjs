// The cube to solve, on the demo page: an unfolded net of its 54 stickers
// and the six colours to paint them with (pick a colour, then click or drag
// over stickers). The centres are fixed: they say which face is which.
// Sticker order and moves: cube-model.mjs.

import { FACES, SOLVED, checkStickers } from './cube-model.mjs';

/** The usual scheme: white on U, green on F (red R, yellow D, orange L, blue B). */
export const COLOURS = {
  U: '#f6f6f2',
  R: '#d6262f',
  F: '#1f9a3b',
  D: '#f4d10b',
  L: '#f37a12',
  B: '#2360cf',
};
const STORE = 'egg-solver-painted';
const SVG = 'http://www.w3.org/2000/svg';

function el(tag, attrs = {}, ns = null) {
  const e = ns ? document.createElementNS(ns, tag) : document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

const STYLE = `
.ci-net { display: block; width: 100%; max-width: 27rem; margin: 0 auto; user-select: none; -webkit-user-select: none; }
.ci-net.ci-brush { touch-action: none; }
.ci-net rect { stroke: var(--sticker-gap, #222); stroke-width: 1.5; }
.ci-net.ci-brush rect.ci-paint { cursor: pointer; }
.ci-net rect.ci-bad { stroke: #e0119a; stroke-width: 3.5; }
.ci-net text { font: 600 11px system-ui, sans-serif; pointer-events: none; fill: #111; }
.ci-net text.ci-dark { fill: #fff; }
.ci-palette { display: flex; justify-content: center; gap: 0.5rem; margin: 0.75rem 0 0; }
.ci-palette button { width: 2.1rem; height: 2.1rem; padding: 0; border: 2px solid var(--sticker-gap, #222); border-radius: 50%; cursor: pointer; }
.ci-palette button[aria-pressed="true"] { outline: 3px solid var(--accent, #2f5fa7); outline-offset: 2px; }
.ci-palette.ci-nudge button { animation: ci-nudge 0.5s; }
@keyframes ci-nudge { 50% { transform: scale(1.25); } }
@media (prefers-reduced-motion: reduce) { .ci-palette.ci-nudge button { animation: none; } }
`;

// The net: each face's top-left cell (column, row) in a 12 x 9 grid of stickers.
const NET = { U: [3, 0], L: [0, 3], F: [3, 3], R: [6, 3], B: [9, 3], D: [3, 6] };
const CELL = 26;
const PAD = 2;

/**
 * The net and its palette inside `root`. `onPaint()` is called after every painted sticker.
 * `text`: what screen readers and tooltips say, `{ net, palette, colours: { U, R, F, D, L, B } }`.
 */
export class CubeInput {
  constructor(root, { onPaint = () => {}, text }) {
    this.onPaint = onPaint;
    this.cells = [...SOLVED];
    this.brush = null;
    if (!document.getElementById('ci-style')) {
      const style = el('style', { id: 'ci-style' });
      style.textContent = STYLE;
      document.head.append(style);
    }
    this.net = el('svg', {
      class: 'ci-net',
      viewBox: `0 0 ${12 * CELL + 2 * PAD} ${9 * CELL + 2 * PAD}`,
      role: 'group',
      'aria-label': text.net,
    }, SVG);
    this.rects = [];
    for (let i = 0; i < 54; i++) {
      const face = FACES[Math.floor(i / 9)];
      const [c0, r0] = NET[face];
      const x = PAD + (c0 + (i % 3)) * CELL;
      const y = PAD + (r0 + Math.floor((i % 9) / 3)) * CELL;
      const rect = el('rect', { x, y, width: CELL, height: CELL, rx: 3 }, SVG);
      if (i % 9 !== 4) {
        rect.dataset.i = i;
        rect.classList.add('ci-paint');
      }
      this.net.append(rect);
      this.rects.push(rect);
    }
    for (const face of FACES) {
      const [c0, r0] = NET[face];
      const t = el('text', { x: PAD + (c0 + 1.5) * CELL, y: PAD + (r0 + 1.5) * CELL + 4, 'text-anchor': 'middle' }, SVG);
      if (face === 'R' || face === 'B' || face === 'F') t.classList.add('ci-dark');
      t.textContent = face;
      this.net.append(t);
    }
    this.palette = el('div', { class: 'ci-palette', role: 'group', 'aria-label': text.palette });
    this.swatches = {};
    for (const face of FACES) {
      const b = el('button', { type: 'button', 'aria-pressed': 'false', 'aria-label': text.colours[face], title: text.colours[face] });
      b.style.background = COLOURS[face];
      b.addEventListener('click', () => this.setBrush(this.brush === face ? null : face));
      this.swatches[face] = b;
      this.palette.append(b);
    }
    root.replaceChildren(this.net, this.palette);
    this.enablePainting();
    this.render();
  }

  /** Paint with the colour of `face`, or with none (null): then a touch on the net scrolls the page. */
  setBrush(face) {
    this.brush = face;
    this.net.classList.toggle('ci-brush', Boolean(face));
    for (const [f, b] of Object.entries(this.swatches)) b.setAttribute('aria-pressed', String(f === face));
  }

  enablePainting() {
    let painting = false;
    const paintAt = (e) => {
      const i = document.elementFromPoint(e.clientX, e.clientY)?.dataset?.i;
      if (i === undefined || this.cells[i] === this.brush) return;
      this.cells[i] = this.brush;
      this.highlight([]);
      this.render();
      try {
        localStorage.setItem(STORE, this.stickers());
      } catch {
        // (private windows and blocked storage: the page works without it)
      }
      this.onPaint();
    };
    this.net.addEventListener('pointerdown', (e) => {
      if (!this.brush) return;
      painting = true;
      try {
        this.net.setPointerCapture(e.pointerId);
      } catch {
        // (dragging still paints while the pointer stays over the picture)
      }
      paintAt(e);
    });
    this.net.addEventListener('pointermove', (e) => { if (painting && this.brush) paintAt(e); });
    const stop = () => { painting = false; };
    this.net.addEventListener('pointerup', stop);
    this.net.addEventListener('pointercancel', stop);
    // A sticker clicked with no colour picked: point at the palette.
    this.net.addEventListener('click', (e) => {
      if (this.brush || e.target.dataset?.i === undefined) return;
      this.palette.classList.remove('ci-nudge');
      void this.palette.offsetWidth;
      this.palette.classList.add('ci-nudge');
    });
  }

  render() {
    this.rects.forEach((r, i) => r.setAttribute('fill', COLOURS[this.cells[i]]));
  }

  /** The stickers as a 54-letter string (faces U R F D L B, each row by row). */
  stickers() {
    return this.cells.join('');
  }

  /** Show a cube (54 face letters). */
  setStickers(text) {
    this.cells = [...text];
    this.highlight([]);
    this.render();
  }

  /** The cube last painted in this browser, or null. */
  static painted() {
    try {
      const text = localStorage.getItem(STORE) ?? '';
      const ok = text.length === 54 && [...text].every((c, i) => (i % 9 === 4 ? c === FACES[Math.floor(i / 9)] : FACES.includes(c)));
      return ok ? text : null;
    } catch {
      return null;
    }
  }

  /** Forget the cube last painted (the page shows a typed scramble instead). */
  static forget() {
    try {
      localStorage.removeItem(STORE);
    } catch {
      // (blocked storage: nothing was kept)
    }
  }

  /** null if the stickers make a cube that can be solved, else what is wrong (`checkStickers`). */
  problem() {
    return checkStickers(this.cells);
  }

  /** Outline stickers (those of a mistake). */
  highlight(stickers) {
    const bad = new Set(stickers);
    this.rects.forEach((r, i) => r.classList.toggle('ci-bad', bad.has(i)));
  }
}
