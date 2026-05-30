export type ThemeMode = "light" | "dark";

export type AppRoute =
  | "home"
  | "dashboard"
  | "tables"
  | "logs"
  | "cash"
  | "inventory"
  | "reports"
  | "users"
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
  tableCount: number;
  backupTime?: string | null;
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
  barcode?: string | null;
  costPriceCents: number;
  unit: ProductUnit;
  categoryId?: number | null;
  categoryName?: string | null;
  stock: number;
  reorderLevel: number;
  description?: string | null;
  createdAt: number;
  updatedAt: number;
}

export type ProductUnit = "UN" | "KG" | "L" | "CX" | "PCT";

export interface Category {
  id: number;
  name: string;
  createdAt: number;
}

export interface ProductInput {
  name: string;
  priceCents: number;
  barcode?: string | null;
  costPriceCents: number;
  unit: ProductUnit;
  categoryId?: number | null;
  stock: number;
  reorderLevel: number;
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
  barcode: string;
  costPrice: string;
  markupPercent: string;
  unit: ProductUnit;
  categoryId: string;
  stock: string;
  reorderLevel: string;
  description: string;
}

export type MesaStatus = "livre" | "ativa";
export type FormaPagamento = "pix" | "dinheiro" | "debito" | "credito";

export interface Mesa {
  id: number;
  numero: number;
  capacidade?: number | null;
  criadaEm: number;
  status: MesaStatus;
  tempoInicio?: number | null;
}

export interface MesaProdutoDetalhado {
  id: number;
  idMesa: number;
  idProduto: number;
  quantidade: number;
  adicionadoEm: number;
  produto: Product;
  subtotalCents: number;
}

export interface MesaSessao {
  id: number;
  idMesa: number;
  tempoInicio: number;
  tempoFim?: number | null;
  nomeCliente?: string | null;
  formaPagamento?: FormaPagamento | null;
  valorTotalCents?: number | null;
  idUnico: string;
}

export interface MesaDetailed {
  mesa: Mesa;
  sessao?: MesaSessao | null;
  produtos: MesaProdutoDetalhado[];
  subtotalCents: number;
}

export interface MesaProdutoInput {
  idMesa: number;
  idProduto: number;
  quantidade: number;
}

export interface SaveMesaInput {
  idMesa: number;
  nomeCliente?: string | null;
  produtos: MesaProdutoInput[];
}

export interface FecharMesaInput {
  idMesa: number;
  formaPagamento: FormaPagamento;
  valorPagoCents?: number | null;
  operatorName?: string | null;
}

export interface TicketProduto {
  nome: string;
  quantidade: number;
  precoUnitCents: number;
  subtotalCents: number;
}

export interface TicketData {
  numeroMesa: number;
  nomeCliente?: string | null;
  tempoPermanencia: string;
  idUnico: string;
  formaPagamento: FormaPagamento;
  subtotalCents: number;
  acrescimoCents: number;
  totalCents: number;
  valorPagoCents?: number | null;
  trocoCents?: number | null;
  produtos: TicketProduto[];
}

export interface LogEntry {
  id: number;
  tipo: "ticket_gerado" | "mesa_fechada" | "produto_criado" | string;
  numeroMesa?: number | null;
  nomeCliente?: string | null;
  valorTotalCents?: number | null;
  formaPagamento?: FormaPagamento | null;
  tempoPermanencia?: string | null;
  listaProdutosJson?: string | null;
  dataHora: number;
  idMesaUnico?: string | null;
}

export interface LogFiltros {
  tipo?: string | null;
  numeroMesa?: number | null;
  dataInicio?: number | null;
  dataFim?: number | null;
}

export interface ExportCsvResult {
  path: string;
}

export interface LocalUser {
  id: number;
  username: string;
  role: "admin" | "operator";
  active: boolean;
  createdAt: number;
}

export interface AuthPayload {
  user: LocalUser;
}

export interface CashRegister {
  id: number;
  openedAt: number;
  closedAt?: number | null;
  initialBalanceCents: number;
  finalCountedCents?: number | null;
  expectedBalanceCents: number;
  differenceCents?: number | null;
  operatorName: string;
}

export interface CashMovement {
  id: number;
  cashRegisterId: number;
  movementType: "sangria" | "suprimento" | string;
  amountCents: number;
  note?: string | null;
  operatorName: string;
  createdAt: number;
}

export interface StockMovement {
  id: number;
  productId: number;
  productName: string;
  movementType: string;
  quantity: number;
  previousStock: number;
  newStock: number;
  operatorName: string;
  note?: string | null;
  createdAt: number;
}

export interface ReportsPayload {
  totalRevenueCents: number;
  estimatedProfitCents: number;
  salesByDay: Array<{ dateLabel: string; totalCents: number }>;
  topProducts: Array<{ productName: string; quantity: number; totalCents: number }>;
  lowStockProducts: Product[];
}

export interface BackupResult {
  path: string;
}
