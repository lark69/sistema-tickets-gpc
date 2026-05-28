use crate::error::{CommandError, CommandResult};
use crate::models::{
    AppConfig, AppConfigInput, AppStatePayload, DeleteProductInput, PrintResult,
    PrintTicketsInput, PrinterInfo, Product, ProductInput, ProductUpdateInput, VerifyTicketInput,
    VerifyTicketResult,
};
use crate::{printer, AppContext};
use tauri::State;

#[tauri::command]
pub fn get_app_state(state: State<'_, AppContext>) -> CommandResult<AppStatePayload> {
    let config = state.database.get_config().map_err(CommandError::from)?;
    let is_first_run = !config.onboarding_completed || !config.setup_completed;

    Ok(AppStatePayload {
        config,
        is_first_run,
    })
}

#[tauri::command]
pub fn complete_onboarding(state: State<'_, AppContext>) -> CommandResult<AppConfig> {
    state
        .database
        .complete_onboarding()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn save_app_config(
    input: AppConfigInput,
    state: State<'_, AppContext>,
) -> CommandResult<AppConfig> {
    state.database.save_config(input).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_products(state: State<'_, AppContext>) -> CommandResult<Vec<Product>> {
    state.database.list_products().map_err(CommandError::from)
}

#[tauri::command]
pub fn create_product(
    input: ProductInput,
    state: State<'_, AppContext>,
) -> CommandResult<Product> {
    state.database.create_product(input).map_err(CommandError::from)
}

#[tauri::command]
pub fn update_product(
    input: ProductUpdateInput,
    state: State<'_, AppContext>,
) -> CommandResult<Product> {
    state.database.update_product(input).map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_product(input: DeleteProductInput, state: State<'_, AppContext>) -> CommandResult<()> {
    state
        .database
        .delete_product(input.product_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn print_tickets(
    input: PrintTicketsInput,
    state: State<'_, AppContext>,
) -> CommandResult<PrintResult> {
    state
        .database
        .cleanup_expired_tickets()
        .map_err(CommandError::from)?;
    let config = state.database.get_config().map_err(CommandError::from)?;
    let product = state
        .database
        .get_product(input.product_id)
        .map_err(CommandError::from)?;
    let issued_tickets = state
        .database
        .issue_tickets(&product, input.quantity, config.validity_days)
        .map_err(CommandError::from)?;
    let ticket_ids = issued_tickets
        .iter()
        .map(|ticket| ticket.ticket_id.clone())
        .collect::<Vec<_>>();

    match printer::print_tickets(&config, &product, &issued_tickets) {
        Ok(result) => Ok(result),
        Err(error) => {
            let _ = state.database.delete_issued_tickets(&ticket_ids);
            Err(CommandError::from(error))
        }
    }
}

#[tauri::command]
pub fn list_printers() -> CommandResult<Vec<PrinterInfo>> {
    printer::list_printers().map_err(CommandError::from)
}

#[tauri::command]
pub fn verify_ticket(
    input: VerifyTicketInput,
    state: State<'_, AppContext>,
) -> CommandResult<VerifyTicketResult> {
    let ticket_id = input.ticket_id.trim().to_uppercase();
    let valid = state
        .database
        .verify_ticket(&ticket_id)
        .map_err(CommandError::from)?;
    let message = if valid {
        "Este ticket é válido e foi impresso usano o Sistema de Tickets GPC"
    } else {
        "Este ticket é inválido, ou passou da válidade."
    };

    Ok(VerifyTicketResult {
        valid,
        message: message.to_string(),
        ticket_id,
    })
}

#[tauri::command]
pub fn open_creator_portfolio() -> CommandResult<()> {
    open_external_url("https://lark69.github.io/Gabriel-Portela-Portfolio/").map_err(|error| {
        CommandError::from(crate::error::AppError::System(format!(
            "Nao foi possivel abrir o navegador: {error}"
        )))
    })?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn open_external_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_external_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("open").arg(url).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn open_external_url(_url: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "sistema operacional sem abridor de URL configurado",
    ))
}
