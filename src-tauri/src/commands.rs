use crate::error::{CommandError, CommandResult};
use crate::models::{
    AbrirTurnoInput, AppConfig, AppConfigInput, AppDataExport, AppStatePayload, AuthPayload,
    BackupResult, BloquearPeriodoInput, CashMovement, CashMovementInput, CashRegister,
    CashierStatus, Category, CategoryInput, CategoryUpdateInput, CloseCashRegisterInput,
    ConsolidarPeriodoInput, ContaMesa, CreateMesaInput, CreateUserInput, DeleteCategoryInput,
    EditarVendaInput, FiscalDayConfigInput, PagamentoMesaResult, PeriodoContabil,
    RegistrarPagamentoMesaInput, DeleteProductInput, DeleteUserInput, ExportAppConfigResult,
    ExportCsvInput, ExportCsvResult, FecharMesaInput, FecharTurnoInput, FecharVendaCaixaInput,
    ImportAppConfigContentInput, ImportAppConfigInput, LocalUser, LogEntry, LogFiltros, LoginInput,
    Mesa, MesaDetailed, MesaProdutoDetalhado, MesaProdutoInput, MesaSessao, OpenCashRegisterInput,
    PrintResult, PrintSalesReportInput, PrintSalesReportResult, PrintTicketsInput, PrinterInfo,
    Product, ProductInput, ProductUpdateInput, ProdutoVencendo, ReportsPayload, ResetSalesInput,
    SaleAuditEntry, SaveMesaInput, StockAdjustInput, StockMovement, TicketData,
    TurnoOperacional, UpdateMesaClienteInput, UpdateUserInput, VerifyTicketInput,
    VerifyTicketResult,
};
use crate::{printer, AppContext};
use tauri::{Manager, State};

