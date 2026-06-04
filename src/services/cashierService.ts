import type {
  AbrirTurnoInput,
  BloquearPeriodoInput,
  CashierStatus,
  ConsolidarPeriodoInput,
  EditarVendaInput,
  FecharTurnoInput,
  PeriodoContabil,
  SaleAuditEntry,
  TurnoOperacional
} from "../types";
import { callCommand } from "./tauri";

export const cashierService = {
  getStatus(): Promise<CashierStatus> {
    return callCommand<CashierStatus>("get_cashier_status");
  },

  abrirTurno(input: AbrirTurnoInput): Promise<TurnoOperacional> {
    return callCommand<TurnoOperacional>("abrir_turno", { input });
  },

  fecharTurno(input: FecharTurnoInput): Promise<TurnoOperacional> {
    return callCommand<TurnoOperacional>("fechar_turno", { input });
  },

  listarTurnosDia(data: string): Promise<TurnoOperacional[]> {
    return callCommand<TurnoOperacional[]>("listar_turnos_dia", { data });
  },

  consolidarPeriodo(input: ConsolidarPeriodoInput): Promise<PeriodoContabil> {
    return callCommand<PeriodoContabil>("consolidar_periodo", { input });
  },

  bloquearPeriodo(input: BloquearPeriodoInput): Promise<PeriodoContabil> {
    return callCommand<PeriodoContabil>("bloquear_periodo", { input });
  },

  editarVenda(input: EditarVendaInput): Promise<SaleAuditEntry> {
    return callCommand<SaleAuditEntry>("editar_venda", { input });
  },

  listarAuditoriaVenda(saleId: number): Promise<SaleAuditEntry[]> {
    return callCommand<SaleAuditEntry[]>("listar_auditoria_venda", { saleId });
  },

  getFiscalDayConfig(): Promise<number> {
    return callCommand<number>("get_fiscal_day_config");
  },

  setFiscalDayConfig(fiscalDayStartMinutes: number): Promise<number> {
    return callCommand<number>("set_fiscal_day_config", {
      input: { fiscalDayStartMinutes }
    });
  }
};
