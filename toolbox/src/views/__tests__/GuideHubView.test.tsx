import userEvent from "@testing-library/user-event";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import App from "../../App";
import { createTestApi } from "../../test-fixtures";
it("exports only the selected bundled guide sources through native backend selection", async () => {
  const api = createTestApi();
  api.listGuides = vi.fn().mockResolvedValue([
    {
      guideId: "backend-mysql",
      serviceId: "mysql",
      title: "MySQL 教学",
      clientFamily: "JDBC",
      lockedClientVersion: "26.10.0",
      requiredTools: ["JDK17"],
      sourceFiles: ["guides/backend-mysql.md", "examples/java/pom.xml"],
      invocation: "mvn package",
      expectedOutput: "MYSQL_QUERY_OK",
      cleanup: "教学表",
      scopeNotice: "独立教学范围",
      status: "pending",
    },
  ]);
  api.readGuide = vi.fn().mockResolvedValue("# 教学说明");
  const exportSources = vi
    .fn()
    .mockResolvedValue({ exportId: "sources", path: "/selected/tutorial" });
  render(<App api={{ ...api, exportGuideSources: exportSources }} />);
  await userEvent.click(
    await screen.findByRole("button", { name: "使用指南" }),
  );
  await userEvent.click(
    await screen.findByRole("button", { name: /阅读指南/ }),
  );
  expect(await screen.findByText("# 教学说明")).toBeInTheDocument();
  expect(exportSources).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: "导出教学源码" }));
  await waitFor(() =>
    expect(exportSources).toHaveBeenCalledWith("backend-mysql"),
  );
  expect(await screen.findByText("教学源码已导出")).toBeInTheDocument();
  expect(screen.queryByText("/selected/tutorial")).not.toBeInTheDocument();
});
