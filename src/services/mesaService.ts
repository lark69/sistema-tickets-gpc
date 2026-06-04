import type {
  ContaMesa,
  FecharMesaInput,
  LogEntry,
  LogFiltros,
  ExportCsvResult,
  Mesa,
  MesaDetailed,
  MesaProdutoDetalhado,
  MesaProdutoInput,
  MesaSessao,
  PagamentoMesaResult,
  RegistrarPagamentoMesaInput,
  SaveMesaInput,
  TicketData
} from "../types";
import { callCommand } from "./tauri";

export const mesaService = {
  listMesas(): Promise<Mesa[]> {
    return callCommand<Mesa[]>("get_all_mesas");
  },

  getContaMesa(idMesa: number): Promise<ContaMesa> {
    return callCommand<ContaMesa>("get_conta_mesa", { idMesa });
  },

  registrarPagamento(input: RegistrarPagamentoMesaInput): Promise<PagamentoMesaResult> {
    return callCommand<PagamentoMesaResult>("registrar_pagamento_mesa", { input });
  },

  getDetails(idMesa: number): Promise<MesaDetailed> {
    return callCommand<MesaDetailed>("get_mesa_details", { idMesa });
  },

  addProduto(input: MesaProdutoInput): Promise<void> {
    return callCommand<void>("add_produto_to_mesa", { input });
  },

  removeProduto(input: MesaProdutoInput): Promise<void> {
    return callCommand<void>("remove_produto_from_mesa", { input });
  },

  listProdutos(idMesa: number): Promise<MesaProdutoDetalhado[]> {
    return callCommand<MesaProdutoDetalhado[]>("get_mesa_produtos", { idMesa });
  },

  saveMesa(input: SaveMesaInput): Promise<MesaDetailed> {
    return callCommand<MesaDetailed>("save_mesa", { input });
  },

  updateCliente(idMesa: number, nomeCliente?: string | null): Promise<void> {
    return callCommand<void>("update_mesa_cliente", {
      input: { idMesa, nomeCliente: nomeCliente || null }
    });
  },

  getSessao(idMesa: number): Promise<MesaSessao> {
    return callCommand<MesaSessao>("get_mesa_sessao", { idMesa });
  },

  fecharMesa(input: FecharMesaInput): Promise<TicketData> {
    return callCommand<TicketData>("fechar_mesa", { input });
  },

  getLogs(filtros?: LogFiltros): Promise<LogEntry[]> {
    return callCommand<LogEntry[]>("get_logs", { filtros: filtros ?? null });
  },

  exportCsv(filename: string, content: string): Promise<ExportCsvResult> {
    return callCommand<ExportCsvResult>("export_csv", {
      input: { filename, content }
    });
  }
};
