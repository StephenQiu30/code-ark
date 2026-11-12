import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, lstat, readdir, rename, rm } from 'node:fs/promises';
import { resolve, relative, dirname, join, isAbsolute, sep } from 'node:path';
import { parseDocument } from 'yaml';

export const sha256 = value => createHash('sha256').update(value).digest('hex');
const TARGETS = { 'mac-arm64': 'linux/arm64', 'mac-x64': 'linux/amd64', 'win-x64': 'linux/amd64', 'linux-x64': 'linux/amd64' };
const forbidden = p => p.split('/').some(x => ['.env', 'data', 'logs', 'mysql-data', 'pgsql-data', 'rabbitmq-data', '.git', '.runtime', '.minio.sys'].includes(x) || (x.startsWith('.env.') && x !== '.env.example')) || /\.(?:log|bak|pem|key)$/i.test(p);
function safeRelative(path) {
  if (typeof path !== 'string' || !path || isAbsolute(path) || path.includes('\\') || path.split('/').some(x => ['', '.', '..'].includes(x))) throw new Error('UnsafePath');
  return path;
}
async function noLinks(root, path) {
  let cursor = root;
  const rootStat = await lstat(root);
  if (rootStat.isSymbolicLink()) throw new Error('UnsafePath');
  for (const part of path.split('/')) {
    cursor = join(cursor, part);
    const stat = await lstat(cursor);
    if (stat.isSymbolicLink()) throw new Error('UnsafePath');
  }
  return cursor;
}
function checkCatalog(catalog) {
  if (catalog?.schemaVersion !== 1 || !Array.isArray(catalog.services) || !Array.isArray(catalog.scenes) || !Array.isArray(catalog.guides)) throw new Error('InvalidCatalog');
  const ids = new Set();
  for (const service of [...catalog.services, ...catalog.guides]) {
    if (!/^[a-z][a-z0-9-]*$/.test(service.id) || ids.has(service.id)) throw new Error('InvalidCatalog');
    ids.add(service.id);
    safeRelative(service.sourceDir);
    if (!service.sourceDir.startsWith('services/') || service.sourceDir.split('/').length !== 2 || !Array.isArray(service.assets)) throw new Error('InvalidCatalog');
    for (const p of service.assets) { safeRelative(p); if (forbidden(p)) throw new Error('ForbiddenAsset'); }
    for (const p of service.dataPaths ?? []) safeRelative(p);
  }
  for (const scene of catalog.scenes) {
    if (!/^[a-z][a-z0-9-]*$/.test(scene.id) || ids.has(scene.id) || !Array.isArray(scene.services) || scene.services.some(id => !catalog.services.some(s => s.id === id))) throw new Error('InvalidCatalog');
    ids.add(scene.id);
  }
}

