import { fileURLToPath } from 'node:url';
import { resolve, dirname } from 'node:path';
import { readFile } from 'node:fs/promises';
import { verifyBundle } from './blueprints.mjs';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const value = key => { const i = args.indexOf(key); return i === -1 ? undefined : args[i + 1]; };
try {
  const [catalog, images, evidence] = await Promise.all([readFile(resolve(root, 'catalog/services.json'), 'utf8').then(JSON.parse), readFile(resolve(root, 'catalog/images.lock.json'), 'utf8').then(JSON.parse), value('--evidence') ? readFile(value('--evidence'), 'utf8').then(JSON.parse) : undefined]);
  const report = await verifyBundle({ bundleDir: resolve(root, 'src-tauri/resources/blueprints'), catalog, images, mode: value('--mode') ?? 'candidate', target: value('--target'), evidence, candidatePath: value('--candidate') });
  console.log(JSON.stringify(report, null, 2)); if (!report.ok) process.exitCode = 1;
} catch (error) { console.error(`Blueprint verification failed: ${error.message}`); process.exitCode = 1; }
