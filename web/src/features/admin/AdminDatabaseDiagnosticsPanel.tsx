import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Database, Download, RefreshCw } from "lucide-react";
import { api } from "../../lib/api/client";
import { queryKeys } from "../../lib/api/query-keys";

const statusCopy = {
  WAITING: "体检将在 Lux 启动约 5 分钟后自动开始，也可以立即开始。",
  RUNNING: "正在只读检查数据库结构和统计信息。",
  READY: "体检报告已生成，可以导出 JSON 或重新采集。",
  FAILED: "体检未能完成，可以查看错误代码并重新采集。",
} as const;

export function AdminDatabaseDiagnosticsPanel() {
  const queryClient = useQueryClient();
  const diagnostics = useQuery({
    queryKey: queryKeys.adminDatabaseDiagnostics,
    queryFn: () => api.adminDatabaseDiagnostics(),
    refetchInterval: (query) =>
      query.state.data?.status === "WAITING" || query.state.data?.status === "RUNNING"
        ? 15_000
        : false,
  });
  const startCollection = useMutation({
    mutationFn: () => api.startAdminDatabaseDiagnostics(),
    onSuccess: (status) => {
      queryClient.setQueryData(queryKeys.adminDatabaseDiagnostics, status);
    },
  });
  const status = diagnostics.data?.status;

  return (
    <section className="lux-admin-panel lux-admin-settings-panel" aria-labelledby="database-diagnostics-heading">
      <div className="lux-admin-panel-heading">
        <div>
          <h2 id="database-diagnostics-heading"><Database size={18} /> 临时数据库体检</h2>
          <p>只读统计数据库、表和索引占用，报告仅保存在 Lux 进程内存中。</p>
        </div>
      </div>
      {diagnostics.isPending ? <p className="lux-admin-muted" role="status">正在读取体检状态…</p> : null}
      {diagnostics.error ? <p className="lux-error-copy" role="alert">体检状态读取失败：{diagnostics.error.message}</p> : null}
      {status ? (
        <div className="lux-admin-database-diagnostics-status" data-status={status}>
          <p role="status">
            {status === "RUNNING" && diagnostics.data?.hasReport
              ? "正在重新采集；上一份报告仍可下载。"
              : status === "FAILED" && diagnostics.data?.hasReport
                ? "重新采集失败；上一份报告仍可下载。"
                : statusCopy[status]}
          </p>
          {status === "FAILED" && diagnostics.data?.errorCode ? <code>{diagnostics.data.errorCode}</code> : null}
          {diagnostics.data?.hasReport ? (
            <a className="lux-button lux-button-secondary" href="/api/v1/admin/database-diagnostics/export">
              <Download size={15} /> 下载体检报告
            </a>
          ) : null}
          <button
            className="lux-button lux-button-secondary"
            type="button"
            disabled={status === "RUNNING" || startCollection.isPending}
            onClick={() => startCollection.mutate()}
          >
            <RefreshCw size={15} />
            {status === "WAITING" ? "立即开始体检" : status === "RUNNING" ? "正在采集…" : "重新采集"}
          </button>
          {startCollection.error ? <p className="lux-error-copy" role="alert">启动体检失败：{startCollection.error.message}</p> : null}
        </div>
      ) : null}
    </section>
  );
}
