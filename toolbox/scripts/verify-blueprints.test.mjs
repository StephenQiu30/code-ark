import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { syncBlueprints, verifyBundle } from './blueprints.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'codeark-assets-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const dir = join(root, 'services/redis-start-local');
  await mkdir(dir, { recursive: true });
  await writeFile(join(dir, 'docker-compose.yml'), 'services:\n  redis:\n    image: redis:7\n    volumes:\n      - ./data:/data\n      - ./start.sh:/start.sh:ro\n');
  await writeFile(join(dir, 'start.sh'), '#!/bin/sh\r\necho ready\r\n');
  await writeFile(join(dir, '.env'), 'PRIVATE_SECRET=DO_NOT_COPY');
  execFileSync('git', ['init', '-q', root]);
  execFileSync('git', ['-C', root, 'add', 'services/redis-start-local/docker-compose.yml', 'services/redis-start-local/start.sh']);
  const catalog = { schemaVersion: 1, services: [{ id: 'redis', sourceDir: 'services/redis-start-local', sourceProject: 'redis-start-local', assets: ['docker-compose.yml', 'start.sh'], dataPaths: ['data'], imageRefs: ['redis'] }], scenes: [], guides: [] };
  const images = { schemaVersion: 1, sources: [{ sourceId: 'redis', kind: 'registry', platform: 'linux/arm64', registryRef: 'docker.io/library/redis', manifestDigest: `sha256:${'a'.repeat(64)}`, status: 'resolved' }] };
  return { root, dir, outDir: join(root, 'bundle'), catalog, images };
}

test('asset_closure: copy only tracked assets, normalize container scripts, retain source bytes', async t => {
  const f = await fixture(t);
  const manifest = await syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog });
  assert.equal(await readFile(join(f.outDir, 'redis/start.sh'), 'utf8'), '#!/bin/sh\necho ready\n');
  assert.equal(await readFile(join(f.dir, 'start.sh'), 'utf8'), '#!/bin/sh\r\necho ready\r\n');
  await assert.rejects(readFile(join(f.outDir, 'redis/.env')), { code: 'ENOENT' });
  assert.equal(manifest.files.length, 2);
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' })).ok, true);
});

test('reject_forbidden_assets: explicit secret, untracked file and traversal cannot be packaged', async t => {
  for (const asset of ['.env', '.env.production', '../private.txt', 'unknown.sh']) {
    const f = await fixture(t);
    f.catalog.services[0].assets.push(asset);
    await assert.rejects(syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog }), /ForbiddenAsset|UntrackedAsset|UnsafePath/);
  }
});

test('reject_forbidden_assets: symlink never reads an external file', async t => {
  const f = await fixture(t);
  await symlink('/etc/hosts', join(f.dir, 'external.sh'));
  execFileSync('git', ['-C', f.root, 'add', 'services/redis-start-local/external.sh']);
  f.catalog.services[0].assets.push('external.sh');
  await assert.rejects(syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog }), /UnsafePath/);
});

test('asset_closure: missing readonly mount and edited output fail verification', async t => {
  const f = await fixture(t);
  await syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog });
  await writeFile(join(f.outDir, 'redis/docker-compose.yml'), 'services:\n  redis:\n    volumes:\n      - ./missing.yml:/config:ro\n');
  const report = await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' });
  assert.equal(report.ok, false);
  assert.ok(report.errors.some(e => e.code === 'AssetHashMismatch'));
  assert.ok(report.errors.some(e => e.code === 'MissingAssetReference'));
});

test('staged_artifact_gates: development reports pending; candidate rejects unresolved or wrong architecture', async t => {
  const f = await fixture(t);
  await syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog });
  f.images.sources[0].status = 'pending';
  delete f.images.sources[0].manifestDigest;
  const development = await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'development' });
  assert.equal(development.ok, true);
  assert.equal(development.pending.length, 1);
  const candidate = await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' });
  assert.equal(candidate.ok, false);
  f.images.sources[0].status = 'resolved';
  f.images.sources[0].manifestDigest = `sha256:${'a'.repeat(64)}`;
  const x64 = await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'win-x64' });
  assert.equal(x64.ok, false);
});

test('candidate_requires_resolved_architecture: registry/archive fields are mutually exclusive and hashes mandatory', async t => {
  const f = await fixture(t);
  await syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog });
  f.images.sources[0].archiveUrl = 'https://github.com/example/release.tar';
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' })).ok, false);
  f.images.sources[0] = { sourceId: 'redis', kind: 'release-image-archive', platform: 'linux/arm64', status: 'resolved', archiveUrl: 'https://github.com/StephenQiu30/code-ark/releases/download/v0/test.tar', archiveSha256: 'a'.repeat(64), expectedImageId: `sha256:${'b'.repeat(64)}`, localContentTag: `codeark/redis:sha256-${'b'.repeat(64)}` };
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' })).ok, true);
  delete f.images.sources[0].expectedImageId;
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'candidate', target: 'mac-arm64' })).ok, false);
});

test('release_requires_matching_evidence: no acceptance evidence never becomes a release', async t => {
  const f = await fixture(t);
  await syncBlueprints({ repoRoot: f.root, outDir: f.outDir, catalog: f.catalog });
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'release', target: 'mac-arm64' })).ok, false);
  const forged = { targetId: 'mac-arm64', status: 'passed', sourceCommit: 'wrong', candidateSha256: 'a'.repeat(64) };
  assert.equal((await verifyBundle({ bundleDir: f.outDir, catalog: f.catalog, images: f.images, mode: 'release', target: 'mac-arm64', evidence: forged })).ok, false);
});
