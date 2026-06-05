import { Banknote, CreditCard, HandPlatter, Landmark, Percent, QrCode } from "lucide-react";
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
    aplicarAcrescimo: boolean,
    aplicarGarcom: boolean
  ) => Promise<void>;
}

const FORMAS: Array<{ forma: FormaPagamento; label: string; icon: typeof QrCode }> = [
  { forma: "pix", label: "PIX", icon: QrCode },
  { forma: "dinheiro", label: "Dinheiro", icon: Banknote },
  { forma: "debito", label: "Débito", icon: Landmark },
  { forma: "credito", label: "Crédito", icon: CreditCard }
];

export function MesaCheckout({ details, closing, onClose, onConfirm }: MesaCheckoutProps) {
  const [conta, setConta] = useState<ContaMesa | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [valorReceber, setValorReceber] = useState(centsToInput(details.subtotalCents));
  const [selectedForma, setSelectedForma] = useState<FormaPagamento | null>(null);
  const [aplicarAcrescimo, setAplicarAcrescimo] = useState(false);
  const [aplicarGarcom, setAplicarGarcom] = useState(false);

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

  // ao (re)carregar a conta, sugere o saldo como valor a receber e zera opções
  useEffect(() => {
    setValorReceber(centsToInput(saldo));
    setSelectedForma(null);
    setAplicarAcrescimo(false);
    setAplicarGarcom(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [conta]);

  const valorCents = currencyToCents(valorReceber);
  const acrescimoCreditoCents =
    selectedForma === "credito" && aplicarAcrescimo ? Math.round(valorCents * 0.05) : 0;
  const acrescimoGarcomCents = aplicarGarcom ? Math.round(valorCents * 0.1) : 0;
  const acrescimoCents = acrescimoCreditoCents + acrescimoGarcomCents;
  const totalCobrar = valorCents + acrescimoCents;
  const trocoCents = Math.max(0, valorCents - saldo);
  const restante = Math.max(0, saldo - valorCents);
  const excedeSaldo = valorCents > saldo;
  const naoDinheiroExcede = selectedForma !== null && selectedForma !== "dinheiro" && excedeSaldo;
  const elapsed = useMemo(() => {
    if (!details.sessao?.tempoInicio) return "00:00:00";
    return formatElapsed(Date.now() - details.sessao.tempoInicio);
  }, [details.sessao?.tempoInicio]);

  async function pay() {
    if (!selectedForma) return;
    if (valorCents <= 0) return;
    if (selectedForma !== "dinheiro" && valorCents > saldo) return;
    await onConfirm(
      selectedForma,
      valorCents,
      selectedForma === "credito" ? aplicarAcrescimo : false,
      aplicarGarcom
    );
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

        {/* 1) Escolher a forma de pagamento */}
        <span className="field-label">Forma de pagamento</span>
        <div className="payment-grid">
          {FORMAS.map(({ forma, label, icon: Icon }) => (
            <Button
              key={forma}
              type="button"
              icon={<Icon size={18} />}
              variant={selectedForma === forma ? "primary" : "secondary"}
              disabled={valorCents <= 0 || (forma !== "dinheiro" && excedeSaldo)}
              onClick={() => {
                setSelectedForma(forma);
                if (forma !== "credito") setAplicarAcrescimo(false);
              }}
            >
              {label}
            </Button>
          ))}
        </div>

        {/* 2) Acréscimos: 5% só no crédito; garçom 10% sempre */}
        <div className="checkout-fees">
          {selectedForma === "credito" ? (
            <Button
              type="button"
              variant={aplicarAcrescimo ? "primary" : "secondary"}
              icon={<Percent size={16} />}
              onClick={() => setAplicarAcrescimo((value) => !value)}
            >
              {aplicarAcrescimo ? "Acréscimo 5% ativo" : "Acrescentar 5% (crédito)"}
            </Button>
          ) : null}
          <Button
            type="button"
            variant={aplicarGarcom ? "primary" : "secondary"}
            icon={<HandPlatter size={16} />}
            onClick={() => setAplicarGarcom((value) => !value)}
          >
            {aplicarGarcom ? "Garçom 10% ativo" : "Adicionar 10% do garçom"}
          </Button>
        </div>

        <div className="payment-breakdown">
          {acrescimoCents > 0 ? (
            <span>
              Acréscimo: + {formatCurrency(acrescimoCents)}
              {acrescimoGarcomCents > 0 ? ` (garçom ${formatCurrency(acrescimoGarcomCents)})` : ""}
              {acrescimoCreditoCents > 0 ? ` (crédito ${formatCurrency(acrescimoCreditoCents)})` : ""}
            </span>
          ) : null}
          {acrescimoCents > 0 ? <strong>Total a cobrar: {formatCurrency(totalCobrar)}</strong> : null}
          {restante > 0 ? <span>Pagamento parcial — saldo restante: {formatCurrency(restante)}</span> : null}
          {trocoCents > 0 ? <strong>Troco (dinheiro): {formatCurrency(trocoCents)}</strong> : null}
          {naoDinheiroExcede ? (
            <span>PIX, Débito e Crédito não podem passar do saldo (use Dinheiro para troco).</span>
          ) : null}
        </div>

        {/* 3) Confirmar */}
        <Button
          type="button"
          loading={closing}
          disabled={!selectedForma || valorCents <= 0 || naoDinheiroExcede}
          onClick={pay}
        >
          {selectedForma
            ? `Confirmar ${formatCurrency(totalCobrar)} em ${formaLabel(selectedForma)}`
            : "Escolha a forma de pagamento"}
        </Button>
      </div>
    </Modal>
  );
}

function formaLabel(forma: FormaPagamento): string {
  return FORMAS.find((f) => f.forma === forma)?.label ?? forma;
}

function formatElapsed(durationMillis: number): string {
  const totalSeconds = Math.max(0, Math.floor(durationMillis / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}
