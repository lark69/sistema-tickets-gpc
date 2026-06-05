import { useEffect, useState } from "react";
import { MesaCard } from "../components/mesa/MesaCard";
import { MesaModal } from "../components/mesa/MesaModal";
import { cashierService } from "../services/cashierService";
import { mesaService } from "../services/mesaService";
import type { FormaPagamento, LocalUser, Mesa, MesaDetailed, Product } from "../types";
import { getErrorMessage } from "../utils/errors";
import { formatCurrency } from "../utils/currency";
import { hasPermission } from "../utils/permissions";

interface DraftItem {
  product: Product;
  quantidade: number;
}

interface MesasDashboardPageProps {
  products: Product[];
  currentUser: LocalUser | null;
  operatorName: string;
  onProductsChanged: () => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function MesasDashboardPage({
  products,
  currentUser,
  operatorName,
  onProductsChanged,
  onMessage
}: MesasDashboardPageProps) {
  const [mesas, setMesas] = useState<Mesa[]>([]);
  const [details, setDetails] = useState<MesaDetailed | null>(null);
  const [loading, setLoading] = useState(true);
  const [cashOpen, setCashOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [closing, setClosing] = useState(false);
  const [now, setNow] = useState(Date.now());

  async function loadMesas() {
    setLoading(true);
    try {
      const [nextMesas, status] = await Promise.all([
        mesaService.listMesas(),
        cashierService.getStatus()
      ]);
      setMesas(nextMesas);
      setCashOpen(Boolean(status.turnoAtivo));
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoading(false);
    }
  }

  async function openMesa(mesa: Mesa) {
    try {
      const [nextDetails, status] = await Promise.all([
        mesaService.getDetails(mesa.id),
        cashierService.getStatus()
      ]);
      setCashOpen(Boolean(status.turnoAtivo));
      setDetails(nextDetails);
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  async function saveMesa(items: DraftItem[], nomeCliente: string) {
    if (!details) return;
    setSaving(true);
    try {
      const saved = await mesaService.saveMesa({
        idMesa: details.mesa.id,
        nomeCliente,
        produtos: items.map((item) => ({
          idMesa: details.mesa.id,
          idProduto: item.product.id,
          quantidade: item.quantidade
        }))
      });
      setDetails(null);
      await loadMesas();
      onMessage(
        saved.produtos.length > 0 ? "Mesa salva com sucesso." : "Mesa limpa com sucesso.",
        "success"
      );
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function checkoutMesa(
    items: DraftItem[],
    nomeCliente: string,
    formaPagamento: FormaPagamento,
    valorPagoCents?: number | null,
    aplicarAcrescimo = false,
    aplicarGarcom = false
  ) {
    if (!details) return;
    if (!cashOpen) {
      onMessage("Abra um turno antes de fechar uma mesa.", "error");
      return;
    }
    setClosing(true);
    try {
      await mesaService.saveMesa({
        idMesa: details.mesa.id,
        nomeCliente,
        produtos: items.map((item) => ({
          idMesa: details.mesa.id,
          idProduto: item.product.id,
          quantidade: item.quantidade
        }))
      });
      const resultado = await mesaService.registrarPagamento({
        idMesa: details.mesa.id,
        formaPagamento,
        valorCents: valorPagoCents ?? 0,
        aplicarAcrescimo,
        aplicarGarcom,
        operatorName
      });
      if (resultado.finalizada) {
        setDetails(null);
        await loadMesas();
        await onProductsChanged();
        const numero = resultado.ticket?.numeroMesa ?? details.mesa.numero;
        onMessage(`Mesa ${String(numero).padStart(2, "0")} fechada com sucesso.`, "success");
      } else {
        const atualizado = await mesaService.getDetails(details.mesa.id);
        setDetails(atualizado);
        await loadMesas();
        onMessage(
          `Pagamento parcial recebido. Saldo devedor: ${formatCurrency(resultado.saldoRestanteCents)}.`,
          "info"
        );
      }
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setClosing(false);
    }
  }

  useEffect(() => {
    loadMesas().catch(() => undefined);
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, []);

  return (
    <section className="page-stack">
      <div className="section-heading">
        <span>PDV</span>
        <h1>Dashboard de mesas</h1>
        <p>Selecione uma mesa para adicionar produtos, acompanhar permanência e fechar o pagamento.</p>
      </div>

      {loading ? (
        <div className="empty-state"><h2>Carregando mesas...</h2></div>
      ) : (
        <div className="mesas-grid">
          {mesas.map((mesa) => (
            <MesaCard
              key={mesa.id}
              mesa={mesa}
              now={now}
              onClick={openMesa}
            />
          ))}
        </div>
      )}

      {details ? (
        <MesaModal
          details={details}
          products={products}
          saving={saving}
          closing={closing}
          cashOpen={cashOpen}
          canAddProducts={hasPermission(currentUser, "addTableProducts")}
          canRemoveProducts={hasPermission(currentUser, "removeTableProducts")}
          canCloseTable={hasPermission(currentUser, "closeTable")}
          onCancel={() => setDetails(null)}
          onSave={saveMesa}
          onCheckout={checkoutMesa}
        />
      ) : null}
    </section>
  );
}
