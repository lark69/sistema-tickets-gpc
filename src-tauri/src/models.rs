use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatePayload {
    pub config: AppConfig,
    pub is_first_run: bool,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    pub name: String,
    pub price_cents: i64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductUpdateInput {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    pub description: Option<String>,
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
