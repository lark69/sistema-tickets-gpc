import type {
  AuthPayload,
  BackupResult,
  CashMovement,
  CashRegister,
  Category,
  ReportsPayload,
  StockMovement,
  LocalUser
} from "../types";
import { callCommand } from "./tauri";

export const adminService = {
  login(username: string, password: string): Promise<AuthPayload> {
    return callCommand<AuthPayload>("login", { input: { username, password } });
  },

  listUsers(): Promise<LocalUser[]> {
    return callCommand<LocalUser[]>("list_users");
  },

  createUser(username: string, password: string, role: string): Promise<LocalUser> {
    return callCommand<LocalUser>("create_user", { input: { username, password, role } });
  },

  listCategories(): Promise<Category[]> {
    return callCommand<Category[]>("list_categories");
  },

  createCategory(name: string, operatorName?: string): Promise<Category> {
    return callCommand<Category>("create_category", {
      input: { name },
      operatorName: operatorName || null
    });
  },

  adjustStock(input: {
    productId: number;
    quantity: number;
    movementType: string;
    operatorName: string;
    note?: string | null;
  }): Promise<StockMovement> {
    return callCommand<StockMovement>("adjust_stock", { input });
  },

  listStockMovements(): Promise<StockMovement[]> {
    return callCommand<StockMovement[]>("list_stock_movements");
  },

  getCurrentCashRegister(): Promise<CashRegister | null> {
    return callCommand<CashRegister | null>("get_current_cash_register");
  },

  openCashRegister(initialBalanceCents: number, operatorName: string): Promise<CashRegister> {
    return callCommand<CashRegister>("open_cash_register", {
      input: { initialBalanceCents, operatorName }
    });
  },

  closeCashRegister(finalCountedCents: number, operatorName: string): Promise<CashRegister> {
    return callCommand<CashRegister>("close_cash_register", {
      input: { finalCountedCents, operatorName }
    });
  },

  addCashMovement(input: {
    movementType: "sangria" | "suprimento";
    amountCents: number;
    note?: string | null;
    operatorName: string;
  }): Promise<CashMovement> {
    return callCommand<CashMovement>("add_cash_movement", { input });
  },

  listCashMovements(): Promise<CashMovement[]> {
    return callCommand<CashMovement[]>("list_cash_movements");
  },

  getReports(): Promise<ReportsPayload> {
    return callCommand<ReportsPayload>("get_reports");
  },

  backupDatabase(): Promise<BackupResult> {
    return callCommand<BackupResult>("backup_database");
  }
};
