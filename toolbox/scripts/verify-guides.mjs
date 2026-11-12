import { readFile, realpath, lstat, readdir } from "node:fs/promises";
import { resolve, relative, isAbsolute, dirname, extname } from "node:path";
import { createHash } from "node:crypto";
const args = process.argv.slice(2);
const option = (name) => {
  const index = args.indexOf(name);
  return index < 0 ? undefined : args[index + 1];
};
let root = resolve(option("--root") ?? resolve(import.meta.dirname, ".."));
const mode = option("--mode") ?? "candidate";
const issues = [];
const add = (code, context) => issues.push({ code, context });
const hash = (value) => createHash("sha256").update(value).digest("hex");
const safePath = (value) =>
  typeof value === "string" &&
  value.length > 0 &&
  !isAbsolute(value) &&
  !value.includes("\\") &&
  !value.split("/").includes("..") &&
  !/(^|\/)\.env($|\.)|(^|\/)(data|target|node_modules|\.git)\//.test(value);
const blueprintAliases = new Set([
  "blueprints/mediacrawler/hotkey-safe.patch",
  "blueprints/mediacrawler/hotkey-safety-protocol.patch",
]);
const sourcePath = (name) =>
  resolve(
    root,
    name.startsWith("blueprints/") ? "src-tauri/resources/" + name : name,
  );
