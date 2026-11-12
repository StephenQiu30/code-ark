import { readFile, writeFile, mkdir, copyFile, mkdtemp } from 'node:fs/promises';
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
async function cli(context,args){
  const env={...process.env};for(const key of Object.keys(env))if(key.startsWith('COMPOSE_')||(key.startsWith('DOCKER_')&&key!=='DOCKER_CONFIG'))delete env[key];
  return await new Promise((done,fail)=>{const child=spawn('docker',['--context',context,...args],{env,stdio:['ignore','pipe','pipe'],windowsHide:true});const timer=setTimeout(()=>child.kill(),30*60*1000);let out='',err='';child.stdout.on('data',b=>{out+=b;if(out.length>2*1024*1024)child.kill();});child.stderr.on('data',b=>{err=(err+b).slice(-16384);});child.on('error',error=>{clearTimeout(timer);fail(error);});child.on('close',code=>{clearTimeout(timer);code===0?done(out):fail(new Error(`ProducerDockerFailed (${code}): ${err.slice(-4000)}`));});});
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
    if(sha(source)!==recipe.recipeSha256)throw new Error('RecipeChanged');
    const response=await fetch(recipe.sourceUrl);if(!response.ok)throw new Error('SourceDownloadFailed');
    const bytes=Buffer.from(await response.arrayBuffer());if(sha(bytes)!==recipe.sourceSha256)throw new Error('SourceHashMismatch');
    await writeFile(join(build,'analysis-ik.zip'),bytes);
    const adapted=source.replace('FROM docker.elastic.co/elasticsearch/elasticsearch:${ES_VERSION}',`FROM ${recipe.platforms[platform].runtime}`).replace('https://release.infinilabs.com/analysis-ik/stable/elasticsearch-analysis-ik-${IK_VERSION}.zip','file:///tmp/analysis-ik.zip').replace('ARG IK_VERSION=9.3.0','COPY analysis-ik.zip /tmp/analysis-ik.zip\nARG IK_VERSION=9.3.0');
    await writeFile(join(build,'Dockerfile'),adapted);
  }else{
    await copyFile(join(root,`toolbox/image-builds/${id}/Dockerfile`),join(build,'Dockerfile'));
    if(id==='xxjob')await copyFile(join(root,'toolbox/image-builds/xxjob/entrypoint.sh'),join(build,'entrypoint.sh'));
  }
  const tag=`codeark/${id}:producer-${recipe.sourceCommit.slice(0,12)}-${platform.split('/')[1]}`;
  if(JSON.parse(await cli(context,['info','--format','{{json .}}'])).ID!==daemon.ID)throw new Error('TargetChanged');
  await cli(context,['build','--platform',platform,'--build-arg',`BUILDER_IMAGE=${recipe.platforms[platform].builder}`,'--build-arg',`RUNTIME_IMAGE=${recipe.platforms[platform].runtime}`,'--build-arg',`SOURCE_URL=${recipe.sourceUrl}`,'--build-arg',`SOURCE_SHA256=${recipe.sourceSha256}`,'--tag',tag,build]);
  const info=JSON.parse(await cli(context,['image','inspect',tag]))[0];if(`linux/${info.Architecture}`!==platform)throw new Error('ImageArchitectureMismatch');
  const contentTag=`codeark/${id}:sha256-${info.Id.replace('sha256:','')}`;await cli(context,['image','tag',info.Id,contentTag]);
  const output=resolve(root,'.tools/artifacts');await mkdir(output,{recursive:true});const archive=join(output,`${id}-${platform.split('/')[1]}.tar`);
  await cli(context,['image','save','--output',archive,contentTag]);
  const receipt={sourceId:id,kind:'release-image-archive',platform,sourceCommit:recipe.sourceCommit,sourceSha256:recipe.sourceSha256,expectedImageId:info.Id,localContentTag:contentTag,archiveSha256:await (async()=>{const h=createHash('sha256');for await(const chunk of createReadStream(archive))h.update(chunk);return h.digest('hex');})(),archivePath:archive,functionalAcceptance:'pending',archiveUrl:null};
  await writeFile(archive+'.json',JSON.stringify(receipt,null,2)+'\n');
  console.log('Image archive produced. Functional acceptance, license review and a public immutable URL are still required; image lock not auto-approved.');
}
if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url))main().catch(e=>{console.error(e.message);process.exitCode=1;});
