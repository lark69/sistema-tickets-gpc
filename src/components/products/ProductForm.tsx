import { Save, X } from "lucide-react";
import { FormEvent, useMemo, useState } from "react";
import type { Product, ProductFormState, ProductInput } from "../../types";
import { centsToInput, currencyToCents } from "../../utils/currency";
import { validateProductForm } from "../../utils/validation";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";

interface ProductFormProps {
  product?: Product | null;
  saving: boolean;
  onCancel: () => void;
  onSubmit: (input: ProductInput) => Promise<void>;
}

export function ProductForm({ product, saving, onCancel, onSubmit }: ProductFormProps) {
  const initialState = useMemo<ProductFormState>(
    () => ({
      name: product?.name ?? "",
      price: product ? centsToInput(product.priceCents) : "",
      description: product?.description ?? ""
    }),
    [product]
  );

  const [form, setForm] = useState<ProductFormState>(initialState);
  const [error, setError] = useState<string | null>(null);

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
      description: form.description || null
    });
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

      <TextInput
        label="Valor em R$"
        value={form.price}
        onChange={(event) => setForm((current) => ({ ...current, price: event.target.value }))}
        placeholder="0,00"
        inputMode="decimal"
      />

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
