import { ExternalLink, Moon, Printer, RefreshCw, Save, Sun } from "lucide-react";
import { FormEvent, useEffect, useState } from "react";
import { Button } from "../components/ui/Button";
import { Select } from "../components/ui/Select";
import { TextInput } from "../components/ui/TextInput";
import { configService } from "../services/configService";
import { printerService } from "../services/printerService";
import type { AppConfig, PrinterInfo } from "../types";
import { getErrorMessage } from "../utils/errors";
import { validateConfig } from "../utils/validation";

interface SettingsPageProps {
  config: AppConfig;
  saving: boolean;
  onSave: (config: AppConfig) => Promise<void>;
  onMessage: (message: string, tone: "success" | "error" | "info") => void;
}

export function SettingsPage({ config, saving, onSave, onMessage }: SettingsPageProps) {
  const [draft, setDraft] = useState(config);
  const [printers, setPrinters] = useState<PrinterInfo[]>([]);
  const [loadingPrinters, setLoadingPrinters] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(config);
  }, [config]);

  async function loadPrinters() {
    setLoadingPrinters(true);

    try {
      const availablePrinters = await printerService.listPrinters();
      setPrinters(availablePrinters);

      if (availablePrinters.length === 0) {
        onMessage("Nenhuma impressora foi encontrada no Windows.", "info");
      }
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    } finally {
      setLoadingPrinters(false);
    }
  }

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

  async function handleOpenPortfolio() {
    try {
      await configService.openCreatorPortfolio();
    } catch (err) {
      onMessage(getErrorMessage(err), "error");
    }
  }

  const printerOptions = [
    { value: "", label: "Usar impressora padrão do Windows" },
    ...printers.map((printer) => ({
      value: printer.name,
      label: printer.isDefault ? `${printer.name} (padrão)` : printer.name
    }))
  ];

  return (
    <form className="page-stack settings-page" onSubmit={handleSubmit}>
      <div className="section-heading">
        <span>Configurações</span>
        <h1>Preferências do sistema</h1>
        <p>Ajuste empresa, tema e impressora térmica.</p>
      </div>

      <section className="settings-section">
        <div className="settings-section-title">
          <h2>Tema</h2>
          <p>Escolha a aparência da interface.</p>
        </div>
        <div className="segmented-control">
          <button
            type="button"
            className={draft.theme === "light" ? "active" : ""}
            onClick={() => setDraft((current) => ({ ...current, theme: "light" }))}
          >
            <Sun size={18} />
            Claro
          </button>
          <button
            type="button"
            className={draft.theme === "dark" ? "active" : ""}
            onClick={() => setDraft((current) => ({ ...current, theme: "dark" }))}
          >
            <Moon size={18} />
            Escuro
          </button>
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-title">
          <h2>Informações</h2>
          <p>Dados impressos no ticket térmico.</p>
        </div>
        <div className="form-grid">
          <TextInput
            label="Nome da empresa"
            value={draft.companyName}
            onChange={(event) =>
              setDraft((current) => ({ ...current, companyName: event.target.value }))
            }
          />
          <TextInput
            label="CPF/CNPJ"
            value={draft.taxId}
            onChange={(event) =>
              setDraft((current) => ({ ...current, taxId: event.target.value }))
            }
          />
        </div>
        <TextInput
          label="Frase de agradecimento"
          value={draft.thankYouMessage ?? ""}
          onChange={(event) =>
            setDraft((current) => ({ ...current, thankYouMessage: event.target.value }))
          }
          placeholder="Obrigado pela preferência"
        />
        <TextInput
          label="Tempo de validade"
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
        />
        <div className="form-grid">
          <TextInput
            label="Quantidade de mesas"
            type="number"
            min={1}
            max={100}
            value={draft.tableCount}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                tableCount: Number.parseInt(event.target.value, 10) || 40
              }))
            }
            hint="Portex PDV permite configurar de 1 a 100 mesas."
          />
          <TextInput
            label="Horário diário de backup"
            type="time"
            value={draft.backupTime ?? "23:00"}
            onChange={(event) =>
              setDraft((current) => ({ ...current, backupTime: event.target.value }))
            }
            hint="O backup automático roda nesse horário enquanto o app estiver aberto. O backup manual fica em Relatórios."
          />
        </div>
      </section>

      <section className="settings-section">
        <div className="settings-section-title">
          <h2>Impressora</h2>
          <p>Padrão otimizado para Elgin i8 em modo ESC/POS RAW. Clique em Atualizar para carregar as impressoras do Windows.</p>
        </div>
        <div className="printer-row">
          <Select
            label="Selecionar impressora"
            value={draft.printerName ?? ""}
            onChange={(event) =>
              setDraft((current) => ({ ...current, printerName: event.target.value || null }))
            }
            options={printerOptions}
            hint="Instale o driver da Elgin i8 no Windows antes de imprimir."
          />
          <Button
            type="button"
            variant="secondary"
            icon={<RefreshCw size={18} />}
            loading={loadingPrinters}
            onClick={loadPrinters}
          >
            Atualizar
          </Button>
        </div>
        <TextInput
          label="Largura de impressão"
          type="number"
          min={32}
          max={64}
          value={draft.printWidthChars}
          icon={<Printer size={18} />}
          onChange={(event) =>
            setDraft((current) => ({
              ...current,
              printWidthChars: Number.parseInt(event.target.value, 10) || 48
            }))
          }
          hint="Use 48 caracteres para Elgin i8 80mm. Ajuste se usar fonte/papel diferente."
        />
      </section>

      <section className="settings-section about-section">
        <div className="settings-section-title">
          <h2>About</h2>
          <p>Criado por Gabriel Portela Carmo.</p>
        </div>
        <p className="about-text">
          Desenvolvedor focado em soluções digitais práticas, interfaces profissionais e sistemas
          orientados a produtividade para negócios locais.
        </p>
        <div className="about-action">
          <span>Saber mais sobre o criador</span>
          <Button
            type="button"
            variant="secondary"
            icon={<ExternalLink size={18} />}
            onClick={handleOpenPortfolio}
          >
            Abrir portfólio
          </Button>
        </div>
      </section>

      {error ? <div className="inline-alert">{error}</div> : null}

      <div className="sticky-actions">
        <Button type="submit" icon={<Save size={18} />} loading={saving}>
          Salvar configurações
        </Button>
      </div>
    </form>
  );
}
