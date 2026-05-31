import { Search } from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import { adminService } from "../services/adminService";
import type { LocalUser, Product } from "../types";
import { getErrorMessage } from "../utils/errors";

interface InventoryPageProps {
  products: Product[];
  currentUser: LocalUser;
  onRefresh: () => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function InventoryPage({ products, currentUser, onRefresh, onMessage }: InventoryPageProps) {
  const [quantities, setQuantities] = useState<Record<number, string>>({});
  const [query, setQuery] = useState("");

  const filteredProducts = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase("pt-BR");

    if (!normalized) {
      return products;
    }

    return products.filter((product) => {
      const searchable = [
        product.name,
        product.categoryName ?? "",
        product.barcode ?? "",
        String(product.id).padStart(3, "0"),
        String(product.id)
      ]
        .join(" ")
        .toLocaleLowerCase("pt-BR");

      return searchable.includes(normalized);
    });
  }, [products, query]);

  async function adjust(product: Product, movementType: "entrada" | "saida" | "ajuste") {
    try {
      await adminService.adjustStock({
        productId: product.id,
        quantity: Number.parseInt(quantities[product.id] || "0", 10),
        movementType,
        operatorName: currentUser.username,
        note: "Ajuste manual"
      });
      await onRefresh();
      onMessage("Estoque atualizado.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  return (
    <section className="page-stack">
      <div className="section-heading">
        <span>Estoque</span>
        <h1>Inventário</h1>
        <p>Produtos abaixo do mínimo aparecem destacados.</p>
      </div>
      <div className="toolbar">
        <TextInput
          label="Pesquisar no estoque"
          value={query}
          icon={<Search size={18} />}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Nome, categoria, codigo ou ID"
        />
      </div>
      <div className="product-list">
        {filteredProducts.map((product) => (
          <article key={product.id} className={`product-card ${product.stock <= product.reorderLevel ? "stock-low" : ""}`}>
            <div className="product-card-content">
              <h3>{product.name}</h3>
              <p>{product.categoryName || "Sem categoria"} · {product.unit} · Código: {product.barcode || "-"}</p>
              <strong>Estoque: {product.stock} / mínimo {product.reorderLevel}</strong>
              {product.stock < 0 ? <p className="danger-text">Estoque negativo – adicione mais produtos</p> : null}
            </div>
            <div className="product-card-actions">
              <TextInput
                label="Qtd"
                type="number"
                value={quantities[product.id] || ""}
                onChange={(event) => setQuantities((current) => ({ ...current, [product.id]: event.target.value }))}
              />
              <Button onClick={() => adjust(product, "entrada")}>Entrada</Button>
              <Button variant="secondary" onClick={() => adjust(product, "saida")}>Saída</Button>
              <Button variant="secondary" onClick={() => adjust(product, "ajuste")}>Ajustar para</Button>
            </div>
          </article>
        ))}
        {filteredProducts.length === 0 ? <div className="empty-state"><h2>Nenhum produto encontrado.</h2></div> : null}
      </div>
    </section>
  );
}
