import type { Product, ProductInput, ProductUpdateInput } from "../types";
import { callCommand } from "./tauri";

export const productService = {
  list(): Promise<Product[]> {
    return callCommand<Product[]>("list_products");
  },

  create(input: ProductInput): Promise<Product> {
    return callCommand<Product>("create_product", { input });
  },

  update(input: ProductUpdateInput): Promise<Product> {
    return callCommand<Product>("update_product", { input });
  },

  remove(productId: number): Promise<void> {
    return callCommand<void>("delete_product", { input: { productId } });
  }
};
