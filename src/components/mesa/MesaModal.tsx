import { Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import type { FormaPagamento, MesaDetailed, Product } from "../../types";
import { formatCurrency } from "../../utils/currency";
import { Button } from "../ui/Button";
import { Modal } from "../ui/Modal";
import { TextInput } from "../ui/TextInput";
import { MesaCheckout } from "./MesaCheckout";

interface DraftItem {
  product: Product;
  quantidade: number;
}

interface MesaModalProps {
  details: MesaDetailed;
  products: Product[];
  saving: boolean;
  closing: boolean;
  cashOpen: boolean;
  canAddProducts: boolean;
  canRemoveProducts: boolean;
  canCloseTable: boolean;
  onCancel: () => void;
  onSave: (items: DraftItem[], nomeCliente: string) => Promise<void>;
  onCheckout: (
    items: DraftItem[],
    nomeCliente: string,
    formaPagamento: FormaPagamento,
    valorPagoCents: number,
    aplicarAcrescimo: boolean
  ) => Promise<void>;
}

export function MesaModal({
  details,
  products,
  saving,
  closing,
  cashOpen,
  canAddProducts,
  canRemoveProducts,
  canCloseTable,
  onCancel,
  onSave,
  onCheckout
}: MesaModalProps) {
  const [query, setQuery] = useState("");
  const [nomeCliente, setNomeCliente] = useState(details.sessao?.nomeCliente ?? "");
  const [checkoutOpen, setCheckoutOpen] = useState(false);
  const [items, setItems] = useState<DraftItem[]>(
    details.produtos.map((item) => ({
      product: item.produto,
      quantidade: item.quantidade
    }))
  );

  const filteredProducts = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("pt-BR");
    if (!normalized) return products;
    return products.filter((product) => product.name.toLocaleLowerCase("pt-BR").includes(normalized));
  }, [products, query]);

  const subtotal = items.reduce((sum, item) => sum + item.product.priceCents * item.quantidade, 0);
  const canUseCatalog = cashOpen && canAddProducts;

  function addProduct(product: Product) {
    if (!canUseCatalog) return;

    setItems((current) => {
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
    if (!canRemoveProducts) return;

    setItems((current) =>
      current
        .map((item) =>
          item.product.id === productId ? { ...item, quantidade: item.quantidade - 1 } : item
        )
        .filter((item) => item.quantidade > 0)
    );
  }

  function quantityFor(productId: number) {
    return items.find((item) => item.product.id === productId)?.quantidade ?? 0;
  }

  const checkoutDetails: MesaDetailed = {
    ...details,
    produtos: items.map((item, index) => ({
      id: index + 1,
      idMesa: details.mesa.id,
      idProduto: item.product.id,
      quantidade: item.quantidade,
      adicionadoEm: Date.now(),
      produto: item.product,
      subtotalCents: item.product.priceCents * item.quantidade
    })),
    subtotalCents: subtotal,
    sessao: details.sessao
      ? { ...details.sessao, nomeCliente }
      : details.sessao
  };

  return (
    <>
      <Modal
        title={`Mesa ${String(details.mesa.numero).padStart(2, "0")}`}
        onClose={onCancel}
        footer={
          <>
            {items.length > 0 ? (
              <Button
                type="button"
                variant="secondary"
                disabled={!cashOpen || !canCloseTable}
                onClick={() => setCheckoutOpen(true)}
              >
                Fechar Mesa
              </Button>
            ) : null}
            <Button type="button" loading={saving} onClick={() => onSave(items, nomeCliente)}>
              Salvar
            </Button>
            <Button type="button" variant="secondary" onClick={onCancel}>
              Cancelar
            </Button>
          </>
        }
      >
        <div className="mesa-modal-grid">
          <section className="mesa-column">
            <h3>Produtos adicionados</h3>
            {!cashOpen ? (
              <div className="inline-alert">Nenhum turno aberto. Abra um turno em Fechamento para adicionar produtos ou fechar a mesa.</div>
            ) : null}
            {cashOpen && !canAddProducts ? (
              <div className="inline-alert">Usuario sem permissao para adicionar produtos a mesa.</div>
            ) : null}
            <div className="mesa-items">
              {items.length === 0 ? (
                <p className="muted-text">Nenhum produto adicionado</p>
              ) : (
                items.map((item) => (
                  <div key={item.product.id} className="mesa-item-row">
                    <span>{item.product.name}</span>
                    <span>x{item.quantidade}</span>
                    <span>{formatCurrency(item.product.priceCents)}</span>
                    <strong>{formatCurrency(item.product.priceCents * item.quantidade)}</strong>
                    <button
                      type="button"
                      className="icon-button"
                      disabled={!canRemoveProducts}
                      onClick={() => removeOne(item.product.id)}
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                ))
              )}
            </div>
            <div className="mesa-subtotal">Subtotal: {formatCurrency(subtotal)}</div>
            <TextInput
              label="Nome do Cliente (opcional)"
              placeholder="Ex: João Silva"
              value={nomeCliente}
              onChange={(event) => setNomeCliente(event.target.value)}
            />
          </section>

          <section className="mesa-column">
            <h3>Catálogo de Produtos</h3>
            <TextInput
              label="Buscar produto"
              value={query}
              icon={<Search size={18} />}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Filtrar por nome"
            />
            <div className="catalog-list">
              {filteredProducts.map((product) => {
                const quantity = quantityFor(product.id);
                return (
                  <button
                    key={product.id}
                    type="button"
                    className={`catalog-item ${product.stock <= quantity ? "catalog-item-warning" : ""}`}
                    disabled={!canUseCatalog}
                    onClick={() => addProduct(product)}
                  >
                    <span>{product.name}</span>
                    <strong>{formatCurrency(product.priceCents)}</strong>
                    {product.stock <= quantity ? <small>Estoque negativo</small> : null}
                    {quantity > 0 ? <em>{quantity}</em> : null}
                  </button>
                );
              })}
            </div>
          </section>
        </div>
      </Modal>

      {checkoutOpen ? (
        <MesaCheckout
          details={checkoutDetails}
          closing={closing}
          onClose={() => setCheckoutOpen(false)}
          onConfirm={(formaPagamento, valorPagoCents, aplicarAcrescimo) =>
            onCheckout(items, nomeCliente, formaPagamento, valorPagoCents, aplicarAcrescimo)
          }
        />
      ) : null}
    </>
  );
}
