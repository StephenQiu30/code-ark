import { readFile, writeFile, mkdir, copyFile, mkdtemp, rename, rm } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../..');
const sha=value=>createHash('sha256').update(value).digest('hex');
export function validateProducer(recipe,platform,daemon){
  if(!['linux/amd64','linux/arm64'].includes(platform)||daemon.OSType!=='linux')throw new Error('UnsupportedTarget');
  const arch=({aarch64:'arm64',arm64:'arm64',x86_64:'amd64',amd64:'amd64'})[daemon.Architecture];
  if(platform!==`linux/${arch}`)throw new Error('NativeBuilderRequired');
  if(!/^[a-f0-9]{40}$/.test(recipe.sourceCommit??'')||!/^https:\/\//.test(recipe.sourceUrl??'')||!/^[a-f0-9]{64}$/.test(recipe.sourceSha256??''))throw new Error('SourceNotLocked');
  const url=new URL(recipe.sourceUrl);if(url.username||url.password||!['github.com','codeload.github.com','release.infinilabs.com'].includes(url.hostname))throw new Error('UnsafeSource');
  for(const kind of ['builder','runtime'])if(!recipe.platforms?.[platform]?.[kind]?.includes('@sha256:')||!/@sha256:[a-f0-9]{64}$/.test(recipe.platforms[platform][kind]))throw new Error('BuilderNotLocked');
  return true;
}
export function verifyProducerBuildInputs(recipe,dockerfile,entrypoint){
  if(sha(dockerfile)!==recipe.recipeSha256)throw new Error('RecipeChanged');
  if(recipe.entrypointSha256&&sha(entrypoint??'')!==recipe.entrypointSha256)throw new Error('EntrypointChanged');
  return true;
}
export async function runProducerProcess(command,args,{env=process.env,timeoutMs=30*60*1000,killGraceMs=2000,maxOutputBytes=2*1024*1024}={}){
  return await new Promise((done,fail)=>{
    const child=spawn(command,args,{env,stdio:['ignore','pipe','pipe'],windowsHide:true});
    let out='',err='',failure=null,killTimer;
    const stop=reason=>{if(failure)return;failure=reason;child.kill('SIGTERM');killTimer=setTimeout(()=>child.kill('SIGKILL'),killGraceMs);};
    const timer=setTimeout(()=>stop('ProducerCommandTimeout'),timeoutMs);
    const clear=()=>{clearTimeout(timer);clearTimeout(killTimer);};
    child.stdout.on('data',b=>{if(failure)return;out+=b;if(Buffer.byteLength(out)>maxOutputBytes){out='';stop('ProducerOutputLimit');}});
    child.stderr.on('data',b=>{err=(err+b).slice(-16384);});
    child.on('error',error=>{clear();fail(error);});
    child.on('close',code=>{clear();failure?fail(new Error(`${failure}: ${err.slice(-4000)}`)):code===0?done(out):fail(new Error(`ProducerCommandFailed (${code}): ${err.slice(-4000)}`));});
  });
}
async function cli(context,args){
  const env={...process.env};for(const key of Object.keys(env))if(key.startsWith('COMPOSE_')||(key.startsWith('DOCKER_')&&key!=='DOCKER_CONFIG'))delete env[key];
  return runProducerProcess('docker',['--context',context,...args],{env});
}
export async function downloadProducerSource(url,recipe,{timeoutMs=120000}={}){
  const signal=AbortSignal.timeout(timeoutMs);
  try{
    const response=await fetch(url,{signal});if(!response.ok)throw new Error(`SourceDownloadFailed (${response.status})`);
    const chunks=[];let size=0;
    for await(const chunk of response.body){size+=chunk.length;if(size>recipe.sourceBytes)throw new Error('SourceSizeMismatch');chunks.push(chunk);}
    if(size!==recipe.sourceBytes)throw new Error('SourceSizeMismatch');
    const bytes=Buffer.concat(chunks);if(sha(bytes)!==recipe.sourceSha256)throw new Error('SourceHashMismatch');return bytes;
  }catch(error){if(signal.aborted)throw new Error('SourceDownloadTimeout');throw error;}
}
export async function saveProducerArchive(archive,save){
  const temporary=await mkdtemp(join(dirname(archive),'.codeark-export-'));
  const partial=join(temporary,'image.tar');
  try{
    await save(partial);
    const hash=createHash('sha256');for await(const chunk of createReadStream(partial))hash.update(chunk);
    const digest=hash.digest('hex');await rename(partial,archive);return digest;
  }finally{await rm(temporary,{recursive:true,force:true});}
}
async function main(){
  const args=process.argv.slice(2);const arg=n=>args[args.indexOf(n)+1];
  if(!args.includes('--recipe')||!args.includes('--context')||!args.includes('--expected-daemon')||!args.includes('--platform'))throw new Error('Explicit recipe/context/daemon/platform required');
  const id=arg('--recipe');if(!['minio','xxjob','es-ik'].includes(id))throw new Error('UnknownRecipe');
  const context=arg('--context'),platform=arg('--platform');const daemon=JSON.parse(await cli(context,['info','--format','{{json .}}']));
  if(daemon.ID!==arg('--expected-daemon'))throw new Error('TargetChanged');
  const recipe=JSON.parse(await readFile(join(root,`toolbox/image-builds/${id}/recipe.json`),'utf8'));
  validateProducer(recipe,platform,daemon);
  const build=await mkdtemp(join(tmpdir(),'codeark-producer-'));
  if(id==='es-ik'){
    const source=await readFile(join(root,'services/elastic-start-local/elasticsearch/Dockerfile'),'utf8');
    verifyProducerBuildInputs(recipe,source);
    const bytes=await downloadProducerSource(recipe.sourceUrl,recipe);
    await writeFile(join(build,'analysis-ik.zip'),bytes);
    const adapted=source.replace('FROM docker.elastic.co/elasticsearch/elasticsearch:${ES_VERSION}',`FROM ${recipe.platforms[platform].runtime}`).replace('https://release.infinilabs.com/analysis-ik/stable/elasticsearch-analysis-ik-${IK_VERSION}.zip','file:///tmp/analysis-ik.zip').replace('ARG IK_VERSION=9.3.0','COPY analysis-ik.zip /tmp/analysis-ik.zip\nARG IK_VERSION=9.3.0');
    await writeFile(join(build,'Dockerfile'),adapted);
  }else{
    const dockerfile=await readFile(join(root,`toolbox/image-builds/${id}/Dockerfile`),'utf8');
    const entrypoint=id==='xxjob'?await readFile(join(root,'toolbox/image-builds/xxjob/entrypoint.sh'),'utf8'):undefined;
    verifyProducerBuildInputs(recipe,dockerfile,entrypoint);
    await writeFile(join(build,'Dockerfile'),dockerfile);
    if(id==='xxjob')await copyFile(join(root,'toolbox/image-builds/xxjob/entrypoint.sh'),join(build,'entrypoint.sh'));
  }
  const tag=`codeark/${id}:producer-${recipe.sourceCommit.slice(0,12)}-${platform.split('/')[1]}`;
  if(JSON.parse(await cli(context,['info','--format','{{json .}}'])).ID!==daemon.ID)throw new Error('TargetChanged');
  await cli(context,['build','--platform',platform,'--build-arg',`BUILDER_IMAGE=${recipe.platforms[platform].builder}`,'--build-arg',`RUNTIME_IMAGE=${recipe.platforms[platform].runtime}`,'--build-arg',`SOURCE_URL=${recipe.sourceUrl}`,'--build-arg',`SOURCE_SHA256=${recipe.sourceSha256}`,'--tag',tag,build]);
  const info=JSON.parse(await cli(context,['image','inspect',tag]))[0];if(info.Os!=='linux'||`linux/${info.Architecture}`!==platform)throw new Error('ImageArchitectureMismatch');
  const contentTag=`codeark/${id}:sha256-${info.Id.replace('sha256:','')}`;await cli(context,['image','tag',info.Id,contentTag]);
  const output=resolve(root,'.tools/artifacts');await mkdir(output,{recursive:true});const archive=join(output,`${id}-${platform.split('/')[1]}.tar`);
  const archiveSha256=await saveProducerArchive(archive,partial=>cli(context,['image','save','--output',partial,contentTag]));
  const receipt={sourceId:id,kind:'release-image-archive',platform,sourceCommit:recipe.sourceCommit,sourceSha256:recipe.sourceSha256,recipeSha256:recipe.recipeSha256,builder:recipe.platforms[platform].builder,runtime:recipe.platforms[platform].runtime,nativeDaemon:{id:daemon.ID,os:daemon.OSType,architecture:daemon.Architecture},expectedImageId:info.Id,imageDescriptor:info.Descriptor??null,identityFormat:'docker-inspect-id',localContentTag:contentTag,archiveSha256,archivePath:archive,functionalAcceptance:'pending',archiveUrl:null};
  await writeFile(archive+'.json',JSON.stringify(receipt,null,2)+'\n');
  console.log('Image archive produced. Functional acceptance, license review and a public immutable URL are still required; image lock not auto-approved.');
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(e=>{console.error(e.message);process.exitCode=1;});
