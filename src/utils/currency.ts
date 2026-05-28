export function formatCurrency(priceCents: number): string {
  return new Intl.NumberFormat("pt-BR", {
    style: "currency",
    currency: "BRL"
  }).format(priceCents / 100);
}

export function currencyToCents(value: string): number {
  const normalized = value
    .replace(/[^\d,.-]/g, "")
    .replace(/\./g, "")
    .replace(",", ".");

  const parsed = Number.parseFloat(normalized);

  if (Number.isNaN(parsed)) {
    return 0;
  }

  return Math.round(parsed * 100);
}

export function centsToInput(priceCents: number): string {
  return (priceCents / 100).toLocaleString("pt-BR", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2
  });
}
