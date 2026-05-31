import { Pencil, Plus, Save, Trash2, X } from "lucide-react";
import { FormEvent, useState } from "react";
import { adminService } from "../../services/adminService";
import type { Category, LocalUser } from "../../types";
import { getErrorMessage } from "../../utils/errors";
import { Button } from "../ui/Button";
import { TextInput } from "../ui/TextInput";

interface CategoryManagerProps {
  categories: Category[];
  operatorName?: string;
  requester?: LocalUser | null;
  onChanged: () => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function CategoryManager({
  categories,
  operatorName,
  requester,
  onChanged,
  onMessage
}: CategoryManagerProps) {
  const [name, setName] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editingName, setEditingName] = useState("");
  const [saving, setSaving] = useState(false);

  async function create(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);

    try {
      await adminService.createCategory(name, operatorName, requester);
      setName("");
      await onChanged();
      onMessage("Categoria criada.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function update(category: Category) {
    setSaving(true);

    try {
      await adminService.updateCategory(category.id, editingName, requester);
      setEditingId(null);
      setEditingName("");
      await onChanged();
      onMessage("Categoria atualizada.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function remove(category: Category) {
    const confirmed = window.confirm(`Excluir a categoria "${category.name}"? Os produtos dessa categoria ficarão sem categoria.`);

    if (!confirmed) {
      return;
    }

    setSaving(true);

    try {
      await adminService.deleteCategory(category.id, requester);
      await onChanged();
      onMessage("Categoria excluida.", "success");
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  return (
    <section className="settings-section">
      <div className="settings-section-title">
        <h2>Categorias</h2>
        <p>Adicione, altere ou exclua categorias usadas no cadastro de produtos.</p>
      </div>

      <form className="category-create-row" onSubmit={create}>
        <TextInput
          label="Nova categoria"
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="Ex: Bebidas"
        />
        <Button type="submit" icon={<Plus size={18} />} loading={saving}>
          Adicionar
        </Button>
      </form>

      <div className="category-list">
        {categories.map((category) => (
          <div key={category.id} className="category-row">
            {editingId === category.id ? (
              <TextInput
                label="Nome"
                value={editingName}
                onChange={(event) => setEditingName(event.target.value)}
                autoFocus
              />
            ) : (
              <span>{category.name}</span>
            )}

            <div className="category-actions">
              {editingId === category.id ? (
                <>
                  <Button type="button" icon={<Save size={16} />} loading={saving} onClick={() => update(category)}>
                    Salvar
                  </Button>
                  <Button
                    type="button"
                    variant="secondary"
                    icon={<X size={16} />}
                    onClick={() => {
                      setEditingId(null);
                      setEditingName("");
                    }}
                  >
                    Cancelar
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    type="button"
                    variant="secondary"
                    icon={<Pencil size={16} />}
                    onClick={() => {
                      setEditingId(category.id);
                      setEditingName(category.name);
                    }}
                  >
                    Editar
                  </Button>
                  <Button type="button" variant="danger" icon={<Trash2 size={16} />} loading={saving} onClick={() => remove(category)}>
                    Excluir
                  </Button>
                </>
              )}
            </div>
          </div>
        ))}
        {categories.length === 0 ? <p className="muted-text">Nenhuma categoria cadastrada.</p> : null}
      </div>
    </section>
  );
}
