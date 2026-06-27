import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { deflateSync, inflateSync } from "node:zlib";

const repoRoot = resolve(dirname(new URL(import.meta.url).pathname), "..");

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let i = 0; i < 8; i += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  typeBuffer.copy(out, 4);
  data.copy(out, 8);
  out.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])), 8 + data.length);
  return out;
}

function readPng(path) {
  const png = readFileSync(path);
  if (!png.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) {
    throw new Error(`${path} is not a PNG file`);
  }

  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  const idat = [];

  while (offset < png.length) {
    const length = png.readUInt32BE(offset);
    const type = png.subarray(offset + 4, offset + 8).toString("ascii");
    const data = png.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
    } else if (type === "IDAT") {
      idat.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += 12 + length;
  }

  if (bitDepth !== 8 || ![2, 6].includes(colorType)) {
    throw new Error(`${path} must be 8-bit RGB/RGBA PNG, got bitDepth=${bitDepth} colorType=${colorType}`);
  }

  const bytesPerPixel = colorType === 6 ? 4 : 3;
  const stride = width * bytesPerPixel;
  const raw = inflateSync(Buffer.concat(idat));
  const decoded = Buffer.alloc(width * height * bytesPerPixel);
  let input = 0;
  for (let y = 0; y < height; y += 1) {
    const filter = raw[input];
    input += 1;
    const row = raw.subarray(input, input + stride);
    const prior = y === 0 ? null : decoded.subarray((y - 1) * stride, y * stride);
    const target = decoded.subarray(y * stride, (y + 1) * stride);
    for (let x = 0; x < stride; x += 1) {
      const left = x >= bytesPerPixel ? target[x - bytesPerPixel] : 0;
      const up = prior ? prior[x] : 0;
      const upLeft = prior && x >= bytesPerPixel ? prior[x - bytesPerPixel] : 0;
      let predictor = 0;
      if (filter === 1) predictor = left;
      if (filter === 2) predictor = up;
      if (filter === 3) predictor = Math.floor((left + up) / 2);
      if (filter === 4) {
        const p = left + up - upLeft;
        const pa = Math.abs(p - left);
        const pb = Math.abs(p - up);
        const pc = Math.abs(p - upLeft);
        predictor = pa <= pb && pa <= pc ? left : pb <= pc ? up : upLeft;
      }
      target[x] = (row[x] + predictor) & 0xff;
    }
    input += stride;
  }
  const pixels = Buffer.alloc(width * height * 4);
  for (let i = 0, j = 0; i < decoded.length; i += bytesPerPixel, j += 4) {
    pixels[j] = decoded[i];
    pixels[j + 1] = decoded[i + 1];
    pixels[j + 2] = decoded[i + 2];
    pixels[j + 3] = colorType === 6 ? decoded[i + 3] : 255;
  }
  return { width, height, pixels };
}

function writePng(path, image) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(image.width, 0);
  header.writeUInt32BE(image.height, 4);
  header[8] = 8;
  header[9] = 6;
  header[10] = 0;
  header[11] = 0;
  header[12] = 0;

  const stride = image.width * 4;
  const raw = Buffer.alloc((stride + 1) * image.height);
  for (let y = 0; y < image.height; y += 1) {
    const src = image.pixels.subarray(y * stride, (y + 1) * stride);
    const dst = y * (stride + 1);
    raw[dst] = 0;
    src.copy(raw, dst + 1);
  }

  writeFileSync(path, Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]));
}

function roundAlpha(image, radiusRatio = 0.22) {
  const radius = Math.min(image.width, image.height) * radiusRatio;
  const samples = [
    [0.25, 0.25], [0.75, 0.25],
    [0.25, 0.75], [0.75, 0.75],
  ];
  for (let y = 0; y < image.height; y += 1) {
    for (let x = 0; x < image.width; x += 1) {
      let inside = 0;
      for (const [sx, sy] of samples) {
        const px = x + sx;
        const py = y + sy;
        const cx = px < radius ? radius : px > image.width - radius ? image.width - radius : px;
        const cy = py < radius ? radius : py > image.height - radius ? image.height - radius : py;
        if ((px - cx) ** 2 + (py - cy) ** 2 <= radius ** 2) inside += 1;
      }
      const offset = (y * image.width + x) * 4 + 3;
      image.pixels[offset] = Math.round(image.pixels[offset] * (inside / samples.length));
    }
  }
}

