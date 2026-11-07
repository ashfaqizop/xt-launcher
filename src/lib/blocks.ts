// Profile icons: little pixel-art blocks drawn in code (no image files, nothing to
// download). Every block is a 16x16 grid with a seeded texture, rendered as crisp SVG.

export interface Block {
  id: string;
  name: string;
}

type Grid = string[][];
type Rng = () => number;

const SIZE = 16;

function seeded(seed: number): Rng {
  let a = seed | 0;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function hash(text: string): number {
  let h = 2166136261;
  for (let i = 0; i < text.length; i++) h = Math.imul(h ^ text.charCodeAt(i), 16777619);
  return h >>> 0;
}

function parse(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Lighten (amount > 0) or darken (amount < 0) a #rrggbb colour. */
function shade(hex: string, amount: number): string {
  const [r, g, b] = parse(hex);
  const mix = (c: number) => {
    const v = amount >= 0 ? c + (255 - c) * amount : c * (1 + amount);
    return Math.max(0, Math.min(255, Math.round(v)));
  };
  return '#' + [mix(r), mix(g), mix(b)].map((c) => c.toString(16).padStart(2, '0')).join('');
}

function blank(color: string): Grid {
  return Array.from({ length: SIZE }, () => Array<string>(SIZE).fill(color));
}

/** Fill every pixel with the base colour, jittered a little for a hand-made look. */
function noise(g: Grid, base: string, r: Rng, amount = 0.09, area?: { x0: number; y0: number; x1: number; y1: number }) {
  const { x0, y0, x1, y1 } = area ?? { x0: 0, y0: 0, x1: SIZE - 1, y1: SIZE - 1 };
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) g[y][x] = shade(base, (r() - 0.5) * 2 * amount);
  }
}

function put(g: Grid, x: number, y: number, color: string) {
  if (x >= 0 && y >= 0 && x < SIZE && y < SIZE) g[y][x] = color;
}

function rect(g: Grid, x: number, y: number, w: number, h: number, color: string) {
  for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) put(g, x + i, y + j, color);
}

/** A 1px light top-left and dark bottom-right edge so every block reads as a cube face. */
function bevel(g: Grid) {
  for (let i = 0; i < SIZE; i++) {
    g[0][i] = shade(g[0][i], 0.14);
    g[i][0] = shade(g[i][0], 0.1);
    g[SIZE - 1][i] = shade(g[SIZE - 1][i], -0.22);
    g[i][SIZE - 1] = shade(g[i][SIZE - 1], -0.18);
  }
}

function stone(r: Rng): Grid {
  const g = blank('#8b8b8b');
  noise(g, '#8b8b8b', r, 0.1);
  return g;
}

function ore(color: string, r: Rng): Grid {
  const g = stone(r);
  const spots: [number, number][] = [
    [2, 3], [9, 2], [5, 8], [11, 9], [3, 12], [8, 13],
  ];
  for (const [x, y] of spots) {
    put(g, x, y, color);
    put(g, x + 1, y, shade(color, 0.12));
    put(g, x, y + 1, shade(color, -0.12));
    if (r() > 0.4) put(g, x + 1, y + 1, color);
  }
  return g;
}

const GLYPH_T = ['111', '010', '010', '010', '010'];
const GLYPH_N = ['101', '111', '111', '101', '101'];

function glyph(g: Grid, shape: string[], x: number, y: number, color: string) {
  shape.forEach((row, j) => [...row].forEach((c, i) => c === '1' && put(g, x + i, y + j, color)));
}

