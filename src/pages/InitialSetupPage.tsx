import { Save } from "lucide-react";
import { FormEvent, useState } from "react";
import { Button } from "../components/ui/Button";
import { TextInput } from "../components/ui/TextInput";
import type { AppConfig } from "../types";
import { validateConfig } from "../utils/validation";

interface InitialSetupPageProps {
  config: AppConfig;
  saving: boolean;
  onSave: (config: AppConfig) => Promise<void>;
}

export function InitialSetupPage({ config, saving, onSave }: InitialSetupPageProps) {
  const [draft, setDraft] = useState<AppConfig>({
    ...config,
    setupCompleted: true,
    onboardingCompleted: true
  });
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validationError = validateConfig(draft);

    if (validationError) {
      setError(validationError);
      return;
    }

    setError(null);
    await onSave(draft);
  }

  return (
    <main className="setup-screen">
      <form className="setup-panel" onSubmit={handleSubmit}>
        <div className="section-heading">
          <span>Configuração inicial</span>
          <h1>Prepare o sistema para sua empresa</h1>
          <p>Esses dados serão usados no cabeçalho e nas informações do ticket térmico.</p>
        </div>

        <div className="form-grid">
          <TextInput
            label="Nome da empresa"
            value={draft.companyName}
            onChange={(event) =>
              setDraft((current) => ({ ...current, companyName: event.target.value }))
            }
            placeholder="Ex: Portex Comercio"
            autoFocus
          />

          <TextInput
            label="CPF/CNPJ"
            value={draft.taxId}
            onChange={(event) =>
              setDraft((current) => ({ ...current, taxId: event.target.value }))
            }
            placeholder="00.000.000/0000-00"
          />
        </div>

        <TextInput
          label="Frase curta de agradecimento (opcional)"
          value={draft.thankYouMessage ?? ""}
          onChange={(event) =>
            setDraft((current) => ({ ...current, thankYouMessage: event.target.value }))
          }
          placeholder="Obrigado pela preferência"
        />

        <TextInput
          label="Tempo de validade do ticket"
          type="number"
          min={1}
          max={3650}
          value={draft.validityDays}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              validityDays: Number.parseInt(event.target.value, 10) || 1
            }))
          }
          hint="Informe a quantidade de dias de validade."
        />

        {error ? <div className="inline-alert">{error}</div> : null}

        <div className="form-actions">
          <Button type="submit" icon={<Save size={18} />} loading={saving}>
            Salvar e continuar
          </Button>
        </div>
      </form>
    </main>
  );
}
