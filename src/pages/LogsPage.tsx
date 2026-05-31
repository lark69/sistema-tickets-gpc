import { Download, Filter, Search } from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { Button } from "../components/ui/Button";
import { Select } from "../components/ui/Select";
import { TextInput } from "../components/ui/TextInput";
import { mesaService } from "../services/mesaService";
import type { LogEntry, LogFiltros } from "../types";
import { formatCurrency } from "../utils/currency";
import { formatDateFromMillis } from "../utils/dates";
import { getErrorMessage } from "../utils/errors";

interface LogsPageProps {
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function LogsPage({ onMessage }: LogsPageProps) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [tipo, setTipo] = useState("");
  const [numeroMesa, setNumeroMesa] = useState("");
  const [searchTerm, setSearchTerm] = useState("");

  async function loadLogs(filtros?: LogFiltros) {
    setLoading(true);
    try {
      setLogs(await mesaService.getLogs(filtros));
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoading(false);
    }
  }

  function handleFilter(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    loadLogs({
      tipo: tipo || null,
      numeroMesa: numeroMesa ? Number.parseInt(numeroMesa, 10) : null
    }).catch(() => undefined);
  }

  const filteredLogs = useMemo(() => {
    const term = normalizeSearch(searchTerm);
    if (!term) {
      return logs;
    }

    return logs.filter((log) =>
      normalizeSearch([
        log.id,
        log.tipo,
        log.numeroMesa ? String(log.numeroMesa).padStart(2, "0") : "",
        log.nomeCliente ?? "",
        log.valorTotalCents ? formatCurrency(log.valorTotalCents) : "",
        log.formaPagamento ?? "",
        log.tempoPermanencia ?? "",
        log.idMesaUnico ?? "",
        log.listaProdutosJson ?? "",
        formatDateFromMillis(log.dataHora),
        new Date(log.dataHora).toLocaleString("pt-BR")
      ].join(" ")).includes(term)
    );
  }, [logs, searchTerm]);

  const csv = useMemo(() => {
    const rows = [
      ["Data/Hora", "Tipo", "Mesa", "Cliente", "Valor", "Pagamento", "Tempo", "ID"],
      ...filteredLogs.map((log) => [
        new Date(log.dataHora).toLocaleString("pt-BR"),
        log.tipo,
        log.numeroMesa ? String(log.numeroMesa).padStart(2, "0") : "",
        log.nomeCliente ?? "",
        log.valorTotalCents ? formatCurrency(log.valorTotalCents) : "",
        log.formaPagamento ?? "",
        log.tempoPermanencia ?? "",
        log.idMesaUnico ?? ""
      ])
    ];
    return rows.map((row) => row.map((cell) => `"${cell.replace(/"/g, '""')}"`).join(";")).join("\n");
  }, [filteredLogs]);

  function exportCsv() {
    mesaService
      .exportCsv("logs-portex-pdv.csv", csv)
      .then((result) => onMessage(`CSV exportado em: ${result.path}`, "success"))
      .catch((err) => onMessage(getErrorMessage(err), "error"));
  }

  useEffect(() => {
    loadLogs().catch(() => undefined);
  }, []);

  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Auditoria</span>
          <h1>Logs do sistema</h1>
          <p>Acompanhe tickets gerados, mesas fechadas e produtos criados.</p>
        </div>
        <Button variant="secondary" icon={<Download size={18} />} onClick={exportCsv}>
          Exportar CSV
        </Button>
      </div>

      <form className="logs-filters" onSubmit={handleFilter}>
        <Select
          label="Tipo"
          value={tipo}
          onChange={(event) => setTipo(event.target.value)}
          options={[
            { value: "", label: "Todos" },
            { value: "ticket_gerado", label: "Ticket gerado" },
            { value: "mesa_fechada", label: "Mesa fechada" },
            { value: "venda_caixa", label: "Venda de caixa" },
            { value: "produto_criado", label: "Produto criado" },
            { value: "produto_editado", label: "Produto editado" },
            { value: "categoria_criada", label: "Categoria criada" }
          ]}
        />
        <TextInput
          label="Número da mesa"
          type="number"
          min={1}
          max={100}
          value={numeroMesa}
          onChange={(event) => setNumeroMesa(event.target.value)}
          placeholder="01"
        />
        <TextInput
          label="Busca geral"
          icon={<Search size={18} />}
          value={searchTerm}
          onChange={(event) => setSearchTerm(event.target.value)}
          placeholder="ID, nome, data, usuario, ticket, mesa, caixa..."
        />
        <Button type="submit" icon={<Filter size={18} />} loading={loading}>
          Filtrar
        </Button>
      </form>

      <div className="logs-table">
        <div className="logs-row logs-row-head">
          <span>Data/Hora</span>
          <span>Tipo</span>
          <span>Detalhes</span>
        </div>
        {filteredLogs.map((log) => (
          <div key={log.id} className="logs-row">
            <span>{formatDateFromMillis(log.dataHora)} {new Date(log.dataHora).toLocaleTimeString("pt-BR")}</span>
            <span>{log.tipo.replace("_", " ")}</span>
            <span>
              {log.numeroMesa ? `Mesa ${String(log.numeroMesa).padStart(2, "0")} · ` : ""}
              {log.nomeCliente ? `${log.nomeCliente} · ` : ""}
              {log.valorTotalCents ? `${formatCurrency(log.valorTotalCents)} · ` : ""}
              {log.formaPagamento ? `${log.formaPagamento} · ` : ""}
              {log.tempoPermanencia ? `Tempo ${log.tempoPermanencia}` : ""}
            </span>
          </div>
        ))}
        {filteredLogs.length === 0 ? <div className="logs-empty">Nenhum log encontrado.</div> : null}
      </div>
    </section>
  );
}

function normalizeSearch(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .trim();
}
