import { Banknote, CreditCard, HandPlatter, Landmark, QrCode, X } from "lucide-react";
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
  onConfirm: (
    formaPagamento: FormaPagamento,
    valorPagoCents?: number | null,
    aplicarAcrescimo?: boolean,
    aplicarGarcom?: boolean
  ) => Promise<void>;
}

export function CashierCheckout({ items, closing, onClose, onConfirm }: CashierCheckoutProps) {
  const [cashModal, setCashModal] = useState(false);
  const [creditModal, setCreditModal] = useState(false);
  const [cashValue, setCashValue] = useState("");
  const [aplicarGarcom, setAplicarGarcom] = useState(false);
  const total = items.reduce((sum, item) => sum + item.product.priceCents * item.quantidade, 0);
  const garcomFee = aplicarGarcom ? Math.round(total * 0.1) : 0;
  const creditFee = Math.round(total * 0.05);
  // Dinheiro/PIX/Débito pagam total + garçom; Crédito soma também os 5%.
  const totalComGarcom = total + garcomFee;
  const creditTotal = total + creditFee + garcomFee;
  const paidCents = currencyToCents(cashValue);
  const changeCents = Math.max(0, paidCents - totalComGarcom);

  async function confirmCash() {
    if (paidCents < totalComGarcom) {
      return;
    }
    await onConfirm("dinheiro", paidCents, false, aplicarGarcom);
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

          {/* Taxa de garçom (10%) — vale para qualquer forma de pagamento */}
          <div className="checkout-fees">
            <Button
              type="button"
              variant={aplicarGarcom ? "primary" : "secondary"}
              icon={<HandPlatter size={16} />}
              onClick={() => setAplicarGarcom((value) => !value)}
            >
              {aplicarGarcom ? "Garçom 10% ativo" : "Adicionar 10% do garçom"}
            </Button>
            {garcomFee > 0 ? (
              <span>+ {formatCurrency(garcomFee)} • total {formatCurrency(totalComGarcom)}</span>
            ) : null}
          </div>

          <div className="payment-grid">
            <Button
              icon={<QrCode size={18} />}
              loading={closing}
              onClick={() => onConfirm("pix", null, false, aplicarGarcom)}
            >
              PIX
            </Button>
            <Button icon={<Banknote size={18} />} variant="secondary" onClick={() => setCashModal(true)}>
              Dinheiro
            </Button>
            <Button
              icon={<Landmark size={18} />}
              loading={closing}
              onClick={() => onConfirm("debito", null, false, aplicarGarcom)}
            >
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
              placeholder={centsToInput(totalComGarcom)}
              autoFocus
            />
            <div className="payment-breakdown">
              <span>Valor da venda: {formatCurrency(total)}</span>
              {garcomFee > 0 ? <span>Garçom (10%): {formatCurrency(garcomFee)}</span> : null}
              <span>Total a pagar: {formatCurrency(totalComGarcom)}</span>
              <span>Valor pago: {formatCurrency(paidCents)}</span>
              <strong>Troco: {formatCurrency(changeCents)}</strong>
            </div>
            <div className="form-actions">
              <Button loading={closing} disabled={paidCents < totalComGarcom} onClick={confirmCash}>
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
              {garcomFee > 0 ? <span>Garçom (10%): {formatCurrency(garcomFee)}</span> : null}
              <strong>TOTAL COM ACRESCIMO: {formatCurrency(creditTotal)}</strong>
            </div>
            <div className="form-actions">
              <Button loading={closing} onClick={() => onConfirm("credito", null, true, aplicarGarcom)}>
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
