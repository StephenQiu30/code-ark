import test from 'node:test';
import assert from 'node:assert/strict';
import {validateProducer} from './producer.mjs';
const locked={sourceCommit:'a'.repeat(40),sourceUrl:'https://github.com/minio/minio/archive/test.tar.gz',sourceSha256:'b'.repeat(64),platforms:{'linux/arm64':{builder:`golang@sha256:${'c'.repeat(64)}`,runtime:`debian@sha256:${'d'.repeat(64)}`}}};
test('native image producer rejects emulation and unpinned builders',()=>{assert.throws(()=>validateProducer(locked,'linux/arm64',{OSType:'linux',Architecture:'x86_64'}),/NativeBuilderRequired/);assert.equal(validateProducer(locked,'linux/arm64',{OSType:'linux',Architecture:'aarch64'}),true);assert.throws(()=>validateProducer({...locked,sourceSha256:undefined},'linux/arm64',{OSType:'linux',Architecture:'arm64'}),/SourceNotLocked/);});
test('producer accepts only public upstream sources without URL credentials',()=>{for(const sourceUrl of ['https://example.com/source.tar','https://user:password@github.com/minio/minio/source.tar'])assert.throws(()=>validateProducer({...locked,sourceUrl},'linux/arm64',{OSType:'linux',Architecture:'arm64'}),/UnsafeSource/);});
