// Rasterize the delivered SVGs, not a second hand-drawn copy of their geometry.
const fs = require('node:fs');
const path = require('node:path');
const { Resvg } = require('../../../artifacts/build-tools/icon-renderer/node_modules/@resvg/resvg-js');

const root = path.resolve(__dirname, '..');
const destination = path.resolve(__dirname, '../../..', 'artifacts/build-tools/icon-renderer/renders');
fs.mkdirSync(destination, { recursive: true });
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'manifest.json'), 'utf8'));
const sprite = fs.readFileSync(path.join(root, 'forge-room-sprite.svg'), 'utf8');
const symbols = sprite.replace(/^<svg[^>]*>\s*/, '').replace(/<\/svg>\s*$/, '');

for (const icon of manifest.icons) {
  const source = fs.readFileSync(path.join(root, icon.file), 'utf8');
  const color = icon.name === 'room-active' ? '#EFEFEE' : '#A0A1A5';
  for (const size of [24, 48]) {
    const options = { fitTo: { mode: 'width', value: size }, font: { loadSystemFonts: false } };
    const standalone = source.replace('<svg ', `<svg color="${color}" `);
    const fromSprite = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" color="${color}">${symbols}<use href="#${icon.symbol}"/></svg>`;
    fs.writeFileSync(path.join(destination, `${icon.name}-${size}.png`), new Resvg(standalone, options).render().asPng());
    fs.writeFileSync(path.join(destination, `${icon.name}-${size}-sprite.png`), new Resvg(fromSprite, options).render().asPng());
  }
  const plain = source.replace('<svg ', '<svg color="#FFFFFF" ');
  fs.writeFileSync(path.join(destination, `${icon.name}-bounds.png`), new Resvg(plain, { fitTo: { mode: 'width', value: 384 }, font: { loadSystemFonts: false } }).render().asPng());
}
console.log(`Rendered ${manifest.icons.length} icons at 24 px, 48 px, and 384 px; also rendered every sprite symbol at both preview sizes.`);
