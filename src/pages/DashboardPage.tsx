import { Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { CategoryManager } from "../components/products/CategoryManager";
import { ProductCard } from "../components/products/ProductCard";
import { Button } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { TextInput } from "../components/ui/TextInput";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import type { Category, LocalUser, Product } from "../types";

interface DashboardPageProps {
  products: Product[];
  categories: Category[];
  canManage?: boolean;
  canManageTickets?: boolean;
  operatorName?: string;
  requester?: LocalUser | null;
  onAdd: () => void;
  onEdit: (product: Product) => void;
  onDelete: (product: Product) => void;
  onPrint: (product: Product) => void;
  onCategoriesChanged: () => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function DashboardPage({
  products,
  categories,
  canManage = true,
  canManageTickets = true,
  operatorName,
  requester,
  onAdd,
  onEdit,
  onDelete,
  onPrint,
  onCategoriesChanged,
  onMessage
}: DashboardPageProps) {
  const [query, setQuery] = useState("");
  const debouncedQuery = useDebouncedValue(query);

  const filteredProducts = useMemo(() => {
    const normalizedQuery = debouncedQuery.trim().toLocaleLowerCase("pt-BR");

    if (!normalizedQuery) {
      return products;
    }

    return products.filter((product) => {
      const searchable = `${product.name} ${product.description ?? ""}`.toLocaleLowerCase("pt-BR");
      return searchable.includes(normalizedQuery);
    });
  }, [debouncedQuery, products]);

  return (
    <section className="page-stack">
      <div className="dashboard-header">
        <div className="section-heading">
          <span>Dashboard</span>
          <h1>Gestao de produtos</h1>
          <p>Pesquise, edite, exclua e imprima tickets a partir da sua lista local.</p>
        </div>
        {canManage ? (
          <Button icon={<Plus size={18} />} onClick={onAdd}>
            Novo produto
          </Button>
        ) : null}
      </div>

      <div className="toolbar">
        <TextInput
          label="Pesquisar produtos"
          value={query}
          icon={<Search size={18} />}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Digite nome ou descrição"
        />
      </div>

      {products.length === 0 ? (
        <EmptyState
          title="Você ainda não tem nenhum produto adicionado. ;-("
          action={
            canManage ? (
              <Button icon={<Plus size={18} />} onClick={onAdd}>
                Adicionar produtos
              </Button>
            ) : null
          }
        />
      ) : filteredProducts.length === 0 ? (
        <EmptyState title="Nenhum produto encontrado para essa busca." />
      ) : (
        <div className="product-list">
          {filteredProducts.map((product) => (
            <ProductCard
              key={product.id}
              product={product}
              onEdit={canManage ? onEdit : undefined}
              onDelete={canManage ? onDelete : undefined}
              onPrint={canManageTickets ? onPrint : undefined}
            />
          ))}
        </div>
      )}

      {canManage ? (
        <CategoryManager
          categories={categories}
          operatorName={operatorName}
          requester={requester}
          onChanged={onCategoriesChanged}
          onMessage={onMessage}
        />
      ) : null}
    </section>
  );
}
