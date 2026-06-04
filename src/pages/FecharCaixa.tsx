import { CalendarClock, ClipboardCheck, HelpCircle, Lock, PlayCircle, StopCircle } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { cashierService } from "../services/cashierService";
import type { CashierStatus, LocalUser, TurnoOperacional, TurnoStatus } from "../types";
import { currencyToCents, formatCurrency } from "../utils/currency";
import { getErrorMessage } from "../utils/errors";

interface FecharCaixaPageProps {
  currentUser: LocalUser;
  canManageCash: boolean;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
  onOpenGuide?: () => void;
}

const TURNO_LABEL: Record<TurnoStatus, string> = {
  aberto: "Aberto",
  fechado: "Fechado",
  reconciliado: "Reconciliado"
};

function minutesToHHMM(minutes: number): string {
  const safe = Math.max(0, Math.min(1439, Math.trunc(minutes)));
  const hh = String(Math.trunc(safe / 60)).padStart(2, "0");
  const mm = String(safe % 60).padStart(2, "0");
  return `${hh}:${mm}`;
}

function hhmmToMinutes(value: string): number {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!match) {
    return 0;
  }
  const hours = Number(match[1]);
  const mins = Number(match[2]);
  return Math.max(0, Math.min(1439, hours * 60 + mins));
}

function formatDateTime(value?: number | null): string {
  if (!value) {
    return "—";
  }
  return new Date(value).toLocaleString("pt-BR");
}

