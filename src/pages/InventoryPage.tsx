import { useState } from "react";
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
      <div className="product-list">
        {products.map((product) => (
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
      </div>
    </section>
  );
}
