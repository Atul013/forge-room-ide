const fs = require('node:fs');
const path = require('node:path');
const { Resvg } = require('../../../artifacts/build-tools/icon-renderer/node_modules/@resvg/resvg-js');
const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(root, 'forge-room-layered.svg'), 'utf8');
const fontFiles = ['segoeui.ttf', 'seguisb.ttf', 'segoeuib.ttf', 'consola.ttf', 'consolab.ttf', 'seguisym.ttf'].map(f => path.join('C:/Windows/Fonts', f));
const image = new Resvg(source, { font: { loadSystemFonts: false, fontFiles }, fitTo: { mode: 'original' } }).render();
fs.writeFileSync(path.join(root, 'redraw.png'), image.asPng());
console.log(`Rendered layered SVG: ${image.width} x ${image.height}. Fonts resolved locally; none embedded.`);
