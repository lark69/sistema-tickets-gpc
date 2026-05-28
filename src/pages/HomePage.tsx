import { Plus } from "lucide-react";
import { EmptyState } from "../components/ui/EmptyState";
import { Button } from "../components/ui/Button";
import { ProductCard } from "../components/products/ProductCard";
import type { Product } from "../types";

interface HomePageProps {
  products: Product[];
  onAdd: () => void;
  onPrint: (product: Product) => void;
}

export function HomePage({ products, onAdd, onPrint }: HomePageProps) {
  if (products.length === 0) {
    return (
      <EmptyState
        title="Você ainda não tem nenhum produto adicionado. ;-("
        action={
          <Button icon={<Plus size={18} />} onClick={onAdd}>
            Adicionar produtos
          </Button>
        }
      />
    );
  }

  return (
    <section className="page-stack">
      <div className="section-heading">
        <span>Home</span>
        <h1>Produtos cadastrados</h1>
        <p>Selecione um produto para imprimir tickets rapidamente.</p>
      </div>

      <div className="product-grid">
        {products.map((product) => (
          <ProductCard key={product.id} product={product} compact onPrint={onPrint} />
        ))}
      </div>
    </section>
  );
}
