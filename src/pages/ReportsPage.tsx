import { Download, Printer, RotateCcw, Upload } from "lucide-react";
import { type ChangeEvent, useEffect, useRef, useState } from "react";
import { Button } from "../components/ui/Button";
import { Select } from "../components/ui/Select";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser, ReportsPayload, SalesReportPeriod } from "../types";
import { formatCurrency } from "../utils/currency";
import { getErrorMessage } from "../utils/errors";

interface ReportsPageProps {
  currentUser: LocalUser | null;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function ReportsPage({ currentUser, onMessage }: ReportsPageProps) {
  const [reports, setReports] = useState<ReportsPayload | null>(null);
  const [reportPeriod, setReportPeriod] = useState<SalesReportPeriod>("day");
  const [printingReport, setPrintingReport] = useState(false);
  const [resettingSales, setResettingSales] = useState(false);
  const [exportingConfig, setExportingConfig] = useState(false);
  const [importingConfig, setImportingConfig] = useState(false);
  const [importPath, setImportPath] = useState("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const isAdmin = currentUser?.role === "admin";

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

  async function printReport() {
    setPrintingReport(true);

    try {
      const result = await adminService.printSalesReport(reportPeriod);
      onMessage(`Relatorio impresso em ${result.printerName}: ${result.periodLabel}.`, "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setPrintingReport(false);
    }
  }

  async function resetSales() {
    if (!isAdmin || !currentUser) {
      onMessage("Apenas administradores podem resetar vendas.", "error");
      return;
    }

    const password = window.prompt("Digite sua senha para concluir esta operação");
    if (password === null) {
      return;
    }

    setResettingSales(true);

    try {
      await adminService.resetSales(currentUser.username, password);
      await load();
      onMessage("Vendas resetadas com sucesso.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setResettingSales(false);
    }
  }

  async function exportConfig() {
    setExportingConfig(true);

    try {
      const result = await adminService.exportAppConfig();
      setImportPath(result.path);
      onMessage(`Configurações exportadas em: ${result.path}`, "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setExportingConfig(false);
    }
  }

  async function importConfig() {
    if (!importPath.trim()) {
      fileInputRef.current?.click();
      return;
    }

    setImportingConfig(true);

    try {
      await adminService.importAppConfig(importPath);
      onMessage("Configurações importadas. O aplicativo será recarregado.", "success");
      window.setTimeout(() => window.location.reload(), 700);
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setImportingConfig(false);
    }
  }

  async function handleConfigFile(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    setImportingConfig(true);

    try {
      const content = await file.text();
      await adminService.importAppConfigContent(content);
      setImportPath(file.name);
      onMessage("Configurações importadas. O aplicativo será recarregado.", "success");
      window.setTimeout(() => window.location.reload(), 700);
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      event.target.value = "";
      setImportingConfig(false);
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
        <div className="report-actions">
          <Select
            label="Periodo"
            value={reportPeriod}
            onChange={(event) => setReportPeriod(event.target.value as SalesReportPeriod)}
            options={[
              { value: "day", label: "Vendas do dia" },
              { value: "month", label: "Vendas do mes" }
            ]}
          />
          <Button icon={<Printer size={18} />} loading={printingReport} onClick={printReport}>
            Imprimir relatorio
          </Button>
          <Button variant="secondary" onClick={backup}>Backup agora</Button>
        </div>
      </div>
      <div className="report-cards">
        <div className="settings-section"><h2>Receita</h2><strong>{formatCurrency(reports.totalRevenueCents)}</strong></div>
        <div className="settings-section"><h2>Lucro estimado</h2><strong>{formatCurrency(reports.estimatedProfitCents)}</strong></div>
      </div>
      {isAdmin ? (
        <section className="settings-section">
          <div className="settings-section-title">
            <div>
              <h2>Manutenção administrativa</h2>
              <p>Exporte ou importe produtos, categorias e dados básicos da empresa.</p>
            </div>
          </div>
          <div className="form-grid">
            <input
              ref={fileInputRef}
              type="file"
              accept="application/json,.json"
              className="sr-only"
              onChange={handleConfigFile}
            />
            <TextInput
              label="Arquivo de configuração"
              value={importPath}
              onChange={(event) => setImportPath(event.target.value)}
              placeholder="C:\\Users\\...\\portex-pdv-config.json"
              hint="Use o caminho gerado na exportação ou cole o caminho de outro arquivo Portex PDV."
            />
            <div className="report-actions">
              <Button
                variant="secondary"
                icon={<Download size={18} />}
                loading={exportingConfig}
                onClick={exportConfig}
              >
                Exportar configurações
              </Button>
              <Button
                variant="secondary"
                icon={<Upload size={18} />}
                loading={importingConfig}
                onClick={importConfig}
              >
                Importar configurações
              </Button>
              <Button
                variant="danger"
                icon={<RotateCcw size={18} />}
                loading={resettingSales}
                onClick={resetSales}
              >
                Resetar vendas
              </Button>
            </div>
          </div>
        </section>
      ) : null}
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
