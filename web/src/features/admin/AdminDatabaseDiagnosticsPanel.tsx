import { useQuery } from "@tanstack/react-query";
import { Database, Download, RefreshCw } from "lucide-react";
import { api } from "../../lib/api/client";
import { queryKeys } from "../../lib/api/query-keys";

const statusCopy = {
  WAITING: "体检将在 Lux 启动约 5 分钟后自动开始。",
  RUNNING: "正在只读检查数据库结构和统计信息。",
  READY: "体检报告已生成，可以导出 JSON。",
  FAILED: "体检未能完成，请查看错误代码并稍后重启 Lux 重试。",
} as const;

export function AdminDatabaseDiagnosticsPanel() {
  const diagnostics = useQuery({
    queryKey: queryKeys.adminDatabaseDiagnostics,
    queryFn: () => api.adminDatabaseDiagnostics(),
    refetchInterval: (query) =>
      query.state.data?.status === "WAITING" || query.state.data?.status === "RUNNING"
        ? 15_000
        : false,
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
          <p role="status">{statusCopy[status]}</p>
          {status === "FAILED" && diagnostics.data?.errorCode ? <code>{diagnostics.data.errorCode}</code> : null}
          {status === "READY" ? (
            <a className="lux-button lux-button-secondary" href="/api/v1/admin/database-diagnostics/export">
              <Download size={15} /> 下载体检报告
            </a>
          ) : status === "WAITING" || status === "RUNNING" ? (
            <span className="lux-admin-muted"><RefreshCw size={14} /> 完成后会在这里提供下载</span>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
