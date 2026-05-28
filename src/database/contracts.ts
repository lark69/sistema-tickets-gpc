import type { AppConfig, Product } from "../types";

export const DEFAULT_CONFIG: AppConfig = {
  companyName: "",
  taxId: "",
  thankYouMessage: "",
  validityDays: 30,
  theme: "light",
  printerName: "",
  printWidthChars: 48,
  onboardingCompleted: false,
  setupCompleted: false,
  updatedAt: Date.now()
};

export const PRODUCT_TABLE_COLUMNS: Array<keyof Product> = [
  "id",
  "name",
  "priceCents",
  "description",
  "createdAt",
  "updatedAt"
];