const PAINTERS: Record<string, (r: Rng) => Grid> = {
  grass(r) {
    const g = blank('#8a5a38');
    noise(g, '#8a5a38', r, 0.1);
    noise(g, '#5ea03a', r, 0.1, { x0: 0, y0: 0, x1: 15, y1: 3 });
    for (let x = 0; x < SIZE; x++) {
      if (r() > 0.45) put(g, x, 4, shade('#5ea03a', (r() - 0.5) * 0.2));
      if (r() > 0.8) put(g, x, 5, shade('#5ea03a', -0.1));
    }
    return g;
  },
  dirt(r) {
    const g = blank('#8a5a38');
    noise(g, '#8a5a38', r, 0.12);
    for (let i = 0; i < 9; i++) put(g, Math.floor(r() * 16), Math.floor(r() * 16), '#6d4326');
    return g;
  },
  stone,
  planks(r) {
    const g = blank('#b8945f');
    noise(g, '#b8945f', r, 0.05);
    for (const y of [3, 7, 11, 15]) for (let x = 0; x < SIZE; x++) put(g, x, y, '#8d6d3f');
    const seams: [number, number][] = [[5, 0], [11, 4], [3, 8], [9, 12]];
    for (const [x, y] of seams) rect(g, x, y, 1, 3, '#8d6d3f');
    return g;
  },
  sand(r) {
    const g = blank('#dccb8e');
    noise(g, '#dccb8e', r, 0.07);
    for (let i = 0; i < 8; i++) put(g, Math.floor(r() * 16), Math.floor(r() * 16), '#c4b176');
    return g;
  },
  diamond: (r) => ore('#4ee6d6', r),
  gold: (r) => ore('#f7d23c', r),
  redstone: (r) => ore('#d32a2a', r),
  emerald: (r) => ore('#2fd06b', r),
  tnt(r) {
    const g = blank('#c9331f');
    noise(g, '#c9331f', r, 0.06);
    rect(g, 0, 5, 16, 7, '#e9e1d3');
    for (let x = 0; x < SIZE; x++) {
      put(g, x, 5, '#bdb4a3');
      put(g, x, 11, '#bdb4a3');
    }
    glyph(g, GLYPH_T, 2, 6, '#2a2523');
    glyph(g, GLYPH_N, 6, 6, '#2a2523');
    glyph(g, GLYPH_T, 10, 6, '#2a2523');
    return g;
  },
  creeper(r) {
    const g = blank('#5db04a');
    noise(g, '#5db04a', r, 0.13);
    const face = '#101510';
    rect(g, 3, 4, 3, 3, face);
    rect(g, 10, 4, 3, 3, face);
    rect(g, 7, 7, 2, 3, face);
    rect(g, 5, 9, 2, 4, face);
    rect(g, 9, 9, 2, 4, face);
    return g;
  },
  obsidian(r) {
    const g = blank('#1c1130');
    noise(g, '#1c1130', r, 0.16);
    for (let i = 0; i < 14; i++) put(g, Math.floor(r() * 16), Math.floor(r() * 16), '#3b2766');
    return g;
  },
  lava(r) {
    const g = blank('#e8650e');
    noise(g, '#e8650e', r, 0.12);
    for (let i = 0; i < 10; i++) {
      const x = Math.floor(r() * 14);
      const y = Math.floor(r() * 14);
      rect(g, x, y, 2, 2, '#ffb02e');
    }
    for (let i = 0; i < 7; i++) put(g, Math.floor(r() * 16), Math.floor(r() * 16), '#8a2a08');
    return g;
  },
};

export const BLOCKS: Block[] = [
  { id: 'grass', name: 'Grass' },
  { id: 'dirt', name: 'Dirt' },
  { id: 'stone', name: 'Stone' },
  { id: 'planks', name: 'Planks' },
  { id: 'sand', name: 'Sand' },
  { id: 'diamond', name: 'Diamond ore' },
  { id: 'gold', name: 'Gold ore' },
  { id: 'redstone', name: 'Redstone ore' },
  { id: 'emerald', name: 'Emerald ore' },
  { id: 'tnt', name: 'TNT' },
  { id: 'creeper', name: 'Creeper' },
  { id: 'obsidian', name: 'Obsidian' },
  { id: 'lava', name: 'Lava' },
];

const cache = new Map<string, string>();

/** A block as a data: URI of a crisp SVG (use as an <img> source). */
export function blockUri(id: string): string {
  const hit = cache.get(id);
  if (hit) return hit;

  const paint = PAINTERS[id] ?? PAINTERS.stone;
  const grid = paint(seeded(hash(id)));
  bevel(grid);

  // Merge horizontal runs of one colour into a single rect
  let rects = '';
  for (let y = 0; y < SIZE; y++) {
    let x = 0;
    while (x < SIZE) {
      const color = grid[y][x];
      let run = 1;
      while (x + run < SIZE && grid[y][x + run] === color) run++;
      rects += `<rect x="${x}" y="${y}" width="${run}" height="1" fill="${color}"/>`;
      x += run;
    }
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" shape-rendering="crispEdges">${rects}</svg>`;
  const uri = 'data:image/svg+xml,' + encodeURIComponent(svg);
  cache.set(id, uri);
  return uri;
}

// ---- accent colours ---------------------------------------------------------

export interface Accent {
  id: string;
  name: string;
  hex: string;
}

export const ACCENTS: Accent[] = [
  { id: 'ember', name: 'Ember', hex: '#d9482e' },
  { id: 'amber', name: 'Amber', hex: '#e39b14' },
  { id: 'lime', name: 'Lime', hex: '#7bb339' },
  { id: 'emerald', name: 'Emerald', hex: '#25a36a' },
  { id: 'teal', name: 'Teal', hex: '#1c9fa6' },
  { id: 'sky', name: 'Sky', hex: '#3a8de0' },
  { id: 'indigo', name: 'Indigo', hex: '#5b67d8' },
  { id: 'violet', name: 'Violet', hex: '#9a5ad6' },
  { id: 'rose', name: 'Rose', hex: '#d9508a' },
  { id: 'slate', name: 'Slate', hex: '#6e7b8c' },
];

/** Readable text colour on top of a background colour. */
export function onColor(hex: string): string {
  const [r, g, b] = parse(hex);
  const luminance = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
  return luminance > 0.62 ? '#1b1210' : '#ffffff';
}
