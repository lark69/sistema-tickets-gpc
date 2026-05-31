import { Save, X } from "lucide-react";
import { FormEvent, useEffect, useMemo, useState } from "react";
import { adminService } from "../../services/adminService";
import type { Category, LocalUser, Product, ProductFormState, ProductInput, ProductUnit } from "../../types";
import { centsToInput, currencyToCents } from "../../utils/currency";
import { getErrorMessage } from "../../utils/errors";
import { validateProductForm } from "../../utils/validation";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";

interface ProductFormProps {
  product?: Product | null;
  categories?: Category[];
  operatorName?: string;
  requester?: LocalUser | null;
  onCategoryCreated?: (category: Category) => void;
  onMessage?: (message: string, tone: "success" | "error" | "info") => void;
  saving: boolean;
  onCancel: () => void;
  onSubmit: (input: ProductInput) => Promise<void>;
}

export function ProductForm({
  product,
  categories = [],
  operatorName,
  requester,
  onCategoryCreated,
  onMessage,
  saving,
  onCancel,
  onSubmit
}: ProductFormProps) {
  const initialState = useMemo<ProductFormState>(
    () => ({
      name: product?.name ?? "",
      barcode: product?.barcode ?? "",
      costPrice: product ? centsToInput(product.costPriceCents) : "",
      markupPercent: "",
      unit: product?.unit ?? "UN",
      categoryId: product?.categoryId ? String(product.categoryId) : "",
      stock: product ? String(product.stock) : "0",
      reorderLevel: product ? String(product.reorderLevel) : "0",
      price: product ? centsToInput(product.priceCents) : "",
      description: product?.description ?? ""
    }),
    [product]
  );

  const [form, setForm] = useState<ProductFormState>(initialState);
  const [localCategories, setLocalCategories] = useState(categories);
  const [newCategory, setNewCategory] = useState("");
  const [creatingCategory, setCreatingCategory] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLocalCategories(categories);
  }, [categories]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validationError = validateProductForm(form);

    if (validationError) {
      setError(validationError);
      return;
    }

    setError(null);
    await onSubmit({
      name: form.name,
      priceCents: currencyToCents(form.price),
      barcode: form.barcode || null,
      costPriceCents: currencyToCents(form.costPrice),
      unit: form.unit,
      categoryId: form.categoryId ? Number.parseInt(form.categoryId, 10) : null,
      stock: Number.parseInt(form.stock, 10) || 0,
      reorderLevel: Number.parseInt(form.reorderLevel, 10) || 0,
      description: form.description || null
    });
  }

  function applyMarkup() {
    const cost = currencyToCents(form.costPrice);
    const markup = Number.parseFloat(form.markupPercent.replace(",", "."));
    if (cost <= 0 || Number.isNaN(markup)) return;
    const nextPrice = Math.round(cost * (1 + markup / 100));
    setForm((current) => ({ ...current, price: centsToInput(nextPrice) }));
  }

  async function handleCreateCategory() {
    const name = newCategory.trim();
    if (!name) {
      setError("Informe o nome da categoria.");
      return;
    }

    setCreatingCategory(true);
    try {
      const category = await adminService.createCategory(name, operatorName, requester);
      setLocalCategories((current) => [...current, category]);
      setForm((current) => ({ ...current, categoryId: String(category.id) }));
      setNewCategory("");
      setError(null);
      onCategoryCreated?.(category);
      onMessage?.("Categoria criada com sucesso.", "success");
    } catch (err) {
      onMessage?.(getErrorMessage(err), "error");
    } finally {
      setCreatingCategory(false);
    }
  }

  return (
    <form className="form-panel product-form" onSubmit={handleSubmit}>
      <div className="section-heading">
        <span>{product ? "Editar produto" : "Novo produto"}</span>
        <h1>{product ? "Atualize os dados do produto" : "Cadastre um produto"}</h1>
      </div>

      <TextInput
        label="Nome do produto"
        value={form.name}
        onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
        placeholder="Ex: Pastel de carne"
        autoFocus
      />

      <div className="form-grid">
        <TextInput
          label="Código de barras"
          value={form.barcode}
          onChange={(event) => setForm((current) => ({ ...current, barcode: event.target.value }))}
          placeholder="Opcional"
        />
        <label className="field">
          <span className="field-label">Unidade</span>
          <span className="field-control">
            <select
              value={form.unit}
              onChange={(event) =>
                setForm((current) => ({ ...current, unit: event.target.value as ProductUnit }))
              }
            >
              <option value="UN">UN</option>
              <option value="KG">KG</option>
              <option value="L">L</option>
              <option value="CX">CX</option>
              <option value="PCT">PCT</option>
            </select>
          </span>
        </label>
      </div>

      <div className="form-grid">
        <TextInput
          label="Custo em R$"
          value={form.costPrice}
          onChange={(event) => setForm((current) => ({ ...current, costPrice: event.target.value }))}
          placeholder="0,00"
          inputMode="decimal"
        />
        <TextInput
          label="Markup %"
          value={form.markupPercent}
          onChange={(event) => setForm((current) => ({ ...current, markupPercent: event.target.value }))}
          onBlur={applyMarkup}
          placeholder="Ex: 30"
          inputMode="decimal"
        />
      </div>

      <div className="form-grid">
        <TextInput
          label="Valor de venda em R$"
          value={form.price}
          onChange={(event) => setForm((current) => ({ ...current, price: event.target.value }))}
          placeholder="0,00"
          inputMode="decimal"
        />
        <label className="field">
          <span className="field-label">Categoria</span>
          <span className="field-control">
            <select
              value={form.categoryId}
              onChange={(event) => setForm((current) => ({ ...current, categoryId: event.target.value }))}
            >
              <option value="">Sem categoria</option>
              {localCategories.map((category) => (
                <option key={category.id} value={category.id}>
                  {category.name}
                </option>
              ))}
            </select>
          </span>
        </label>
      </div>

      <div className="category-create-row">
        <TextInput
          label="Nova categoria"
          value={newCategory}
          onChange={(event) => setNewCategory(event.target.value)}
          placeholder="Ex: Bebidas"
        />
        <Button
          type="button"
          variant="secondary"
          loading={creatingCategory}
          onClick={handleCreateCategory}
        >
          Criar categoria
        </Button>
      </div>

      <div className="form-grid">
        <TextInput
          label="Estoque"
          type="number"
          value={form.stock}
          onChange={(event) => setForm((current) => ({ ...current, stock: event.target.value }))}
        />
        <TextInput
          label="Alerta mínimo"
          type="number"
          value={form.reorderLevel}
          onChange={(event) => setForm((current) => ({ ...current, reorderLevel: event.target.value }))}
        />
      </div>

      <TextInput
        label="Descrição (opcional)"
        value={form.description}
        onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))}
        placeholder="Detalhe curto para identificar o produto"
        multiline
        rows={4}
      />

      {error ? <div className="inline-alert">{error}</div> : null}

      <div className="form-actions">
        <Button type="submit" icon={<Save size={18} />} loading={saving}>
          Salvar
        </Button>
        <Button type="button" variant="secondary" icon={<X size={18} />} onClick={onCancel}>
          Cancelar
        </Button>
      </div>
    </form>
  );
}