try {
  root = await realpath(root);
  if (!["candidate", "release"].includes(mode)) throw Error("InvalidMode");
  const manifestBytes = await readFile(resolve(root, "guides/manifest.json"));
  const manifest = JSON.parse(manifestBytes);
  if (
    manifest.schemaVersion !== 1 ||
    !Array.isArray(manifest.guides) ||
    !Array.isArray(manifest.sceneIds)
  )
    throw Error("InvalidManifest");
  let sceneIds = manifest.sceneIds;
  try {
    const catalog = JSON.parse(
      await readFile(resolve(root, "catalog/services.json"), "utf8"),
    );
    sceneIds = catalog.scenes.map((scene) => scene.id);
    if (
      JSON.stringify([...sceneIds].sort()) !==
      JSON.stringify([...manifest.sceneIds].sort())
    )
      add("SceneCatalogDrift", "manifest.sceneIds");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const content = new Map();
  const ids = new Set();
  for (const guide of manifest.guides) {
    if (!guide.guideId || ids.has(guide.guideId))
      add("DuplicateGuideId", guide.guideId);
    ids.add(guide.guideId);
    for (const key of [
      "title",
      "clientFamily",
      "invocation",
      "expectedOutput",
      "cleanup",
      "scopeNotice",
      "status",
    ])
      if (typeof guide[key] !== "string" || !guide[key].trim())
        add("MissingGuideField", `${guide.guideId}.${key}`);
    if (
      !Array.isArray(guide.requiredTools) ||
      !Array.isArray(guide.sourceFiles) ||
      !guide.sourceFiles.length
    )
      add("MissingGuideSources", guide.guideId);
    if (
      guide.clientFamily !== "offline" &&
      (!guide.lockedClientVersion ||
        /(latest|\*|\^|~|SNAPSHOT|pending|待)/i.test(
          guide.lockedClientVersion,
        ) ||
        !/\d+\.\d+\.\d+/.test(guide.lockedClientVersion))
    )
      add("UnpinnedClient", guide.guideId);
    if (guide.status === "verified")
      add("UnprovenRuntimeStatus", guide.guideId);
    if (!["pending", "source-only", "verified"].includes(guide.status))
      add("InvalidGuideStatus", guide.guideId);
    for (const source of guide.sourceFiles ?? []) {
      if (
        !safePath(source) ||
        (source.startsWith("blueprints/") && !blueprintAliases.has(source))
      ) {
        add("UnsafePath", source);
        continue;
      }
      try {
        const file = sourcePath(source);
        const stat = await lstat(file);
        const actual = await realpath(file);
        if (
          !stat.isFile() ||
          stat.isSymbolicLink() ||
          relative(root, actual).startsWith("..")
        ) {
          add("UnsafePath", source);
          continue;
        }
        const bytes = await readFile(file);
        if (source.startsWith("blueprints/")) {
          const blueprintManifest = JSON.parse(
            await readFile(
              resolve(root, "src-tauri/resources/blueprints/manifest.json"),
              "utf8",
            ),
          );
          const expected = blueprintManifest.files?.find(
            (file) => file.path === source.slice("blueprints/".length),
          )?.sha256;
          if (!expected || hash(bytes) !== expected) {
            add("BlueprintAssetMismatch", source);
            continue;
          }
        }
        content.set(source, bytes);
        new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      } catch (error) {
        add(
          error.code === "ENOENT" ? "MissingSource" : "UnreadableSource",
          source,
        );
      }
    }
  }
  for (const sceneId of sceneIds)
    if (!ids.has(sceneId)) add("MissingSceneGuide", sceneId);
  for (const [file, bytes] of content) {
    if (extname(file) !== ".md") continue;
    const text = bytes.toString("utf8");
    for (const match of text.matchAll(
      /\[[^\]]*\]\(([^)\s]+)(?:\s+"[^"]*")?\)/g,
    )) {
      const link = match[1];
      if (/^(https?:|mailto:|#)/.test(link)) continue;
      let decoded;
      try {
        decoded = decodeURIComponent(link.split("#")[0]);
      } catch {
        add("BrokenLink", `${file}:${link}`);
        continue;
      }
      const target = resolve(dirname(resolve(root, file)), decoded);
      const rel = relative(root, target).split("\\").join("/");
      if (!safePath(rel)) {
        add("UnsafePath", `${file}:${link}`);
        continue;
      }
      try {
        const info = await lstat(sourcePath(rel));
        if (!info.isFile() || info.isSymbolicLink())
          add("BrokenLink", `${file}:${link}`);
        else if (!content.has(rel)) add("UndeclaredLink", `${file}:${link}`);
      } catch {
        add("BrokenLink", `${file}:${link}`);
      }
    }
  }

  try {
    const clients = JSON.parse(
      await readFile(resolve(root, "examples/clients.lock.json"), "utf8"),
    );
    for (const [module, expected] of Object.entries(clients.modules ?? {})) {
      if (!/^[a-z][a-z0-9-]*$/.test(module)) {
        add("UnsafeClientModule", module);
        continue;
      }
      const pom = await readFile(
        resolve(root, `examples/java/${module}/pom.xml`),
        "utf8",
      );
      const actual = [...pom.matchAll(/<dependency>([\s\S]*?)<\/dependency>/g)]
        .filter(([, body]) => !/<scope>test<\/scope>/.test(body))
        .map(([, body]) => ({
          group: body.match(/<groupId>([^<]+)<\/groupId>/)?.[1],
          artifact: body.match(/<artifactId>([^<]+)<\/artifactId>/)?.[1],
          version: body.match(/<version>([^<]+)<\/version>/)?.[1],
        }))
        .filter((dependency) => dependency.group !== "io.codeark.tutorial");
      const key = (dependency) =>
        `${dependency.group}:${dependency.artifact}:${dependency.version}`;
      if (
        JSON.stringify(actual.map(key).sort()) !==
        JSON.stringify(expected.map(key).sort())
      )
        add("ClientVersionDrift", module);
    }
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  if (args.includes("--check-clients")) {
    let artifacts;
    try {
      artifacts = JSON.parse(
        await readFile(
          resolve(root, "examples/dependency-artifacts.lock.json"),
          "utf8",
        ),
      );
    } catch {
      add("MissingClientLock", "examples/dependency-artifacts.lock.json");
    }
    for (const [module, files] of Object.entries(artifacts?.artifacts ?? {})) {
      if (!/^[a-z][a-z0-9-]*$/.test(module)) {
        add("UnsafeClientModule", module);
        continue;
      }
      const directory = resolve(root, `examples/java/${module}/target`);
      for (const [name, digest] of Object.entries(files)) {
        if (
          !/^[A-Za-z0-9._-]+\.jar$/.test(name) ||
          !/^[a-f0-9]{64}$/.test(digest)
        ) {
          add("UnsafeClientArtifact", module);
          continue;
        }
        try {
          const file = resolve(directory, name);
          const stat = await lstat(file);
          if (!stat.isFile() || stat.isSymbolicLink()) {
            add("UnsafeClientArtifact", name);
            continue;
          }
          if (hash(await readFile(file)) !== digest)
            add("ClientArtifactMismatch", `${module}/${name}`);
        } catch {
          add("MissingClientArtifact", `${module}/${name}`);
        }
      }
      try {
        for (const name of await readdir(directory))
          if (
            name.endsWith(".jar") &&
            name !== "example.jar" &&
            !name.startsWith("io.codeark.tutorial.") &&
            !Object.hasOwn(files, name)
          )
            add("UnexpectedClientArtifact", `${module}/${name}`);
      } catch {}
    }
  }
  const sourceDigest = hash(
    JSON.stringify(
      [...content]
        .map(([name, bytes]) => [name, hash(bytes)])
        .sort(([a], [b]) => a.localeCompare(b)),
    ),
  );
  const manifestSha256 = hash(manifestBytes);
  let runtimeAccepted = false;
  if (mode === "release") {
    const evidencePath = option("--evidence");
    if (evidencePath) {
      const evidence = JSON.parse(await readFile(evidencePath, "utf8"));
      const candidatePath = option("--candidate");
      const target = option("--target");
      const candidateSha256 = candidatePath
        ? hash(await readFile(candidatePath))
        : null;
      runtimeAccepted =
        !!candidateSha256 &&
        ["mac-arm64", "mac-x64", "win-x64", "linux-x64"].includes(target) &&
        evidence.candidateSha256 === candidateSha256 &&
        evidence.manifestSha256 === manifestSha256 &&
        evidence.sourceDigest === sourceDigest &&
        evidence.targetId === target &&
        evidence.installer?.status === "passed" &&
        sceneIds.every((id) =>
          evidence.guides?.some(
            (result) =>
              result.guideId === id &&
              result.status === "passed" &&
              result.clientVersion ===
                manifest.guides.find((guide) => guide.guideId === id)
                  ?.lockedClientVersion,
          ),
        );
    }
    if (!runtimeAccepted)
      add(
        "RuntimeAcceptancePending",
        "Matching external SDK and installer acceptance required; source checks cannot satisfy this gate.",
      );
  }
  console.log(
    JSON.stringify(
      {
        ok: issues.length === 0,
        mode,
        sceneCount: sceneIds.length,
        guideCount: manifest.guides.length,
        manifestSha256,
        sourceDigest,
        runtimeAccepted,
        issues,
      },
      null,
      2,
    ),
  );
  if (issues.length) process.exitCode = 1;
} catch (error) {
  console.log(
    JSON.stringify(
      {
        ok: false,
        runtimeAccepted: false,
        issues: [
          {
            code:
              error.code === "ENOENT"
                ? "MissingManifest"
                : "GuideVerificationFailed",
            context: error.message,
          },
        ],
      },
      null,
      2,
    ),
  );
  process.exitCode = 1;
}
