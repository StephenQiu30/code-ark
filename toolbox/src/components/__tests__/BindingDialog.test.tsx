import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import App from "../../App";
import {
  createTestApi,
  fixtureCatalog,
  fixtureSnapshot,
} from "../../test-fixtures";

function databaseApi(serviceId = "seata") {
  const api = createTestApi();
  api.serviceCatalog = vi.fn().mockResolvedValue({
    ...fixtureCatalog,
    services: fixtureCatalog.services.map((service) => ({
      ...service,
      dependencies:
        service.id === serviceId && !["mysql", "pgsql"].includes(serviceId)
          ? [serviceId === "temporal" ? "pgsql" : "mysql"]
          : service.dependencies,
    })),
  });
  api.databaseCandidates = vi.fn().mockResolvedValue([
    {
      candidateId: "candidate",
      engine:
        serviceId === "temporal" || serviceId === "pgsql" ? "pgsql" : "mysql",
      name: "existing-database",
      containerId: "existing-container",
      networkNames: ["existing-network"],
      hostEndpoints: [],
    },
  ]);
  const check = api.preflightService;
  api.preflightService = vi
    .fn()
    .mockRejectedValueOnce({ code: "DependencyBindingRequired" })
    .mockImplementation(check);
  return api;
}

async function startFromCatalog(
  api: ReturnType<typeof createTestApi>,
  serviceId: string,
) {
  render(<App api={api} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "全部服务" }),
  );
  await userEvent.click(
    await screen.findByRole("button", { name: `启动 ${serviceId}` }),
  );
  await screen.findByRole("dialog", { name: "连接并授权已有数据库" });
}

async function verifyConnection() {
  await userEvent.type(await screen.findByLabelText("用户名"), "root");
  await userEvent.type(screen.getByLabelText("连接密码"), "test-password");
  await userEvent.click(
    screen.getByRole("button", { name: "验证连接并预览授权" }),
  );
}

describe("binding authorization and resumed startup", () => {
  it("keeps derived databases read-only and resumes the original start only after explicit authorization", async () => {
    const api = databaseApi();
    await startFromCatalog(api, "seata");
    const databases =
      await screen.findByLabelText("允许的专属数据库（逗号分隔）");
    expect(databases).toHaveAttribute("readonly");
    expect(databases).toHaveValue("ca_fixture_db");
    await verifyConnection();
    expect(api.verifyDatabaseBinding).toHaveBeenCalledWith({
      candidateId: "candidate",
      username: "root",
      password: "test-password",
      allowedDatabases: ["ca_fixture_db"],
    });
    await screen.findByRole("button", { name: "确认以上专属初始化授权" });
    expect(api.startServices).not.toHaveBeenCalled();
    await userEvent.click(
      screen.getByRole("button", { name: "确认以上专属初始化授权" }),
    );
    await waitFor(() => expect(api.startServices).toHaveBeenCalledTimes(1));
    expect(api.startServices).toHaveBeenCalledWith(
      expect.objectContaining({
        serviceIds: ["seata"],
        acceptUnknownCapacity: false,
      }),
    );
    expect(api.preflightService).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("reuses a saved verified binding without requesting its password again", async () => {
    const api = databaseApi();
    const binding = await api.verifyDatabaseBinding({
      candidateId: "candidate",
      username: "root",
      password: "",
      allowedDatabases: [],
    });
    vi.mocked(api.verifyDatabaseBinding).mockClear();
    api.dashboardSnapshot = vi
      .fn()
      .mockResolvedValue({ ...fixtureSnapshot, bindings: [binding] });
    await startFromCatalog(api, "seata");
    await screen.findByRole("button", { name: "确认以上专属初始化授权" });
    expect(api.bindingGrantPreview).toHaveBeenCalledWith(
      binding.bindingId,
      "seata",
    );
    expect(screen.queryByLabelText("连接密码")).not.toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: "确认以上专属初始化授权" }),
    );
    await waitFor(() => expect(api.startServices).toHaveBeenCalledTimes(1));
    expect(api.verifyDatabaseBinding).not.toHaveBeenCalled();
  });

  it.each(["mysql", "pgsql"])(
    "verifies %s for observation without requesting an initialization grant",
    async (serviceId) => {
      const api = databaseApi(serviceId);
      api.bindingRequirements = vi
        .fn()
        .mockResolvedValue({
          serviceId,
          engine: serviceId,
          databaseNames: [],
          editableDatabases: false,
        });
      await startFromCatalog(api, serviceId);
      await verifyConnection();
      await waitFor(() => expect(api.startServices).toHaveBeenCalledTimes(1));
      expect(api.bindingGrantPreview).not.toHaveBeenCalled();
      expect(api.authorizeDatabaseBinding).not.toHaveBeenCalled();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    },
  );

  it("lets Canal choose existing databases and passes only that scope for verification", async () => {
    const api = databaseApi("canal");
    api.bindingRequirements = vi
      .fn()
      .mockResolvedValue({
        serviceId: "canal",
        engine: "mysql",
        databaseNames: [],
        editableDatabases: true,
      });
    await startFromCatalog(api, "canal");
    const databases =
      await screen.findByLabelText("允许的专属数据库（逗号分隔）");
    expect(databases).not.toHaveAttribute("readonly");
    await userEvent.type(databases, "orders, inventory");
    await verifyConnection();
    await screen.findByRole("button", { name: "确认以上专属初始化授权" });
    expect(api.verifyDatabaseBinding).toHaveBeenCalledWith(
      expect.objectContaining({ allowedDatabases: ["orders", "inventory"] }),
    );
    expect(api.startServices).not.toHaveBeenCalled();
  });

  it("does not resume a cancelled startup when authorization finishes later", async () => {
    const api = databaseApi();
    const binding = await api.verifyDatabaseBinding({
      candidateId: "candidate",
      username: "root",
      password: "",
      allowedDatabases: [],
    });
    let resolve!: (value: typeof binding) => void;
    api.authorizeDatabaseBinding = vi.fn().mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    await startFromCatalog(api, "seata");
    await verifyConnection();
    await userEvent.click(
      await screen.findByRole("button", { name: "确认以上专属初始化授权" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "取消" }));
    await act(async () => resolve(binding));
    expect(api.startServices).not.toHaveBeenCalled();
    expect(api.preflightService).toHaveBeenCalledTimes(1);
  });
});
