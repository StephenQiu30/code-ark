import test from 'node:test';
import assert from 'node:assert/strict';
import {validateProducer} from './producer.mjs';
const locked={sourceCommit:'a'.repeat(40),sourceUrl:'https://github.com/minio/minio/archive/test.tar.gz',sourceSha256:'b'.repeat(64),platforms:{'linux/arm64':{builder:`golang@sha256:${'c'.repeat(64)}`,runtime:`debian@sha256:${'d'.repeat(64)}`}}};
test('native image producer rejects emulation and unpinned builders',()=>{assert.throws(()=>validateProducer(locked,'linux/arm64',{OSType:'linux',Architecture:'x86_64'}),/NativeBuilderRequired/);assert.equal(validateProducer(locked,'linux/arm64',{OSType:'linux',Architecture:'aarch64'}),true);assert.throws(()=>validateProducer({...locked,sourceSha256:undefined},'linux/arm64',{OSType:'linux',Architecture:'arm64'}),/SourceNotLocked/);});
test('producer accepts only public upstream sources without URL credentials',()=>{for(const sourceUrl of ['https://example.com/source.tar','https://user:password@github.com/minio/minio/source.tar'])assert.throws(()=>validateProducer({...locked,sourceUrl},'linux/arm64',{OSType:'linux',Architecture:'arm64'}),/UnsafeSource/);});

// Missing termination escalation would leave a SIGTERM-resistant builder running forever.
test('producer bounds a child which ignores SIGTERM and reports timeout', async()=>{
  const {runProducerProcess}=await import('./producer.mjs');
  const started=Date.now();
  await assert.rejects(runProducerProcess(process.execPath,['-e',"process.on('SIGTERM',()=>{});setInterval(()=>{},1000)"],{timeoutMs:150,killGraceMs:50}),/ProducerCommandTimeout/);
  assert.ok(Date.now()-started<2000);
});
// A verbose child must fail truthfully rather than report a generic Docker cancellation.
test('producer bounds command stdout and preserves command failure evidence',async()=>{
  const {runProducerProcess}=await import('./producer.mjs');
  await assert.rejects(runProducerProcess(process.execPath,['-e',"process.stdout.write('x'.repeat(10000));setInterval(()=>{},1000)"],{maxOutputBytes:512,timeoutMs:2000,killGraceMs:50}),/ProducerOutputLimit/);
  await assert.rejects(runProducerProcess(process.execPath,['-e',"process.stderr.write('upstream EOF');process.exit(7)"]),/ProducerCommandFailed \(7\): upstream EOF/);
});

test('producer source download rejects oversized, corrupt and stalled responses',async()=>{
  const {createServer}=await import('node:http');
  const {downloadProducerSource}=await import('./producer.mjs');
  const server=createServer((req,res)=>{if(req.url==='/stall')return;res.end(req.url==='/large'?'abcd':'abc');});
  await new Promise(done=>server.listen(0,'127.0.0.1',done));
  const url=`http://127.0.0.1:${server.address().port}`;
  const recipe={sourceBytes:3,sourceSha256:'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'};
  try{
    assert.equal((await downloadProducerSource(url,recipe)).toString(),'abc');
    await assert.rejects(downloadProducerSource(url+'/large',recipe),/SourceSizeMismatch/);
    await assert.rejects(downloadProducerSource(url,{...recipe,sourceSha256:'0'.repeat(64)}),/SourceHashMismatch/);
    await assert.rejects(downloadProducerSource(url+'/stall',recipe,{timeoutMs:50}),/SourceDownloadTimeout/);
  }finally{server.closeAllConnections();await new Promise(done=>server.close(done));}
});

test('producer refuses modified Dockerfile or runtime entrypoint before building',async()=>{
  const {verifyProducerBuildInputs}=await import('./producer.mjs');
  const recipe={recipeSha256:'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad',entrypointSha256:'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'};
  assert.equal(verifyProducerBuildInputs(recipe,'abc','abc'),true);
  assert.throws(()=>verifyProducerBuildInputs(recipe,'tampered','abc'),/RecipeChanged/);
  assert.throws(()=>verifyProducerBuildInputs(recipe,'abc','tampered'),/EntrypointChanged/);
});

test('failed image export preserves the previous archive and removes partial output',async()=>{
  const {mkdtemp,writeFile,readFile,readdir,rm}=await import('node:fs/promises');
  const {tmpdir}=await import('node:os');const {join}=await import('node:path');
  const {saveProducerArchive}=await import('./producer.mjs');
  const dir=await mkdtemp(join(tmpdir(),'codeark-export-test-'));const path=join(dir,'image.tar');
  try{
    await writeFile(path,'previous');
    await assert.rejects(saveProducerArchive(path,async partial=>{await writeFile(partial,'incomplete');throw new Error('ExportDisconnected');}),/ExportDisconnected/);
    assert.equal(await readFile(path,'utf8'),'previous');assert.deepEqual(await readdir(dir),['image.tar']);
    assert.equal(await saveProducerArchive(path,partial=>writeFile(partial,'abc')),'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
    assert.equal(await readFile(path,'utf8'),'abc');assert.deepEqual(await readdir(dir),['image.tar']);
  }finally{await rm(dir,{recursive:true,force:true});}
});
