import fs from "node:fs";
import path from "node:path";

const repoRoot = path.resolve(process.cwd(), "../..");
const fixtureDir = path.join(repoRoot, "fixtures", "phase2", "modular_pack");
fs.mkdirSync(fixtureDir, { recursive: true });

const positions = new Float32Array([-0.5, 0, 0, 0.5, 0, 0, 0, 1, 0]);
const binary = Buffer.from(positions.buffer);
const json = {
  asset: { version: "2.0", generator: "Asset Mapper phase 2 fixture" },
  scene: 0,
  scenes: [{ nodes: [0] }],
  nodes: [{ mesh: 0, name: "WallFixture" }],
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
      count: 3,
      type: "VEC3",
      min: [-0.5, 0, 0],
      max: [0.5, 1, 0],
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
output.writeUInt32LE(0x46546c67, offset);
offset += 4;
output.writeUInt32LE(2, offset);
offset += 4;
output.writeUInt32LE(totalLength, offset);
offset += 4;
output.writeUInt32LE(jsonChunk.length, offset);
offset += 4;
output.writeUInt32LE(0x4e4f534a, offset);
offset += 4;
jsonChunk.copy(output, offset);
offset += jsonChunk.length;
output.writeUInt32LE(binChunk.length, offset);
offset += 4;
output.writeUInt32LE(0x004e4942, offset);
offset += 4;
binChunk.copy(output, offset);

fs.writeFileSync(path.join(fixtureDir, "wall.glb"), output);
fs.writeFileSync(path.join(fixtureDir, ".gitkeep"), "");
console.log(path.join(fixtureDir, "wall.glb"));
