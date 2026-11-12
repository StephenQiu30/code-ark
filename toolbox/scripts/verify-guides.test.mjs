import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
const script = resolve(import.meta.dirname, "verify-guides.mjs");
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), "codeark-guides-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, "guides"));
  await mkdir(join(root, "examples"));
  await writeFile(
    join(root, "guides/start.md"),
    "# 教程\n\n[示例](../examples/Main.java)\n",
  );
  await writeFile(join(root, "examples/Main.java"), "class Main {}\n");
  const guide = {
    guideId: "backend-mysql",
    serviceId: "mysql",
    title: "SQL 教程",
    clientFamily: "JDBC",
    lockedClientVersion: "26.10.0",
    requiredTools: ["JDK17"],
    sourceFiles: ["guides/start.md", "examples/Main.java"],
    invocation: "java Main",
    expectedOutput: "QUERY_OK",
    cleanup: "仅删除 codeark_tutorial_ 测试记录",
    scopeNotice: "仅教学scope，不操作业务数据",
    status: "pending",
  };
  return {
    root,
    manifest: {
      schemaVersion: 1,
      sceneIds: ["backend-mysql"],
      guides: [guide],
    },
  };
}
async function runFixture(data, mode = "candidate") {
  await writeFile(
    join(data.root, "guides/manifest.json"),
    JSON.stringify(data.manifest),
  );
  return spawnSync(
    process.execPath,
    [script, "--root", data.root, "--mode", mode],
    { encoding: "utf8" },
  );
}
test("candidate accepts complete source resources while retaining pending runtime status", async (t) => {
  const data = await fixture(t);
  const result = await runFixture(data);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.runtimeAccepted, false);
});
test("rejects missing exact client versions and missing source files", async (t) => {
  const data = await fixture(t);
  data.manifest.guides[0].lockedClientVersion = "latest";
  data.manifest.guides[0].sourceFiles.push("examples/missing.java");
  const result = await runFixture(data);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /UnpinnedClient/);
  assert.match(result.stdout, /MissingSource/);
});
test("rejects path traversal and package-local broken Markdown links", async (t) => {
  const data = await fixture(t);
  data.manifest.guides[0].sourceFiles.push("../secret");
  await writeFile(join(data.root, "guides/start.md"), "[broken](missing.md)");
  const result = await runFixture(data);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /UnsafePath/);
  assert.match(result.stdout, /BrokenLink/);
});
test("rejects missing scene coverage and false verified claims without evidence", async (t) => {
  const data = await fixture(t);
  data.manifest.sceneIds.push("workflow-temporal");
  data.manifest.guides[0].status = "verified";
  const result = await runFixture(data);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /MissingSceneGuide/);
  assert.match(result.stdout, /UnprovenRuntimeStatus/);
});
test("release refuses pending SDK runtime and installer acceptance", async (t) => {
  const data = await fixture(t);
  const result = await runFixture(data, "release");
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /RuntimeAcceptancePending/);
});
test("bundled resources cover every catalog scene and keep unexecuted examples pending", () => {
  const result = spawnSync(process.execPath, [script, "--mode", "candidate"], {
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stdout + result.stderr);
  const report = JSON.parse(result.stdout);
  assert.equal(report.sceneCount, 7);
  assert.equal(report.runtimeAccepted, false);
});
test("release never accepts an evidence blob without a target and actual candidate bytes", async (t) => {
  const data = await fixture(t);
  const candidate = await runFixture(data);
  const report = JSON.parse(candidate.stdout);
  const evidence = {
    manifestSha256: report.manifestSha256,
    sourceDigest: report.sourceDigest,
    installer: { status: "passed" },
    guides: [
      { guideId: "backend-mysql", status: "passed", clientVersion: "26.10.0" },
    ],
  };
  const path = join(data.root, "evidence.json");
  await writeFile(path, JSON.stringify(evidence));
  const result = spawnSync(
    process.execPath,
    [script, "--root", data.root, "--mode", "release", "--evidence", path],
    { encoding: "utf8" },
  );
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /RuntimeAcceptancePending/);
});
test("optional client artifact gate rejects missing and mismatched downloaded SDK jars", async (t) => {
  const data = await fixture(t);
  await writeFile(
    join(data.root, "examples/dependency-artifacts.lock.json"),
    JSON.stringify({
      schemaVersion: 1,
      artifacts: { "sql-mysql": { "driver.jar": "0".repeat(64) } },
    }),
  );
  await runFixture(data);
  const run = () =>
    spawnSync(
      process.execPath,
      [script, "--root", data.root, "--check-clients"],
      { encoding: "utf8" },
    );
  const missing = run();
  assert.notEqual(missing.status, 0);
  assert.match(missing.stdout, /MissingClientArtifact/);
  await mkdir(join(data.root, "examples/java/sql-mysql/target"), {
    recursive: true,
  });
  await writeFile(
    join(data.root, "examples/java/sql-mysql/target/driver.jar"),
    "changed",
  );
  const mismatch = run();
  assert.notEqual(mismatch.status, 0);
  assert.match(mismatch.stdout, /ClientArtifactMismatch/);
});
test("rejects a POM whose SDK version differs from the client lock", async (t) => {
  const data = await fixture(t);
  await mkdir(join(data.root, "examples/java/sql-mysql"), { recursive: true });
  await writeFile(
    join(data.root, "examples/clients.lock.json"),
    JSON.stringify({
      schemaVersion: 1,
      modules: {
        "sql-mysql": [
          {
            group: "com.mysql",
            artifact: "mysql-connector-j",
            version: "26.10.0",
          },
        ],
      },
    }),
  );
  await writeFile(
    join(data.root, "examples/java/sql-mysql/pom.xml"),
    "<project><dependencies><dependency><groupId>com.mysql</groupId><artifactId>mysql-connector-j</artifactId><version>9.0.0</version></dependency></dependencies></project>",
  );
  const result = await runFixture(data);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /ClientVersionDrift/);
});
test("bundles only the two MediaCrawler blueprint aliases and detects changed patch bytes", async (t) => {
  const data = await fixture(t);
  const { createHash } = await import("node:crypto");
  const path = "blueprints/mediacrawler/hotkey-safe.patch";
  const folder = join(data.root, "src-tauri/resources/blueprints");
  await mkdir(join(folder, "mediacrawler"), { recursive: true });
  await writeFile(
    join(folder, "mediacrawler/hotkey-safe.patch"),
    "reviewed patch\n",
  );
  await writeFile(
    join(folder, "manifest.json"),
    JSON.stringify({
      files: [
        {
          path: "mediacrawler/hotkey-safe.patch",
          sha256: createHash("sha256").update("reviewed patch\n").digest("hex"),
        },
      ],
    }),
  );
  data.manifest.guides[0].sourceFiles.push(path);
  await writeFile(
    join(data.root, "guides/start.md"),
    "[patch](../blueprints/mediacrawler/hotkey-safe.patch)",
  );
  const valid = await runFixture(data);
  assert.equal(valid.status, 0, valid.stdout);
  await writeFile(join(folder, "mediacrawler/hotkey-safe.patch"), "modified");
  const changed = await runFixture(data);
  assert.notEqual(changed.status, 0);
  assert.match(changed.stdout, /BlueprintAssetMismatch/);
});
test("rejects a local link that exists in checkout but is absent from the exported source union", async (t) => {
  const data = await fixture(t);
  await writeFile(
    join(data.root, "examples/not-exported.md"),
    "private to checkout",
  );
  await writeFile(
    join(data.root, "guides/start.md"),
    "[missing from bundle](../examples/not-exported.md)",
  );
  const result = await runFixture(data);
  assert.notEqual(result.status, 0);
  assert.match(result.stdout, /UndeclaredLink/);
});
