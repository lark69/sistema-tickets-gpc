import { useEffect, useMemo, useState } from "react";
import { AppLayout } from "./components/layout/AppLayout";
import { ProductForm } from "./components/products/ProductForm";
import { PrintModal } from "./components/products/PrintModal";
import { Toast } from "./components/ui/Toast";
import { useAppData } from "./hooks/useAppData";
import { useTheme } from "./hooks/useTheme";
import { InitialSetupPage } from "./pages/InitialSetupPage";
import { DashboardPage } from "./pages/DashboardPage";
import { LogsPage } from "./pages/LogsPage";
import { MesasDashboardPage } from "./pages/MesasDashboardPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { SettingsPage } from "./pages/SettingsPage";
import { VerifyTicketPage } from "./pages/VerifyTicketPage";
import { configService } from "./services/configService";
import { printerService } from "./services/printerService";
import { productService } from "./services/productService";
import type { AppConfig, AppRoute, Product, ProductInput, ToastState } from "./types";
import { getErrorMessage } from "./utils/errors";

type ShellMode = "boot" | "onboarding" | "setup" | "app";

export function App() {
  const { config, setConfig, products, loading, error, refreshProducts } = useAppData();
  const [mode, setMode] = useState<ShellMode>("boot");
  const [route, setRoute] = useState<AppRoute>("home");
  const [saving, setSaving] = useState(false);
  const [printing, setPrinting] = useState(false);
  const [editingProduct, setEditingProduct] = useState<Product | null>(null);
  const [printProduct, setPrintProduct] = useState<Product | null>(null);
  const [toast, setToast] = useState<ToastState | null>(null);

  useTheme(config.theme);

  useEffect(() => {
    if (loading || error || mode !== "boot") {
      return;
    }

    if (!config.onboardingCompleted) {
      setMode("onboarding");
      return;
    }

    if (!config.setupCompleted) {
      setMode("setup");
      return;
    }

    setMode("app");
    setRoute("home");
  }, [config.onboardingCompleted, config.setupCompleted, error, loading, mode]);

  function showMessage(message: string, tone: ToastState["tone"] = "info") {
    setToast({ message, tone });
    window.setTimeout(() => setToast(null), 3800);
  }

  async function handleFinishOnboarding() {
    setSaving(true);

    try {
      const nextConfig = await configService.completeOnboarding();
      setConfig(nextConfig);
      setMode("setup");
    } catch (err) {
      showMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function persistConfig(nextConfig: AppConfig) {
    setSaving(true);

    try {
      const savedConfig = await configService.saveConfig({
        ...nextConfig,
        onboardingCompleted: true,
        setupCompleted: true
      });
      setConfig(savedConfig);
      showMessage("Configurações salvas com sucesso.", "success");
      return savedConfig;
    } catch (err) {
      showMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleInitialSetup(nextConfig: AppConfig) {
    const savedConfig = await persistConfig(nextConfig);

    if (savedConfig) {
      setMode("app");
      setRoute("dashboard");
    }
  }

  async function handleSaveSettings(nextConfig: AppConfig) {
    await persistConfig(nextConfig);
  }

  async function handleSaveProduct(input: ProductInput) {
    setSaving(true);

    try {
      if (editingProduct) {
        await productService.update({ ...input, id: editingProduct.id });
        showMessage("Produto atualizado com sucesso.", "success");
      } else {
        await productService.create(input);
        showMessage("Produto criado com sucesso.", "success");
      }

      await refreshProducts();
      setEditingProduct(null);
      setRoute("dashboard");
    } catch (err) {
      showMessage(getErrorMessage(err), "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleDeleteProduct(product: Product) {
    const confirmed = window.confirm(`Excluir o produto "${product.name}"?`);

    if (!confirmed) {
      return;
    }

    try {
      await productService.remove(product.id);
      await refreshProducts();
      showMessage("Produto excluído com sucesso.", "success");
    } catch (err) {
      showMessage(getErrorMessage(err), "error");
    }
  }

  async function handlePrint(quantity: number) {
    if (!printProduct) {
      return;
    }

    setPrinting(true);

    try {
      const result = await printerService.printTickets({
        productId: printProduct.id,
        quantity
      });
      setPrintProduct(null);
      showMessage(`${result.printed} ticket(s) enviado(s) para ${result.printerName}.`, "success");
    } catch (err) {
      showMessage(getErrorMessage(err), "error");
    } finally {
      setPrinting(false);
    }
  }

  const content = useMemo(() => {
    if (route === "new-product") {
      return (
        <ProductForm
          saving={saving}
          onSubmit={handleSaveProduct}
          onCancel={() => {
            setEditingProduct(null);
            setRoute("dashboard");
          }}
        />
      );
    }

    if (route === "edit-product" && editingProduct) {
      return (
        <ProductForm
          product={editingProduct}
          saving={saving}
          onSubmit={handleSaveProduct}
          onCancel={() => {
            setEditingProduct(null);
            setRoute("dashboard");
          }}
        />
      );
    }

    if (route === "dashboard") {
      return (
        <DashboardPage
          products={products}
          onAdd={() => {
            setEditingProduct(null);
            setRoute("new-product");
          }}
          onEdit={(product) => {
            setEditingProduct(product);
            setRoute("edit-product");
          }}
          onDelete={handleDeleteProduct}
          onPrint={setPrintProduct}
        />
      );
    }

    if (route === "settings") {
      return (
        <SettingsPage
          config={config}
          saving={saving}
          onSave={handleSaveSettings}
          onMessage={showMessage}
        />
      );
    }

    if (route === "verify-ticket") {
      return <VerifyTicketPage onMessage={showMessage} />;
    }

    if (route === "logs") {
      return <LogsPage onMessage={showMessage} />;
    }

    return (
      <MesasDashboardPage
        products={products}
        onMessage={showMessage}
      />
    );
  }, [config, editingProduct, products, route, saving]);

  const topbarRoute: AppRoute =
    route === "new-product" || route === "edit-product" ? "dashboard" : route;

  if (loading || mode === "boot") {
    return (
      <main className="boot-screen">
        <div className="boot-card">
          <span className="brand-symbol">GPC</span>
          <strong>Sistema de Tickets GPC</strong>
          <p>{error ?? "Carregando dados locais..."}</p>
        </div>
      </main>
    );
  }

  return (
    <>
      {mode === "onboarding" ? (
        <OnboardingPage saving={saving} onFinish={handleFinishOnboarding} />
      ) : mode === "setup" ? (
        <InitialSetupPage config={config} saving={saving} onSave={handleInitialSetup} />
      ) : (
        <AppLayout route={topbarRoute} onNavigate={setRoute}>
          {content}
        </AppLayout>
      )}

      {printProduct ? (
        <PrintModal
          product={printProduct}
          printing={printing}
          onClose={() => setPrintProduct(null)}
          onPrint={handlePrint}
        />
      ) : null}

      <Toast toast={toast} onDismiss={() => setToast(null)} />
    </>
  );
}
