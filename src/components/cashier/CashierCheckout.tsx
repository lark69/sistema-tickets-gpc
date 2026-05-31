import { Banknote, CreditCard, Landmark, QrCode, X } from "lucide-react";
import { useState } from "react";
import type { FormaPagamento, Product } from "../../types";
import { centsToInput, currencyToCents, formatCurrency } from "../../utils/currency";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";

export interface CashierCartItem {
  product: Product;
  quantidade: number;
}

interface CashierCheckoutProps {
  items: CashierCartItem[];
  closing: boolean;
  onClose: () => void;
  onConfirm: (formaPagamento: FormaPagamento, valorPagoCents?: number | null) => Promise<void>;
}

export function CashierCheckout({ items, closing, onClose, onConfirm }: CashierCheckoutProps) {
  const [cashModal, setCashModal] = useState(false);
  const [creditModal, setCreditModal] = useState(false);
  const [cashValue, setCashValue] = useState("");
  const total = items.reduce((sum, item) => sum + item.product.priceCents * item.quantidade, 0);
  const creditFee = Math.round(total * 0.05);
  const creditTotal = total + creditFee;
  const paidCents = currencyToCents(cashValue);
  const changeCents = Math.max(0, paidCents - total);

  async function confirmCash() {
    if (paidCents < total) {
      return;
    }
    await onConfirm("dinheiro", paidCents);
  }

  return (
    <>
      <Modal title="Finalizar venda" onClose={onClose}>
        <div className="checkout-content">
          <section className="checkout-summary">
            <strong>Venda de caixa</strong>
            <span>{items.length} item(ns) no carrinho</span>
          </section>

          <div className="checkout-table">
            {items.map((item) => (
              <div key={item.product.id} className="checkout-row">
                <span>{item.product.name}</span>
                <span>x{item.quantidade}</span>
                <span>{formatCurrency(item.product.priceCents)}</span>
                <strong>{formatCurrency(item.product.priceCents * item.quantidade)}</strong>
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
              Debito
            </Button>
            <Button icon={<CreditCard size={18} />} variant="secondary" onClick={() => setCreditModal(true)}>
              Credito
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
              <span>Valor da venda: {formatCurrency(total)}</span>
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
        <Modal title="Pagamento no credito" onClose={() => setCreditModal(false)}>
          <div className="payment-modal-content">
            <div className="payment-breakdown">
              <span>Subtotal: {formatCurrency(total)}</span>
              <span>Acrescimo (5%): {formatCurrency(creditFee)}</span>
              <strong>TOTAL COM ACRESCIMO: {formatCurrency(creditTotal)}</strong>
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
