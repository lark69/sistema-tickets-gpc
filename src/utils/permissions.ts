import type { AppRoute, LocalUser, UserPermission } from "../types";

export const ALL_USER_PERMISSIONS: UserPermission[] = [
  "addTableProducts",
  "removeTableProducts",
  "closeTable",
  "manageProducts",
  "manageUsers",
  "manageTickets",
  "viewLogsReports",
  "manageCompanyInfo",
  "manageTicketValidity",
  "manageTableCount",
  "manageBackupTime",
  "configurePrinters",
  "manageCash",
  "manageCashMovements"
];

export const DEFAULT_OPERATOR_PERMISSIONS: UserPermission[] = [
  "addTableProducts",
  "removeTableProducts",
  "closeTable",
  "manageTickets",
  "manageCash",
  "manageCashMovements"
];

export const USER_PERMISSION_OPTIONS: Array<{
  value: UserPermission;
  label: string;
  description: string;
}> = [
  {
    value: "addTableProducts",
    label: "Adicionar produtos a mesa",
    description: "Permite incluir itens no consumo das mesas."
  },
  {
    value: "removeTableProducts",
    label: "Retirar produtos da mesa",
    description: "Permite remover unidades adicionadas em uma mesa."
  },
  {
    value: "closeTable",
    label: "Fechar mesa",
    description: "Permite finalizar pagamentos de mesas."
  },
  {
    value: "manageProducts",
    label: "Produtos, categorias e estoque",
    description: "Permite criar, editar, excluir produtos, categorias e ajustar estoque."
  },
  {
    value: "manageUsers",
    label: "Gerenciar usuarios",
    description: "Permite acessar a administracao de usuarios."
  },
  {
    value: "manageTickets",
    label: "Gerenciar tickets",
    description: "Permite imprimir, verificar e desativar tickets."
  },
  {
    value: "viewLogsReports",
    label: "Logs e relatorios",
    description: "Permite visualizar auditoria, relatorios e backups manuais."
  },
  {
    value: "manageCompanyInfo",
    label: "Informacoes da empresa",
    description: "Permite editar nome, CPF/CNPJ e frase de agradecimento."
  },
  {
    value: "manageTicketValidity",
    label: "Validade do ticket",
    description: "Permite alterar o tempo de validade dos tickets."
  },
  {
    value: "manageTableCount",
    label: "Quantidade de mesas",
    description: "Permite ajustar o total de mesas exibidas no PDV."
  },
  {
    value: "manageBackupTime",
    label: "Horario diario de backup",
    description: "Permite configurar o horario de backup automatico."
  },
  {
    value: "configurePrinters",
    label: "Configurar impressoras",
    description: "Permite selecionar impressora e largura de impressao."
  },
  {
    value: "manageCash",
    label: "Abrir ou fechar caixa",
    description: "Permite controlar a abertura e o fechamento do caixa."
  },
  {
    value: "manageCashMovements",
    label: "Sangria e suprimento",
    description: "Permite registrar entradas e retiradas no caixa."
  }
];

export function hasPermission(user: LocalUser | null | undefined, permission: UserPermission): boolean {
  if (!user) return false;
  if (user.role === "admin") return true;
  return user.permissions.includes(permission);
}

export function hasAnyPermission(
  user: LocalUser | null | undefined,
  permissions: UserPermission[]
): boolean {
  return permissions.some((permission) => hasPermission(user, permission));
}

export function permissionsForRole(role: string, permissions: UserPermission[]): UserPermission[] {
  return role === "admin" ? ALL_USER_PERMISSIONS : permissions;
}

export function routeIsAllowed(route: AppRoute, user: LocalUser | null | undefined): boolean {
  if (!user) return false;
  if (route === "home" || route === "tables") return true;
  if (route === "cash") return hasAnyPermission(user, ["manageCash", "manageCashMovements"]);
  if (route === "dashboard" || route === "new-product" || route === "edit-product" || route === "inventory") {
    return hasPermission(user, "manageProducts") || hasPermission(user, "manageTickets");
  }
  if (route === "logs" || route === "reports") return hasPermission(user, "viewLogsReports");
  if (route === "users") return hasPermission(user, "manageUsers");
  if (route === "verify-ticket") return hasPermission(user, "manageTickets");
  if (route === "settings") {
    return hasAnyPermission(user, [
      "manageCompanyInfo",
      "manageTicketValidity",
      "manageTableCount",
      "manageBackupTime",
      "configurePrinters"
    ]);
  }
  return false;
}
