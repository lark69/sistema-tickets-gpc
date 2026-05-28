import { BadgeCheck, SearchCheck } from "lucide-react";
import { FormEvent, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { printerService } from "../services/printerService";
import type { VerifyTicketResult } from "../types";
import { getErrorMessage } from "../utils/errors";

interface VerifyTicketPageProps {
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function VerifyTicketPage({ onMessage }: VerifyTicketPageProps) {
  const [ticketId, setTicketId] = useState("");
  const [checking, setChecking] = useState(false);
  const [result, setResult] = useState<VerifyTicketResult | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!ticketId.trim()) {
      onMessage("Informe o ID do ticket.", "error");
      return;
    }

    setChecking(true);

    try {
      const verification = await printerService.verifyTicket(ticketId);
      setResult(verification);
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setChecking(false);
    }
  }

  return (
    <section className="page-stack verify-page">
      <div className="section-heading">
        <span>Verificação</span>
        <h1>Verificar veracidade do ticket</h1>
        <p>Digite o ID impresso no ticket para confirmar se ele ainda existe e está dentro da validade configurada.</p>
      </div>

      <form className="verify-panel" onSubmit={handleSubmit}>
        <TextInput
          label="ID do ticket"
          value={ticketId}
          onChange={(event) => {
            setTicketId(event.target.value);
            setResult(null);
          }}
          placeholder="Ex: A7K92B"
          icon={<BadgeCheck size={18} />}
          autoFocus
        />

        <Button type="submit" icon={<SearchCheck size={18} />} loading={checking}>
          Verificar ticket
        </Button>
      </form>

      {result ? (
        <section className={`verification-result ${result.valid ? "valid" : "invalid"}`}>
          <strong>{result.message}</strong>
          <span>ID consultado: {result.ticketId}</span>
        </section>
      ) : null}
    </section>
  );
}
