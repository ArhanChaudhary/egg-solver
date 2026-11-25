// The cube in 3D, for replaying a solution on the demo page: an SVG picture
// that can be turned around by dragging and that animates one face turn at
// a time. Sticker order and moves: cube-model.mjs.
//
// Drawing: every sticker is a black cell of the cube's surface with its
// colour inset (a square with rounded corners). Only the cells that face the viewer are drawn. During a turn
// the cube is two boxes, the turning layer and the rest, closed where they
// were cut apart by a black square each; the box on the viewer's side of the
// cut is drawn last. Both boxes are convex and do not reach into each other,
// so that order is always right.

import { FACES, STICKERS } from './cube-model.mjs';

const SVG = 'http://www.w3.org/2000/svg';
const DISTANCE = 11; // of the eye from the cube's centre, in cubie widths
const INSET = 0.44; // half the width of a sticker's colour (a cell is 1 wide)
const RADIUS = 0.12; // of the colour's rounded corners

const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const scale = (a, s) => [a[0] * s, a[1] * s, a[2] * s];

/** `v` rotated by `angle` (radians, right-handed) about the unit vector `axis`. */
export function rotate(v, axis, angle) {
  const [c, s] = [Math.cos(angle), Math.sin(angle)];
  return add(add(scale(v, c), scale(cross(axis, v), s)), scale(axis, dot(axis, v) * (1 - c)));
}

/** The outward normal of each face: x points to R, y to U, z to F. */
export const NORMALS = FACES.map((_, face) => STICKERS[9 * face + 4].normal);

/**
 * The angle by which a move turns its layer about its face's outward
 * normal: a clockwise quarter turn (seen from outside) is -90 degrees.
 */
export const turnAngle = ({ turns }) => [0, -1, -2, 1][turns] * (Math.PI / 2);

/** A square in space: its centre, two half-edges and the normal it faces. */
const square = (centre, normal, half) => {
  const [a, b] = [0, 1, 2].filter((axis) => !normal[axis]);
  const u = [0, 0, 0];
  const v = [0, 0, 0];
  u[a] = half;
  v[b] = half;
  return { centre, normal, corners: [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([s, t]) => add(centre, add(scale(u, s), scale(v, t)))) };
};

/** `square` with rounded corners (three points on each corner's arc). */
const rounded = (centre, normal, half, radius) => {
  const [a, b] = [0, 1, 2].filter((axis) => !normal[axis]);
  const corners = [];
  [[1, 1], [-1, 1], [-1, -1], [1, -1]].forEach(([s, t], quarter) => {
    for (const step of [0, 0.5, 1]) {
      const angle = (quarter + step) * (Math.PI / 2);
      const point = [...centre];
      point[a] += s * (half - radius) + radius * Math.cos(angle);
      point[b] += t * (half - radius) + radius * Math.sin(angle);
      corners.push(point);
    }
  });
  return { centre, normal, corners };
};

const CELLS = STICKERS.map(({ position, normal }) => {
  const centre = add(position, scale(normal, 0.5));
  return { position, cell: square(centre, normal, 0.5), colour: rounded(centre, normal, INSET, RADIUS) };
});

const el = (tag, attrs = {}) => {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
};

const STYLE = `
.c3 { display: block; width: 100%; max-width: 23rem; margin: 0 auto; overflow: visible; touch-action: pan-y; cursor: grab; user-select: none; -webkit-user-select: none; }
.c3:active { cursor: grabbing; }
.c3 .c3-body { fill: var(--sticker-gap, #222); stroke: var(--sticker-gap, #222); stroke-width: 0.02; stroke-linejoin: round; }
`;

export class Cube3D {
  /** A picture inside `root`; `colours`: the colour of each face letter. */
  constructor(root, colours) {
    this.colours = colours;
    this.yaw = -0.6;
    this.pitch = 0.48;
    this.stickers = null;
    this.turning = null; // { face, angle } while a layer turns
    this.run = 0;
    if (!document.getElementById('c3-style')) {
      const style = document.createElement('style');
      style.id = 'c3-style';
      style.textContent = STYLE;
      document.head.append(style);
    }
    this.svg = el('svg', { class: 'c3', viewBox: '-2.9 -2.9 5.8 5.8', role: 'img', 'aria-label': 'The cube in 3D: drag to turn it around' });
    this.rest = el('g');
    this.layer = el('g');
    this.svg.append(this.rest, this.layer);
    this.cells = CELLS.map(() => {
      const g = el('g');
      const body = el('polygon', { class: 'c3-body' });
      const colour = el('polygon', { class: 'c3-colour' });
      g.append(body, colour);
      this.rest.append(g);
      return { g, body, colour };
    });
    // The two sides of the cut between a turning layer and the rest.
    this.cut = [el('polygon', { class: 'c3-body' }), el('polygon', { class: 'c3-body' })];
    this.rest.append(this.cut[0]);
    this.layer.append(this.cut[1]);
    root.replaceChildren(this.svg);
    this.orbit();
  }

