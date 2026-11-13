import { vi } from "vitest";
import type { ToolboxApi } from "./api/tauri";
import type {
  DashboardSnapshot,
  ServiceCatalog,
  OperationRecord,
  DatabaseBinding,
  BindingGrantPreview,
} from "./types/toolbox";
const names = [
  "redis",
  "mysql",
  "pgsql",
  "minio",
  "nacos",
  "sentinel",
  "seata",
  "xxl-job",
  "temporal",
  "canal",
  "kafka",
  "rabbitmq",
  "rocketmq",
  "elastic",
  "monitoring",
  "mongodb",
  "etcd",
];
export const fixtureCatalog: ServiceCatalog = {
  schemaVersion: 1,
  services: names.map((id) => ({
    id,
    name: id === "redis" ? "Redis" : id,
    category: "基础设施",
    sourceDir: id,
    sourceProject: id,
    assets: [],
    dataPaths: ["data/"],
    imageRefs: ["image:1"],
    components: [],
    ports: [],
    dependencies: [],
    envSchema: [],
    probes: [],
    minMemoryBytes: 134217728,
    minWorkspaceFreeBytes: null,
    minDaemonFreeBytes: null,
    estimatedPullBytes: null,
    capacityStatus: "Unknown",
    dataCompatibility: "initial",
  })),
  scenes: [
    "backend-mysql",
    "backend-pgsql",
    "java-microservices",
    "java-distributed",
    "workflow-temporal",
    "cdc-messaging",
    "search-observability",
  ].map((id, i) => ({
    id,
    name: i === 0 ? "Backend MySQL" : id,
    services: i === 0 ? ["mysql", "redis", "minio"] : ["redis"],
  })),
  guides: [],
};
export const fixtureSnapshot: DashboardSnapshot = {
  workspace: { workspaceId: "workspace", root: "/workspace" },
  target: {
    targetId: "target",
    cliPath: "/usr/bin/docker",
    contextName: "desktop-linux",
    endpointKind: "LocalSocket",
    daemonId: "daemon",
    osType: "linux",
    architecture: "aarch64",
  },
  services: names.map((serviceId) => ({
    serviceId,
    instanceId: serviceId === "redis" ? "redis-1" : null,
    phase: serviceId === "redis" ? "RunningUnverified" : "Uninitialized",
    ownership: "Managed",
    desiredGeneration: null,
    appliedGeneration: serviceId === "redis" ? "applied-1" : null,
    lastReadyGeneration: null,
    messageCode: null,
  })),
  operations: [],
  bindings: [],
  sampledAt: Date.now(),
  environmentError: null,
};
const fixtureOperation: OperationRecord = {
  operationId: "operation",
  targetId: "target",
  instanceIds: ["redis-1"],
  requestDigest: "test-request",
  desiredGeneration: "desired-1",
  appliedGeneration: "applied-1",
  lastReadyGeneration: null,
  phase: "Queued",
  steps: [],
  createdResourceIds: [],
  errorCode: null,
  createdAt: 1,
  seq: 1,
  allowedActions: ["cancel"],
};
const fixtureBinding: DatabaseBinding = {
  bindingId: "binding-test",
  targetId: "target",
  daemonId: "daemon",
  engine: "mysql",
  lifecycleOwner: "External",
  containerId: "container-test",
  networkName: "test-network",
  networkAlias: "test-mysql",
  containerPort: 3306,
  hostEndpoint: null,
  credentialRef: { key: "test-only-reference" },
  allowedDatabases: ["codeark_tutorial_test1234"],
  grant: "Observed",
  serverFingerprint: "test-fingerprint",
};
const fixtureGrant: BindingGrantPreview = {
  previewId: "grant-preview",
  bindingId: fixtureBinding.bindingId,
  serviceId: "seata",
  grant: {
    targetId: "target",
    serverFingerprint: "test-fingerprint",
    allowedDatabases: ["codeark_tutorial_test1234"],
    allowedAccounts: ["codeark_tutorial"],
    operations: ["CreateDedicatedSchema"],
    confirmedAt: 1,
  },
  scopeNotice: "仅专属教学初始化范围",
};
export function createTestApi(): ToolboxApi {
  return {
    serviceCatalog: vi.fn().mockResolvedValue(fixtureCatalog),
    dashboardSnapshot: vi.fn().mockResolvedValue(fixtureSnapshot),
    detectEnvironment: vi.fn().mockResolvedValue(fixtureSnapshot),
    preflightService: vi.fn().mockResolvedValue({
      issues: [],
      capacity: {
        workspaceFreeBytes: 1e10,
        daemonFreeBytes: 1e10,
        daemonStatus: "Known",
      },
      requiredMemoryBytes: 0,
      daemonMemoryBytes: 1e10,
      requiresCapacityConsent: false,
    }),
    startServices: vi
      .fn()
      .mockResolvedValue({ ...fixtureOperation, operationId: "new" }),
    stopServices: vi
      .fn()
      .mockResolvedValue({ ...fixtureOperation, operationId: "stop" }),
    retryOperation: vi
      .fn()
      .mockResolvedValue({ ...fixtureOperation, operationId: "retry" }),
    cancelOperation: vi.fn().mockResolvedValue(undefined),
    connectionView: vi.fn().mockResolvedValue({
      instanceId: "redis-1",
      appliedGeneration: "applied-1",
      hostEndpoints: [
        {
          label: "Redis",
          componentId: "redis",
          host: "127.0.0.1",
          port: 16379,
          protocol: "redis",
        },
      ],
      containerEndpoints: [],
      requiredNetworks: ["real-network"],
      credentialHandles: [
        { handle: "password", label: "密码", sensitive: true },
      ],
      snippets: [
        { label: "Redis CLI", body: "redis-cli -h 127.0.0.1 -p 16379 ping" },
      ],
    }),
    revealSecret: vi.fn().mockResolvedValue("test-password"),
    configDraft: vi.fn().mockResolvedValue({
      draftId: "draft-1",
      instanceId: "redis-1",
      baseGeneration: "applied-1",
      fields: [
        { key: "PORT", value: "26379", sensitive: false, editable: true },
      ],
      validationErrors: [],
      applyImpact: ["重建 Redis 托管组件；保留数据"],
      requiresRebuild: true,
      preflight: null,
    }),
    validateDraft: vi.fn().mockResolvedValue({
      draftId: "draft-1",
      instanceId: "redis-1",
      baseGeneration: "applied-1",
      fields: [],
      validationErrors: [],
      applyImpact: ["重建 Redis 托管组件；保留数据"],
      requiresRebuild: true,
      preflight: {
        issues: [],
        capacity: {
          workspaceFreeBytes: 1e10,
          daemonFreeBytes: 1e10,
          daemonStatus: "Known",
        },
        requiredMemoryBytes: 134217728,
        daemonMemoryBytes: 1e10,
        requiresCapacityConsent: false,
      },
    }),
    applyDraft: vi
      .fn()
      .mockResolvedValue({ ...fixtureOperation, operationId: "apply" }),
    planReset: vi.fn().mockResolvedValue({
      planId: "plan-1",
      targetId: "target",
      instanceId: "redis-1",
      serviceId: "redis",
      exactResourceIds: ["owned-volume"],
      databaseClaims: [],
      impact: ["删除教学数据"],
      fingerprint: "fp",
      expiresAt: Date.now() + 60000,
    }),
    executeReset: vi
      .fn()
      .mockResolvedValue({ ...fixtureOperation, operationId: "reset" }),
    previewDiagnostics: vi.fn().mockResolvedValue({
      previewId: "preview",
      sections: { summary: "已脱敏摘要" },
      excludedFields: [".env"],
      riskFlags: [],
    }),
    exportDiagnostics: vi.fn().mockResolvedValue({
      exportId: "export",
      path: "/workspace/diagnostics.json",
    }),
    exportGuideSources: vi
      .fn()
      .mockResolvedValue({ exportId: "sources", path: "/selected/tutorial" }),
    listGuides: vi.fn().mockResolvedValue([]),
    readGuide: vi.fn().mockResolvedValue("# Guide"),
    openOfficial: vi.fn().mockResolvedValue(undefined),
    openServiceConsole: vi.fn().mockResolvedValue(undefined),
    chooseWorkspace: vi.fn().mockResolvedValue(fixtureSnapshot),
    databaseCandidates: vi.fn().mockResolvedValue([]),
    verifyDatabaseBinding: vi.fn().mockResolvedValue(fixtureBinding),
    bindingRequirements: vi
      .fn()
      .mockImplementation(async (serviceId: string) => ({
        serviceId,
        engine: serviceId === "temporal" ? "pgsql" : "mysql",
        databaseNames:
          serviceId === "temporal"
            ? ["temporal", "temporal_visibility"]
            : ["ca_fixture_db"],
        editableDatabases: serviceId === "canal",
      })),
    bindingGrantPreview: vi.fn().mockResolvedValue(fixtureGrant),
    authorizeDatabaseBinding: vi.fn().mockResolvedValue(fixtureBinding),
    startLogStream: vi
      .fn()
      .mockResolvedValue({ streamId: "stream-1", phase: "Active" }),
    ackLogBatch: vi.fn().mockResolvedValue(undefined),
    stopLogStream: vi.fn().mockResolvedValue(undefined),
    subscribeLogStreamState: vi.fn().mockResolvedValue(() => {}),
    subscribeProgress: vi.fn().mockResolvedValue(() => {}),
  };
}
