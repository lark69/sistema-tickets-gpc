import { useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { adminService } from "../services/adminService";
import type { ReportsPayload } from "../types";
import { formatCurrency } from "../utils/currency";
import { getErrorMessage } from "../utils/errors";

interface ReportsPageProps {
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function ReportsPage({ onMessage }: ReportsPageProps) {
  const [reports, setReports] = useState<ReportsPayload | null>(null);

  async function load() {
    try {
      setReports(await adminService.getReports());
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  async function backup() {
    try {
      const result = await adminService.backupDatabase();
      onMessage(`Backup salvo em: ${result.path}`, "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  useEffect(() => {
    load().catch(() => undefined);
  }, []);

  if (!reports) return <section className="empty-state"><h2>Carregando relatórios...</h2></section>;

  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Relatórios</span>
          <h1>Vendas e estoque</h1>
          <p>Receita, lucro estimado, produtos mais vendidos e alertas de estoque.</p>
        </div>
        <Button variant="secondary" onClick={backup}>Backup agora</Button>
      </div>
      <div className="report-cards">
        <div className="settings-section"><h2>Receita</h2><strong>{formatCurrency(reports.totalRevenueCents)}</strong></div>
        <div className="settings-section"><h2>Lucro estimado</h2><strong>{formatCurrency(reports.estimatedProfitCents)}</strong></div>
      </div>
      <section className="logs-table">
        <div className="logs-row logs-row-head"><span>Dia</span><span>Total</span><span>Resumo</span></div>
        {reports.salesByDay.map((row) => (
          <div className="logs-row" key={row.dateLabel}><span>{row.dateLabel}</span><span>{formatCurrency(row.totalCents)}</span><span>Vendas do dia</span></div>
        ))}
      </section>
      <section className="logs-table">
        <div className="logs-row logs-row-head"><span>Produto</span><span>Qtd</span><span>Total</span></div>
        {reports.topProducts.map((row) => (
          <div className="logs-row" key={row.productName}><span>{row.productName}</span><span>{row.quantity}</span><span>{formatCurrency(row.totalCents)}</span></div>
        ))}
      </section>
      <section className="logs-table">
        <div className="logs-row logs-row-head"><span>Produto</span><span>Estoque</span><span>Status</span></div>
        {reports.lowStockProducts.map((product) => (
          <div className="logs-row" key={product.id}><span>{product.name}</span><span>{product.stock}</span><span>Estoque baixo</span></div>
        ))}
      </section>
    </section>
  );
}
