// Run from a temporary folder with @resvg/resvg-js installed:
// NODE_PATH=/path/to/node_modules node /path/to/render-hero.cjs
// PowerShell: $env:NODE_PATH='C:\path\to\node_modules'; node .\render-hero.cjs
const fs = require('node:fs');
const path = require('node:path');
const { Resvg } = require('@resvg/resvg-js');

// Inline the local layer only in memory; keep the editable SVG small on disk.
const subject = fs.readFileSync(path.join(__dirname, 'hero-subject.png'));
const svg = fs.readFileSync(path.join(__dirname, 'hero-layout.svg'), 'utf8')
  .replace('xlink:href="hero-subject.png"', `xlink:href="data:image/png;base64,${subject.toString('base64')}"`);
const renderer = new Resvg(svg, {
  fitTo: { mode: 'width', value: 1200 },
  font: { loadSystemFonts: true, sansSerifFamily: 'Segoe UI', monospaceFamily: 'Consolas' },
});
const destination = path.resolve(__dirname, '..', 'hero.png');
fs.writeFileSync(destination, renderer.render().asPng());
console.log(`Rendered ${destination}`);
