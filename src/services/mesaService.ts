import type {
  FecharMesaInput,
  LogEntry,
  LogFiltros,
  Mesa,
  MesaDetailed,
  MesaProdutoDetalhado,
  MesaProdutoInput,
  MesaSessao,
  SaveMesaInput,
  TicketData
} from "../types";
import { callCommand } from "./tauri";

export const mesaService = {
  listMesas(): Promise<Mesa[]> {
    return callCommand<Mesa[]>("get_all_mesas");
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
  }
};
