import type { AppConfig, ProductFormState } from "../types";
import { currencyToCents } from "./currency";

export function validateProductForm(form: ProductFormState): string | null {
  if (!form.name.trim()) {
    return "Informe o nome do produto.";
  }

  if (form.name.trim().length > 120) {
    return "O nome do produto deve ter no maximo 120 caracteres.";
  }

  if (currencyToCents(form.price) <= 0) {
    return "Informe um valor maior que zero.";
  }

  if (currencyToCents(form.costPrice) < 0) {
    return "O custo não pode ser negativo.";
  }

  return null;
}

export function validateConfig(config: AppConfig): string | null {
  if (!config.companyName.trim()) {
    return "Informe o nome da empresa.";
  }

  if (!config.taxId.trim()) {
    return "Informe o CPF ou CNPJ.";
  }

  if (config.validityDays < 1 || config.validityDays > 3650) {
    return "A validade deve ficar entre 1 e 3650 dias.";
  }

  if (config.printWidthChars < 32 || config.printWidthChars > 64) {
    return "A largura deve ficar entre 32 e 64 caracteres.";
  }

  if (config.tableCount < 1 || config.tableCount > 100) {
    return "A quantidade de mesas deve ficar entre 1 e 100.";
  }

  return null;
}

const USERNAME_RE = /^[A-Za-z0-9]{1,20}$/;

export function validateUsername(username: string): string | null {
  if (!USERNAME_RE.test(username.trim())) {
    return "Usuário: apenas letras e números, até 20 caracteres, sem espaços.";
  }
  return null;
}

export function validatePassword(password: string): string | null {
  if (password.length < 4 || password.length > 30) {
    return "A senha deve ter de 4 a 30 caracteres.";
  }
  return null;
}
