import { useEffect, useState } from "react";
import { MesaCard } from "../components/mesa/MesaCard";
import { MesaModal } from "../components/mesa/MesaModal";
import { mesaService } from "../services/mesaService";
import type { FormaPagamento, Mesa, MesaDetailed, Product } from "../types";
import { getErrorMessage } from "../utils/errors";

interface DraftItem {
  product: Product;
  quantidade: number;
}

interface MesasDashboardPageProps {
  products: Product[];
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function MesasDashboardPage({ products, onMessage }: MesasDashboardPageProps) {
  const [mesas, setMesas] = useState<Mesa[]>([]);
  const [details, setDetails] = useState<MesaDetailed | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [closing, setClosing] = useState(false);
  const [now, setNow] = useState(Date.now());

  async function loadMesas() {
    setLoading(true);
    try {
      setMesas(await mesaService.listMesas());
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoading(false);
    }
  }

  async function openMesa(mesa: Mesa) {
    try {
      setDetails(await mesaService.getDetails(mesa.id));
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
    valorPagoCents?: number | null
  ) {
    if (!details) return;
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
      const ticket = await mesaService.fecharMesa({
        idMesa: details.mesa.id,
        formaPagamento,
        valorPagoCents
      });
      setDetails(null);
      await loadMesas();
      onMessage(`Mesa ${String(ticket.numeroMesa).padStart(2, "0")} fechada com sucesso.`, "success");
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
          onCancel={() => setDetails(null)}
          onSave={saveMesa}
          onCheckout={checkoutMesa}
        />
      ) : null}
    </section>
  );
}
