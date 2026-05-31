import { Pencil, Printer, Trash2 } from "lucide-react";
import type { Product } from "../../types";
import { formatCurrency } from "../../utils/currency";
import { Button } from "../ui/Button";

interface ProductCardProps {
  product: Product;
  compact?: boolean;
  onPrint?: (product: Product) => void;
  onEdit?: (product: Product) => void;
  onDelete?: (product: Product) => void;
}

export function ProductCard({
  product,
  compact = false,
  onPrint,
  onEdit,
  onDelete
}: ProductCardProps) {
  return (
    <article className={`product-card ${compact ? "product-card-compact" : ""} ${product.stock <= product.reorderLevel ? "stock-low" : ""}`}>
      <div className="product-card-content">
        <h3>{product.name}</h3>
        <p>
          {product.categoryName || "Sem categoria"} · {product.unit} · Estoque: {product.stock}
        </p>
        {product.description ? <p>{product.description}</p> : null}
        {product.stock < 0 ? <p className="danger-text">Estoque negativo - adicione mais produtos</p> : null}
        <strong>{formatCurrency(product.priceCents)}</strong>
      </div>
      <div className="product-card-actions">
        {onPrint ? (
          <Button icon={<Printer size={17} />} onClick={() => onPrint(product)}>
            Imprimir
          </Button>
        ) : null}
        {onEdit ? (
          <Button variant="secondary" icon={<Pencil size={17} />} onClick={() => onEdit(product)}>
            Editar
          </Button>
        ) : null}
        {onDelete ? (
          <Button variant="danger" icon={<Trash2 size={17} />} onClick={() => onDelete(product)}>
            Excluir
          </Button>
        ) : null}
      </div>
    </article>
  );
}
