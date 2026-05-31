import { useCallback, useEffect, useMemo, useState } from "react";
import { DEFAULT_CONFIG } from "../database/contracts";
import { configService } from "../services/configService";
import { productService } from "../services/productService";
import type { AppConfig, Product } from "../types";
import { getErrorMessage } from "../utils/errors";

export function useAppData() {
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);
  const [products, setProducts] = useState<Product[]>([]);
  const [hasConfiguredUsers, setHasConfiguredUsers] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshProducts = useCallback(async () => {
    const nextProducts = await productService.list();
    setProducts(nextProducts);
    return nextProducts;
  }, []);

  const refreshApp = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [state, nextProducts] = await Promise.all([
        configService.getAppState(),
        productService.list()
      ]);

      setConfig(state.config);
      setHasConfiguredUsers(state.hasConfiguredUsers);
      setProducts(nextProducts);
      return state;
    } catch (err) {
      const message = getErrorMessage(err);
      setError(message);
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshApp().catch(() => undefined);
  }, [refreshApp]);

  return useMemo(
    () => ({
      config,
      setConfig,
      products,
      setProducts,
      hasConfiguredUsers,
      setHasConfiguredUsers,
      loading,
      error,
      refreshApp,
      refreshProducts
    }),
    [config, products, hasConfiguredUsers, loading, error, refreshApp, refreshProducts]
  );
}