export function FecharCaixaPage({ currentUser, canManageCash, onMessage, onOpenGuide }: FecharCaixaPageProps) {
  const [status, setStatus] = useState<CashierStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [finalCounted, setFinalCounted] = useState("");
  const [observacoes, setObservacoes] = useState("");
  const [saldoInicial, setSaldoInicial] = useState("");
  const [fiscalStart, setFiscalStart] = useState("00:00");

  const isAdmin = currentUser.role === "admin";

  const load = useCallback(async () => {
    try {
      const next = await cashierService.getStatus();
      setStatus(next);
      setFiscalStart(minutesToHHMM(next.fiscalDayStartMinutes));
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoading(false);
    }
  }, [onMessage]);

  useEffect(() => {
    load().catch(() => undefined);
  }, [load]);

  const turnoAtivo = status?.turnoAtivo ?? null;
  const periodo = status?.periodoHoje ?? null;
  const turnos = useMemo(() => status?.turnosDoDia ?? [], [status]);

  const todosFechados = turnos.length > 0 && turnos.every((t) => t.status !== "aberto");
  const periodoBloqueado = periodo?.status === "bloqueado";
  const periodoConsolidado = periodo?.status === "fechado" || periodoBloqueado;

  const totalEsperadoDia = useMemo(
    () => turnos.reduce((sum, t) => sum + t.valorEsperadoCents, 0),
    [turnos]
  );
  const totalRealDia = useMemo(
    () => turnos.reduce((sum, t) => sum + (t.valorFisicoCents ?? 0), 0),
    [turnos]
  );

  async function abrirTurno() {
    if (!canManageCash) {
      onMessage("Usuario sem permissao para operar o caixa.", "error");
      return;
    }
    setBusy(true);
    try {
      await cashierService.abrirTurno({
        operador: currentUser.username,
        saldoInicialCents: currencyToCents(saldoInicial)
      });
      setSaldoInicial("");
      await load();
      onMessage("Turno aberto com sucesso.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function fecharTurno() {
    if (!turnoAtivo) {
      return;
    }
    const fisicoCents = currencyToCents(finalCounted);
    setBusy(true);
    try {
      const turno = await cashierService.fecharTurno({
        turnoId: turnoAtivo.id,
        valorFisicoCents: fisicoCents,
        observacoes: observacoes || null
      });
      const diff = turno.diferencaCents ?? 0;
      if (diff !== 0 && !observacoes.trim()) {
        onMessage(
          `Turno fechado com diferenca de ${formatCurrency(diff)}. Registre uma observacao se necessario.`,
          "info"
        );
      } else {
        onMessage("Turno fechado e reconciliado.", "success");
      }
      setFinalCounted("");
      setObservacoes("");
      await load();
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function consolidar() {
    setBusy(true);
    try {
      await cashierService.consolidarPeriodo({
        data: status?.dataContabil ?? null,
        usuario: currentUser.username
      });
      await load();
      onMessage("Periodo contabil consolidado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function bloquear() {
    if (!periodo) {
      return;
    }
    const confirmed = window.confirm(
      "Bloquear o periodo o torna imutavel para auditoria fiscal. Continuar?"
    );
    if (!confirmed) {
      return;
    }
    setBusy(true);
    try {
      await cashierService.bloquearPeriodo({
        periodoId: periodo.id,
        usuario: currentUser.username
      });
      await load();
      onMessage("Periodo bloqueado para auditoria.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  }

  async function salvarDiaFiscal() {
    setBusy(true);
    try {
      await cashierService.setFiscalDayConfig(hhmmToMinutes(fiscalStart));
      await load();
      onMessage("Inicio do dia fiscal atualizado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setBusy(false);
    }
  }

  if (loading) {
    return (
      <section className="page-stack">
        <p className="muted-text">Carregando fechamento de caixa...</p>
      </section>
    );
  }

  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Fechamento de caixa</span>
          <h1>Turnos e periodo contabil</h1>
          <p>
            Reconcilie o turno do operador e consolide o dia contabil ({status?.dataContabil}). Periodo:{" "}
            <strong>{periodo ? periodo.status.toUpperCase() : "—"}</strong>.
          </p>
        </div>
        <div className="cash-register-pill">
          <CalendarClock size={18} />
          {turnoAtivo ? "Turno aberto" : periodoConsolidado ? "Periodo consolidado" : "Sem turno aberto"}
        </div>
      </div>

      {/* Etapa 1/2: turno operacional */}
      <section className="settings-section">
        {turnoAtivo ? (
          <>
            <h2>Turno em andamento</h2>
            <div className="form-grid">
              <p>Operador: <strong>{turnoAtivo.operador}</strong></p>
              <p>Inicio: <strong>{formatDateTime(turnoAtivo.dataInicio)}</strong></p>
              <p>Fundo de troco: <strong>{formatCurrency(turnoAtivo.saldoInicialCents)}</strong></p>
              <p>
                Esperado na gaveta:{" "}
                <strong>{formatCurrency(status?.esperadoAtualCents ?? turnoAtivo.saldoInicialCents)}</strong>
              </p>
            </div>
            <TextInput
              label="Dinheiro fisico contado"
              value={finalCounted}
              onChange={(e) => setFinalCounted(e.target.value)}
              placeholder="0,00"
              hint="Gaveta esperada = fundo de troco + vendas em dinheiro + suprimentos - sangrias."
            />
            <TextInput
              label="Observacoes (obrigatorio se houver diferenca relevante)"
              value={observacoes}
              multiline
              onChange={(e) => setObservacoes(e.target.value)}
            />
            <div className="form-actions">
              <Button
                type="button"
                variant="danger"
                icon={<StopCircle size={18} />}
                loading={busy}
                disabled={!canManageCash}
                onClick={fecharTurno}
              >
                Fechar turno
              </Button>
            </div>
          </>
        ) : periodoBloqueado ? (
          <>
            <h2>Periodo bloqueado</h2>
            <p className="muted-text">
              O dia contabil esta selado para auditoria fiscal. Nao e possivel abrir novos turnos.
            </p>
          </>
        ) : (
          <>
            <h2>Abrir turno</h2>
            <p className="muted-text">
              Inicie um turno para registrar vendas. Multiplos turnos podem ocorrer no mesmo dia contabil.
            </p>
            <div className="form-grid">
              <TextInput
                label="Fundo de troco (dinheiro inicial na gaveta)"
                value={saldoInicial}
                onChange={(e) => setSaldoInicial(e.target.value)}
                placeholder="0,00"
              />
            </div>
            <div className="form-actions">
              <Button
                type="button"
                icon={<PlayCircle size={18} />}
                loading={busy}
                disabled={!canManageCash || periodoConsolidado}
                onClick={abrirTurno}
              >
                Abrir turno ({currentUser.username})
              </Button>
            </div>
          </>
        )}
      </section>

      {/* Turnos do dia */}
      <section className="settings-section">
        <h2>Turnos do dia contabil</h2>
        {turnos.length === 0 ? (
          <p className="muted-text">Nenhum turno registrado neste dia.</p>
        ) : (
          <div className="logs-table">
            <div className="logs-row logs-row-head">
              <span>Operador</span>
              <span>Inicio / Fim</span>
              <span>Esperado</span>
              <span>Contado</span>
              <span>Diferenca</span>
              <span>Status</span>
            </div>
            {turnos.map((turno: TurnoOperacional) => (
              <div className="logs-row" key={turno.id}>
                <span>{turno.operador}</span>
                <span>
                  {formatDateTime(turno.dataInicio)}
                  <br />
                  {formatDateTime(turno.dataFim)}
                </span>
                <span>{formatCurrency(turno.valorEsperadoCents)}</span>
                <span>{turno.valorFisicoCents != null ? formatCurrency(turno.valorFisicoCents) : "—"}</span>
                <span className={turno.diferencaCents ? "danger-text" : ""}>
                  {turno.diferencaCents != null ? formatCurrency(turno.diferencaCents) : "—"}
                </span>
                <span>{TURNO_LABEL[turno.status]}</span>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Etapa 3/4: periodo contabil */}
      <section className="settings-section">
        <h2>Periodo contabil ({status?.dataContabil})</h2>
        <p className="muted-text">
          Esta etapa confere a <strong>gaveta de dinheiro</strong> do dia (soma dos turnos). Nao e o
          faturamento: cartao e PIX nao entram aqui. Para a receita total, veja Relatorios.
        </p>
        {periodoConsolidado ? (
          <div className="form-grid">
            <p>Esperado na gaveta (dinheiro): <strong>{formatCurrency(periodo?.totalEsperadoCents ?? 0)}</strong></p>
            <p>Contado na gaveta (dinheiro): <strong>{formatCurrency(periodo?.totalRealCents ?? 0)}</strong></p>
            <p>
              Diferenca:{" "}
              <strong>{formatCurrency((periodo?.totalRealCents ?? 0) - (periodo?.totalEsperadoCents ?? 0))}</strong>
            </p>
            {periodoBloqueado ? (
              <p>Bloqueado por: <strong>{periodo?.bloqueadoPor}</strong> em {formatDateTime(periodo?.bloqueadoEm)}</p>
            ) : null}
          </div>
        ) : (
          <div className="form-grid">
            <p>Esperado na gaveta (turnos): <strong>{formatCurrency(totalEsperadoDia)}</strong></p>
            <p>Contado na gaveta (turnos): <strong>{formatCurrency(totalRealDia)}</strong></p>
          </div>
        )}

        <div className="form-actions">
          {!periodoConsolidado ? (
            <Button
              type="button"
              icon={<ClipboardCheck size={18} />}
              loading={busy}
              disabled={!canManageCash || !todosFechados}
              onClick={consolidar}
            >
              Consolidar periodo
            </Button>
          ) : null}
          {periodo?.status === "fechado" ? (
            <Button
              type="button"
              variant="danger"
              icon={<Lock size={18} />}
              loading={busy}
              disabled={!isAdmin}
              onClick={bloquear}
            >
              Bloquear periodo
            </Button>
          ) : null}
        </div>
        {!periodoConsolidado && !todosFechados ? (
          <p className="muted-text">Feche todos os turnos do dia para liberar a consolidacao.</p>
        ) : null}
        {periodo?.status === "fechado" && !isAdmin ? (
          <p className="muted-text">Apenas administradores podem bloquear o periodo.</p>
        ) : null}
      </section>

      {/* Dia fiscal deslocado */}
      {isAdmin ? (
        <section className="settings-section">
          <h2>Dia fiscal deslocado</h2>
          <p className="muted-text">
            Defina a hora em que o dia contabil vira (ex.: 06:00 para bares). Vendas entre a virada anterior
            e este horario sao agrupadas no mesmo periodo.
          </p>
          <div className="form-grid">
            <TextInput
              label="Inicio do dia fiscal (HH:MM)"
              value={fiscalStart}
              onChange={(e) => setFiscalStart(e.target.value)}
              placeholder="00:00"
            />
          </div>
          <div className="form-actions">
            <Button type="button" variant="secondary" loading={busy} onClick={salvarDiaFiscal}>
              Salvar dia fiscal
            </Button>
          </div>
        </section>
      ) : null}

      {onOpenGuide ? (
        <div className="form-actions">
          <Button type="button" variant="ghost" icon={<HelpCircle size={18} />} onClick={onOpenGuide}>
            Como funciona? Ver guia rápido
          </Button>
        </div>
      ) : null}
    </section>
  );
}