function alphaBounds(image) {
  let minX = image.width;
  let minY = image.height;
  let maxX = -1;
  let maxY = -1;
  for (let y = 0; y < image.height; y += 1) {
    for (let x = 0; x < image.width; x += 1) {
      if (image.pixels[(y * image.width + x) * 4 + 3] <= 8) continue;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
  }
  return maxX < minX ? null : { minX, minY, maxX, maxY };
}

function sampleBilinear(image, x, y) {
  const x0 = Math.max(0, Math.min(image.width - 1, Math.floor(x)));
  const y0 = Math.max(0, Math.min(image.height - 1, Math.floor(y)));
  const x1 = Math.max(0, Math.min(image.width - 1, x0 + 1));
  const y1 = Math.max(0, Math.min(image.height - 1, y0 + 1));
  const tx = x - x0;
  const ty = y - y0;
  const out = [0, 0, 0, 0];
  for (let c = 0; c < 4; c += 1) {
    const a = image.pixels[(y0 * image.width + x0) * 4 + c];
    const b = image.pixels[(y0 * image.width + x1) * 4 + c];
    const d = image.pixels[(y1 * image.width + x0) * 4 + c];
    const e = image.pixels[(y1 * image.width + x1) * 4 + c];
    out[c] = Math.round((a * (1 - tx) + b * tx) * (1 - ty) + (d * (1 - tx) + e * tx) * ty);
  }
  return out;
}

function resizeAlphaContent(image, targetSize, fillSize) {
  const bounds = alphaBounds(image);
  if (!bounds) return image;
  const cropWidth = bounds.maxX - bounds.minX + 1;
  const cropHeight = bounds.maxY - bounds.minY + 1;
  const scale = Math.min(fillSize / cropWidth, fillSize / cropHeight);
  const out = {
    width: targetSize,
    height: targetSize,
    pixels: Buffer.alloc(targetSize * targetSize * 4),
  };
  const drawnWidth = cropWidth * scale;
  const drawnHeight = cropHeight * scale;
  const left = (targetSize - drawnWidth) / 2;
  const top = (targetSize - drawnHeight) / 2;
  for (let y = 0; y < targetSize; y += 1) {
    for (let x = 0; x < targetSize; x += 1) {
      const sourceX = (x - left) / scale + bounds.minX;
      const sourceY = (y - top) / scale + bounds.minY;
      if (sourceX < bounds.minX || sourceX > bounds.maxX || sourceY < bounds.minY || sourceY > bounds.maxY) continue;
      const rgba = sampleBilinear(image, sourceX, sourceY);
      const offset = (y * targetSize + x) * 4;
      out.pixels[offset] = rgba[0];
      out.pixels[offset + 1] = rgba[1];
      out.pixels[offset + 2] = rgba[2];
      out.pixels[offset + 3] = rgba[3];
    }
  }
  return out;
}

function writeRgba(path, image) {
  writeFileSync(path, image.pixels);
}

const preferredSource = "/Users/charlie/Downloads/codex_switch图标.png";
const appSource = existsSync(preferredSource)
  ? preferredSource
  : resolve(repoRoot, "src-tauri/icons/icon.png");
const roundedAppPath = resolve(repoRoot, "src-tauri/icons/icon-rounded-source.png");
const trayPngPath = resolve(repoRoot, "src-tauri/icons/tray-template.png");
const trayRgbaPath = resolve(repoRoot, "src-tauri/icons/tray-template.rgba");

const appIcon = readPng(appSource);
roundAlpha(appIcon);
writePng(roundedAppPath, appIcon);

const trayIcon = resizeAlphaContent(readPng(trayPngPath), 32, 30);
writePng(trayPngPath, trayIcon);
writeRgba(trayRgbaPath, trayIcon);

console.log(`wrote ${roundedAppPath}`);
console.log(`updated ${trayPngPath}`);
console.log(`updated ${trayRgbaPath}`);
