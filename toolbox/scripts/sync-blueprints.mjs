import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import { readFile } from 'node:fs/promises';
import { syncBlueprints } from './blueprints.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
try {
  const catalog = JSON.parse(await readFile(resolve(root, 'toolbox/catalog/services.json'), 'utf8'));
  const manifest = await syncBlueprints({ repoRoot: root, outDir: resolve(root, 'toolbox/src-tauri/resources/blueprints'), catalog });
  console.log(`Synced ${manifest.files.length} tracked assets; no local configuration or runtime data copied.`);
} catch (error) { console.error(`Blueprint sync failed: ${error.message}`); process.exitCode = 1; }
