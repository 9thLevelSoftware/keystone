/**
 * Procedurally generate real-kit-ish glTF/GLB fixtures for auto-map bake-off.
 *
 * Pieces:
 *   - wall_box.glb          solid box wall
 *   - wall_door.glb         wall with off-center door opening (portal)
 *   - corridor_l.glb        L-shaped corridor volume
 *   - floor_tile.glb        flat floor slab
 *   - door_piece.glb        thin door leaf / plug piece
 *
 * Usage (from repo root or editor package):
 *   node scripts/write-vibe-fixtures.mjs
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const outDir = path.join(repoRoot, "fixtures", "vibe", "modular_kit");
fs.mkdirSync(outDir, { recursive: true });

/** @typedef {[number, number, number]} V3 */

/**
 * Build a triangle-list mesh from an array of boxes (min,max) + optional cutouts
 * on +Z face approximated by NOT filling the hole region (solid shell via boxes).
 */
function boxesToPositions(boxes) {
  /** @type {number[]} */
  const pos = [];
  for (const [min, max] of boxes) {
    const [x0, y0, z0] = min;
    const [x1, y1, z1] = max;
    // 12 triangles = 36 verts (non-indexed for simplicity)
    const faces = [
      // +X
      [x1, y0, z0], [x1, y1, z0], [x1, y1, z1],
      [x1, y0, z0], [x1, y1, z1], [x1, y0, z1],
      // -X
      [x0, y0, z1], [x0, y1, z1], [x0, y1, z0],
      [x0, y0, z1], [x0, y1, z0], [x0, y0, z0],
      // +Y
      [x0, y1, z0], [x0, y1, z1], [x1, y1, z1],
      [x0, y1, z0], [x1, y1, z1], [x1, y1, z0],
      // -Y
      [x0, y0, z1], [x0, y0, z0], [x1, y0, z0],
      [x0, y0, z1], [x1, y0, z0], [x1, y0, z1],
      // +Z
      [x0, y0, z1], [x1, y0, z1], [x1, y1, z1],
      [x0, y0, z1], [x1, y1, z1], [x0, y1, z1],
      // -Z
      [x1, y0, z0], [x0, y0, z0], [x0, y1, z0],
      [x1, y0, z0], [x0, y1, z0], [x1, y1, z0],
    ];
    for (const p of faces) {
      pos.push(p[0], p[1], p[2]);
    }
  }
  return new Float32Array(pos);
}

/**
 * Dense point cloud on solid regions of a wall face with a rectangular hole.
 * Used so portal occupancy detection sees the opening.
 */
function wallWithDoorSamples() {
  // Wall AABB: x[-1,1], y[0,2.5], z[-0.1,0.1]
  // Door hole off-center: x[0.1,0.7], y[0,1.8]
  const boxes = [
    // left solid
    [[-1.0, 0.0, -0.1], [0.1, 2.5, 0.1]],
    // right solid
    [[0.7, 0.0, -0.1], [1.0, 2.5, 0.1]],
    // lintel above door
    [[0.1, 1.8, -0.1], [0.7, 2.5, 0.1]],
  ];
  const base = boxesToPositions(boxes);
  // Extra dense +Z samples for portal grid (excluding hole)
  const extra = [];
  const z = 0.1;
  for (let i = 0; i < 40; i++) {
    for (let j = 0; j < 50; j++) {
      const x = -1 + (i / 39) * 2;
      const y = 0 + (j / 49) * 2.5;
      const inHole = x > 0.1 && x < 0.7 && y >= 0 && y < 1.8;
      if (!inHole) {
        extra.push(x, y, z);
        extra.push(x, y, -z);
      }
    }
  }
  const out = new Float32Array(base.length + extra.length);
  out.set(base, 0);
  out.set(extra, base.length);
  return out;
}

function solidBox(min, max) {
  return boxesToPositions([[min, max]]);
}

function lCorridor() {
  // L: stem along +Z and arm along +X
  const boxes = [
    [[-0.5, 0.0, -0.5], [0.5, 2.2, 1.5]], // stem
    [[-0.5, 0.0, 1.0], [1.5, 2.2, 1.5]], // arm
  ];
  return boxesToPositions(boxes);
}

function writeGlb(filePath, positions, name) {
  const binary = Buffer.from(positions.buffer, positions.byteOffset, positions.byteLength);
  let min = [Infinity, Infinity, Infinity];
  let max = [-Infinity, -Infinity, -Infinity];
  const count = positions.length / 3;
  for (let i = 0; i < count; i++) {
    for (let a = 0; a < 3; a++) {
      const v = positions[i * 3 + a];
      if (v < min[a]) min[a] = v;
      if (v > max[a]) max[a] = v;
    }
  }

  const json = {
    asset: { version: "2.0", generator: "keystone write-vibe-fixtures" },
    scene: 0,
    scenes: [{ nodes: [0] }],
    nodes: [{ mesh: 0, name }],
    meshes: [
      {
        primitives: [
          {
            attributes: { POSITION: 0 },
            mode: 4,
          },
        ],
      },
    ],
    accessors: [
      {
        bufferView: 0,
        componentType: 5126,
        count,
        type: "VEC3",
        min,
        max,
      },
    ],
    bufferViews: [{ buffer: 0, byteOffset: 0, byteLength: binary.byteLength }],
    buffers: [{ byteLength: binary.byteLength }],
  };

  function padTo4(buffer, padByte) {
    const padding = (4 - (buffer.length % 4)) % 4;
    return padding === 0
      ? buffer
      : Buffer.concat([buffer, Buffer.alloc(padding, padByte)]);
  }

  const jsonChunk = padTo4(Buffer.from(JSON.stringify(json), "utf8"), 0x20);
  const binChunk = padTo4(binary, 0x00);
  const totalLength = 12 + 8 + jsonChunk.length + 8 + binChunk.length;
  const output = Buffer.alloc(totalLength);
  let offset = 0;
  output.writeUInt32LE(0x46546c67, offset); offset += 4;
  output.writeUInt32LE(2, offset); offset += 4;
  output.writeUInt32LE(totalLength, offset); offset += 4;
  output.writeUInt32LE(jsonChunk.length, offset); offset += 4;
  output.writeUInt32LE(0x4e4f534a, offset); offset += 4;
  jsonChunk.copy(output, offset); offset += jsonChunk.length;
  output.writeUInt32LE(binChunk.length, offset); offset += 4;
  output.writeUInt32LE(0x004e4942, offset); offset += 4;
  binChunk.copy(output, offset);
  fs.writeFileSync(filePath, output);
  console.log(filePath);
}

const pieces = [
  {
    file: "wall_box.glb",
    name: "WallBox",
    positions: solidBox([-1.0, 0.0, -0.1], [1.0, 2.5, 0.1]),
  },
  {
    file: "wall_door.glb",
    name: "WallDoor",
    positions: wallWithDoorSamples(),
  },
  {
    file: "corridor_l.glb",
    name: "CorridorL",
    positions: lCorridor(),
  },
  {
    file: "floor_tile.glb",
    name: "FloorTile",
    positions: solidBox([-1.0, 0.0, -1.0], [1.0, 0.05, 1.0]),
  },
  {
    file: "door_piece.glb",
    name: "DoorPiece",
    positions: solidBox([-0.3, 0.0, -0.04], [0.3, 1.8, 0.04]),
  },
];

for (const p of pieces) {
  writeGlb(path.join(outDir, p.file), p.positions, p.name);
}

fs.writeFileSync(path.join(outDir, ".gitkeep"), "");
console.log(`Wrote ${pieces.length} vibe fixtures to ${outDir}`);
