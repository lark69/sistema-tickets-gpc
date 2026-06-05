import { Minus, Plus, Search, ShoppingCart, Trash2, Wallet } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CashierCheckout, type CashierCartItem } from "../components/cashier/CashierCheckout";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import { cashierService } from "../services/cashierService";
import type {
  CashierStatus,
  CashMovement,
  FormaPagamento,
  LocalUser,
  Product,
  TurnoOperacional
} from "../types";
import { centsToInput, currencyToCents, formatCurrency } from "../utils/currency";
import { getErrorMessage } from "../utils/errors";

interface CashRegisterPageProps {
  currentUser: LocalUser;
  products: Product[];
  canManageCash: boolean;
  canManageCashMovements: boolean;
  onProductsChanged: () => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
  onNavigateFechamento?: () => void;
}

export function CashRegisterPage({
  currentUser,
  products,
  canManageCash,
  canManageCashMovements,
  onProductsChanged,
  onMessage,
  onNavigateFechamento
}: CashRegisterPageProps) {
  const [status, setStatus] = useState<CashierStatus | null>(null);
  const [movements, setMovements] = useState<CashMovement[]>([]);
  const [query, setQuery] = useState("");
  const [cart, setCart] = useState<CashierCartItem[]>([]);
  const [checkoutOpen, setCheckoutOpen] = useState(false);
  const [closingSale, setClosingSale] = useState(false);
  const [movementValue, setMovementValue] = useState("");
  const [note, setNote] = useState("");

  const turno: TurnoOperacional | null = status?.turnoAtivo ?? null;
  const esperadoCents = status?.esperadoAtualCents ?? turno?.saldoInicialCents ?? 0;

  const rankedProducts = useMemo(() => {
    return [...products]
      .sort((a, b) => {
        if (b.soldQuantity !== a.soldQuantity) {
          return b.soldQuantity - a.soldQuantity;
        }
        return a.name.localeCompare(b.name, "pt-BR");
      })
      .map((product, index) => ({ product, rank: index + 1 }));
  }, [products]);

  const filteredProducts = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("pt-BR");

    if (!normalized) {
      return rankedProducts;
    }

    return rankedProducts.filter(({ product }) => {
      const idLabel = String(product.id).padStart(3, "0");
      const searchable = [
        product.name,
        product.categoryName ?? "",
        product.barcode ?? "",
        idLabel,
        String(product.id),
        formatCurrency(product.priceCents),
        centsToInput(product.priceCents)
      ]
        .join(" ")
        .toLocaleLowerCase("pt-BR");

      return searchable.includes(normalized);
    });
  }, [query, rankedProducts]);

  const cartTotal = cart.reduce((sum, item) => sum + item.product.priceCents * item.quantidade, 0);

  async function load() {
    setStatus(await cashierService.getStatus());
    setMovements(await adminService.listCashMovements());
  }

  function addProduct(product: Product) {
    setCart((current) => {
      const existing = current.find((item) => item.product.id === product.id);
      if (existing) {
        return current.map((item) =>
          item.product.id === product.id ? { ...item, quantidade: item.quantidade + 1 } : item
        );
      }
      return [...current, { product, quantidade: 1 }];
    });
  }

  function removeOne(productId: number) {
    setCart((current) =>
      current
        .map((item) =>
          item.product.id === productId ? { ...item, quantidade: item.quantidade - 1 } : item
        )
        .filter((item) => item.quantidade > 0)
    );
  }

  function removeItem(productId: number) {
    setCart((current) => current.filter((item) => item.product.id !== productId));
  }

  function quantityFor(productId: number) {
    return cart.find((item) => item.product.id === productId)?.quantidade ?? 0;
  }

  async function movement(type: "sangria" | "suprimento") {
    if (!canManageCashMovements) {
      onMessage("Usuario sem permissao para registrar sangria ou suprimento.", "error");
      return;
    }

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

  async function finishSale(
    formaPagamento: FormaPagamento,
    valorPagoCents?: number | null,
    aplicarAcrescimo = false,
    aplicarGarcom = false
  ) {
    if (!turno) {
      onMessage("Abra um turno na area de Fechamento antes de vender.", "error");
      return;
    }
    if (!canManageCash) {
      onMessage("Usuario sem permissao para finalizar venda direta.", "error");
      return;
    }

    setClosingSale(true);
    try {
      const ticket = await adminService.fecharVendaCaixa({
        formaPagamento,
        valorPagoCents,
        aplicarAcrescimo,
        aplicarGarcom,
        operatorName: currentUser.username,
        items: cart.map((item) => ({
          productId: item.product.id,
          quantidade: item.quantidade
        }))
      });
      setCart([]);
      setCheckoutOpen(false);
      await onProductsChanged();
      await load();
      onMessage(`Venda finalizada: ${formatCurrency(ticket.totalCents)}.`, "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setClosingSale(false);
    }
  }

  useEffect(() => {
    load().catch((err) => onMessage(getErrorMessage(err), "error"));
  }, []);

  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Frente de caixa</span>
          <h1>Venda direta</h1>
          <p>Pesquise por ID, categoria, valor ou nome, adicione ao carrinho e finalize o pagamento.</p>
        </div>
        <div className="cash-register-pill">
          <Wallet size={18} />
          {turno ? `Turno aberto · ${formatCurrency(esperadoCents)}` : "Sem turno aberto"}
        </div>
      </div>

      <div className="cashier-grid">
        <section className="settings-section cashier-products-panel">
          <TextInput
            label="Pesquisar produtos"
            value={query}
            icon={<Search size={18} />}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="ID, categoria, valor ou nome"
          />

          <div className="cashier-product-list">
            {filteredProducts.map(({ product, rank }) => {
              const quantity = quantityFor(product.id);
              return (
                <button
                  key={product.id}
                  type="button"
                  className={`cashier-product ${product.stock <= quantity ? "cashier-product-warning" : ""}`}
                  onClick={() => addProduct(product)}
                >
                  <span className="cashier-product-rank">#{rank}</span>
                  <span className="cashier-product-main">
                    <strong>{product.name}</strong>
                    <small>{product.categoryName || "Sem categoria"} · {product.soldQuantity} vendido(s)</small>
                  </span>
                  <span>{formatCurrency(product.priceCents)}</span>
                  <span>ESTQ. {product.stock}</span>
                  <span>ID:{String(product.id).padStart(3, "0")}</span>
                  {quantity > 0 ? <em>{quantity}</em> : null}
                </button>
              );
            })}
            {filteredProducts.length === 0 ? <p className="muted-text">Nenhum produto encontrado.</p> : null}
          </div>
        </section>

        <aside className="settings-section cashier-cart-panel">
          <div className="cart-title">
            <ShoppingCart size={20} />
            <h2>Carrinho</h2>
          </div>

          <div className="cart-list">
            {cart.length === 0 ? (
              <p className="muted-text">Nenhum produto adicionado.</p>
            ) : (
              cart.map((item) => (
                <div key={item.product.id} className="cart-row">
                  <span>
                    <strong>{item.product.name}</strong>
                    <small>{formatCurrency(item.product.priceCents)} · x{item.quantidade}</small>
                  </span>
                  <strong>{formatCurrency(item.product.priceCents * item.quantidade)}</strong>
                  <button type="button" className="icon-button" onClick={() => removeOne(item.product.id)}>
                    <Minus size={16} />
                  </button>
                  <button type="button" className="icon-button" onClick={() => addProduct(item.product)}>
                    <Plus size={16} />
                  </button>
                  <button type="button" className="icon-button" onClick={() => removeItem(item.product.id)}>
                    <Trash2 size={16} />
                  </button>
                </div>
              ))
            )}
          </div>

          <div className="checkout-total">TOTAL: {formatCurrency(cartTotal)}</div>
          <Button
            icon={<ShoppingCart size={18} />}
            disabled={cart.length === 0 || !turno || !canManageCash}
            onClick={() => setCheckoutOpen(true)}
          >
            Finalizar venda
          </Button>
          {!turno ? (
            <p className="danger-text">
              Nenhum turno aberto.{" "}
              {onNavigateFechamento ? (
                <button type="button" className="link-button" onClick={onNavigateFechamento}>
                  Abrir turno em Fechamento
                </button>
              ) : (
                "Abra um turno na area de Fechamento."
              )}
            </p>
          ) : null}
          {turno && !canManageCash ? <p className="danger-text">Usuario sem permissao para finalizar venda direta.</p> : null}
        </aside>
      </div>

      <section className="settings-section">
        {turno ? (
          <>
            <h2>Movimentos do turno</h2>
            <p>
              Operador: <strong>{turno.operador}</strong> · Fundo de troco:{" "}
              <strong>{formatCurrency(turno.saldoInicialCents)}</strong> · Esperado na gaveta:{" "}
              <strong>{formatCurrency(esperadoCents)}</strong>
            </p>
            <div className="form-grid">
              <TextInput label="Valor do movimento" value={movementValue} onChange={(e) => setMovementValue(e.target.value)} />
              <TextInput label="Observação" value={note} onChange={(e) => setNote(e.target.value)} />
            </div>
            <div className="form-actions">
              <Button type="button" variant="secondary" disabled={!canManageCashMovements} onClick={() => movement("sangria")}>Sangria</Button>
              <Button type="button" disabled={!canManageCashMovements} onClick={() => movement("suprimento")}>Suprimento</Button>
            </div>
            <p className="muted-text">
              A abertura e o fechamento do turno (contagem da gaveta) sao feitos na area de Fechamento.
            </p>
          </>
        ) : (
          <>
            <h2>Sem turno aberto</h2>
            <p className="muted-text">
              Para vender, registrar sangrias ou suprimentos, abra um turno na area de Fechamento.
            </p>
            {onNavigateFechamento ? (
              <div className="form-actions">
                <Button type="button" onClick={onNavigateFechamento}>Ir para Fechamento</Button>
              </div>
            ) : null}
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

      {checkoutOpen ? (
        <CashierCheckout
          items={cart}
          closing={closingSale}
          onClose={() => setCheckoutOpen(false)}
          onConfirm={finishSale}
        />
      ) : null}
    </section>
  );
}
