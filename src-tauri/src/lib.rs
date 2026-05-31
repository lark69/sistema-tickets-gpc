mod commands;
mod database;
mod error;
mod models;
mod printer;

use database::Database;
use tauri::Manager;

#[derive(Clone)]
pub struct AppContext {
    database: Database,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("Nao foi possivel localizar a pasta de dados: {error}"))?;

            std::fs::create_dir_all(&data_dir)
                .map_err(|error| format!("Nao foi possivel criar a pasta de dados: {error}"))?;

            let database = Database::initialize(data_dir.join("portex_pdv.sqlite"))
                .map_err(|error| error.to_string())?;

            app.manage(AppContext { database });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::complete_onboarding,
            commands::save_app_config,
            commands::list_products,
            commands::create_product,
            commands::update_product,
            commands::delete_product,
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            commands::adjust_stock,
            commands::list_stock_movements,
            commands::print_tickets,
            commands::verify_ticket,
            commands::deactivate_ticket,
            commands::get_all_mesas,
            commands::create_mesa,
            commands::get_mesa_by_id,
            commands::get_mesa_details,
            commands::add_produto_to_mesa,
            commands::remove_produto_from_mesa,
            commands::get_mesa_produtos,
            commands::save_mesa,
            commands::update_mesa_cliente,
            commands::get_mesa_sessao,
            commands::fechar_mesa,
            commands::fechar_venda_caixa,
            commands::get_logs,
            commands::login,
            commands::list_users,
            commands::create_user,
            commands::update_user,
            commands::delete_user,
            commands::get_current_cash_register,
            commands::open_cash_register,
            commands::close_cash_register,
            commands::add_cash_movement,
            commands::list_cash_movements,
            commands::get_reports,
            commands::backup_database,
            commands::export_csv,
            commands::list_printers,
            commands::open_creator_portfolio
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Portex PDV");
}