#[tauri::command]
pub fn get_app_state(state: State<'_, AppContext>) -> CommandResult<AppStatePayload> {
    let config = state.database.get_config().map_err(CommandError::from)?;
    let is_first_run = !config.onboarding_completed || !config.setup_completed;
    let has_configured_users = state
        .database
        .has_configured_users()
        .map_err(CommandError::from)?;

    Ok(AppStatePayload {
        config,
        is_first_run,
        has_configured_users,
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
    state
        .database
        .save_config(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_products(state: State<'_, AppContext>) -> CommandResult<Vec<Product>> {
    state.database.list_products().map_err(CommandError::from)
}

#[tauri::command]
pub fn create_product(input: ProductInput, state: State<'_, AppContext>) -> CommandResult<Product> {
    state
        .database
        .create_product(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_product(
    input: ProductUpdateInput,
    state: State<'_, AppContext>,
) -> CommandResult<Product> {
    state
        .database
        .update_product(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_product(
    input: DeleteProductInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    state
        .database
        .delete_product(input.product_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_categories(state: State<'_, AppContext>) -> CommandResult<Vec<Category>> {
    state.database.list_categories().map_err(CommandError::from)
}

#[tauri::command]
pub fn create_category(
    input: CategoryInput,
    operator_name: Option<String>,
    requester_role: Option<String>,
    requester_permissions: Option<Vec<String>>,
    state: State<'_, AppContext>,
) -> CommandResult<Category> {
    state
        .database
        .create_category(input, operator_name, requester_role, requester_permissions)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_category(
    input: CategoryUpdateInput,
    state: State<'_, AppContext>,
) -> CommandResult<Category> {
    state
        .database
        .update_category(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_category(
    input: DeleteCategoryInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    state
        .database
        .delete_category(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn adjust_stock(
    input: StockAdjustInput,
    state: State<'_, AppContext>,
) -> CommandResult<StockMovement> {
    state
        .database
        .adjust_stock(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_stock_movements(state: State<'_, AppContext>) -> CommandResult<Vec<StockMovement>> {
    state
        .database
        .list_stock_movements()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn print_tickets(
    input: PrintTicketsInput,
    state: State<'_, AppContext>,
) -> CommandResult<PrintResult> {
    state
        .database
        .ensure_tickets_can_be_printed()
        .map_err(CommandError::from)?;
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
        Ok(result) => {
            state
                .database
                .record_ticket_sale(&product, result.printed)
                .map_err(CommandError::from)?;
            let _ = state.database.log_ticket_gerado(&product, result.printed);
            Ok(result)
        }
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
        "Este ticket é válido e foi impresso usando o Portex PDV"
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
pub fn deactivate_ticket(
    input: VerifyTicketInput,
    state: State<'_, AppContext>,
) -> CommandResult<VerifyTicketResult> {
    let ticket_id = input.ticket_id.trim().to_uppercase();
    let deactivated = state
        .database
        .deactivate_ticket(&ticket_id)
        .map_err(CommandError::from)?;
    let message = if deactivated {
        "Ticket desativado com sucesso. Ele não poderá ser utilizado novamente."
    } else {
        "Este ticket é inválido, ou passou da válidade."
    };

    Ok(VerifyTicketResult {
        valid: false,
        message: message.to_string(),
        ticket_id,
    })
}

#[tauri::command]
pub fn get_all_mesas(state: State<'_, AppContext>) -> CommandResult<Vec<Mesa>> {
    state.database.get_all_mesas().map_err(CommandError::from)
}

#[tauri::command]
pub fn create_mesa(input: CreateMesaInput, state: State<'_, AppContext>) -> CommandResult<Mesa> {
    state
        .database
        .create_mesa(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_mesa_by_id(id_mesa: i64, state: State<'_, AppContext>) -> CommandResult<Mesa> {
    state
        .database
        .get_mesa_by_id(id_mesa)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_mesa_details(id_mesa: i64, state: State<'_, AppContext>) -> CommandResult<MesaDetailed> {
    state
        .database
        .get_mesa_details(id_mesa)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn add_produto_to_mesa(
    input: MesaProdutoInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    state
        .database
        .add_produto_to_mesa(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn remove_produto_from_mesa(
    input: MesaProdutoInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    state
        .database
        .remove_produto_from_mesa(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_mesa_produtos(
    id_mesa: i64,
    state: State<'_, AppContext>,
) -> CommandResult<Vec<MesaProdutoDetalhado>> {
    state
        .database
        .get_mesa_produtos(id_mesa)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn save_mesa(
    input: SaveMesaInput,
    state: State<'_, AppContext>,
) -> CommandResult<MesaDetailed> {
    state
        .database
        .replace_mesa_produtos(input.id_mesa, input.nome_cliente, input.produtos)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_mesa_cliente(
    input: UpdateMesaClienteInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    state
        .database
        .update_mesa_cliente(input.id_mesa, input.nome_cliente)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_mesa_sessao(id_mesa: i64, state: State<'_, AppContext>) -> CommandResult<MesaSessao> {
    state
        .database
        .get_mesa_sessao(id_mesa)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn fechar_mesa(
    input: FecharMesaInput,
    state: State<'_, AppContext>,
) -> CommandResult<TicketData> {
    let config = state.database.get_config().map_err(CommandError::from)?;
    let ticket_data = state
        .database
        .fechar_mesa(input)
        .map_err(CommandError::from)?;

    printer::print_pdv_ticket(&config, &ticket_data).map_err(CommandError::from)?;
    Ok(ticket_data)
}

#[tauri::command]
pub fn fechar_venda_caixa(
    input: FecharVendaCaixaInput,
    state: State<'_, AppContext>,
) -> CommandResult<TicketData> {
    let config = state.database.get_config().map_err(CommandError::from)?;
    let ticket_data = state
        .database
        .fechar_venda_caixa(input)
        .map_err(CommandError::from)?;

    printer::print_pdv_ticket(&config, &ticket_data).map_err(CommandError::from)?;
    Ok(ticket_data)
}

#[tauri::command]
pub fn get_logs(
    filtros: Option<LogFiltros>,
    state: State<'_, AppContext>,
) -> CommandResult<Vec<LogEntry>> {
    state.database.get_logs(filtros).map_err(CommandError::from)
}

#[tauri::command]
pub fn login(input: LoginInput, state: State<'_, AppContext>) -> CommandResult<AuthPayload> {
    state.database.login(input).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_users(state: State<'_, AppContext>) -> CommandResult<Vec<LocalUser>> {
    state.database.list_users().map_err(CommandError::from)
}

#[tauri::command]
pub fn create_user(
    input: CreateUserInput,
    state: State<'_, AppContext>,
) -> CommandResult<LocalUser> {
    state
        .database
        .create_user(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn update_user(
    input: UpdateUserInput,
    state: State<'_, AppContext>,
) -> CommandResult<LocalUser> {
    state
        .database
        .update_user(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn delete_user(input: DeleteUserInput, state: State<'_, AppContext>) -> CommandResult<()> {
    state
        .database
        .delete_user(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_current_cash_register(
    state: State<'_, AppContext>,
) -> CommandResult<Option<CashRegister>> {
    state
        .database
        .get_current_cash_register()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn open_cash_register(
    input: OpenCashRegisterInput,
    state: State<'_, AppContext>,
) -> CommandResult<CashRegister> {
    state
        .database
        .open_cash_register(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn close_cash_register(
    input: CloseCashRegisterInput,
    state: State<'_, AppContext>,
) -> CommandResult<CashRegister> {
    state
        .database
        .close_cash_register(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn add_cash_movement(
    input: CashMovementInput,
    state: State<'_, AppContext>,
) -> CommandResult<CashMovement> {
    state
        .database
        .add_cash_movement(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn list_cash_movements(state: State<'_, AppContext>) -> CommandResult<Vec<CashMovement>> {
    state
        .database
        .list_cash_movements()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_reports(state: State<'_, AppContext>) -> CommandResult<ReportsPayload> {
    state.database.get_reports().map_err(CommandError::from)
}

#[tauri::command]
pub fn reset_sales(input: ResetSalesInput, state: State<'_, AppContext>) -> CommandResult<()> {
    state
        .database
        .reset_sales(&input.username, &input.password)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn print_sales_report(
    input: PrintSalesReportInput,
    state: State<'_, AppContext>,
) -> CommandResult<PrintSalesReportResult> {
    let config = state.database.get_config().map_err(CommandError::from)?;
    let report = state
        .database
        .get_sales_report(&input.period)
        .map_err(CommandError::from)?;

    printer::print_sales_report(&config, &report).map_err(CommandError::from)
}

#[tauri::command]
pub fn backup_database(app: tauri::AppHandle) -> CommandResult<BackupResult> {
    let app_data = app.path().app_data_dir().map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel localizar o banco de dados: {error}"
        )))
    })?;
    let source = app_data.join("portex_pdv.sqlite");
    let backup_dir = app
        .path()
        .download_dir()
        .unwrap_or(app_data)
        .join("portex-pdv-backups");
    std::fs::create_dir_all(&backup_dir).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel criar a pasta de backup: {error}"
        )))
    })?;
    let filename = format!("portex-pdv-backup-{}.sqlite", chrono_like_timestamp());
    let destination = backup_dir.join(filename);
    std::fs::copy(&source, &destination).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel copiar o banco de dados: {error}"
        )))
    })?;
    Ok(BackupResult {
        path: destination.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn export_app_config(
    app: tauri::AppHandle,
    state: State<'_, AppContext>,
) -> CommandResult<ExportAppConfigResult> {
    let data = state
        .database
        .export_app_data()
        .map_err(CommandError::from)?;
    let json = serde_json::to_string_pretty(&data).map_err(|error| {
        CommandError::from(crate::error::AppError::System(format!(
            "Nao foi possivel preparar o arquivo de configuracao: {error}"
        )))
    })?;
    let base_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|error| {
            CommandError::from(crate::error::AppError::Io(format!(
                "Nao foi possivel localizar uma pasta para exportar: {error}"
            )))
        })?
        .join("portex-pdv-configs");

    std::fs::create_dir_all(&base_dir).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel criar a pasta de configuracoes: {error}"
        )))
    })?;
    let path = base_dir.join(format!(
        "portex-pdv-config-{}.json",
        chrono_like_timestamp()
    ));
    std::fs::write(&path, json).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel salvar as configuracoes: {error}"
        )))
    })?;

    Ok(ExportAppConfigResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn import_app_config(
    input: ImportAppConfigInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    let path = input.path.trim();
    if path.is_empty() {
        return Err(CommandError::from(crate::error::AppError::InvalidInput(
            "Informe o caminho do arquivo de configuracao.".to_string(),
        )));
    }

    let content = std::fs::read_to_string(std::path::PathBuf::from(path)).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel ler o arquivo de configuracao: {error}"
        )))
    })?;
    let data = parse_app_config_export(&content)?;

    state
        .database
        .import_app_data(data)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn import_app_config_content(
    input: ImportAppConfigContentInput,
    state: State<'_, AppContext>,
) -> CommandResult<()> {
    let data = parse_app_config_export(&input.content)?;

    state
        .database
        .import_app_data(data)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn export_csv(app: tauri::AppHandle, input: ExportCsvInput) -> CommandResult<ExportCsvResult> {
    let safe_filename = input
        .filename
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => character,
            _ => '_',
        })
        .collect::<String>();
    let filename = if safe_filename.ends_with(".csv") {
        safe_filename
    } else {
        format!("{safe_filename}.csv")
    };
    let base_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|error| {
            CommandError::from(crate::error::AppError::Io(format!(
                "Nao foi possivel localizar uma pasta para exportar: {error}"
            )))
        })?;
    let path = base_dir.join(filename);

    std::fs::write(&path, input.content).map_err(|error| {
        CommandError::from(crate::error::AppError::Io(format!(
            "Nao foi possivel salvar o CSV: {error}"
        )))
    })?;

    Ok(ExportCsvResult {
        path: path.to_string_lossy().to_string(),
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

fn chrono_like_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

fn parse_app_config_export(content: &str) -> CommandResult<AppDataExport> {
    serde_json::from_str::<AppDataExport>(content).map_err(|error| {
        CommandError::from(crate::error::AppError::InvalidInput(format!(
            "Arquivo de configuracao invalido: {error}"
        )))
    })
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

#[tauri::command]
pub fn registrar_pagamento_mesa(
    input: RegistrarPagamentoMesaInput,
    state: State<'_, AppContext>,
) -> CommandResult<PagamentoMesaResult> {
    let result = state
        .database
        .registrar_pagamento_mesa(input)
        .map_err(CommandError::from)?;
    if let Some(ticket) = &result.ticket {
        if let Ok(config) = state.database.get_config() {
            let _ = printer::print_pdv_ticket(&config, ticket);
        }
    }
    Ok(result)
}

#[tauri::command]
pub fn get_conta_mesa(id_mesa: i64, state: State<'_, AppContext>) -> CommandResult<ContaMesa> {
    state
        .database
        .get_conta_mesa(id_mesa)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_produtos_vencendo(
    dias: i64,
    state: State<'_, AppContext>,
) -> CommandResult<Vec<ProdutoVencendo>> {
    state
        .database
        .produtos_vencendo(dias)
        .map_err(CommandError::from)
}

// ===========================================================================
// FASE 5: Turno operacional + periodo contabil
// ===========================================================================

#[tauri::command]
pub fn get_cashier_status(state: State<'_, AppContext>) -> CommandResult<CashierStatus> {
    state
        .database
        .get_cashier_status()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn abrir_turno(
    input: AbrirTurnoInput,
    state: State<'_, AppContext>,
) -> CommandResult<TurnoOperacional> {
    state.database.abrir_turno(input).map_err(CommandError::from)
}

#[tauri::command]
pub fn fechar_turno(
    input: FecharTurnoInput,
    state: State<'_, AppContext>,
) -> CommandResult<TurnoOperacional> {
    state
        .database
        .fechar_turno(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn listar_turnos_dia(
    data: String,
    state: State<'_, AppContext>,
) -> CommandResult<Vec<TurnoOperacional>> {
    state
        .database
        .listar_turnos_do_dia(&data)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn consolidar_periodo(
    input: ConsolidarPeriodoInput,
    state: State<'_, AppContext>,
) -> CommandResult<PeriodoContabil> {
    state
        .database
        .consolidar_periodo(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn bloquear_periodo(
    input: BloquearPeriodoInput,
    state: State<'_, AppContext>,
) -> CommandResult<PeriodoContabil> {
    state
        .database
        .bloquear_periodo(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn editar_venda(
    input: EditarVendaInput,
    state: State<'_, AppContext>,
) -> CommandResult<SaleAuditEntry> {
    state
        .database
        .editar_venda(input)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn listar_auditoria_venda(
    sale_id: i64,
    state: State<'_, AppContext>,
) -> CommandResult<Vec<SaleAuditEntry>> {
    state
        .database
        .listar_auditoria_venda(sale_id)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_fiscal_day_config(state: State<'_, AppContext>) -> CommandResult<i64> {
    state
        .database
        .get_fiscal_day_start_minutes()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn set_fiscal_day_config(
    input: FiscalDayConfigInput,
    state: State<'_, AppContext>,
) -> CommandResult<i64> {
    state
        .database
        .set_fiscal_day_start_minutes(input.fiscal_day_start_minutes)
        .map_err(CommandError::from)
}