  /** Dragging turns the whole cube around. */
  orbit() {
    let last = null;
    this.svg.addEventListener('pointerdown', (e) => {
      last = [e.clientX, e.clientY];
      try {
        this.svg.setPointerCapture(e.pointerId);
      } catch {
        // (dragging still works while the pointer stays over the picture)
      }
    });
    this.svg.addEventListener('pointermove', (e) => {
      if (!last) return;
      this.yaw += (e.clientX - last[0]) * 0.01;
      this.pitch = Math.max(-1.4, Math.min(1.4, this.pitch + (e.clientY - last[1]) * 0.01));
      last = [e.clientX, e.clientY];
      this.draw();
    });
    const end = () => { last = null; };
    this.svg.addEventListener('pointerup', end);
    this.svg.addEventListener('pointercancel', end);
  }

  /** Show `stickers` (54 face letters) at rest; ends a turn in progress. */
  show(stickers) {
    this.run++;
    this.settle(stickers);
  }

  settle(stickers) {
    this.stickers = stickers;
    if (this.turning) {
      for (const { g } of this.cells) this.rest.append(g);
      this.rest.append(this.cut[0]);
      this.turning = null;
    }
    this.cells.forEach(({ colour }, i) => {
      colour.setAttribute('fill', this.colours[stickers[i]] ?? 'none');
    });
    this.draw();
  }

  /**
   * Animate `move` from the cube `before` over `ms` milliseconds, then show
   * `after` (the cube it makes). Resolves to true when the turn has ended, or
   * to false if `show` or another turn replaced it.
   */
  turn(before, move, after, ms) {
    this.show(before);
    const run = this.run;
    const axis = NORMALS[move.face];
    this.turning = { face: move.face, angle: 0 };
    CELLS.forEach(({ position }, i) => {
      if (dot(position, axis) === 1) this.layer.append(this.cells[i].g);
    });
    this.layer.append(this.cut[1]);
    const target = turnAngle(move);
    const start = performance.now();
    return new Promise((resolve) => {
      const frame = (now) => {
        if (run !== this.run) {
          resolve(false);
          return;
        }
        const t = Math.min(1, (now - start) / ms);
        if (t >= 1) {
          this.settle(after);
          resolve(true);
          return;
        }
        // (ease in and out)
        this.turning.angle = target * (t * t * (3 - 2 * t));
        this.draw();
        requestAnimationFrame(frame);
      };
      requestAnimationFrame(frame);
    });
  }

  /** A point of the cube as the eye sees it: [x, y] of the picture, and its depth. */
  view(p) {
    const [cy, sy] = [Math.cos(this.yaw), Math.sin(this.yaw)];
    const [cp, sp] = [Math.cos(this.pitch), Math.sin(this.pitch)];
    const x = p[0] * cy + p[2] * sy;
    const z0 = -p[0] * sy + p[2] * cy;
    const y = p[1] * cp - z0 * sp;
    const z = p[1] * sp + z0 * cp;
    return [x, y, z];
  }

  draw() {
    const turning = this.turning;
    const axis = turning ? NORMALS[turning.face] : null;
    const moved = (p) => (turning ? rotate(p, axis, turning.angle) : p);
    const eye = [0, 0, DISTANCE];
    /** Set `polygon` to the square `sq` (of the turning layer if `inLayer`); false if it faces away. */
    const place = (polygon, sq, inLayer) => {
      const at = (p) => this.view(inLayer ? moved(p) : p);
      const centre = at(sq.centre);
      const normal = at(add(sq.centre, sq.normal)).map((v, i) => v - centre[i]);
      const seen = dot(normal, [eye[0] - centre[0], eye[1] - centre[1], eye[2] - centre[2]]) > 0;
      if (!seen) {
        polygon.setAttribute('points', '');
        return false;
      }
      const points = sq.corners.map((corner) => {
        const [x, y, z] = at(corner);
        const s = DISTANCE / (DISTANCE - z);
        return `${(x * s).toFixed(3)},${(-y * s).toFixed(3)}`;
      });
      polygon.setAttribute('points', points.join(' '));
      return true;
    };
    CELLS.forEach(({ position, cell, colour }, i) => {
      const inLayer = Boolean(turning) && dot(position, axis) === 1;
      const { body, colour: paint } = this.cells[i];
      if (place(body, cell, inLayer)) place(paint, colour, inLayer);
      else paint.setAttribute('points', '');
    });
    if (!turning) {
      for (const polygon of this.cut) polygon.setAttribute('points', '');
      return;
    }
    // The cut: half a cubie out from the centre; the rest's side faces the
    // layer, the layer's side faces the rest.
    const centre = scale(axis, 0.5);
    place(this.cut[0], square(centre, axis, 1.5), false);
    place(this.cut[1], square(centre, scale(axis, -1), 1.5), true);
    // The box on the eye's side of the cut is drawn last.
    const layerNear = dot(this.view(axis), eye) > 0.5;
    const last = layerNear ? this.layer : this.rest;
    if (this.svg.lastElementChild !== last) this.svg.append(last);
  }
}
