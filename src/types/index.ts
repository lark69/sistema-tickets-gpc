export type ThemeMode = "light" | "dark";

export type AppRoute =
  | "home"
  | "dashboard"
  | "tables"
  | "logs"
  | "cash"
  | "fechar-caixa"
  | "guia-caixa"
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
  hasConfiguredUsers: boolean;
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
  soldQuantity: number;
  description?: string | null;
  createdAt: number;
  updatedAt: number;
  validade?: number | null;
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
  validade?: number | null;
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
  validade: string;
}

export interface RegistrarPagamentoMesaInput {
  idMesa: number;
  formaPagamento: FormaPagamento;
  valorCents: number;
  aplicarAcrescimo?: boolean;
  aplicarGarcom?: boolean;
  operatorName?: string | null;
}

export interface PagamentoMesa {
  id: number;
  formaPagamento: FormaPagamento;
  valorCents: number;
  trocoCents: number;
  surchargeCents: number;
  createdAt: number;
}

export interface ContaMesa {
  idMesa: number;
  totalCents: number;
  pagoCents: number;
  saldoCents: number;
  pagamentos: PagamentoMesa[];
}

export interface ProdutoVencendo {
  id: number;
  name: string;
  validade: number;
  diasRestantes: number;
}

export interface PagamentoMesaResult {
  finalizada: boolean;
  saldoRestanteCents: number;
  trocoCents: number;
  ticket?: TicketData | null;
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
  totalCents: number;
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
  aplicarAcrescimo?: boolean;
  aplicarGarcom?: boolean;
  operatorName?: string | null;
}

export interface SaleCartItemInput {
  productId: number;
  quantidade: number;
}

export interface FecharVendaCaixaInput {
  formaPagamento: FormaPagamento;
  valorPagoCents?: number | null;
  aplicarAcrescimo?: boolean;
  aplicarGarcom?: boolean;
  items: SaleCartItemInput[];
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

export interface ExportAppConfigResult {
  path: string;
}

export type UserPermission =
  | "addTableProducts"
  | "removeTableProducts"
  | "closeTable"
  | "manageProducts"
  | "manageUsers"
  | "manageTickets"
  | "viewLogsReports"
  | "manageCompanyInfo"
  | "manageTicketValidity"
  | "manageTableCount"
  | "manageBackupTime"
  | "configurePrinters"
  | "manageCash"
  | "manageCashMovements";

export interface LocalUser {
  id: number;
  username: string;
  role: "admin" | "operator";
  permissions: UserPermission[];
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
  turnoId?: number | null;
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

export type SalesReportPeriod = "day" | "month";

export interface PrintSalesReportResult {
  printerName: string;
  periodLabel: string;
}

export interface BackupResult {
  path: string;
}

// ===========================================================================
// FASE 5: Fechamento em cascata — Turno Operacional + Periodo Contabil
// ===========================================================================

export type TurnoStatus = "aberto" | "fechado" | "reconciliado";
export type PeriodoStatus = "aberto" | "fechado" | "bloqueado";

export interface TurnoOperacional {
  id: number;
  lojaId: number;
  caixaId?: number | null;
  operador: string;
  dataInicio: number;
  dataFim?: number | null;
  status: TurnoStatus;
  valorEsperadoCents: number;
  valorFisicoCents?: number | null;
  diferencaCents?: number | null;
  observacoes?: string | null;
  periodoContabilId?: number | null;
  createdAt: number;
  updatedAt: number;
  saldoInicialCents: number;
}

export interface PeriodoContabil {
  id: number;
  lojaId: number;
  data: string;
  status: PeriodoStatus;
  totalEsperadoCents: number;
  totalRealCents: number;
  bloqueadoEm?: number | null;
  bloqueadoPor?: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface SaleAuditEntry {
  id: number;
  saleId: number;
  turnoOperacionalId?: number | null;
  periodoContabilId?: number | null;
  valorAnteriorCents: number;
  valorNovoCents: number;
  motivo: string;
  usuario: string;
  createdAt: number;
}

export interface CashierStatus {
  dataContabil: string;
  fiscalDayStartMinutes: number;
  turnoAtivo?: TurnoOperacional | null;
  periodoHoje: PeriodoContabil;
  turnosDoDia: TurnoOperacional[];
  esperadoAtualCents?: number | null;
}

export interface AbrirTurnoInput {
  operador: string;
  caixaId?: number | null;
  saldoInicialCents: number;
}

export interface FecharTurnoInput {
  turnoId: number;
  valorFisicoCents: number;
  observacoes?: string | null;
}

export interface ConsolidarPeriodoInput {
  data?: string | null;
  usuario: string;
}

export interface BloquearPeriodoInput {
  periodoId: number;
  usuario: string;
}

export interface EditarVendaInput {
  saleId: number;
  novoTotalCents: number;
  motivo: string;
  usuario: string;
}
