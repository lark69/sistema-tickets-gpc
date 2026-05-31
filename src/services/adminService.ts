import type {
  AuthPayload,
  BackupResult,
  CashMovement,
  CashRegister,
  Category,
  FecharVendaCaixaInput,
  ReportsPayload,
  StockMovement,
  TicketData,
  LocalUser,
  UserPermission
} from "../types";
import { callCommand } from "./tauri";

export const adminService = {
  login(username: string, password: string): Promise<AuthPayload> {
    return callCommand<AuthPayload>("login", { input: { username, password } });
  },

  listUsers(): Promise<LocalUser[]> {
    return callCommand<LocalUser[]>("list_users");
  },

  createUser(
    username: string,
    password: string,
    role: string,
    permissions: UserPermission[],
    requester?: LocalUser | null
  ): Promise<LocalUser> {
    return callCommand<LocalUser>("create_user", {
      input: {
        username,
        password,
        role,
        permissions,
        requesterRole: requester?.role || null,
        requesterPermissions: requester?.permissions || null
      }
    });
  },

  updateUser(input: {
    id: number;
    username: string;
    password?: string | null;
    role: string;
    permissions: UserPermission[];
    requester?: LocalUser | null;
  }): Promise<LocalUser> {
    return callCommand<LocalUser>("update_user", {
      input: {
        id: input.id,
        username: input.username,
        password: input.password,
        role: input.role,
        permissions: input.permissions,
        requesterRole: input.requester?.role || null,
        requesterPermissions: input.requester?.permissions || null
      }
    });
  },

  deleteUser(id: number, requester?: LocalUser | null): Promise<void> {
    return callCommand<void>("delete_user", {
      input: {
        id,
        requesterRole: requester?.role || null,
        requesterPermissions: requester?.permissions || null
      }
    });
  },

  listCategories(): Promise<Category[]> {
    return callCommand<Category[]>("list_categories");
  },

  createCategory(name: string, operatorName?: string, requester?: LocalUser | null): Promise<Category> {
    return callCommand<Category>("create_category", {
      input: { name },
      operatorName: operatorName || null,
      requesterRole: requester?.role || null,
      requesterPermissions: requester?.permissions || null
    });
  },

  updateCategory(id: number, name: string, requester?: LocalUser | null): Promise<Category> {
    return callCommand<Category>("update_category", {
      input: {
        id,
        name,
        requesterRole: requester?.role || null,
        requesterPermissions: requester?.permissions || null
      }
    });
  },

  deleteCategory(id: number, requester?: LocalUser | null): Promise<void> {
    return callCommand<void>("delete_category", {
      input: {
        id,
        requesterRole: requester?.role || null,
        requesterPermissions: requester?.permissions || null
      }
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

  fecharVendaCaixa(input: FecharVendaCaixaInput): Promise<TicketData> {
    return callCommand<TicketData>("fechar_venda_caixa", { input });
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
