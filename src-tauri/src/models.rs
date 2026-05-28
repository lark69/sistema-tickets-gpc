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