export async function syncBlueprints({ repoRoot, outDir, catalog }) {
  checkCatalog(catalog);
  repoRoot = resolve(repoRoot); outDir = resolve(outDir);
  // Build output must never alias a source directory or the repository root.
  if (outDir === repoRoot || relative(repoRoot, outDir).startsWith('..') || !relative(repoRoot, outDir) || [...catalog.services, ...catalog.guides].some(s => outDir.startsWith(resolve(repoRoot, s.sourceDir) + sep))) throw new Error('UnsafePath');
  const tracked = new Set(execFileSync('git', ['-C', repoRoot, 'ls-files', '-z'], { encoding: 'utf8' }).split('\0'));
  let sourceCommit;
  try { sourceCommit = execFileSync('git', ['-C', repoRoot, 'rev-parse', 'HEAD'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); } catch { sourceCommit = 'uncommitted'; }
  const files = [], pending = [];
  for (const service of [...catalog.services, ...catalog.guides]) {
    for (const asset of service.assets) {
      const source = `${service.sourceDir}/${asset}`;
      if (!tracked.has(source)) throw new Error('UntrackedAsset');
      const path = await noLinks(repoRoot, source);
      if (!(await lstat(path)).isFile()) throw new Error('UnsafePath');
      const raw = await readFile(path);
      const bytes = asset.endsWith('.sh') ? Buffer.from(raw.toString('utf8').replace(/\r\n/g, '\n')) : raw;
      files.push({ path: `${service.id}/${asset}`, sha256: sha256(bytes), sourceSha256: sha256(raw), bytes: bytes.length });
      pending.push({ path: `${service.id}/${asset}`, bytes });
    }
  }
  const manifest = { producer: 'CodeArkBlueprintSync', schemaVersion: 1, sourceCommit, catalogDigest: sha256(JSON.stringify(catalog)), files };
  await mkdir(dirname(outDir), { recursive: true });
  // Refuse symlinked ancestors and refuse to replace non-generated directories.
  await noLinks(repoRoot, relative(repoRoot, dirname(outDir)).split(sep).join('/'));
  const temporary = `${outDir}.tmp-${process.pid}`;
  await mkdir(temporary, { recursive: false, mode: 0o700 });
  try {
    for (const { path, bytes } of pending) { await mkdir(dirname(join(temporary, path)), { recursive: true }); await writeFile(join(temporary, path), bytes); }
    await writeFile(join(temporary, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
    try {
      if ((await lstat(outDir)).isSymbolicLink()) throw new Error('UnsafePath');
      const previous = JSON.parse(await readFile(join(outDir, 'manifest.json'), 'utf8'));
      if (previous.producer !== manifest.producer) throw new Error('UnsafeOutputDirectory');
      await rm(outDir, { recursive: true });
    } catch (error) { if (error.code !== 'ENOENT') throw error; }
    await rename(temporary, outDir);
  } catch (error) { await rm(temporary, { recursive: true, force: true }); throw error; }
  return manifest;
}

async function referencedAssets(bundleDir, service, compose) {
  const errors = [];
  const check = async value => {
    if (typeof value !== 'string' || !value.startsWith('.')) return;
    const clean = value.replace(/^\.\//, '');
    if (clean === '.' || clean === '') return;
    try {
      safeRelative(clean);
      if ((service.dataPaths ?? []).some(p => clean === p || clean.startsWith(p + '/'))) return;
      await noLinks(join(bundleDir, service.id), clean);
    } catch { errors.push({ code: 'MissingAssetReference', serviceId: service.id }); }
  };
  for (const component of Object.values(compose.services ?? {})) {
    for (const volume of component.volumes ?? []) await check(typeof volume === 'string' ? volume.split(':')[0] : volume.source);
    for (const env of (Array.isArray(component.env_file) ? component.env_file : component.env_file ? [component.env_file] : [])) await check(typeof env === 'string' ? env : env.path);
    if (component.build) {
      const b = component.build;
      const context = typeof b === 'string' ? b : b.context ?? '.';
      await check(context);
      if (!context.includes('${')) await check('./' + join(context, typeof b === 'object' ? b.dockerfile ?? 'Dockerfile' : 'Dockerfile').replace(/^\.\//, ''));
    }
  }
  for (const group of [compose.configs, compose.secrets]) for (const entry of Object.values(group ?? {})) if (entry.file) await check(entry.file);
  return errors;
}
function validImage(source) {
  if (!['linux/amd64', 'linux/arm64'].includes(source.platform)) return false;
  if (source.kind === 'registry') return typeof source.registryRef === 'string' && !!source.registryRef && /^sha256:[a-f0-9]{64}$/.test(source.manifestDigest ?? '') && !['archiveUrl', 'archiveSha256', 'expectedImageId', 'localContentTag'].some(k => source[k] !== undefined);
  if (source.kind === 'release-image-archive') {
    let url;
    try { url = new URL(source.archiveUrl); } catch { return false; }
    return url.protocol === 'https:' && !url.username && !url.password && url.hostname === 'github.com' && url.pathname.startsWith('/StephenQiu30/code-ark/releases/download/') && /^[a-f0-9]{64}$/.test(source.archiveSha256 ?? '') && /^sha256:[a-f0-9]{64}$/.test(source.expectedImageId ?? '') && /^codeark\/[a-z0-9-]+:sha256-[a-f0-9]+$/.test(source.localContentTag ?? '') && !['registryRef', 'manifestDigest'].some(k => source[k] !== undefined);
  }
  return false;
}
export async function verifyBundle({ bundleDir, catalog, images, mode = 'candidate', target, evidence, candidatePath }) {
  const errors = [], pending = [];
  const add = (code, serviceId) => errors.push({ code, ...(serviceId ? { serviceId } : {}) });
  try { checkCatalog(catalog); } catch (error) { add(error.message); }
  if (!['development', 'candidate', 'release'].includes(mode)) add('InvalidMode');
  if (mode !== 'development' && !TARGETS[target]) add('UnsupportedTarget');
  const manifest = JSON.parse(await readFile(join(bundleDir, 'manifest.json'), 'utf8'));
  if (manifest.schemaVersion !== 1 || manifest.catalogDigest !== sha256(JSON.stringify(catalog))) add('ManifestMismatch');
  const listed = new Map(manifest.files.map(f => [f.path, f]));
  for (const service of [...catalog.services, ...catalog.guides]) {
    for (const asset of service.assets) {
      const path = `${service.id}/${asset}`;
      try {
        const file = await noLinks(bundleDir, path); const bytes = await readFile(file);
        if (!listed.has(path) || sha256(bytes) !== listed.get(path).sha256) add('AssetHashMismatch', service.id);
        if (asset.endsWith('.sh') && bytes.includes(13)) add('ScriptLineEndings', service.id);
        if (asset === 'docker-compose.yml') {
          const doc = parseDocument(bytes.toString('utf8'), { maxAliasCount: 100, uniqueKeys: true });
          if (doc.errors.length) add('InvalidCompose', service.id);
          else errors.push(...await referencedAssets(bundleDir, service, doc.toJS({ maxAliasCount: 100 })));
        }
      } catch { add('MissingAsset', service.id); }
    }
  }
  async function walk(dir, prefix = '') {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const path = prefix + entry.name;
      if (entry.isSymbolicLink() || forbidden(path)) add('ForbiddenAsset');
      else if (entry.isDirectory()) await walk(join(dir, entry.name), path + '/');
      else if (path !== 'manifest.json' && !listed.has(path)) add('UnexpectedAsset');
    }
  }
  await walk(bundleDir);
  if (images?.schemaVersion !== 1 || !Array.isArray(images.sources)) add('InvalidImageLock');
  const platform = TARGETS[target];
  const refs = new Set(catalog.services.flatMap(s => s.imageRefs ?? []));
  for (const ref of refs) {
    const candidates = images.sources.filter(s => s.sourceId === ref && (!platform || s.platform === platform));
    if (!candidates.length) { if (mode === 'development') pending.push(ref); else add('UnresolvedImageArtifact'); continue; }
    const seen = new Set();
    for (const source of candidates) {
      if (seen.has(source.platform)) { add('DuplicateImageArtifact'); continue; } seen.add(source.platform);
      if (source.status === 'pending' && mode === 'development') { pending.push(`${ref}:${source.platform}`); continue; }
      if (source.status !== 'resolved' || !validImage(source)) add('UnresolvedImageArtifact');
    }
  }
  if (mode === 'release') {
    if (!evidence || !candidatePath) add('AcceptanceEvidenceRequired');
    else {
      const bytes = await readFile(candidatePath);
      const matches = evidence.targetId === target && evidence.sourceCommit === manifest.sourceCommit && evidence.catalogDigest === manifest.catalogDigest && evidence.imageLockDigest === sha256(JSON.stringify(images)) && evidence.candidateSha256 === sha256(bytes) && evidence.status === 'passed' && evidence.services?.length === catalog.services.length && catalog.services.every(s => evidence.services.some(x => x.id === s.id && x.status === 'passed')) && evidence.scenarios?.length === catalog.scenes.length && evidence.scenarios.every(x => x.status === 'passed') && evidence.installer?.status === 'passed';
      if (!matches) add('AcceptanceEvidenceMismatch');
    }
  }
  return { ok: errors.length === 0, mode, target: target ?? null, errors, pending, sourceCommit: manifest.sourceCommit };
}
