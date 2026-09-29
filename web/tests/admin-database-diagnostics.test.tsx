// @vitest-environment jsdom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AdminDatabaseDiagnosticsPanel } from "../src/features/admin/AdminDatabaseDiagnosticsPanel";
import { api } from "../src/lib/api/client";

globalThis.IS_REACT_ACT_ENVIRONMENT = true;

describe("AdminDatabaseDiagnosticsPanel", () => {
  let container: HTMLDivElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  async function render(status: "WAITING" | "RUNNING" | "READY" | "FAILED", errorCode?: string) {
    vi.spyOn(api, "adminDatabaseDiagnostics").mockResolvedValue({ status, errorCode });
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    await act(async () => {
      root.render(
        <QueryClientProvider client={queryClient}>
          <AdminDatabaseDiagnosticsPanel />
        </QueryClientProvider>,
      );
    });
    await act(async () => {
      await vi.waitFor(() => expect(container.querySelector("[data-status]")).not.toBeNull());
    });
  }

  it("shows the delayed start while waiting", async () => {
    await render("WAITING");
    expect(container.textContent).toContain("启动约 5 分钟后");
    expect(container.querySelector("a[href$='/export']")).toBeNull();
  });

  it("shows progress without offering an unfinished report", async () => {
    await render("RUNNING");
    expect(container.textContent).toContain("正在只读检查数据库结构");
    expect(container.querySelector("a[href$='/export']")).toBeNull();
  });

  it("offers JSON download after the report is ready", async () => {
    await render("READY");
    const link = container.querySelector<HTMLAnchorElement>("a[href='/api/v1/admin/database-diagnostics/export']");
    expect(link?.textContent).toContain("下载体检报告");
  });

  it("shows a safe failure code", async () => {
    await render("FAILED", "DIAGNOSTICS_TIMEOUT");
    expect(container.textContent).toContain("DIAGNOSTICS_TIMEOUT");
    expect(container.querySelector("a[href$='/export']")).toBeNull();
  });
});
