export type ThemeMode = "light" | "dark";

export type AppRoute =
  | "home"
  | "dashboard"
  | "verify-ticket"
  | "settings"
  | "new-product"
  | "edit-product";

export interface AppConfig {
  companyName: string;
  taxId: string;
  thankYouMessage?: string | null;
  validityDays: number;
  theme: ThemeMode;
  printerName?: string | null;
  printWidthChars: number;
  onboardingCompleted: boolean;
  setupCompleted: boolean;
  updatedAt: number;
}

export interface AppStatePayload {
  config: AppConfig;
  isFirstRun: boolean;
}

export interface Product {
  id: number;
  name: string;
  priceCents: number;
  description?: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface ProductInput {
  name: string;
  priceCents: number;
  description?: string | null;
}

export interface ProductUpdateInput extends ProductInput {
  id: number;
}

export interface PrintTicketsInput {
  productId: number;
  quantity: number;
}

export interface PrintResult {
  printed: number;
  printerName: string;
  ticketIds: string[];
}

export interface VerifyTicketResult {
  valid: boolean;
  message: string;
  ticketId: string;
}

export interface PrinterInfo {
  name: string;
  isDefault: boolean;
}

export interface ToastState {
  message: string;
  tone: "success" | "error" | "info";
}

export interface ProductFormState {
  name: string;
  price: string;
  description: string;
}
