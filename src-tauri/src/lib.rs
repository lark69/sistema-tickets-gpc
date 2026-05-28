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

            let database = Database::initialize(data_dir.join("sistema_tickets_gpc.sqlite"))
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
            commands::print_tickets,
            commands::verify_ticket,
            commands::list_printers,
            commands::open_creator_portfolio
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Sistema de Tickets GPC");
}
