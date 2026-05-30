import { useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { CashMovement, CashRegister, LocalUser } from "../types";
import { centsToInput, currencyToCents, formatCurrency } from "../utils/currency";
import { getErrorMessage } from "../utils/errors";

interface CashRegisterPageProps {
  currentUser: LocalUser;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function CashRegisterPage({ currentUser, onMessage }: CashRegisterPageProps) {
  const [register, setRegister] = useState<CashRegister | null>(null);
  const [movements, setMovements] = useState<CashMovement[]>([]);
  const [initialBalance, setInitialBalance] = useState("0,00");
  const [finalCounted, setFinalCounted] = useState("");
  const [movementValue, setMovementValue] = useState("");
  const [note, setNote] = useState("");

  async function load() {
    setRegister(await adminService.getCurrentCashRegister());
    setMovements(await adminService.listCashMovements());
  }

  async function openCash() {
    try {
      await adminService.openCashRegister(currencyToCents(initialBalance), currentUser.username);
      await load();
      onMessage("Caixa aberto com sucesso.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  async function closeCash() {
    try {
      const closed = await adminService.closeCashRegister(currencyToCents(finalCounted), currentUser.username);
      await load();
      onMessage(`Caixa fechado. Diferença: ${formatCurrency(closed.differenceCents ?? 0)}`, "info");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  async function movement(type: "sangria" | "suprimento") {
    try {
      await adminService.addCashMovement({
        movementType: type,
        amountCents: currencyToCents(movementValue),
        note,
        operatorName: currentUser.username
      });
      setMovementValue("");
      setNote("");
      await load();
      onMessage("Movimento registrado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  useEffect(() => {
    load().catch((err) => onMessage(getErrorMessage(err), "error"));
  }, []);

  return (
    <section className="page-stack">
      <div className="section-heading">
        <span>Caixa</span>
        <h1>Frente de caixa</h1>
        <p>Abra o caixa, registre sangrias/suprimentos e feche com conferência.</p>
      </div>
      <section className="settings-section">
        {register ? (
          <>
            <h2>Caixa aberto</h2>
            <p>Saldo inicial: {formatCurrency(register.initialBalanceCents)}</p>
            <div className="form-grid">
              <TextInput label="Valor do movimento" value={movementValue} onChange={(e) => setMovementValue(e.target.value)} />
              <TextInput label="Observação" value={note} onChange={(e) => setNote(e.target.value)} />
            </div>
            <div className="form-actions">
              <Button variant="secondary" onClick={() => movement("sangria")}>Sangria</Button>
              <Button onClick={() => movement("suprimento")}>Suprimento</Button>
            </div>
            <TextInput label="Valor contado no fechamento" value={finalCounted} onChange={(e) => setFinalCounted(e.target.value)} placeholder={centsToInput(register.initialBalanceCents)} />
            <Button variant="danger" onClick={closeCash}>Fechar caixa</Button>
          </>
        ) : (
          <>
            <h2>Abrir caixa</h2>
            <TextInput label="Saldo inicial" value={initialBalance} onChange={(e) => setInitialBalance(e.target.value)} />
            <Button onClick={openCash}>Abrir caixa</Button>
          </>
        )}
      </section>
      <section className="logs-table">
        <div className="logs-row logs-row-head"><span>Data</span><span>Tipo</span><span>Valor</span></div>
        {movements.map((item) => (
          <div className="logs-row" key={item.id}>
            <span>{new Date(item.createdAt).toLocaleString("pt-BR")}</span>
            <span>{item.movementType}</span>
            <span>{formatCurrency(item.amountCents)} · {item.operatorName}</span>
          </div>
        ))}
      </section>
    </section>
  );
}
