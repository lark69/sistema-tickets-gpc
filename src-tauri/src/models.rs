use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub config: AppConfig,
    pub is_first_run: bool,
    pub has_configured_users: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub company_name: String,
    pub tax_id: String,
    pub thank_you_message: Option<String>,
    pub validity_days: i64,
    pub theme: String,
    pub printer_name: Option<String>,
    pub print_width_chars: i64,
    pub onboarding_completed: bool,
    pub setup_completed: bool,
    pub table_count: i64,
    pub backup_time: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigInput {
    pub company_name: String,
    pub tax_id: String,
    pub thank_you_message: Option<String>,
    pub validity_days: i64,
    pub theme: String,
    pub printer_name: Option<String>,
    pub print_width_chars: i64,
    pub setup_completed: bool,
    pub table_count: i64,
    pub backup_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInput {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryUpdateInput {
    pub id: i64,
    pub name: String,
    pub requester_role: Option<String>,
    pub requester_permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCategoryInput {
    pub id: i64,
    pub requester_role: Option<String>,
    pub requester_permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    pub barcode: Option<String>,
    pub cost_price_cents: i64,
    pub unit: String,
    pub category_id: Option<i64>,
    pub category_name: Option<String>,
    pub stock: i64,
    pub reorder_level: i64,
    pub sold_quantity: i64,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub validade: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub name: String,
    pub price_cents: i64,
    pub barcode: Option<String>,
    pub cost_price_cents: i64,
    pub unit: String,
    pub category_id: Option<i64>,
    pub stock: i64,
    pub reorder_level: i64,
    pub description: Option<String>,
    pub validade: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUpdateInput {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    pub barcode: Option<String>,
    pub cost_price_cents: i64,
    pub unit: String,
    pub category_id: Option<i64>,
    pub stock: i64,
    pub reorder_level: i64,
    pub description: Option<String>,
    pub validade: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintTicketsInput {
    pub product_id: i64,
    pub quantity: i64,
}

#[derive(Debug, Clone)]
pub struct IssuedTicket {
    pub ticket_id: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTicketInput {
    pub ticket_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTicketResult {
    pub valid: bool,
    pub message: String,
    pub ticket_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteProductInput {
    pub product_id: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintResult {
    pub printed: i64,
    pub printer_name: String,
    pub ticket_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mesa {
    pub id: i64,
    pub numero: i64,
    pub capacidade: Option<i64>,
    pub criada_em: i64,
    pub status: String,
    pub tempo_inicio: Option<i64>,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMesaInput {
    pub numero: i64,
    pub capacidade: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MesaProdutoDetalhado {
    pub id: i64,
    pub id_mesa: i64,
    pub id_produto: i64,
    pub quantidade: i64,
    pub adicionado_em: i64,
    pub produto: Product,
    pub subtotal_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MesaSessao {
    pub id: i64,
    pub id_mesa: i64,
    pub tempo_inicio: i64,
    pub tempo_fim: Option<i64>,
    pub nome_cliente: Option<String>,
    pub forma_pagamento: Option<String>,
    pub valor_total_cents: Option<i64>,
    pub id_unico: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MesaDetailed {
    pub mesa: Mesa,
    pub sessao: Option<MesaSessao>,
    pub produtos: Vec<MesaProdutoDetalhado>,
    pub subtotal_cents: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MesaProdutoInput {
    pub id_mesa: i64,
    pub id_produto: i64,
    pub quantidade: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMesaInput {
    pub id_mesa: i64,
    pub nome_cliente: Option<String>,
    pub produtos: Vec<MesaProdutoInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMesaClienteInput {
    pub id_mesa: i64,
    pub nome_cliente: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FecharMesaInput {
    pub id_mesa: i64,
    pub forma_pagamento: String,
    pub valor_pago_cents: Option<i64>,
    pub operator_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleCartItemInput {
    pub product_id: i64,
    pub quantidade: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FecharVendaCaixaInput {
    pub forma_pagamento: String,
    pub valor_pago_cents: Option<i64>,
    pub operator_name: Option<String>,
    pub items: Vec<SaleCartItemInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketProduto {
    pub nome: String,
    pub quantidade: i64,
    pub preco_unit_cents: i64,
    pub subtotal_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketData {
    pub numero_mesa: i64,
    pub nome_cliente: Option<String>,
    pub tempo_permanencia: String,
    pub id_unico: String,
    pub forma_pagamento: String,
    pub subtotal_cents: i64,
    pub acrescimo_cents: i64,
    pub total_cents: i64,
    pub valor_pago_cents: Option<i64>,
    pub troco_cents: Option<i64>,
    pub produtos: Vec<TicketProduto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: i64,
    pub tipo: String,
    pub numero_mesa: Option<i64>,
    pub nome_cliente: Option<String>,
    pub valor_total_cents: Option<i64>,
    pub forma_pagamento: Option<String>,
    pub tempo_permanencia: Option<String>,
    pub lista_produtos_json: Option<String>,
    pub data_hora: i64,
    pub id_mesa_unico: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFiltros {
    pub tipo: Option<String>,
    pub numero_mesa: Option<i64>,
    pub data_inicio: Option<i64>,
    pub data_fim: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashRegister {
    pub id: i64,
    pub opened_at: i64,
    pub closed_at: Option<i64>,
    pub initial_balance_cents: i64,
    pub final_counted_cents: Option<i64>,
    pub expected_balance_cents: i64,
    pub difference_cents: Option<i64>,
    pub operator_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashMovement {
    pub id: i64,
    pub cash_register_id: i64,
    pub turno_id: Option<i64>,
    pub movement_type: String,
    pub amount_cents: i64,
    pub note: Option<String>,
    pub operator_name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCashRegisterInput {
    pub initial_balance_cents: i64,
    pub operator_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseCashRegisterInput {
    pub final_counted_cents: i64,
    pub operator_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashMovementInput {
    pub movement_type: String,
    pub amount_cents: i64,
    pub note: Option<String>,
    pub operator_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockMovement {
    pub id: i64,
    pub product_id: i64,
    pub product_name: String,
    pub movement_type: String,
    pub quantity: i64,
    pub previous_stock: i64,
    pub new_stock: i64,
    pub operator_name: String,
    pub note: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockAdjustInput {
    pub product_id: i64,
    pub quantity: i64,
    pub movement_type: String,
    pub operator_name: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalUser {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
    pub active: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub username: String,
    pub password: String,
    pub role: String,
    pub permissions: Option<Vec<String>>,
    pub requester_role: Option<String>,
    pub requester_permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserInput {
    pub id: i64,
    pub username: String,
    pub password: Option<String>,
    pub role: String,
    pub permissions: Option<Vec<String>>,
    pub requester_role: Option<String>,
    pub requester_permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteUserInput {
    pub id: i64,
    pub requester_role: Option<String>,
    pub requester_permissions: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthPayload {
    pub user: LocalUser,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesByDay {
    pub date_label: String,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopProductReport {
    pub product_name: String,
    pub quantity: i64,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportsPayload {
    pub total_revenue_cents: i64,
    pub estimated_profit_cents: i64,
    pub sales_by_day: Vec<SalesByDay>,
    pub top_products: Vec<TopProductReport>,
    pub low_stock_products: Vec<Product>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSalesReportInput {
    pub period: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SalesReportData {
    pub period: String,
    pub period_label: String,
    pub printed_at: i64,
    pub direct_sales_cents: i64,
    pub table_sales_cents: i64,
    pub ticket_sales_cents: i64,
    pub total_sales_cents: i64,
    pub estimated_profit_cents: i64,
    pub previous_total_sales_cents: i64,
    pub comparison_percent: Option<f64>,
    pub comparison_text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintSalesReportResult {
    pub printer_name: String,
    pub period_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupResult {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetSalesInput {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAppConfigResult {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAppConfigInput {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAppConfigContentInput {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDataExport {
    pub version: i64,
    pub exported_at: i64,
    pub company_name: String,
    pub tax_id: String,
    pub print_width_chars: i64,
    pub categories: Vec<ExportedCategory>,
    pub products: Vec<ExportedProduct>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedCategory {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedProduct {
    pub name: String,
    pub price_cents: i64,
    pub barcode: Option<String>,
    pub cost_price_cents: i64,
    pub unit: String,
    pub category_name: Option<String>,
    pub stock: i64,
    pub reorder_level: i64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCsvInput {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCsvResult {
    pub path: String,
}

// ===== FASE 4: pagamento parcial + validade =====

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrarPagamentoMesaInput {
    pub id_mesa: i64,
    pub forma_pagamento: String,
    pub valor_cents: i64,
    pub aplicar_acrescimo: Option<bool>,
    pub operator_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagamentoMesa {
    pub id: i64,
    pub forma_pagamento: String,
    pub valor_cents: i64,
    pub troco_cents: i64,
    pub surcharge_cents: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContaMesa {
    pub id_mesa: i64,
    pub total_cents: i64,
    pub pago_cents: i64,
    pub saldo_cents: i64,
    pub pagamentos: Vec<PagamentoMesa>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagamentoMesaResult {
    pub finalizada: bool,
    pub saldo_restante_cents: i64,
    pub troco_cents: i64,
    pub ticket: Option<TicketData>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProdutoVencendo {
    pub id: i64,
    pub name: String,
    pub validade: i64,
    pub dias_restantes: i64,
}

// ===========================================================================
// FASE 5: Fechamento em cascata — Turno Operacional + Periodo Contabil
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TurnoStatus {
    Aberto,
    Fechado,
    Reconciliado,
}

impl TurnoStatus {
    pub fn parse(value: &str) -> Self {
        match value {
            "fechado" => TurnoStatus::Fechado,
            "reconciliado" => TurnoStatus::Reconciliado,
            _ => TurnoStatus::Aberto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeriodoStatus {
    Aberto,
    Fechado,
    Bloqueado,
}

impl PeriodoStatus {
    pub fn parse(value: &str) -> Self {
        match value {
            "fechado" => PeriodoStatus::Fechado,
            "bloqueado" => PeriodoStatus::Bloqueado,
            _ => PeriodoStatus::Aberto,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnoOperacional {
    pub id: i64,
    pub loja_id: i64,
    pub caixa_id: Option<i64>,
    pub operador: String,
    pub data_inicio: i64,
    pub data_fim: Option<i64>,
    pub status: TurnoStatus,
    pub valor_esperado_cents: i64,
    pub valor_fisico_cents: Option<i64>,
    pub diferenca_cents: Option<i64>,
    pub observacoes: Option<String>,
    pub periodo_contabil_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub saldo_inicial_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodoContabil {
    pub id: i64,
    pub loja_id: i64,
    pub data: String,
    pub status: PeriodoStatus,
    pub total_esperado_cents: i64,
    pub total_real_cents: i64,
    pub bloqueado_em: Option<i64>,
    pub bloqueado_por: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleAuditEntry {
    pub id: i64,
    pub sale_id: i64,
    pub turno_operacional_id: Option<i64>,
    pub periodo_contabil_id: Option<i64>,
    pub valor_anterior_cents: i64,
    pub valor_novo_cents: i64,
    pub motivo: String,
    pub usuario: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CashierStatus {
    pub data_contabil: String,
    pub fiscal_day_start_minutes: i64,
    pub turno_ativo: Option<TurnoOperacional>,
    pub periodo_hoje: PeriodoContabil,
    pub turnos_do_dia: Vec<TurnoOperacional>,
    /// Valor esperado da gaveta agora (ao vivo) para o turno ativo, se houver.
    pub esperado_atual_cents: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbrirTurnoInput {
    pub operador: String,
    pub caixa_id: Option<i64>,
    #[serde(default)]
    pub saldo_inicial_cents: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FecharTurnoInput {
    pub turno_id: i64,
    pub valor_fisico_cents: i64,
    pub observacoes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidarPeriodoInput {
    pub data: Option<String>,
    pub usuario: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BloquearPeriodoInput {
    pub periodo_id: i64,
    pub usuario: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditarVendaInput {
    pub sale_id: i64,
    pub novo_total_cents: i64,
    pub motivo: String,
    pub usuario: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiscalDayConfigInput {
    pub fiscal_day_start_minutes: i64,
}
