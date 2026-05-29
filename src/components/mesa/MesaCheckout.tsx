import { Banknote, CreditCard, Landmark, QrCode, X } from "lucide-react";
import { useMemo, useState } from "react";
import type { FormaPagamento, MesaDetailed } from "../../types";
import { centsToInput, currencyToCents, formatCurrency } from "../../utils/currency";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

interface MesaCheckoutProps {
  details: MesaDetailed;
  closing: boolean;
  onClose: () => void;
  onConfirm: (formaPagamento: FormaPagamento, valorPagoCents?: number | null) => Promise<void>;
}

export function MesaCheckout({ details, closing, onClose, onConfirm }: MesaCheckoutProps) {
  const [cashModal, setCashModal] = useState(false);
  const [creditModal, setCreditModal] = useState(false);
  const [cashValue, setCashValue] = useState("");
  const total = details.subtotalCents;
  const creditFee = Math.round(total * 0.05);
  const creditTotal = total + creditFee;
  const paidCents = currencyToCents(cashValue);
  const changeCents = Math.max(0, paidCents - total);
  const elapsed = useMemo(() => {
    if (!details.sessao?.tempoInicio) return "00:00:00";
    return formatElapsed(Date.now() - details.sessao.tempoInicio);
  }, [details.sessao?.tempoInicio]);

  async function confirmCash() {
    if (paidCents < total) {
      return;
    }
    await onConfirm("dinheiro", paidCents);
  }

  return (
    <>
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

          <div className="checkout-total">TOTAL: {formatCurrency(total)}</div>

          <div className="payment-grid">
            <Button icon={<QrCode size={18} />} loading={closing} onClick={() => onConfirm("pix")}>
              PIX
            </Button>
            <Button icon={<Banknote size={18} />} variant="secondary" onClick={() => setCashModal(true)}>
              Dinheiro
            </Button>
            <Button icon={<Landmark size={18} />} loading={closing} onClick={() => onConfirm("debito")}>
              Débito
            </Button>
            <Button icon={<CreditCard size={18} />} variant="secondary" onClick={() => setCreditModal(true)}>
              Crédito
            </Button>
          </div>
        </div>
      </Modal>

      {cashModal ? (
        <Modal title="Pagamento em dinheiro" onClose={() => setCashModal(false)}>
          <div className="payment-modal-content">
            <TextInput
              label="Quanto o cliente pagou em dinheiro?"
              value={cashValue}
              onChange={(event) => setCashValue(event.target.value)}
              placeholder={centsToInput(total)}
              autoFocus
            />
            <div className="payment-breakdown">
              <span>Valor da mesa: {formatCurrency(total)}</span>
              <span>Valor pago: {formatCurrency(paidCents)}</span>
              <strong>Troco: {formatCurrency(changeCents)}</strong>
            </div>
            <div className="form-actions">
              <Button loading={closing} disabled={paidCents < total} onClick={confirmCash}>
                Confirmar
              </Button>
              <Button variant="secondary" icon={<X size={18} />} onClick={() => setCashModal(false)}>
                Cancelar
              </Button>
            </div>
          </div>
        </Modal>
      ) : null}

      {creditModal ? (
        <Modal title="Pagamento no crédito" onClose={() => setCreditModal(false)}>
          <div className="payment-modal-content">
            <div className="payment-breakdown">
              <span>Subtotal: {formatCurrency(total)}</span>
              <span>Acréscimo (5%): {formatCurrency(creditFee)}</span>
              <strong>TOTAL COM ACRÉSCIMO: {formatCurrency(creditTotal)}</strong>
            </div>
            <div className="form-actions">
              <Button loading={closing} onClick={() => onConfirm("credito")}>
                Confirmar
              </Button>
              <Button variant="secondary" icon={<X size={18} />} onClick={() => setCreditModal(false)}>
                Cancelar
              </Button>
            </div>
          </div>
        </Modal>
      ) : null}
    </>
  );
}

function formatElapsed(durationMillis: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMillis / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
