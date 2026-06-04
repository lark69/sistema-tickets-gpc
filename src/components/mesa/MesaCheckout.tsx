import { Banknote, CreditCard, Landmark, Percent, QrCode } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { ContaMesa, FormaPagamento, MesaDetailed } from "../../types";
import { centsToInput, currencyToCents, formatCurrency } from "../../utils/currency";
import { mesaService } from "../../services/mesaService";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

interface MesaCheckoutProps {
  details: MesaDetailed;
  closing: boolean;
  onClose: () => void;
  onConfirm: (
    formaPagamento: FormaPagamento,
    valorCents: number,
    aplicarAcrescimo: boolean
  ) => Promise<void>;
}

export function MesaCheckout({ details, closing, onClose, onConfirm }: MesaCheckoutProps) {
  const [conta, setConta] = useState<ContaMesa | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [valorReceber, setValorReceber] = useState(centsToInput(details.subtotalCents));
  const [aplicarAcrescimo, setAplicarAcrescimo] = useState(false);

  useEffect(() => {
    let active = true;
    mesaService
      .getContaMesa(details.mesa.id)
      .then((c) => {
        if (active) setConta(c);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [details.mesa.id, reloadToken]);

  const total = details.subtotalCents;
  const pago = conta?.pagoCents ?? 0;
  const saldo = Math.max(0, total - pago);

  // ao (re)carregar a conta, sugere o saldo como valor a receber
  useEffect(() => {
    setValorReceber(centsToInput(saldo));
    setAplicarAcrescimo(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conta]);

  const valorCents = currencyToCents(valorReceber);
  const acrescimoCents = aplicarAcrescimo ? Math.round(valorCents * 0.05) : 0;
  const trocoCents = Math.max(0, valorCents - saldo);
  const restante = Math.max(0, saldo - valorCents);
  const excedeSaldo = valorCents > saldo;
  const elapsed = useMemo(() => {
    if (!details.sessao?.tempoInicio) return "00:00:00";
    return formatElapsed(Date.now() - details.sessao.tempoInicio);
  }, [details.sessao?.tempoInicio]);

  async function pay(forma: FormaPagamento) {
    if (valorCents <= 0) return;
    if (forma !== "dinheiro" && valorCents > saldo) return;
    await onConfirm(forma, valorCents, forma === "credito" ? aplicarAcrescimo : false);
    setReloadToken((token) => token + 1);
  }

  return (
    <Modal title={`Fechar Mesa ${String(details.mesa.numero).padStart(2, "0")}`} onClose={onClose}>
      <div className="checkout-content">
        <section className="checkout-summary">
          <strong>Mesa {String(details.mesa.numero).padStart(2, "0")}</strong>
          {details.sessao?.nomeCliente ? <span>Cliente: {details.sessao.nomeCliente}</span> : null}
          <span>Tempo: {elapsed}</span>
          {details.sessao?.idUnico ? <span>ID Único da Mesa: {details.sessao.idUnico}</span> : null}
        </section>

        <div className="checkout-table">
          {details.produtos.map((item) => (
            <div key={item.id} className="checkout-row">
              <span>{item.produto.name}</span>
              <span>x{item.quantidade}</span>
              <span>{formatCurrency(item.produto.priceCents)}</span>
              <strong>{formatCurrency(item.subtotalCents)}</strong>
            </div>
          ))}
        </div>

        {conta && conta.pagamentos.length > 0 ? (
          <div className="checkout-table">
            {conta.pagamentos.map((p) => (
              <div key={p.id} className="checkout-row">
                <span>Recebido ({p.formaPagamento})</span>
                <strong>- {formatCurrency(p.valorCents)}</strong>
              </div>
            ))}
          </div>
        ) : null}

        <div className="checkout-total">TOTAL: {formatCurrency(total)}</div>
        {pago > 0 ? (
          <div className="payment-breakdown">
            <span>Pago: {formatCurrency(pago)}</span>
            <strong>Saldo devedor: {formatCurrency(saldo)}</strong>
          </div>
        ) : null}

        <TextInput
          label="Valor a receber"
          value={valorReceber}
          onChange={(event) => setValorReceber(event.target.value)}
          placeholder={centsToInput(saldo)}
          inputMode="decimal"
        />

        <div className="payment-breakdown">
          {restante > 0 ? <span>Pagamento parcial — saldo restante: {formatCurrency(restante)}</span> : null}
          {trocoCents > 0 ? <strong>Troco (dinheiro): {formatCurrency(trocoCents)}</strong> : null}
          {excedeSaldo ? <span>PIX, Débito e Crédito não podem passar do saldo (use Dinheiro para troco).</span> : null}
        </div>

        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <Button
            type="button"
            variant={aplicarAcrescimo ? "primary" : "secondary"}
            icon={<Percent size={16} />}
            onClick={() => setAplicarAcrescimo((value) => !value)}
          >
            {aplicarAcrescimo ? "Acréscimo 5% ativo" : "Acrescentar 5% (crédito)"}
          </Button>
          {aplicarAcrescimo ? (
            <span>
              + {formatCurrency(acrescimoCents)} • cobrar no crédito: {formatCurrency(valorCents + acrescimoCents)}
            </span>
          ) : null}
        </div>

        <div className="payment-grid">
          <Button
            icon={<QrCode size={18} />}
            loading={closing}
            disabled={valorCents <= 0 || excedeSaldo}
            onClick={() => pay("pix")}
          >
            PIX
          </Button>
          <Button
            icon={<Banknote size={18} />}
            variant="secondary"
            loading={closing}
            disabled={valorCents <= 0}
            onClick={() => pay("dinheiro")}
          >
            Dinheiro
          </Button>
          <Button
            icon={<Landmark size={18} />}
            loading={closing}
            disabled={valorCents <= 0 || excedeSaldo}
            onClick={() => pay("debito")}
          >
            Débito
          </Button>
          <Button
            icon={<CreditCard size={18} />}
            variant="secondary"
            loading={closing}
            disabled={valorCents <= 0 || excedeSaldo}
            onClick={() => pay("credito")}
          >
            Crédito
          </Button>
        </div>
      </div>
    </Modal>
  );
}

function formatElapsed(durationMillis: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMillis / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
