use crate::error::{AppError, AppResult};
use crate::models::{
    AbrirTurnoInput, AppConfig, AppConfigInput, AppDataExport, AuthPayload, BloquearPeriodoInput,
    CashMovement, CashMovementInput, CashRegister, CashierStatus, Category, CategoryInput,
    CategoryUpdateInput, CloseCashRegisterInput, ConsolidarPeriodoInput, ContaMesa,
    PagamentoMesaResult, ProdutoVencendo, RegistrarPagamentoMesaInput,
    CreateMesaInput, CreateUserInput, DeleteCategoryInput, DeleteUserInput, EditarVendaInput,
    ExportedCategory, ExportedProduct, FecharMesaInput, FecharTurnoInput, FecharVendaCaixaInput,
    IssuedTicket, LocalUser, LogEntry, LogFiltros, LoginInput, Mesa, MesaDetailed,
    MesaProdutoDetalhado, MesaProdutoInput, MesaSessao, OpenCashRegisterInput, PeriodoContabil,
    PeriodoStatus, Product, ProductInput, ProductUpdateInput, ReportsPayload, SaleAuditEntry,
    SaleCartItemInput, SalesByDay, SalesReportData, StockAdjustInput, StockMovement, TicketData,
    TicketProduto, TopProductReport, TurnoOperacional, TurnoStatus, UpdateUserInput,
};
use chrono::{Datelike, Days, Local, NaiveDate, NaiveDateTime, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static TICKET_COUNTER: AtomicU64 = AtomicU64::new(1);
const MANAGE_PRODUCTS_PERMISSION: &str = "manageProducts";
const MANAGE_USERS_PERMISSION: &str = "manageUsers";
const DEFAULT_OPERATOR_PERMISSIONS_JSON: &str = "[\"addTableProducts\",\"removeTableProducts\",\"closeTable\",\"manageTickets\",\"manageCash\",\"manageCashMovements\"]";

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn initialize(path: PathBuf) -> AppResult<Self> {
        let database = Self { path };
        database.migrate()?;
        database.cleanup_expired_tickets()?;
        Ok(database)
    }

    pub fn get_config(&self) -> AppResult<AppConfig> {
        let connection = self.connection()?;
        let config = connection.query_row(
            "SELECT company_name, tax_id, thank_you_message, validity_days, theme,
                    printer_name, print_width_chars, onboarding_completed, setup_completed,
                    table_count, backup_time, updated_at
             FROM app_config WHERE id = 1",
            [],
            |row| {
                Ok(AppConfig {
                    company_name: row.get(0)?,
                    tax_id: row.get(1)?,
                    thank_you_message: row.get(2)?,
                    validity_days: row.get(3)?,
                    theme: row.get(4)?,
                    printer_name: row.get(5)?,
                    print_width_chars: row.get(6)?,
                    onboarding_completed: row.get::<_, i64>(7)? == 1,
                    setup_completed: row.get::<_, i64>(8)? == 1,
                    table_count: row.get(9)?,
                    backup_time: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )?;

        Ok(config)
    }

    pub fn complete_onboarding(&self) -> AppResult<AppConfig> {
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_config SET onboarding_completed = 1, updated_at = ?1 WHERE id = 1",
            params![now],
        )?;
        self.get_config()
    }

    pub fn save_config(&self, input: AppConfigInput) -> AppResult<AppConfig> {
        let normalized = validate_config(input)?;
        let now = now_millis();
        let connection = self.connection()?;

        connection.execute(
            "UPDATE app_config
             SET company_name = ?1,
                 tax_id = ?2,
                 thank_you_message = ?3,
                 validity_days = ?4,
                 theme = ?5,
                 printer_name = ?6,
                 print_width_chars = ?7,
                 onboarding_completed = 1,
                 setup_completed = ?8,
                 table_count = ?9,
                 backup_time = ?10,
                 updated_at = ?11
             WHERE id = 1",
            params![
                normalized.company_name,
                normalized.tax_id,
                normalized.thank_you_message,
                normalized.validity_days,
                normalized.theme,
                normalized.printer_name,
                normalized.print_width_chars,
                if normalized.setup_completed { 1 } else { 0 },
                normalized.table_count,
                normalized.backup_time,
                now
            ],
        )?;

        self.ensure_default_mesas()?;
        self.get_config()
    }

    pub fn export_app_data(&self) -> AppResult<AppDataExport> {
        let config = self.get_config()?;
        let categories = self
            .list_categories()?
            .into_iter()
            .map(|category| ExportedCategory {
                name: category.name,
            })
            .collect::<Vec<_>>();
        let products = self
            .list_products()?
            .into_iter()
            .map(|product| ExportedProduct {
                name: product.name,
                price_cents: product.price_cents,
                barcode: product.barcode,
                cost_price_cents: product.cost_price_cents,
                unit: product.unit,
                category_name: product.category_name,
                stock: product.stock,
                reorder_level: product.reorder_level,
                description: product.description,
            })
            .collect::<Vec<_>>();

        Ok(AppDataExport {
            version: 1,
            exported_at: now_millis(),
            company_name: config.company_name,
            tax_id: config.tax_id,
            print_width_chars: config.print_width_chars,
            categories,
            products,
        })
    }

    pub fn import_app_data(&self, data: AppDataExport) -> AppResult<()> {
        let company_name = data.company_name.trim().to_string();
        let tax_id = data.tax_id.trim().to_string();
        if company_name.is_empty() {
            return Err(AppError::InvalidInput(
                "O arquivo importado nao possui nome da empresa.".to_string(),
            ));
        }
        if tax_id.is_empty() {
            return Err(AppError::InvalidInput(
                "O arquivo importado nao possui CPF/CNPJ.".to_string(),
            ));
        }
        if data.print_width_chars < 32 || data.print_width_chars > 64 {
            return Err(AppError::InvalidInput(
                "A largura de impressao importada deve ficar entre 32 e 64 caracteres.".to_string(),
            ));
        }

        let mut category_names = Vec::new();
        let mut category_keys = BTreeMap::new();
        for category in data.categories {
            let name = category.name.trim().to_string();
            if name.is_empty() {
                continue;
            }

            let key = name.to_lowercase();
            if !category_keys.contains_key(&key) {
                category_keys.insert(key, ());
                category_names.push(name);
            }
        }

        let mut normalized_products = Vec::new();
        for product in data.products {
            let category_name = product
                .category_name
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            if let Some(name) = &category_name {
                let key = name.to_lowercase();
                if !category_keys.contains_key(&key) {
                    category_keys.insert(key, ());
                    category_names.push(name.clone());
                }
            }

            let normalized = validate_product(ProductInput {
                name: product.name,
                price_cents: product.price_cents,
                barcode: product.barcode,
                cost_price_cents: product.cost_price_cents,
                unit: product.unit,
                category_id: None,
                stock: product.stock,
                reorder_level: product.reorder_level,
                description: product.description,
                validade: None,
            })?;
            normalized_products.push((normalized, category_name));
        }

        let now = now_millis();
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;

        transaction.execute(
            "UPDATE app_config
             SET company_name = ?1, tax_id = ?2, print_width_chars = ?3, updated_at = ?4
             WHERE id = 1",
            params![company_name, tax_id, data.print_width_chars, now],
        )?;

        transaction.execute("DELETE FROM mesa_produtos", [])?;
        transaction.execute(
            "UPDATE mesa_sessao SET tempo_fim = ?1 WHERE tempo_fim IS NULL",
            params![now],
        )?;
        transaction.execute("DELETE FROM products", [])?;
        transaction.execute("DELETE FROM categories", [])?;

        let mut category_ids = BTreeMap::new();
        for name in category_names {
            transaction.execute(
                "INSERT INTO categories (name, created_at) VALUES (?1, ?2)",
                params![&name, now],
            )?;
            category_ids.insert(name.to_lowercase(), transaction.last_insert_rowid());
        }

        for (product, category_name) in normalized_products {
            let category_id =
                category_name.and_then(|name| category_ids.get(&name.to_lowercase()).copied());
            transaction.execute(
                "INSERT INTO products (
                    name, price_cents, barcode, cost_price_cents, unit, category_id,
                    stock, reorder_level, description, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    product.name,
                    product.price_cents,
                    product.barcode,
                    product.cost_price_cents,
                    product.unit,
                    category_id,
                    product.stock,
                    product.reorder_level,
                    product.description,
                    now,
                    now
                ],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }

    pub fn list_products(&self) -> AppResult<Vec<Product>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT p.id, p.name, p.price_cents, p.barcode, p.cost_price_cents, p.unit,
                    p.category_id, c.name, p.stock, p.reorder_level, p.description,
                    p.created_at, p.updated_at,
                    COALESCE(sold.sold_quantity, 0),
                    p.validade
             FROM products p
             LEFT JOIN categories c ON c.id = p.category_id
             LEFT JOIN (
                SELECT product_id, SUM(quantity) AS sold_quantity
                FROM sale_items
                GROUP BY product_id
             ) sold ON sold.product_id = p.id
             ORDER BY p.name COLLATE NOCASE ASC",
        )?;

        let products = statement
            .query_map([], map_product)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(products)
    }

    pub fn get_product(&self, product_id: i64) -> AppResult<Product> {
        let connection = self.connection()?;
        let product = connection
            .query_row(
                "SELECT p.id, p.name, p.price_cents, p.barcode, p.cost_price_cents, p.unit,
                        p.category_id, c.name, p.stock, p.reorder_level, p.description,
                        p.created_at, p.updated_at,
                        COALESCE(sold.sold_quantity, 0),
                        p.validade
                 FROM products p
                 LEFT JOIN categories c ON c.id = p.category_id
                 LEFT JOIN (
                    SELECT product_id, SUM(quantity) AS sold_quantity
                    FROM sale_items
                    GROUP BY product_id
                 ) sold ON sold.product_id = p.id
                 WHERE p.id = ?1",
                params![product_id],
                map_product,
            )
            .optional()?;

        product.ok_or_else(|| AppError::InvalidInput("Produto nao encontrado.".to_string()))
    }

    pub fn create_product(&self, input: ProductInput) -> AppResult<Product> {
        let normalized = validate_product(input)?;
        let now = now_millis();
        let connection = self.connection()?;

        connection.execute(
            "INSERT INTO products (
                name, price_cents, barcode, cost_price_cents, unit, category_id,
                stock, reorder_level, description, validade, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                normalized.name,
                normalized.price_cents,
                normalized.barcode,
                normalized.cost_price_cents,
                normalized.unit,
                normalized.category_id,
                normalized.stock,
                normalized.reorder_level,
                normalized.description,
                normalized.validade,
                now,
                now
            ],
        )?;

        let product = self.get_product(connection.last_insert_rowid())?;
        let _ = self.insert_log(
            "produto_criado",
            None,
            None,
            None,
            None,
            None,
            Some(format!(
                "[{{\"nome\":\"{}\",\"quantidade\":1,\"precoUnitCents\":{},\"subtotalCents\":{}}}]",
                product.name.replace('"', "'"),
                product.price_cents,
                product.price_cents
            )),
            None,
            None,
        );
        Ok(product)
    }

    pub fn update_product(&self, input: ProductUpdateInput) -> AppResult<Product> {
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Produto invalido.".to_string()));
        }

        let normalized = validate_product(ProductInput {
            name: input.name,
            price_cents: input.price_cents,
            barcode: input.barcode,
            cost_price_cents: input.cost_price_cents,
            unit: input.unit,
            category_id: input.category_id,
            stock: input.stock,
            reorder_level: input.reorder_level,
            description: input.description,
            validade: input.validade,
        })?;
        let now = now_millis();
        let connection = self.connection()?;

        let affected = connection.execute(
            "UPDATE products
             SET name = ?1, price_cents = ?2, barcode = ?3, cost_price_cents = ?4,
                 unit = ?5, category_id = ?6, stock = ?7, reorder_level = ?8,
                 description = ?9, validade = ?10, updated_at = ?11
             WHERE id = ?12",
            params![
                normalized.name,
                normalized.price_cents,
                normalized.barcode,
                normalized.cost_price_cents,
                normalized.unit,
                normalized.category_id,
                normalized.stock,
                normalized.reorder_level,
                normalized.description,
                normalized.validade,
                now,
                input.id
            ],
        )?;

        if affected == 0 {
            return Err(AppError::InvalidInput(
                "Produto nao encontrado.".to_string(),
            ));
        }

        let product = self.get_product(input.id)?;
        let _ = self.insert_log(
            "produto_editado",
            None,
            None,
            Some(product.price_cents),
            None,
            None,
            Some(format!(
                "[{{\"nome\":\"{}\",\"estoque\":{},\"precoUnitCents\":{}}}]",
                product.name.replace('"', "'"),
                product.stock,
                product.price_cents
            )),
            None,
            None,
        );
        Ok(product)
    }

    pub fn delete_product(&self, product_id: i64) -> AppResult<()> {
        if product_id <= 0 {
            return Err(AppError::InvalidInput("Produto invalido.".to_string()));
        }

        let connection = self.connection()?;
        let affected =
            connection.execute("DELETE FROM products WHERE id = ?1", params![product_id])?;

        if affected == 0 {
            return Err(AppError::InvalidInput(
                "Produto nao encontrado.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn issue_tickets(
        &self,
        product: &Product,
        quantity: i64,
        validity_days: i64,
    ) -> AppResult<Vec<IssuedTicket>> {
        if quantity < 1 || quantity > 999 {
            return Err(AppError::InvalidInput(
                "A quantidade deve ficar entre 1 e 999 tickets.".to_string(),
            ));
        }

        let now = now_millis();
        let expires_at = now + validity_days.saturating_mul(86_400_000);
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;
        let mut tickets = Vec::new();
        let batch_counter = TICKET_COUNTER.fetch_add(quantity as u64, Ordering::Relaxed);

        for index in 0..quantity {
            let mut attempt = 0u64;
            let ticket_id = loop {
                let candidate = generate_ticket_id(now, batch_counter + index as u64 + attempt);
                let inserted = transaction.execute(
                    "INSERT OR IGNORE INTO issued_tickets (
                        ticket_id, product_id, product_name, price_cents, issued_at, expires_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        candidate,
                        product.id,
                        product.name,
                        product.price_cents,
                        now,
                        expires_at
                    ],
                )?;

                if inserted == 1 {
                    break candidate;
                }

                attempt += 1;
                if attempt > 64 {
                    return Err(AppError::Database(
                        "Nao foi possivel gerar um ID unico para o ticket.".to_string(),
                    ));
                }
            };

            tickets.push(IssuedTicket {
                ticket_id,
                expires_at,
            });
        }

        transaction.commit()?;
        Ok(tickets)
    }

    pub fn delete_issued_tickets(&self, ticket_ids: &[String]) -> AppResult<()> {
        let connection = self.connection()?;

        for ticket_id in ticket_ids {
            connection.execute(
                "DELETE FROM issued_tickets WHERE ticket_id = ?1",
                params![ticket_id],
            )?;
        }

        Ok(())
    }

    pub fn ensure_tickets_can_be_printed(&self) -> AppResult<()> {
        if self.get_turno_ativo()?.is_none() {
            return Err(AppError::InvalidInput(
                "Abra um turno antes de imprimir tickets.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn record_ticket_sale(&self, product: &Product, quantity: i64) -> AppResult<()> {
        if quantity < 1 {
            return Err(AppError::InvalidInput(
                "A quantidade de tickets deve ser maior que zero.".to_string(),
            ));
        }

        let now = now_millis();
        let total_cents = product.price_cents * quantity;
        let estimated_profit_cents = (product.price_cents - product.cost_price_cents) * quantity;
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;

        transaction.execute(
            "INSERT INTO sales (
                mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                created_at, nfe_status
             ) VALUES (NULL, 'ticket', 'ticket', ?1, 0, 0, ?2, ?3, 'ticket', ?4, 'placeholder')",
            params![total_cents, total_cents, estimated_profit_cents, now],
        )?;
        let sale_id = transaction.last_insert_rowid();

        transaction.execute(
            "INSERT INTO sale_items (
                sale_id, product_id, product_name, quantity, unit_price_cents,
                cost_price_cents, subtotal_cents
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                sale_id,
                product.id,
                product.name,
                quantity,
                product.price_cents,
                product.cost_price_cents,
                total_cents
            ],
        )?;

        transaction.commit()?;
        Ok(())
    }

    pub fn verify_ticket(&self, ticket_id: &str) -> AppResult<bool> {
        self.cleanup_expired_tickets()?;
        let normalized_id = normalize_ticket_id(ticket_id)?;
        let now = now_millis();
        let connection = self.connection()?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM issued_tickets WHERE ticket_id = ?1 AND expires_at >= ?2",
                params![normalized_id, now],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        Ok(exists)
    }

    pub fn deactivate_ticket(&self, ticket_id: &str) -> AppResult<bool> {
        self.cleanup_expired_tickets()?;
        let normalized_id = normalize_ticket_id(ticket_id)?;
        let now = now_millis();
        let connection = self.connection()?;
        let affected = connection.execute(
            "DELETE FROM issued_tickets WHERE ticket_id = ?1 AND expires_at >= ?2",
            params![normalized_id, now],
        )?;

        Ok(affected > 0)
    }

    pub fn cleanup_expired_tickets(&self) -> AppResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "DELETE FROM issued_tickets WHERE expires_at < ?1",
            params![now_millis()],
        )?;
        Ok(())
    }

    pub fn list_categories(&self) -> AppResult<Vec<Category>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT id, name, created_at FROM categories ORDER BY name COLLATE NOCASE")?;
        let categories = statement
            .query_map([], map_category)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(categories)
    }

    pub fn create_category(
        &self,
        input: CategoryInput,
        operator_name: Option<String>,
        requester_role: Option<String>,
        requester_permissions: Option<Vec<String>>,
    ) -> AppResult<Category> {
        ensure_permission(
            requester_role.as_deref(),
            requester_permissions.as_deref(),
            MANAGE_PRODUCTS_PERMISSION,
        )?;
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::InvalidInput(
                "Informe o nome da categoria.".to_string(),
            ));
        }
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO categories (name, created_at) VALUES (?1, ?2)",
            params![name, now],
        )?;
        let category = connection.query_row(
            "SELECT id, name, created_at FROM categories WHERE id = ?1",
            params![connection.last_insert_rowid()],
            map_category,
        )?;
        let _ = self.insert_log(
            "categoria_criada",
            None,
            operator_name,
            None,
            None,
            None,
            Some(format!(
                "[{{\"categoria\":\"{}\"}}]",
                category.name.replace('"', "'")
            )),
            None,
            None,
        );
        Ok(category)
    }

    pub fn update_category(&self, input: CategoryUpdateInput) -> AppResult<Category> {
        ensure_permission(
            input.requester_role.as_deref(),
            input.requester_permissions.as_deref(),
            MANAGE_PRODUCTS_PERMISSION,
        )?;
        let name = input.name.trim().to_string();
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Categoria invalida.".to_string()));
        }
        if name.is_empty() {
            return Err(AppError::InvalidInput(
                "Informe o nome da categoria.".to_string(),
            ));
        }

        let connection = self.connection()?;
        let affected = connection.execute(
            "UPDATE categories SET name = ?1 WHERE id = ?2",
            params![name, input.id],
        )?;

        if affected == 0 {
            return Err(AppError::InvalidInput(
                "Categoria nao encontrada.".to_string(),
            ));
        }

        let category = connection.query_row(
            "SELECT id, name, created_at FROM categories WHERE id = ?1",
            params![input.id],
            map_category,
        )?;
        let _ = self.insert_log(
            "categoria_editada",
            None,
            None,
            None,
            None,
            None,
            Some(format!(
                "[{{\"categoria\":\"{}\"}}]",
                category.name.replace('"', "'")
            )),
            None,
            None,
        );
        Ok(category)
    }

    pub fn delete_category(&self, input: DeleteCategoryInput) -> AppResult<()> {
        ensure_permission(
            input.requester_role.as_deref(),
            input.requester_permissions.as_deref(),
            MANAGE_PRODUCTS_PERMISSION,
        )?;
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Categoria invalida.".to_string()));
        }

        let connection = self.connection()?;
        connection.execute(
            "UPDATE products SET category_id = NULL, updated_at = ?1 WHERE category_id = ?2",
            params![now_millis(), input.id],
        )?;
        let affected =
            connection.execute("DELETE FROM categories WHERE id = ?1", params![input.id])?;

        if affected == 0 {
            return Err(AppError::InvalidInput(
                "Categoria nao encontrada.".to_string(),
            ));
        }

        Ok(())
    }

    pub fn adjust_stock(&self, input: StockAdjustInput) -> AppResult<StockMovement> {
        let movement_type = input.movement_type.trim().to_lowercase();
        if !matches!(movement_type.as_str(), "entrada" | "ajuste" | "saida") {
            return Err(AppError::InvalidInput(
                "Tipo de movimento de estoque invalido.".to_string(),
            ));
        }
        if input.quantity == 0 {
            return Err(AppError::InvalidInput(
                "Informe uma quantidade diferente de zero.".to_string(),
            ));
        }
        let product = self.get_product(input.product_id)?;
        let previous_stock = product.stock;
        let new_stock = match movement_type.as_str() {
            "entrada" => previous_stock + input.quantity.abs(),
            "saida" => previous_stock - input.quantity.abs(),
            _ => input.quantity,
        };
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_stock, now, input.product_id],
        )?;
        connection.execute(
            "INSERT INTO stock_movements (
                product_id, movement_type, quantity, previous_stock, new_stock,
                operator_name, note, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.product_id,
                movement_type,
                input.quantity,
                previous_stock,
                new_stock,
                input.operator_name,
                input.note,
                now
            ],
        )?;
        Ok(StockMovement {
            id: connection.last_insert_rowid(),
            product_id: input.product_id,
            product_name: product.name,
            movement_type,
            quantity: input.quantity,
            previous_stock,
            new_stock,
            operator_name: input.operator_name,
            note: input.note,
            created_at: now,
        })
    }

    pub fn list_stock_movements(&self) -> AppResult<Vec<StockMovement>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT sm.id, sm.product_id, p.name, sm.movement_type, sm.quantity,
                    sm.previous_stock, sm.new_stock, sm.operator_name, sm.note, sm.created_at
             FROM stock_movements sm
             INNER JOIN products p ON p.id = sm.product_id
             ORDER BY sm.created_at DESC
             LIMIT 300",
        )?;
        let movements = statement
            .query_map([], map_stock_movement)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(movements)
    }

    pub fn get_all_mesas(&self) -> AppResult<Vec<Mesa>> {
        self.ensure_default_mesas()?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT m.id, m.numero, m.capacidade, m.criada_em,
                    CASE WHEN COALESCE(SUM(mp.quantidade), 0) > 0 THEN 'ativa' ELSE 'livre' END AS status,
                    ms.tempo_inicio,
                    COALESCE(SUM(mp.quantidade * p.price_cents), 0) AS total_cents
             FROM mesas m
             LEFT JOIN mesa_produtos mp ON mp.id_mesa = m.id
             LEFT JOIN products p ON p.id = mp.id_produto
             LEFT JOIN mesa_sessao ms ON ms.id_mesa = m.id AND ms.tempo_fim IS NULL
             WHERE m.numero <= (SELECT table_count FROM app_config WHERE id = 1)
             GROUP BY m.id, m.numero, m.capacidade, m.criada_em, ms.tempo_inicio
             ORDER BY m.numero ASC",
        )?;

        let mesas = statement
            .query_map([], map_mesa)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mesas)
    }

    pub fn create_mesa(&self, input: CreateMesaInput) -> AppResult<Mesa> {
        if input.numero < 1 || input.numero > 100 {
            return Err(AppError::InvalidInput(
                "O numero da mesa deve ficar entre 1 e 100.".to_string(),
            ));
        }

        let connection = self.connection()?;
        connection.execute(
            "INSERT OR IGNORE INTO mesas (numero, capacidade, criada_em)
             VALUES (?1, ?2, ?3)",
            params![input.numero, input.capacidade, now_millis()],
        )?;

        let id_mesa = connection.query_row(
            "SELECT id FROM mesas WHERE numero = ?1",
            params![input.numero],
            |row| row.get::<_, i64>(0),
        )?;

        self.get_mesa_by_id(id_mesa)
    }

    pub fn get_mesa_by_id(&self, id_mesa: i64) -> AppResult<Mesa> {
        self.ensure_default_mesas()?;
        let connection = self.connection()?;
        let mesa = connection
            .query_row(
                "SELECT m.id, m.numero, m.capacidade, m.criada_em,
                        CASE WHEN COALESCE(SUM(mp.quantidade), 0) > 0 THEN 'ativa' ELSE 'livre' END AS status,
                        ms.tempo_inicio,
                        COALESCE(SUM(mp.quantidade * p.price_cents), 0) AS total_cents
                 FROM mesas m
                 LEFT JOIN mesa_produtos mp ON mp.id_mesa = m.id
                 LEFT JOIN products p ON p.id = mp.id_produto
                 LEFT JOIN mesa_sessao ms ON ms.id_mesa = m.id AND ms.tempo_fim IS NULL
                 WHERE m.id = ?1
                 GROUP BY m.id, m.numero, m.capacidade, m.criada_em, ms.tempo_inicio",
                params![id_mesa],
                map_mesa,
            )
            .optional()?;

        mesa.ok_or_else(|| AppError::InvalidInput("Mesa nao encontrada.".to_string()))
    }

    // ===== FASE 4: pagamento parcial + alerta de validade =====

    pub fn registrar_pagamento_mesa(
        &self,
        input: RegistrarPagamentoMesaInput,
    ) -> AppResult<PagamentoMesaResult> {
        let operator = input
            .operator_name
            .as_deref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or("caixa")
            .to_string();
        let turno = self.get_turno_ativo()?.ok_or_else(|| {
            AppError::InvalidInput(
                "Abra um turno em Fechamento antes de receber pagamentos.".to_string(),
            )
        })?;
        let connection = self.connection()?;
        crate::payments::registrar_pagamento_mesa(
            &connection,
            input.id_mesa,
            &input.forma_pagamento,
            input.valor_cents,
            &operator,
            input.aplicar_acrescimo.unwrap_or(false),
            input.aplicar_garcom.unwrap_or(false),
            turno.id,
        )
    }

    pub fn get_conta_mesa(&self, id_mesa: i64) -> AppResult<ContaMesa> {
        let connection = self.connection()?;
        crate::payments::conta_mesa(&connection, id_mesa)
    }

    pub fn produtos_vencendo(&self, dias: i64) -> AppResult<Vec<ProdutoVencendo>> {
        let connection = self.connection()?;
        let now = now_millis();
        let limite = now + dias.max(0) * 86_400_000;
        let mut stmt = connection.prepare(
            "SELECT id, name, validade FROM products
             WHERE validade IS NOT NULL AND validade <= ?1
             ORDER BY validade ASC",
        )?;
        let rows = stmt
            .query_map(params![limite], |row| {
                let validade: i64 = row.get(2)?;
                Ok(ProdutoVencendo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    validade,
                    dias_restantes: ((validade - now) as f64 / 86_400_000.0).floor() as i64,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_mesa_details(&self, id_mesa: i64) -> AppResult<MesaDetailed> {
        let mesa = self.get_mesa_by_id(id_mesa)?;
        let produtos = self.get_mesa_produtos(id_mesa)?;
        let sessao = self.get_mesa_sessao_optional(id_mesa)?;
        let subtotal_cents = produtos.iter().map(|produto| produto.subtotal_cents).sum();

        Ok(MesaDetailed {
            mesa,
            sessao,
            produtos,
            subtotal_cents,
        })
    }

    pub fn get_mesa_produtos(&self, id_mesa: i64) -> AppResult<Vec<MesaProdutoDetalhado>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT mp.id, mp.id_mesa, mp.id_produto, mp.quantidade, mp.adicionado_em,
                    p.id, p.name, p.price_cents, p.barcode, p.cost_price_cents, p.unit,
                    p.category_id, c.name, p.stock, p.reorder_level, p.description,
                    p.created_at, p.updated_at
             FROM mesa_produtos mp
             INNER JOIN products p ON p.id = mp.id_produto
             LEFT JOIN categories c ON c.id = p.category_id
             WHERE mp.id_mesa = ?1
             ORDER BY mp.adicionado_em ASC, p.name COLLATE NOCASE ASC",
        )?;

        let produtos = statement
            .query_map(params![id_mesa], map_mesa_produto)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(produtos)
    }

    pub fn add_produto_to_mesa(&self, input: MesaProdutoInput) -> AppResult<()> {
        validate_mesa_product_input(&input)?;
        if self.get_turno_ativo()?.is_none() {
            return Err(AppError::InvalidInput(
                "Abra um turno antes de adicionar produtos a mesa.".to_string(),
            ));
        }
        let now = now_millis();
        let connection = self.connection()?;
        self.get_mesa_by_id(input.id_mesa)?;
        self.get_product(input.id_produto)?;

        let existing_id = connection
            .query_row(
                "SELECT id FROM mesa_produtos WHERE id_mesa = ?1 AND id_produto = ?2",
                params![input.id_mesa, input.id_produto],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            connection.execute(
                "UPDATE mesa_produtos SET quantidade = quantidade + ?1 WHERE id = ?2",
                params![input.quantidade, id],
            )?;
        } else {
            connection.execute(
                "INSERT INTO mesa_produtos (id_mesa, id_produto, quantidade, adicionado_em)
                 VALUES (?1, ?2, ?3, ?4)",
                params![input.id_mesa, input.id_produto, input.quantidade, now],
            )?;
        }

        self.ensure_open_session(input.id_mesa, None)?;
        Ok(())
    }

    pub fn remove_produto_from_mesa(&self, input: MesaProdutoInput) -> AppResult<()> {
        validate_mesa_product_input(&input)?;
        let connection = self.connection()?;
        let current_quantity = connection
            .query_row(
                "SELECT quantidade FROM mesa_produtos WHERE id_mesa = ?1 AND id_produto = ?2",
                params![input.id_mesa, input.id_produto],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        let Some(current_quantity) = current_quantity else {
            return Ok(());
        };

        if current_quantity <= input.quantidade {
            connection.execute(
                "DELETE FROM mesa_produtos WHERE id_mesa = ?1 AND id_produto = ?2",
                params![input.id_mesa, input.id_produto],
            )?;
        } else {
            connection.execute(
                "UPDATE mesa_produtos SET quantidade = quantidade - ?1
                 WHERE id_mesa = ?2 AND id_produto = ?3",
                params![input.quantidade, input.id_mesa, input.id_produto],
            )?;
        }

        Ok(())
    }

    pub fn replace_mesa_produtos(
        &self,
        id_mesa: i64,
        nome_cliente: Option<String>,
        items: Vec<MesaProdutoInput>,
    ) -> AppResult<MesaDetailed> {
        self.get_mesa_by_id(id_mesa)?;
        let normalized_cliente = normalize_optional_text(nome_cliente);
        let now = now_millis();
        let connection = self.connection()?;
        let mut current_quantities = BTreeMap::new();
        let mut statement = connection
            .prepare("SELECT id_produto, quantidade FROM mesa_produtos WHERE id_mesa = ?1")?;
        for row in statement.query_map(params![id_mesa], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })? {
            let (product_id, quantity) = row?;
            current_quantities.insert(product_id, quantity);
        }

        let mut next_quantities = BTreeMap::new();
        for item in &items {
            validate_mesa_product_input(item)?;
            if item.id_mesa != id_mesa {
                return Err(AppError::InvalidInput(
                    "Produto enviado para mesa invalida.".to_string(),
                ));
            }
            *next_quantities.entry(item.id_produto).or_insert(0) += item.quantidade;
        }

        let adds_product = next_quantities.iter().any(|(product_id, quantity)| {
            *quantity > current_quantities.get(product_id).copied().unwrap_or(0)
        });

        if adds_product && self.get_turno_ativo()?.is_none() {
            return Err(AppError::InvalidInput(
                "Abra um turno antes de adicionar produtos a mesa.".to_string(),
            ));
        }

        drop(statement);
        let transaction = connection.unchecked_transaction()?;

        transaction.execute(
            "DELETE FROM mesa_produtos WHERE id_mesa = ?1",
            params![id_mesa],
        )?;

        let has_items = !items.is_empty();
        for item in items {
            transaction.execute(
                "INSERT INTO mesa_produtos (id_mesa, id_produto, quantidade, adicionado_em)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id_mesa, item.id_produto, item.quantidade, now],
            )?;
        }

        transaction.commit()?;
        if has_items {
            self.ensure_open_session(id_mesa, normalized_cliente)?;
        } else {
            let connection = self.connection()?;
            connection.execute(
                "UPDATE mesa_sessao SET tempo_fim = ?1 WHERE id_mesa = ?2 AND tempo_fim IS NULL",
                params![now_millis(), id_mesa],
            )?;
        }
        self.get_mesa_details(id_mesa)
    }

    pub fn update_mesa_cliente(&self, id_mesa: i64, nome_cliente: Option<String>) -> AppResult<()> {
        let normalized_cliente = normalize_optional_text(nome_cliente);
        self.ensure_open_session(id_mesa, normalized_cliente)
    }

    pub fn get_mesa_sessao(&self, id_mesa: i64) -> AppResult<MesaSessao> {
        self.get_mesa_sessao_optional(id_mesa)?
            .ok_or_else(|| AppError::InvalidInput("Mesa sem sessao aberta.".to_string()))
    }

    pub fn fechar_mesa(&self, input: FecharMesaInput) -> AppResult<TicketData> {
        let forma_pagamento = validate_payment_method(&input.forma_pagamento)?;
        let operator_name = normalize_optional_text(input.operator_name.clone())
            .unwrap_or_else(|| "caixa".to_string());
        let turno = self.get_turno_ativo()?.ok_or_else(|| {
            AppError::InvalidInput(
                "Abra um turno em Fechamento antes de fechar a mesa.".to_string(),
            )
        })?;
        let details = self.get_mesa_details(input.id_mesa)?;

        if details.produtos.is_empty() {
            return Err(AppError::InvalidInput(
                "Adicione produtos antes de fechar a mesa.".to_string(),
            ));
        }

        let sessao = details
            .sessao
            .ok_or_else(|| AppError::InvalidInput("Mesa sem sessao aberta.".to_string()))?;
        let now = now_millis();
        let subtotal_cents = details.subtotal_cents;
        // Acréscimo de crédito (5%) só quando o operador opta por ele no crédito;
        // taxa de garçom (10%) é independente da forma de pagamento. Podem somar.
        let acrescimo_credito = if forma_pagamento == "credito" && input.aplicar_acrescimo.unwrap_or(false) {
            (subtotal_cents * 500 + 5_000) / 10_000
        } else {
            0
        };
        let acrescimo_garcom = if input.aplicar_garcom.unwrap_or(false) {
            (subtotal_cents * 1_000 + 5_000) / 10_000
        } else {
            0
        };
        let acrescimo_cents = acrescimo_credito + acrescimo_garcom;
        let total_cents = subtotal_cents + acrescimo_cents;
        let valor_pago_cents = if forma_pagamento == "dinheiro" {
            let paid = input.valor_pago_cents.ok_or_else(|| {
                AppError::InvalidInput("Informe o valor pago em dinheiro.".to_string())
            })?;
            if paid < total_cents {
                return Err(AppError::InvalidInput(
                    "O valor pago deve ser maior ou igual ao total.".to_string(),
                ));
            }
            Some(paid)
        } else {
            input.valor_pago_cents
        };
        let troco_cents = valor_pago_cents.map(|paid| (paid - total_cents).max(0));
        let tempo_permanencia = format_duration(now - sessao.tempo_inicio);
        let produtos = details
            .produtos
            .iter()
            .map(|item| TicketProduto {
                nome: item.produto.name.clone(),
                quantidade: item.quantidade,
                preco_unit_cents: item.produto.price_cents,
                subtotal_cents: item.subtotal_cents,
            })
            .collect::<Vec<_>>();
        let ticket_data = TicketData {
            numero_mesa: details.mesa.numero,
            nome_cliente: sessao.nome_cliente.clone(),
            tempo_permanencia: tempo_permanencia.clone(),
            id_unico: sessao.id_unico.clone(),
            forma_pagamento: forma_pagamento.clone(),
            subtotal_cents,
            acrescimo_cents,
            total_cents,
            valor_pago_cents,
            troco_cents,
            produtos: produtos.clone(),
        };
        let produtos_json = serde_json::to_string(&produtos).ok();
        let connection = self.connection()?;
        let estimated_profit_cents = details
            .produtos
            .iter()
            .map(|item| {
                (item.produto.price_cents - item.produto.cost_price_cents) * item.quantidade
            })
            .sum::<i64>();

        connection.execute(
            "INSERT INTO sales (
                mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                created_at, nfe_status
            ) VALUES (?1, 'mesa', ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, 'placeholder')",
            params![
                details.mesa.numero,
                operator_name.clone(),
                subtotal_cents,
                acrescimo_cents,
                total_cents,
                estimated_profit_cents,
                ticket_data.forma_pagamento,
                now
            ],
        )?;
        let sale_id = connection.last_insert_rowid();

        // Registra o pagamento no razao (sale_payments) ja vinculado a venda e
        // carimbado com o turno ativo, para que o dinheiro entre na gaveta do
        // turno correto (mantem fechar_mesa consistente com o fluxo de mesa).
        connection.execute(
            "INSERT INTO sale_payments
                (id_mesa, id_sessao, sale_id, forma_pagamento, valor_cents, troco_cents, surcharge_cents, operator_name, created_at, turno_operacional_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                input.id_mesa,
                sessao.id,
                sale_id,
                ticket_data.forma_pagamento,
                total_cents,
                troco_cents.unwrap_or(0),
                acrescimo_cents,
                operator_name.clone(),
                now,
                turno.id
            ],
        )?;

        for item in &details.produtos {
            connection.execute(
                "INSERT INTO sale_items (
                    sale_id, product_id, product_name, quantity, unit_price_cents,
                    cost_price_cents, subtotal_cents
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    sale_id,
                    item.produto.id,
                    item.produto.name,
                    item.quantidade,
                    item.produto.price_cents,
                    item.produto.cost_price_cents,
                    item.subtotal_cents
                ],
            )?;
            let previous_stock = item.produto.stock;
            let new_stock = previous_stock - item.quantidade;
            connection.execute(
                "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_stock, now, item.produto.id],
            )?;
            connection.execute(
                "INSERT INTO stock_movements (
                    product_id, movement_type, quantity, previous_stock, new_stock,
                    operator_name, note, created_at
                 ) VALUES (?1, 'venda', ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item.produto.id,
                    -item.quantidade,
                    previous_stock,
                    new_stock,
                    operator_name.clone(),
                    if new_stock < 0 {
                        Some("Estoque negativo - adicione mais produtos")
                    } else {
                        None
                    },
                    now
                ],
            )?;
        }

        connection.execute(
            "UPDATE mesa_sessao
             SET tempo_fim = ?1, forma_pagamento = ?2, valor_total_cents = ?3
             WHERE id_mesa = ?4",
            params![now, forma_pagamento, total_cents, input.id_mesa],
        )?;
        connection.execute(
            "DELETE FROM mesa_produtos WHERE id_mesa = ?1",
            params![input.id_mesa],
        )?;

        self.insert_log(
            "mesa_fechada",
            Some(details.mesa.numero),
            sessao.nome_cliente,
            Some(total_cents),
            Some(ticket_data.forma_pagamento.clone()),
            Some(tempo_permanencia),
            produtos_json,
            None,
            Some(ticket_data.id_unico.clone()),
        )?;

        Ok(ticket_data)
    }

    pub fn fechar_venda_caixa(&self, input: FecharVendaCaixaInput) -> AppResult<TicketData> {
        let forma_pagamento = validate_payment_method(&input.forma_pagamento)?;
        let operator_name = normalize_optional_text(input.operator_name.clone())
            .unwrap_or_else(|| "caixa".to_string());

        if self.get_turno_ativo()?.is_none() {
            return Err(AppError::InvalidInput(
                "Abra um turno antes de finalizar uma venda.".to_string(),
            ));
        }

        let normalized_items = normalize_sale_items(&input.items)?;
        let mut produtos = Vec::new();
        let mut product_rows = Vec::new();
        let mut subtotal_cents = 0i64;

        for (product_id, quantidade) in normalized_items {
            let product = self.get_product(product_id)?;
            let subtotal = product.price_cents * quantidade;
            subtotal_cents += subtotal;
            produtos.push(TicketProduto {
                nome: product.name.clone(),
                quantidade,
                preco_unit_cents: product.price_cents,
                subtotal_cents: subtotal,
            });
            product_rows.push((product, quantidade, subtotal));
        }

        // Acréscimo de crédito (5%) só quando o operador opta por ele no crédito;
        // taxa de garçom (10%) é independente da forma de pagamento. Podem somar.
        let acrescimo_credito = if forma_pagamento == "credito" && input.aplicar_acrescimo.unwrap_or(false) {
            (subtotal_cents * 500 + 5_000) / 10_000
        } else {
            0
        };
        let acrescimo_garcom = if input.aplicar_garcom.unwrap_or(false) {
            (subtotal_cents * 1_000 + 5_000) / 10_000
        } else {
            0
        };
        let acrescimo_cents = acrescimo_credito + acrescimo_garcom;
        let total_cents = subtotal_cents + acrescimo_cents;
        let valor_pago_cents = if forma_pagamento == "dinheiro" {
            let paid = input.valor_pago_cents.ok_or_else(|| {
                AppError::InvalidInput("Informe o valor pago em dinheiro.".to_string())
            })?;
            if paid < total_cents {
                return Err(AppError::InvalidInput(
                    "O valor pago deve ser maior ou igual ao total.".to_string(),
                ));
            }
            Some(paid)
        } else {
            input.valor_pago_cents
        };
        let troco_cents = valor_pago_cents.map(|paid| (paid - total_cents).max(0));
        let now = now_millis();
        let ticket_id = generate_short_id(now, TICKET_COUNTER.fetch_add(1, Ordering::Relaxed));
        let ticket_data = TicketData {
            numero_mesa: 0,
            nome_cliente: None,
            tempo_permanencia: "00:00:00".to_string(),
            id_unico: ticket_id,
            forma_pagamento: forma_pagamento.clone(),
            subtotal_cents,
            acrescimo_cents,
            total_cents,
            valor_pago_cents,
            troco_cents,
            produtos: produtos.clone(),
        };
        let produtos_json = serde_json::to_string(&produtos).ok();
        let estimated_profit_cents = product_rows
            .iter()
            .map(|(product, quantidade, _)| {
                (product.price_cents - product.cost_price_cents) * quantidade
            })
            .sum::<i64>();
        let connection = self.connection()?;

        connection.execute(
            "INSERT INTO sales (
                mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                created_at, nfe_status
             ) VALUES (NULL, 'caixa', ?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, 'placeholder')",
            params![
                operator_name.clone(),
                subtotal_cents,
                acrescimo_cents,
                total_cents,
                estimated_profit_cents,
                ticket_data.forma_pagamento,
                now
            ],
        )?;
        let sale_id = connection.last_insert_rowid();

        for (product, quantidade, subtotal) in &product_rows {
            connection.execute(
                "INSERT INTO sale_items (
                    sale_id, product_id, product_name, quantity, unit_price_cents,
                    cost_price_cents, subtotal_cents
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    sale_id,
                    product.id,
                    product.name,
                    quantidade,
                    product.price_cents,
                    product.cost_price_cents,
                    subtotal
                ],
            )?;
            let previous_stock = product.stock;
            let new_stock = previous_stock - quantidade;
            connection.execute(
                "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_stock, now, product.id],
            )?;
            connection.execute(
                "INSERT INTO stock_movements (
                    product_id, movement_type, quantity, previous_stock, new_stock,
                    operator_name, note, created_at
                 ) VALUES (?1, 'venda', ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    product.id,
                    -quantidade,
                    previous_stock,
                    new_stock,
                    operator_name.clone(),
                    if new_stock < 0 {
                        Some("Estoque negativo - adicione mais produtos")
                    } else {
                        None
                    },
                    now
                ],
            )?;
        }

        self.insert_log(
            "venda_caixa",
            None,
            Some(operator_name),
            Some(total_cents),
            Some(ticket_data.forma_pagamento.clone()),
            None,
            produtos_json,
            None,
            Some(ticket_data.id_unico.clone()),
        )?;

        Ok(ticket_data)
    }

    pub fn get_logs(&self, filtros: Option<LogFiltros>) -> AppResult<Vec<LogEntry>> {
        let filtros = filtros.unwrap_or(LogFiltros {
            tipo: None,
            numero_mesa: None,
            data_inicio: None,
            data_fim: None,
        });
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, tipo, numero_mesa, nome_cliente, valor_total_cents, forma_pagamento,
                    tempo_permanencia, lista_produtos_json, data_hora, id_mesa_unico
             FROM logs
             WHERE (?1 IS NULL OR tipo = ?1)
               AND (?2 IS NULL OR numero_mesa = ?2)
               AND (?3 IS NULL OR data_hora >= ?3)
               AND (?4 IS NULL OR data_hora <= ?4)
             ORDER BY data_hora DESC
             LIMIT 500",
        )?;

        let logs = statement
            .query_map(
                params![
                    filtros.tipo,
                    filtros.numero_mesa,
                    filtros.data_inicio,
                    filtros.data_fim
                ],
                map_log_entry,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(logs)
    }

    pub fn log_ticket_gerado(&self, product: &Product, quantity: i64) -> AppResult<()> {
        let produtos_json = format!(
            "[{{\"nome\":\"{}\",\"quantidade\":{},\"precoUnitCents\":{},\"subtotalCents\":{}}}]",
            product.name.replace('"', "'"),
            quantity,
            product.price_cents,
            product.price_cents * quantity
        );
        self.insert_log(
            "ticket_gerado",
            None,
            None,
            Some(product.price_cents * quantity),
            None,
            None,
            Some(produtos_json),
            None,
            None,
        )
    }

    pub fn login(&self, input: LoginInput) -> AppResult<AuthPayload> {
        let username = input.username.trim().to_string();
        let password_hash = hash_password(&input.password);
        let connection = self.connection()?;
        let user = connection
            .query_row(
                "SELECT id, username, role, permissions_json, active, created_at
                 FROM users
                 WHERE username = ?1 AND password_hash = ?2 AND active = 1",
                params![username, password_hash],
                map_local_user,
            )
            .optional()?;

        user.map(|user| AuthPayload { user })
            .ok_or_else(|| AppError::InvalidInput("Usuario ou senha invalidos.".to_string()))
    }

    pub fn list_users(&self) -> AppResult<Vec<LocalUser>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, username, role, permissions_json, active, created_at FROM users ORDER BY username",
        )?;
        let users = statement
            .query_map([], map_local_user)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    pub fn has_configured_users(&self) -> AppResult<bool> {
        let connection = self.connection()?;
        let default_admin_hash = hash_password("admin");
        let count = connection.query_row(
            "SELECT COUNT(*)
             FROM users
             WHERE active = 1
               AND NOT (username = 'admin' AND password_hash = ?1 AND role = 'admin')",
            params![default_admin_hash],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(count > 0)
    }

    fn active_admin_count(&self) -> AppResult<i64> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COUNT(*) FROM users WHERE active = 1 AND role = 'admin'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(AppError::from)
    }

    pub fn create_user(&self, input: CreateUserInput) -> AppResult<LocalUser> {
        let username = input.username.trim().to_string();
        let role = input.role.trim().to_lowercase();
        let permissions_json = normalize_permissions_json(input.permissions, &role)?;
        let has_configured_users = self.has_configured_users()?;
        if has_configured_users {
            ensure_permission(
                input.requester_role.as_deref(),
                input.requester_permissions.as_deref(),
                MANAGE_USERS_PERMISSION,
            )?;
        } else if role != "admin" {
            return Err(AppError::InvalidInput(
                "O primeiro usuario precisa ser administrador.".to_string(),
            ));
        }

        let username = validate_username(&username)?;
        validate_password(&input.password)?;
        if role != "admin" && role != "operator" {
            return Err(AppError::InvalidInput(
                "Perfil de usuario invalido.".to_string(),
            ));
        }
        let now = now_millis();
        let connection = self.connection()?;

        if username == "admin" {
            let default_admin_id = connection
                .query_row(
                    "SELECT id FROM users
                     WHERE username = 'admin' AND password_hash = ?1 AND role = 'admin'",
                    params![hash_password("admin")],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;

            if let Some(user_id) = default_admin_id {
                connection.execute(
                    "UPDATE users
                     SET password_hash = ?1, role = ?2, permissions_json = ?3, active = 1
                     WHERE id = ?4",
                    params![
                        hash_password(&input.password),
                        role,
                        permissions_json,
                        user_id
                    ],
                )?;
                return connection
                    .query_row(
                        "SELECT id, username, role, permissions_json, active, created_at FROM users WHERE id = ?1",
                        params![user_id],
                        map_local_user,
                    )
                    .map_err(AppError::from);
            }
        }

        connection.execute(
            "INSERT INTO users (username, password_hash, role, permissions_json, active, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![username, hash_password(&input.password), role, permissions_json, now],
        )?;
        let user = connection
            .query_row(
                "SELECT id, username, role, permissions_json, active, created_at FROM users WHERE id = ?1",
                params![connection.last_insert_rowid()],
                map_local_user,
            )
            .map_err(AppError::from)?;

        if !has_configured_users {
            connection.execute(
                "DELETE FROM users WHERE username = 'admin' AND password_hash = ?1 AND id != ?2",
                params![hash_password("admin"), user.id],
            )?;
        }

        Ok(user)
    }

    pub fn update_user(&self, input: UpdateUserInput) -> AppResult<LocalUser> {
        ensure_permission(
            input.requester_role.as_deref(),
            input.requester_permissions.as_deref(),
            MANAGE_USERS_PERMISSION,
        )?;
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Usuario invalido.".to_string()));
        }

        let username = input.username.trim().to_string();
        let role = input.role.trim().to_lowercase();
        let permissions_json = normalize_permissions_json(input.permissions, &role)?;
        let password = input
            .password
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if username.is_empty() {
            return Err(AppError::InvalidInput("Informe o usuario.".to_string()));
        }
        let username = validate_username(&username)?;

        if role != "admin" && role != "operator" {
            return Err(AppError::InvalidInput(
                "Perfil de usuario invalido.".to_string(),
            ));
        }

        if let Some(password) = password.as_ref() {
            validate_password(password)?;
        }

        let connection = self.connection()?;
        let current_role = connection
            .query_row(
                "SELECT role FROM users WHERE id = ?1",
                params![input.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidInput("Usuario nao encontrado.".to_string()))?;

        if current_role == "admin" && role != "admin" && self.active_admin_count()? <= 1 {
            return Err(AppError::InvalidInput(
                "Mantenha pelo menos um administrador ativo.".to_string(),
            ));
        }

        if let Some(password) = password {
            connection.execute(
                "UPDATE users SET username = ?1, password_hash = ?2, role = ?3, permissions_json = ?4 WHERE id = ?5",
                params![username, hash_password(&password), role, permissions_json, input.id],
            )?;
        } else {
            connection.execute(
                "UPDATE users SET username = ?1, role = ?2, permissions_json = ?3 WHERE id = ?4",
                params![username, role, permissions_json, input.id],
            )?;
        }

        connection
            .query_row(
                "SELECT id, username, role, permissions_json, active, created_at FROM users WHERE id = ?1",
                params![input.id],
                map_local_user,
            )
            .map_err(AppError::from)
    }

    pub fn delete_user(&self, input: DeleteUserInput) -> AppResult<()> {
        ensure_permission(
            input.requester_role.as_deref(),
            input.requester_permissions.as_deref(),
            MANAGE_USERS_PERMISSION,
        )?;
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Usuario invalido.".to_string()));
        }

        let connection = self.connection()?;
        let current_role = connection
            .query_row(
                "SELECT role FROM users WHERE id = ?1",
                params![input.id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidInput("Usuario nao encontrado.".to_string()))?;

        if current_role == "admin" && self.active_admin_count()? <= 1 {
            return Err(AppError::InvalidInput(
                "Mantenha pelo menos um administrador ativo.".to_string(),
            ));
        }

        connection.execute("DELETE FROM users WHERE id = ?1", params![input.id])?;
        Ok(())
    }

    pub fn get_current_cash_register(&self) -> AppResult<Option<CashRegister>> {
        let connection = self.connection()?;
        let register = connection
            .query_row(
                "SELECT id, opened_at, closed_at, initial_balance_cents, final_counted_cents, operator_name
                 FROM cash_registers
                 WHERE closed_at IS NULL
                 ORDER BY opened_at DESC
                 LIMIT 1",
                [],
                map_cash_register,
            )
            .optional()?;
        register
            .map(|register| self.enrich_cash_register(register))
            .transpose()
    }

    pub fn open_cash_register(&self, input: OpenCashRegisterInput) -> AppResult<CashRegister> {
        if self.get_current_cash_register()?.is_some() {
            return Err(AppError::InvalidInput(
                "Ja existe um caixa aberto.".to_string(),
            ));
        }
        if input.initial_balance_cents < 0 {
            return Err(AppError::InvalidInput(
                "Saldo inicial invalido.".to_string(),
            ));
        }
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO cash_registers (opened_at, closed_at, initial_balance_cents, final_counted_cents, operator_name)
             VALUES (?1, NULL, ?2, NULL, ?3)",
            params![now, input.initial_balance_cents, input.operator_name],
        )?;
        self.get_current_cash_register()?
            .ok_or_else(|| AppError::System("Nao foi possivel abrir o caixa.".to_string()))
    }

    pub fn close_cash_register(&self, input: CloseCashRegisterInput) -> AppResult<CashRegister> {
        let _operator_name = input.operator_name.trim();
        let current = self
            .get_current_cash_register()?
            .ok_or_else(|| AppError::InvalidInput("Nao existe caixa aberto.".to_string()))?;
        let now = now_millis();
        let connection = self.connection()?;
        let today = sales_report_period_bounds("day")?;
        let closed_today = connection.query_row(
            "SELECT COUNT(*)
             FROM cash_registers
             WHERE closed_at >= ?1 AND closed_at < ?2",
            params![today.current_start, today.current_end],
            |row| row.get::<_, i64>(0),
        )?;

        if closed_today > 0 {
            return Err(AppError::InvalidInput(
                "Você não pode fechar o caixa duas vezes no mesmo dia.".to_string(),
            ));
        }

        connection.execute(
            "UPDATE cash_registers
             SET closed_at = ?1, final_counted_cents = ?2
             WHERE id = ?3",
            params![now, input.final_counted_cents, current.id],
        )?;
        let register = connection.query_row(
            "SELECT id, opened_at, closed_at, initial_balance_cents, final_counted_cents, operator_name
             FROM cash_registers WHERE id = ?1",
            params![current.id],
            map_cash_register,
        ).map_err(AppError::from)?;
        self.enrich_cash_register(register)
    }

    pub fn add_cash_movement(&self, input: CashMovementInput) -> AppResult<CashMovement> {
        let turno = self.get_turno_ativo()?.ok_or_else(|| {
            AppError::InvalidInput("Abra um turno antes de registrar movimentos.".to_string())
        })?;
        let movement_type = input.movement_type.trim().to_lowercase();
        if movement_type != "sangria" && movement_type != "suprimento" {
            return Err(AppError::InvalidInput(
                "Movimento de caixa invalido.".to_string(),
            ));
        }
        if input.amount_cents <= 0 {
            return Err(AppError::InvalidInput(
                "Informe um valor maior que zero.".to_string(),
            ));
        }
        let now = now_millis();
        let connection = self.connection()?;
        // Fluxo unificado: movimentos pertencem ao turno. cash_register_id = 0
        // (sentinela "sem caixa") por ser NOT NULL no schema legado.
        connection.execute(
            "INSERT INTO cash_movements (
                cash_register_id, turno_id, movement_type, amount_cents, note, operator_name, created_at
             ) VALUES (0, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                turno.id,
                movement_type,
                input.amount_cents,
                input.note,
                input.operator_name,
                now
            ],
        )?;
        Ok(CashMovement {
            id: connection.last_insert_rowid(),
            cash_register_id: 0,
            turno_id: Some(turno.id),
            movement_type,
            amount_cents: input.amount_cents,
            note: input.note,
            operator_name: input.operator_name,
            created_at: now,
        })
    }

    pub fn list_cash_movements(&self) -> AppResult<Vec<CashMovement>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, cash_register_id, movement_type, amount_cents, note, operator_name, created_at, turno_id
             FROM cash_movements
             ORDER BY created_at DESC
             LIMIT 300",
        )?;
        let movements = statement
            .query_map([], map_cash_movement)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(movements)
    }

    // =======================================================================
    // FASE 5: Turno operacional + periodo contabil (fechamento em cascata)
    // =======================================================================

    pub fn get_fiscal_day_start_minutes(&self) -> AppResult<i64> {
        let connection = self.connection()?;
        let minutes = connection
            .query_row(
                "SELECT fiscal_day_start_minutes FROM app_config WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);
        Ok(minutes.clamp(0, 1439))
    }

    pub fn set_fiscal_day_start_minutes(&self, minutes: i64) -> AppResult<i64> {
        if !(0..=1439).contains(&minutes) {
            return Err(AppError::InvalidInput(
                "Inicio do dia fiscal deve estar entre 0 e 1439 minutos.".to_string(),
            ));
        }
        let connection = self.connection()?;
        connection.execute(
            "UPDATE app_config SET fiscal_day_start_minutes = ?1, updated_at = ?2 WHERE id = 1",
            params![minutes, now_millis()],
        )?;
        Ok(minutes)
    }

    /// Data contabil (YYYY-MM-DD) a que um instante pertence, considerando o
    /// deslocamento do dia fiscal (ex.: 06h -> vendas de 22h-6h caem no dia anterior).
    fn data_contabil_de(&self, ts_millis: i64) -> AppResult<String> {
        let offset_minutes = self.get_fiscal_day_start_minutes()?;
        business_date(ts_millis, offset_minutes)
    }

    pub fn abrir_turno(&self, input: AbrirTurnoInput) -> AppResult<TurnoOperacional> {
        let operador = input.operador.trim().to_string();
        if operador.is_empty() {
            return Err(AppError::InvalidInput("Informe o operador.".to_string()));
        }
        if input.saldo_inicial_cents < 0 {
            return Err(AppError::InvalidInput(
                "Fundo de troco invalido.".to_string(),
            ));
        }
        if self.get_turno_ativo()?.is_some() {
            return Err(AppError::InvalidInput(
                "Ja existe um turno aberto. Feche-o antes de abrir outro.".to_string(),
            ));
        }
        let now = now_millis();
        let data = self.data_contabil_de(now)?;
        let periodo = self.get_or_create_periodo(&data)?;
        if periodo.status == PeriodoStatus::Bloqueado {
            return Err(AppError::InvalidInput(
                "O periodo contabil deste dia esta bloqueado.".to_string(),
            ));
        }
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO turnos_operacionais
                (loja_id, caixa_id, operador, data_inicio, status, saldo_inicial_cents,
                 valor_esperado_cents, created_at, updated_at)
             VALUES (1, ?1, ?2, ?3, 'aberto', ?4, 0, ?3, ?3)",
            params![input.caixa_id, operador, now, input.saldo_inicial_cents],
        )?;
        let id = connection.last_insert_rowid();
        self.get_turno_by_id(id)?
            .ok_or_else(|| AppError::System("Nao foi possivel abrir o turno.".to_string()))
    }

    /// Dinheiro esperado na gaveta de um turno (saldo inicial + vendas em
    /// dinheiro + suprimentos - sangrias), no intervalo [inicio, fim].
    fn drawer_esperado_do_turno(
        &self,
        turno_id: i64,
        saldo_inicial_cents: i64,
        inicio: i64,
        fim: i64,
    ) -> AppResult<i64> {
        let connection = self.connection()?;
        let cash_direct: i64 = connection.query_row(
            "SELECT COALESCE(SUM(total_cents), 0)
             FROM sales
             WHERE payment_method = 'dinheiro' AND sale_type != 'mesa'
               AND created_at >= ?1 AND created_at <= ?2",
            params![inicio, fim],
            |row| row.get(0),
        )?;
        // Dinheiro de mesa: atribuido pelo TURNO em que o pagamento foi recebido
        // (sale_payments.turno_operacional_id), nao pelo created_at da venda. Um
        // pagamento parcial em dinheiro entra na gaveta do turno que o recebeu,
        // mesmo que a conta so feche em outro turno.
        let cash_mesa: i64 = connection.query_row(
            "SELECT COALESCE(SUM(valor_cents), 0)
             FROM sale_payments
             WHERE forma_pagamento = 'dinheiro' AND turno_operacional_id = ?1",
            params![turno_id],
            |row| row.get(0),
        )?;
        let suprimentos: i64 = connection.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0)
             FROM cash_movements
             WHERE turno_id = ?1 AND movement_type = 'suprimento'",
            params![turno_id],
            |row| row.get(0),
        )?;
        let sangrias: i64 = connection.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0)
             FROM cash_movements
             WHERE turno_id = ?1 AND movement_type = 'sangria'",
            params![turno_id],
            |row| row.get(0),
        )?;
        Ok(saldo_inicial_cents + cash_direct + cash_mesa + suprimentos - sangrias)
    }

    pub fn fechar_turno(&self, input: FecharTurnoInput) -> AppResult<TurnoOperacional> {
        if input.valor_fisico_cents < 0 {
            return Err(AppError::InvalidInput(
                "Valor fisico contado invalido.".to_string(),
            ));
        }
        let turno = self
            .get_turno_by_id(input.turno_id)?
            .ok_or_else(|| AppError::InvalidInput("Turno nao encontrado.".to_string()))?;
        if turno.status != TurnoStatus::Aberto {
            return Err(AppError::InvalidInput(
                "Este turno ja foi fechado.".to_string(),
            ));
        }

        let now = now_millis();
        let observacoes = normalize_optional_text(input.observacoes);
        let connection = self.connection()?;
        let tx = connection.unchecked_transaction()?;

        // Vincula as vendas do intervalo do turno (ainda sem turno) a este turno.
        tx.execute(
            "UPDATE sales SET turno_operacional_id = ?1
             WHERE turno_operacional_id IS NULL
               AND created_at >= ?2 AND created_at <= ?3",
            params![turno.id, turno.data_inicio, now],
        )?;

        // Valor esperado = dinheiro da gaveta (fundo + vendas em dinheiro +
        // suprimentos - sangrias), comparado ao fisico contado.
        let valor_esperado = self.drawer_esperado_do_turno(
            turno.id,
            turno.saldo_inicial_cents,
            turno.data_inicio,
            now,
        )?;
        let diferenca = input.valor_fisico_cents - valor_esperado;

        tx.execute(
            "UPDATE turnos_operacionais
             SET status = 'fechado', data_fim = ?1, valor_esperado_cents = ?2,
                 valor_fisico_cents = ?3, diferenca_cents = ?4, observacoes = ?5, updated_at = ?1
             WHERE id = ?6",
            params![
                now,
                valor_esperado,
                input.valor_fisico_cents,
                diferenca,
                observacoes,
                turno.id
            ],
        )?;
        tx.commit()?;

        self.get_turno_by_id(turno.id)?
            .ok_or_else(|| AppError::System("Falha ao fechar o turno.".to_string()))
    }

    pub fn get_turno_by_id(&self, id: i64) -> AppResult<Option<TurnoOperacional>> {
        let connection = self.connection()?;
        connection
            .query_row(
                &format!("SELECT {TURNO_COLUMNS} FROM turnos_operacionais WHERE id = ?1"),
                params![id],
                map_turno,
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn get_turno_ativo(&self) -> AppResult<Option<TurnoOperacional>> {
        let connection = self.connection()?;
        connection
            .query_row(
                &format!(
                    "SELECT {TURNO_COLUMNS} FROM turnos_operacionais
                     WHERE status = 'aberto'
                     ORDER BY data_inicio DESC LIMIT 1"
                ),
                [],
                map_turno,
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn listar_turnos_do_dia(&self, data: &str) -> AppResult<Vec<TurnoOperacional>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(&format!(
            "SELECT {TURNO_COLUMNS} FROM turnos_operacionais t
             WHERE t.periodo_contabil_id IN (SELECT id FROM periodos_contabeis WHERE data = ?1)
                OR (t.periodo_contabil_id IS NULL AND t.data_inicio >= ?2 AND t.data_inicio < ?3)
             ORDER BY t.data_inicio ASC"
        ))?;
        let (inicio, fim) = self.bounds_do_dia(data)?;
        let turnos = statement
            .query_map(params![data, inicio, fim], map_turno)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(turnos)
    }

    pub fn get_or_create_periodo(&self, data: &str) -> AppResult<PeriodoContabil> {
        let connection = self.connection()?;
        let now = now_millis();
        connection.execute(
            "INSERT OR IGNORE INTO periodos_contabeis
                (loja_id, data, status, created_at, updated_at)
             VALUES (1, ?1, 'aberto', ?2, ?2)",
            params![data, now],
        )?;
        connection
            .query_row(
                &format!(
                    "SELECT {PERIODO_COLUMNS} FROM periodos_contabeis WHERE loja_id = 1 AND data = ?1"
                ),
                params![data],
                map_periodo,
            )
            .map_err(AppError::from)
    }

    pub fn consolidar_periodo(&self, input: ConsolidarPeriodoInput) -> AppResult<PeriodoContabil> {
        if input.usuario.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "Informe o usuario responsavel pela consolidacao.".to_string(),
            ));
        }
        let data = match normalize_optional_text(input.data) {
            Some(value) => value,
            None => self.data_contabil_de(now_millis())?,
        };
        let periodo = self.get_or_create_periodo(&data)?;
        if periodo.status == PeriodoStatus::Bloqueado {
            return Err(AppError::InvalidInput(
                "Periodo bloqueado nao pode ser reconsolidado.".to_string(),
            ));
        }
        let turnos = self.listar_turnos_do_dia(&data)?;
        if turnos.is_empty() {
            return Err(AppError::InvalidInput(
                "Nao ha turnos neste dia para consolidar.".to_string(),
            ));
        }
        if turnos.iter().any(|t| t.status == TurnoStatus::Aberto) {
            return Err(AppError::InvalidInput(
                "Feche todos os turnos do dia antes de consolidar o periodo.".to_string(),
            ));
        }

        let total_esperado: i64 = turnos.iter().map(|t| t.valor_esperado_cents).sum();
        let total_real: i64 = turnos.iter().map(|t| t.valor_fisico_cents.unwrap_or(0)).sum();
        let now = now_millis();
        let (inicio, fim) = self.bounds_do_dia(&data)?;

        let connection = self.connection()?;
        let tx = connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE periodos_contabeis
             SET status = 'fechado', total_esperado_cents = ?1, total_real_cents = ?2,
                 updated_at = ?3
             WHERE id = ?4",
            params![total_esperado, total_real, now, periodo.id],
        )?;
        // Marca turnos como reconciliados e os vincula ao periodo.
        tx.execute(
            "UPDATE turnos_operacionais
             SET periodo_contabil_id = ?1, status = 'reconciliado', updated_at = ?2
             WHERE status IN ('fechado', 'reconciliado')
               AND (periodo_contabil_id = ?1
                    OR (periodo_contabil_id IS NULL AND data_inicio >= ?3 AND data_inicio < ?4))",
            params![periodo.id, now, inicio, fim],
        )?;
        // Carimba as vendas dos turnos deste periodo.
        tx.execute(
            "UPDATE sales
             SET periodo_contabil_id = ?1
             WHERE turno_operacional_id IN
                (SELECT id FROM turnos_operacionais WHERE periodo_contabil_id = ?1)",
            params![periodo.id],
        )?;
        tx.commit()?;

        self.get_or_create_periodo(&data)
    }

    pub fn bloquear_periodo(&self, input: BloquearPeriodoInput) -> AppResult<PeriodoContabil> {
        let usuario = input.usuario.trim().to_string();
        if usuario.is_empty() {
            return Err(AppError::InvalidInput(
                "Informe o usuario responsavel.".to_string(),
            ));
        }
        let connection = self.connection()?;
        let status: Option<String> = connection
            .query_row(
                "SELECT status FROM periodos_contabeis WHERE id = ?1",
                params![input.periodo_id],
                |row| row.get(0),
            )
            .optional()?;
        let status = status
            .ok_or_else(|| AppError::InvalidInput("Periodo nao encontrado.".to_string()))?;
        match PeriodoStatus::parse(&status) {
            PeriodoStatus::Aberto => {
                return Err(AppError::InvalidInput(
                    "Consolide o periodo antes de bloquear.".to_string(),
                ))
            }
            PeriodoStatus::Bloqueado => {
                return Err(AppError::InvalidInput(
                    "Periodo ja esta bloqueado.".to_string(),
                ))
            }
            PeriodoStatus::Fechado => {}
        }
        let now = now_millis();
        connection.execute(
            "UPDATE periodos_contabeis
             SET status = 'bloqueado', bloqueado_em = ?1, bloqueado_por = ?2, updated_at = ?1
             WHERE id = ?3",
            params![now, usuario, input.periodo_id],
        )?;
        connection
            .query_row(
                &format!("SELECT {PERIODO_COLUMNS} FROM periodos_contabeis WHERE id = ?1"),
                params![input.periodo_id],
                map_periodo,
            )
            .map_err(AppError::from)
    }

    pub fn pode_editar_venda(&self, sale_id: i64) -> AppResult<bool> {
        let connection = self.connection()?;
        let periodo_id: Option<Option<i64>> = connection
            .query_row(
                "SELECT periodo_contabil_id FROM sales WHERE id = ?1",
                params![sale_id],
                |row| row.get(0),
            )
            .optional()?;
        let periodo_id = match periodo_id {
            Some(value) => value,
            None => return Err(AppError::InvalidInput("Venda nao encontrada.".to_string())),
        };
        let Some(periodo_id) = periodo_id else {
            return Ok(true);
        };
        let status: String = connection.query_row(
            "SELECT status FROM periodos_contabeis WHERE id = ?1",
            params![periodo_id],
            |row| row.get(0),
        )?;
        Ok(PeriodoStatus::parse(&status) != PeriodoStatus::Bloqueado)
    }

    pub fn editar_venda(&self, input: EditarVendaInput) -> AppResult<SaleAuditEntry> {
        let usuario = input.usuario.trim().to_string();
        let motivo = input.motivo.trim().to_string();
        if usuario.is_empty() || motivo.is_empty() {
            return Err(AppError::InvalidInput(
                "Informe usuario e motivo do ajuste.".to_string(),
            ));
        }
        if input.novo_total_cents < 0 {
            return Err(AppError::InvalidInput("Valor invalido.".to_string()));
        }
        if !self.pode_editar_venda(input.sale_id)? {
            return Err(AppError::InvalidInput(
                "Venda pertence a um periodo bloqueado e nao pode ser editada.".to_string(),
            ));
        }

        let connection = self.connection()?;
        let tx = connection.unchecked_transaction()?;
        let (valor_anterior, turno_id, periodo_id): (i64, Option<i64>, Option<i64>) = tx
            .query_row(
                "SELECT total_cents, turno_operacional_id, periodo_contabil_id
                 FROM sales WHERE id = ?1",
                params![input.sale_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| AppError::InvalidInput("Venda nao encontrada.".to_string()))?;
        let now = now_millis();
        tx.execute(
            "UPDATE sales SET total_cents = ?1 WHERE id = ?2",
            params![input.novo_total_cents, input.sale_id],
        )?;
        tx.execute(
            "INSERT INTO sale_audit
                (sale_id, turno_operacional_id, periodo_contabil_id, valor_anterior_cents,
                 valor_novo_cents, motivo, usuario, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.sale_id,
                turno_id,
                periodo_id,
                valor_anterior,
                input.novo_total_cents,
                motivo,
                usuario,
                now
            ],
        )?;
        let audit_id = tx.last_insert_rowid();
        tx.commit()?;

        Ok(SaleAuditEntry {
            id: audit_id,
            sale_id: input.sale_id,
            turno_operacional_id: turno_id,
            periodo_contabil_id: periodo_id,
            valor_anterior_cents: valor_anterior,
            valor_novo_cents: input.novo_total_cents,
            motivo,
            usuario,
            created_at: now,
        })
    }

    pub fn listar_auditoria_venda(&self, sale_id: i64) -> AppResult<Vec<SaleAuditEntry>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, sale_id, turno_operacional_id, periodo_contabil_id, valor_anterior_cents,
                    valor_novo_cents, motivo, usuario, created_at
             FROM sale_audit WHERE sale_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = statement
            .query_map(params![sale_id], map_sale_audit)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_cashier_status(&self) -> AppResult<CashierStatus> {
        let fiscal_day_start_minutes = self.get_fiscal_day_start_minutes()?;
        let data_contabil = self.data_contabil_de(now_millis())?;
        let periodo_hoje = self.get_or_create_periodo(&data_contabil)?;
        let turno_ativo = self.get_turno_ativo()?;
        let turnos_do_dia = self.listar_turnos_do_dia(&data_contabil)?;
        let esperado_atual_cents = match &turno_ativo {
            Some(turno) => Some(self.drawer_esperado_do_turno(
                turno.id,
                turno.saldo_inicial_cents,
                turno.data_inicio,
                now_millis(),
            )?),
            None => None,
        };
        Ok(CashierStatus {
            data_contabil,
            fiscal_day_start_minutes,
            turno_ativo,
            periodo_hoje,
            turnos_do_dia,
            esperado_atual_cents,
        })
    }

    /// Limites [inicio, fim) em millis de uma data contabil YYYY-MM-DD,
    /// deslocados pelo inicio do dia fiscal.
    fn bounds_do_dia(&self, data: &str) -> AppResult<(i64, i64)> {
        let offset_minutes = self.get_fiscal_day_start_minutes()?;
        let date = NaiveDate::parse_from_str(data, "%Y-%m-%d")
            .map_err(|_| AppError::InvalidInput("Data contabil invalida.".to_string()))?;
        let next = date
            .checked_add_days(Days::new(1))
            .ok_or_else(|| AppError::InvalidInput("Data invalida.".to_string()))?;
        let offset_millis = offset_minutes * 60_000;
        Ok((
            local_start_of_day_millis(date)? + offset_millis,
            local_start_of_day_millis(next)? + offset_millis,
        ))
    }

    pub fn get_reports(&self) -> AppResult<ReportsPayload> {
        let connection = self.connection()?;
        let total_revenue_cents = connection.query_row(
            "SELECT COALESCE(SUM(total_cents), 0) FROM sales",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let estimated_profit_cents = connection.query_row(
            "SELECT COALESCE(SUM(estimated_profit_cents), 0) FROM sales",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let mut sales_statement = connection.prepare(
            "SELECT date(created_at / 1000, 'unixepoch', 'localtime') AS date_label,
                    COALESCE(SUM(total_cents), 0)
             FROM sales
             GROUP BY date_label
             ORDER BY date_label DESC
             LIMIT 30",
        )?;
        let sales_by_day = sales_statement
            .query_map([], |row| {
                Ok(SalesByDay {
                    date_label: row.get(0)?,
                    total_cents: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut top_statement = connection.prepare(
            "SELECT product_name, COALESCE(SUM(quantity), 0), COALESCE(SUM(subtotal_cents), 0)
             FROM sale_items
             GROUP BY product_id, product_name
             ORDER BY SUM(quantity) DESC
             LIMIT 20",
        )?;
        let top_products = top_statement
            .query_map([], |row| {
                Ok(TopProductReport {
                    product_name: row.get(0)?,
                    quantity: row.get(1)?,
                    total_cents: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let low_stock_products = self
            .list_products()?
            .into_iter()
            .filter(|product| product.stock <= product.reorder_level)
            .collect::<Vec<_>>();

        Ok(ReportsPayload {
            total_revenue_cents,
            estimated_profit_cents,
            sales_by_day,
            top_products,
            low_stock_products,
        })
    }

    pub fn reset_sales(&self, username: &str, password: &str) -> AppResult<()> {
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            return Err(AppError::InvalidInput(
                "Informe usuario e senha do administrador.".to_string(),
            ));
        }

        let connection = self.connection()?;
        let is_admin = connection
            .query_row(
                "SELECT 1
                 FROM users
                 WHERE username = ?1 AND password_hash = ?2 AND role = 'admin' AND active = 1",
                params![username, hash_password(password)],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        if !is_admin {
            return Err(AppError::InvalidInput(
                "Senha de administrador invalida.".to_string(),
            ));
        }

        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM sale_payments", [])?;
        transaction.execute("DELETE FROM sale_items", [])?;
        transaction.execute("DELETE FROM sales", [])?;
        transaction.commit()?;

        let _ = self.insert_log(
            "vendas_resetadas",
            None,
            Some(username.to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );

        Ok(())
    }

    pub fn get_sales_report(&self, period: &str) -> AppResult<SalesReportData> {
        let bounds = sales_report_period_bounds(period)?;
        let connection = self.connection()?;
        let (
            direct_sales_cents,
            table_sales_cents,
            ticket_sales_cents,
            total_sales_cents,
            estimated_profit_cents,
        ) = connection.query_row(
            "SELECT
                    COALESCE(SUM(CASE WHEN sale_type = 'caixa' THEN total_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN sale_type = 'mesa' THEN total_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN sale_type = 'ticket' THEN total_cents ELSE 0 END), 0),
                    COALESCE(SUM(total_cents), 0),
                    COALESCE(SUM(estimated_profit_cents), 0)
                 FROM sales
                 WHERE created_at >= ?1 AND created_at < ?2",
            params![bounds.current_start, bounds.current_end],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )?;
        let previous_total_sales_cents = connection.query_row(
            "SELECT COALESCE(SUM(total_cents), 0)
             FROM sales
             WHERE created_at >= ?1 AND created_at < ?2",
            params![bounds.previous_start, bounds.previous_end],
            |row| row.get::<_, i64>(0),
        )?;
        let (comparison_percent, comparison_text) = comparison_summary(
            total_sales_cents,
            previous_total_sales_cents,
            &bounds.previous_reference_label,
        );

        Ok(SalesReportData {
            period: bounds.period,
            period_label: bounds.period_label,
            printed_at: now_millis(),
            direct_sales_cents,
            table_sales_cents,
            ticket_sales_cents,
            total_sales_cents,
            estimated_profit_cents,
            previous_total_sales_cents,
            comparison_percent,
            comparison_text,
        })
    }

    fn connection(&self) -> AppResult<Connection> {
        let conn = Connection::open(&self.path)?;
        // foreign_keys e busy_timeout sao POR CONEXAO (precisam ser setados a cada open);
        // journal_mode=WAL e persistente, mas reafirmar e barato e idempotente.
        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
        Ok(conn)
    }

    fn migrate(&self) -> AppResult<()> {
        let connection = self.connection()?;
        let now = now_millis();

        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS app_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                company_name TEXT NOT NULL DEFAULT '',
                tax_id TEXT NOT NULL DEFAULT '',
                thank_you_message TEXT,
                validity_days INTEGER NOT NULL DEFAULT 30,
                theme TEXT NOT NULL DEFAULT 'light',
                printer_name TEXT,
                print_width_chars INTEGER NOT NULL DEFAULT 48,
                onboarding_completed INTEGER NOT NULL DEFAULT 0,
                setup_completed INTEGER NOT NULL DEFAULT 0,
                table_count INTEGER NOT NULL DEFAULT 40,
                backup_time TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                price_cents INTEGER NOT NULL,
                barcode TEXT UNIQUE,
                cost_price_cents INTEGER NOT NULL DEFAULT 0,
                unit TEXT NOT NULL DEFAULT 'UN',
                category_id INTEGER,
                stock INTEGER NOT NULL DEFAULT 0,
                reorder_level INTEGER NOT NULL DEFAULT 0,
                description TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS issued_tickets (
                ticket_id TEXT PRIMARY KEY,
                product_id INTEGER NOT NULL,
                product_name TEXT NOT NULL,
                price_cents INTEGER NOT NULL,
                issued_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mesas (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                numero INTEGER UNIQUE NOT NULL,
                capacidade INTEGER,
                criada_em INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mesa_produtos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                id_mesa INTEGER NOT NULL,
                id_produto INTEGER NOT NULL,
                quantidade INTEGER NOT NULL,
                adicionado_em INTEGER NOT NULL,
                UNIQUE(id_mesa, id_produto)
            );

            CREATE TABLE IF NOT EXISTS mesa_sessao (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                id_mesa INTEGER UNIQUE NOT NULL,
                tempo_inicio INTEGER NOT NULL,
                tempo_fim INTEGER,
                nome_cliente TEXT,
                forma_pagamento TEXT,
                valor_total_cents INTEGER,
                id_unico TEXT UNIQUE NOT NULL
            );

            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tipo TEXT NOT NULL,
                numero_mesa INTEGER,
                nome_cliente TEXT,
                valor_total_cents INTEGER,
                forma_pagamento TEXT,
                tempo_permanencia TEXT,
                lista_produtos_json TEXT,
                data_hora INTEGER NOT NULL,
                id_mesa_unico TEXT
            );

            CREATE TABLE IF NOT EXISTS cash_registers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                opened_at INTEGER NOT NULL,
                closed_at INTEGER,
                initial_balance_cents INTEGER NOT NULL,
                final_counted_cents INTEGER,
                operator_name TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS cash_movements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                cash_register_id INTEGER NOT NULL,
                movement_type TEXT NOT NULL,
                amount_cents INTEGER NOT NULL,
                note TEXT,
                operator_name TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stock_movements (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                product_id INTEGER NOT NULL,
                movement_type TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                previous_stock INTEGER NOT NULL,
                new_stock INTEGER NOT NULL,
                operator_name TEXT NOT NULL,
                note TEXT,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL,
                permissions_json TEXT NOT NULL DEFAULT '[]',
                active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sales (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mesa_numero INTEGER,
                sale_type TEXT NOT NULL,
                operator_name TEXT NOT NULL,
                subtotal_cents INTEGER NOT NULL,
                discount_cents INTEGER NOT NULL DEFAULT 0,
                surcharge_cents INTEGER NOT NULL DEFAULT 0,
                total_cents INTEGER NOT NULL,
                estimated_profit_cents INTEGER NOT NULL DEFAULT 0,
                payment_method TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                nfe_status TEXT NOT NULL DEFAULT 'placeholder'
            );

            CREATE TABLE IF NOT EXISTS sale_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sale_id INTEGER NOT NULL,
                product_id INTEGER NOT NULL,
                product_name TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                unit_price_cents INTEGER NOT NULL,
                cost_price_cents INTEGER NOT NULL,
                subtotal_cents INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_products_name ON products(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_products_barcode ON products(barcode);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_products_barcode_unique ON products(barcode) WHERE barcode IS NOT NULL AND barcode <> '';
            CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_issued_tickets_expires_at ON issued_tickets(expires_at);
            CREATE INDEX IF NOT EXISTS idx_mesa_produtos_mesa ON mesa_produtos(id_mesa);
            CREATE INDEX IF NOT EXISTS idx_logs_data_hora ON logs(data_hora);
            CREATE INDEX IF NOT EXISTS idx_logs_tipo ON logs(tipo);
            CREATE INDEX IF NOT EXISTS idx_stock_movements_product ON stock_movements(product_id);
            CREATE INDEX IF NOT EXISTS idx_sales_created_at ON sales(created_at);
            CREATE INDEX IF NOT EXISTS idx_sale_items_product ON sale_items(product_id);",
        )?;

        add_column_if_missing(
            &connection,
            "app_config",
            "table_count",
            "INTEGER NOT NULL DEFAULT 40",
        )?;
        add_column_if_missing(&connection, "app_config", "backup_time", "TEXT")?;
        add_column_if_missing(&connection, "products", "barcode", "TEXT")?;
        add_column_if_missing(
            &connection,
            "products",
            "cost_price_cents",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        add_column_if_missing(
            &connection,
            "products",
            "unit",
            "TEXT NOT NULL DEFAULT 'UN'",
        )?;
        add_column_if_missing(&connection, "products", "category_id", "INTEGER")?;
        add_column_if_missing(
            &connection,
            "products",
            "stock",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        add_column_if_missing(
            &connection,
            "products",
            "reorder_level",
            "INTEGER NOT NULL DEFAULT 0",
        )?;

        // FASE 4: validade de produtos (millis; NULL = sem validade)
        add_column_if_missing(&connection, "products", "validade", "INTEGER")?;

        // FASE 4: livro-razao de pagamentos (pagamento parcial de mesas)
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS sale_payments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                id_mesa INTEGER,
                id_sessao INTEGER,
                sale_id INTEGER,
                forma_pagamento TEXT NOT NULL,
                valor_cents INTEGER NOT NULL,
                troco_cents INTEGER NOT NULL DEFAULT 0,
                surcharge_cents INTEGER NOT NULL DEFAULT 0,
                operator_name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (id_sessao) REFERENCES mesa_sessao(id) ON DELETE CASCADE,
                FOREIGN KEY (sale_id) REFERENCES sales(id) ON DELETE SET NULL,
                FOREIGN KEY (id_mesa) REFERENCES mesas(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sale_payments_sessao ON sale_payments(id_sessao);
            CREATE INDEX IF NOT EXISTS idx_sale_payments_sale ON sale_payments(sale_id);",
        )?;
        add_column_if_missing(
            &connection,
            "users",
            "permissions_json",
            &format!("TEXT NOT NULL DEFAULT '{DEFAULT_OPERATOR_PERMISSIONS_JSON}'"),
        )?;

        // FASE 5: fechamento em cascata (turno operacional -> periodo contabil).
        // Dia fiscal deslocado (ex.: bar fecha as 06h): minutos a partir da meia-noite.
        add_column_if_missing(
            &connection,
            "app_config",
            "fiscal_day_start_minutes",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS periodos_contabeis (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                loja_id INTEGER NOT NULL DEFAULT 1,
                data TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'aberto',
                total_esperado_cents INTEGER NOT NULL DEFAULT 0,
                total_real_cents INTEGER NOT NULL DEFAULT 0,
                bloqueado_em INTEGER,
                bloqueado_por TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(loja_id, data)
            );

            CREATE TABLE IF NOT EXISTS turnos_operacionais (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                loja_id INTEGER NOT NULL DEFAULT 1,
                caixa_id INTEGER,
                operador TEXT NOT NULL,
                data_inicio INTEGER NOT NULL,
                data_fim INTEGER,
                status TEXT NOT NULL DEFAULT 'aberto',
                saldo_inicial_cents INTEGER NOT NULL DEFAULT 0,
                valor_esperado_cents INTEGER NOT NULL DEFAULT 0,
                valor_fisico_cents INTEGER,
                diferenca_cents INTEGER,
                observacoes TEXT,
                periodo_contabil_id INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (caixa_id) REFERENCES cash_registers(id) ON DELETE SET NULL,
                FOREIGN KEY (periodo_contabil_id) REFERENCES periodos_contabeis(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS sale_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                sale_id INTEGER NOT NULL,
                turno_operacional_id INTEGER,
                periodo_contabil_id INTEGER,
                valor_anterior_cents INTEGER NOT NULL,
                valor_novo_cents INTEGER NOT NULL,
                motivo TEXT NOT NULL,
                usuario TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (sale_id) REFERENCES sales(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_periodos_data ON periodos_contabeis(loja_id, data);
            CREATE INDEX IF NOT EXISTS idx_turnos_status ON turnos_operacionais(status);
            CREATE INDEX IF NOT EXISTS idx_turnos_periodo ON turnos_operacionais(periodo_contabil_id);
            CREATE INDEX IF NOT EXISTS idx_sale_audit_sale ON sale_audit(sale_id);",
        )?;
        add_column_if_missing(&connection, "sales", "turno_operacional_id", "INTEGER")?;
        add_column_if_missing(&connection, "sales", "periodo_contabil_id", "INTEGER")?;
        add_column_if_missing(
            &connection,
            "turnos_operacionais",
            "saldo_inicial_cents",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        add_column_if_missing(&connection, "cash_movements", "turno_id", "INTEGER")?;
        // Dinheiro de mesa entra na gaveta no INSTANTE do pagamento (nao no
        // fechamento da conta). Carimbamos o turno ativo em cada pagamento para
        // que o esperado da gaveta atribua o dinheiro ao turno certo, mesmo que
        // a conta atravesse a virada de turno.
        add_column_if_missing(
            &connection,
            "sale_payments",
            "turno_operacional_id",
            "INTEGER",
        )?;

        connection.execute(
            "INSERT OR IGNORE INTO app_config (
                id, company_name, tax_id, thank_you_message, validity_days, theme,
                printer_name, print_width_chars, onboarding_completed, setup_completed,
                table_count, backup_time, updated_at
             ) VALUES (1, '', '', NULL, 30, 'light', NULL, 48, 0, 0, 40, NULL, ?1)",
            params![now],
        )?;

        self.ensure_default_mesas()?;

        Ok(())
    }

    fn ensure_default_mesas(&self) -> AppResult<()> {
        let connection = self.connection()?;
        let now = now_millis();

        let table_count = connection
            .query_row(
                "SELECT table_count FROM app_config WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(40)
            .clamp(1, 100);

        for numero in 1..=table_count {
            connection.execute(
                "INSERT OR IGNORE INTO mesas (numero, capacidade, criada_em)
                 VALUES (?1, NULL, ?2)",
                params![numero, now],
            )?;
        }

        Ok(())
    }

    fn ensure_open_session(&self, id_mesa: i64, nome_cliente: Option<String>) -> AppResult<()> {
        self.get_mesa_by_id(id_mesa)?;
        let now = now_millis();
        let connection = self.connection()?;
        let existing = self.get_mesa_sessao_optional(id_mesa)?;

        match existing {
            Some(sessao) if sessao.tempo_fim.is_none() => {
                connection.execute(
                    "UPDATE mesa_sessao SET nome_cliente = ?1 WHERE id_mesa = ?2",
                    params![nome_cliente, id_mesa],
                )?;
            }
            Some(_) => {
                connection.execute(
                    "UPDATE mesa_sessao
                     SET tempo_inicio = ?1, tempo_fim = NULL, nome_cliente = ?2,
                         forma_pagamento = NULL, valor_total_cents = NULL, id_unico = ?3
                     WHERE id_mesa = ?4",
                    params![
                        now,
                        nome_cliente,
                        generate_short_id(now, id_mesa as u64),
                        id_mesa
                    ],
                )?;
            }
            None => {
                connection.execute(
                    "INSERT INTO mesa_sessao (id_mesa, tempo_inicio, tempo_fim, nome_cliente,
                         forma_pagamento, valor_total_cents, id_unico)
                     VALUES (?1, ?2, NULL, ?3, NULL, NULL, ?4)",
                    params![
                        id_mesa,
                        now,
                        nome_cliente,
                        generate_short_id(now, id_mesa as u64)
                    ],
                )?;
            }
        }

        Ok(())
    }

    fn get_mesa_sessao_optional(&self, id_mesa: i64) -> AppResult<Option<MesaSessao>> {
        let connection = self.connection()?;
        let sessao = connection
            .query_row(
                "SELECT id, id_mesa, tempo_inicio, tempo_fim, nome_cliente, forma_pagamento,
                        valor_total_cents, id_unico
                 FROM mesa_sessao WHERE id_mesa = ?1",
                params![id_mesa],
                map_mesa_sessao,
            )
            .optional()?;

        Ok(sessao)
    }

    fn insert_log(
        &self,
        tipo: &str,
        numero_mesa: Option<i64>,
        nome_cliente: Option<String>,
        valor_total_cents: Option<i64>,
        forma_pagamento: Option<String>,
        tempo_permanencia: Option<String>,
        lista_produtos_json: Option<String>,
        data_hora: Option<i64>,
        id_mesa_unico: Option<String>,
    ) -> AppResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO logs (
                tipo, numero_mesa, nome_cliente, valor_total_cents, forma_pagamento,
                tempo_permanencia, lista_produtos_json, data_hora, id_mesa_unico
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                tipo,
                numero_mesa,
                nome_cliente,
                valor_total_cents,
                forma_pagamento,
                tempo_permanencia,
                lista_produtos_json,
                data_hora.unwrap_or_else(now_millis),
                id_mesa_unico
            ],
        )?;
        Ok(())
    }

    fn enrich_cash_register(&self, mut register: CashRegister) -> AppResult<CashRegister> {
        let connection = self.connection()?;
        let end = register.closed_at.unwrap_or_else(now_millis);
        // Dinheiro de VENDAS DIRETAS (nao-mesa) vem de `sales`...
        let cash_direct = connection.query_row(
            "SELECT COALESCE(SUM(total_cents), 0)
             FROM sales
             WHERE payment_method = 'dinheiro' AND sale_type != 'mesa'
               AND created_at >= ?1 AND created_at <= ?2",
            params![register.opened_at, end],
            |row| row.get::<_, i64>(0),
        )?;
        // ...e o dinheiro de MESAS vem do razao (apenas o valor aplicado, sem troco):
        let cash_mesa = connection.query_row(
            "SELECT COALESCE(SUM(sp.valor_cents), 0)
             FROM sale_payments sp JOIN sales s ON s.id = sp.sale_id
             WHERE sp.forma_pagamento = 'dinheiro' AND s.sale_type = 'mesa'
               AND s.created_at >= ?1 AND s.created_at <= ?2",
            params![register.opened_at, end],
            |row| row.get::<_, i64>(0),
        )?;
        let cash_sales = cash_direct + cash_mesa;
        let suprimentos = connection.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0)
             FROM cash_movements
             WHERE cash_register_id = ?1 AND movement_type = 'suprimento'",
            params![register.id],
            |row| row.get::<_, i64>(0),
        )?;
        let sangrias = connection.query_row(
            "SELECT COALESCE(SUM(amount_cents), 0)
             FROM cash_movements
             WHERE cash_register_id = ?1 AND movement_type = 'sangria'",
            params![register.id],
            |row| row.get::<_, i64>(0),
        )?;
        register.expected_balance_cents =
            register.initial_balance_cents + cash_sales + suprimentos - sangrias;
        register.difference_cents = register
            .final_counted_cents
            .map(|final_counted| final_counted - register.expected_balance_cents);
        Ok(register)
    }
}

fn generate_ticket_id(issued_at: i64, sequence: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut value = (issued_at as u64)
        .wrapping_mul(1_103_515_245)
        .wrapping_add(sequence.wrapping_mul(12_345))
        .wrapping_add(0x9E37_79B9);
    let mut id = String::with_capacity(6);

    for _ in 0..6 {
        let index = (value % 36) as usize;
        id.push(ALPHABET[index] as char);
        value = value / 36 + 17;
    }

    id
}

fn normalize_ticket_id(ticket_id: &str) -> AppResult<String> {
    let normalized = ticket_id.trim().to_uppercase();

    if normalized.is_empty() {
        return Err(AppError::InvalidInput(
            "Informe o ID do ticket.".to_string(),
        ));
    }

    if normalized.len() > 80 {
        return Err(AppError::InvalidInput(
            "ID do ticket muito longo.".to_string(),
        ));
    }

    Ok(normalized)
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> AppResult<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !columns.iter().any(|existing| existing == column) {
        connection.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }

    Ok(())
}

fn hash_password(password: &str) -> String {
    let mut hash = 1469598103934665603u64;
    for byte in password.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{hash:016x}")
}

fn map_product(row: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        name: row.get(1)?,
        price_cents: row.get(2)?,
        barcode: row.get(3)?,
        cost_price_cents: row.get(4)?,
        unit: row.get(5)?,
        category_id: row.get(6)?,
        category_name: row.get(7)?,
        stock: row.get(8)?,
        reorder_level: row.get(9)?,
        sold_quantity: row.get(13)?,
        description: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        validade: row.get(14)?,
    })
}

fn map_mesa(row: &rusqlite::Row<'_>) -> rusqlite::Result<Mesa> {
    Ok(Mesa {
        id: row.get(0)?,
        numero: row.get(1)?,
        capacidade: row.get(2)?,
        criada_em: row.get(3)?,
        status: row.get(4)?,
        tempo_inicio: row.get(5)?,
        total_cents: row.get(6)?,
    })
}

fn map_mesa_produto(row: &rusqlite::Row<'_>) -> rusqlite::Result<MesaProdutoDetalhado> {
    let quantidade = row.get::<_, i64>(3)?;
    let price_cents = row.get::<_, i64>(7)?;
    Ok(MesaProdutoDetalhado {
        id: row.get(0)?,
        id_mesa: row.get(1)?,
        id_produto: row.get(2)?,
        quantidade,
        adicionado_em: row.get(4)?,
        produto: Product {
            id: row.get(5)?,
            name: row.get(6)?,
            price_cents,
            barcode: row.get(8)?,
            cost_price_cents: row.get(9)?,
            unit: row.get(10)?,
            category_id: row.get(11)?,
            category_name: row.get(12)?,
            stock: row.get(13)?,
            reorder_level: row.get(14)?,
            sold_quantity: 0,
            description: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
            validade: None,
        },
        subtotal_cents: quantidade * price_cents,
    })
}

fn map_mesa_sessao(row: &rusqlite::Row<'_>) -> rusqlite::Result<MesaSessao> {
    Ok(MesaSessao {
        id: row.get(0)?,
        id_mesa: row.get(1)?,
        tempo_inicio: row.get(2)?,
        tempo_fim: row.get(3)?,
        nome_cliente: row.get(4)?,
        forma_pagamento: row.get(5)?,
        valor_total_cents: row.get(6)?,
        id_unico: row.get(7)?,
    })
}

fn map_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<LogEntry> {
    Ok(LogEntry {
        id: row.get(0)?,
        tipo: row.get(1)?,
        numero_mesa: row.get(2)?,
        nome_cliente: row.get(3)?,
        valor_total_cents: row.get(4)?,
        forma_pagamento: row.get(5)?,
        tempo_permanencia: row.get(6)?,
        lista_produtos_json: row.get(7)?,
        data_hora: row.get(8)?,
        id_mesa_unico: row.get(9)?,
    })
}

fn map_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
    })
}

fn map_local_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<LocalUser> {
    let permissions_json = row.get::<_, String>(3)?;
    let permissions = serde_json::from_str::<Vec<String>>(&permissions_json).unwrap_or_default();
    Ok(LocalUser {
        id: row.get(0)?,
        username: row.get(1)?,
        role: row.get(2)?,
        permissions,
        active: row.get::<_, i64>(4)? == 1,
        created_at: row.get(5)?,
    })
}

fn map_cash_register(row: &rusqlite::Row<'_>) -> rusqlite::Result<CashRegister> {
    let initial_balance_cents = row.get::<_, i64>(3)?;
    let final_counted_cents = row.get::<_, Option<i64>>(4)?;
    let expected_balance_cents = initial_balance_cents;
    Ok(CashRegister {
        id: row.get(0)?,
        opened_at: row.get(1)?,
        closed_at: row.get(2)?,
        initial_balance_cents,
        final_counted_cents,
        expected_balance_cents,
        difference_cents: final_counted_cents.map(|value| value - expected_balance_cents),
        operator_name: row.get(5)?,
    })
}

fn map_cash_movement(row: &rusqlite::Row<'_>) -> rusqlite::Result<CashMovement> {
    Ok(CashMovement {
        id: row.get(0)?,
        cash_register_id: row.get(1)?,
        turno_id: row.get(7)?,
        movement_type: row.get(2)?,
        amount_cents: row.get(3)?,
        note: row.get(4)?,
        operator_name: row.get(5)?,
        created_at: row.get(6)?,
    })
}

const TURNO_COLUMNS: &str = "id, loja_id, caixa_id, operador, data_inicio, data_fim, status, \
    valor_esperado_cents, valor_fisico_cents, diferenca_cents, observacoes, periodo_contabil_id, \
    created_at, updated_at, saldo_inicial_cents";

const PERIODO_COLUMNS: &str = "id, loja_id, data, status, total_esperado_cents, total_real_cents, \
    bloqueado_em, bloqueado_por, created_at, updated_at";

fn map_turno(row: &rusqlite::Row<'_>) -> rusqlite::Result<TurnoOperacional> {
    let status = row.get::<_, String>(6)?;
    Ok(TurnoOperacional {
        id: row.get(0)?,
        loja_id: row.get(1)?,
        caixa_id: row.get(2)?,
        operador: row.get(3)?,
        data_inicio: row.get(4)?,
        data_fim: row.get(5)?,
        status: TurnoStatus::parse(&status),
        valor_esperado_cents: row.get(7)?,
        valor_fisico_cents: row.get(8)?,
        diferenca_cents: row.get(9)?,
        observacoes: row.get(10)?,
        periodo_contabil_id: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        saldo_inicial_cents: row.get(14)?,
    })
}

fn map_periodo(row: &rusqlite::Row<'_>) -> rusqlite::Result<PeriodoContabil> {
    let status = row.get::<_, String>(3)?;
    Ok(PeriodoContabil {
        id: row.get(0)?,
        loja_id: row.get(1)?,
        data: row.get(2)?,
        status: PeriodoStatus::parse(&status),
        total_esperado_cents: row.get(4)?,
        total_real_cents: row.get(5)?,
        bloqueado_em: row.get(6)?,
        bloqueado_por: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_sale_audit(row: &rusqlite::Row<'_>) -> rusqlite::Result<SaleAuditEntry> {
    Ok(SaleAuditEntry {
        id: row.get(0)?,
        sale_id: row.get(1)?,
        turno_operacional_id: row.get(2)?,
        periodo_contabil_id: row.get(3)?,
        valor_anterior_cents: row.get(4)?,
        valor_novo_cents: row.get(5)?,
        motivo: row.get(6)?,
        usuario: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn business_date(ts_millis: i64, offset_minutes: i64) -> AppResult<String> {
    let shifted = ts_millis - offset_minutes * 60_000;
    let datetime = Local
        .timestamp_millis_opt(shifted)
        .single()
        .ok_or_else(|| AppError::InvalidInput("Instante invalido.".to_string()))?;
    Ok(datetime.date_naive().format("%Y-%m-%d").to_string())
}

fn map_stock_movement(row: &rusqlite::Row<'_>) -> rusqlite::Result<StockMovement> {
    Ok(StockMovement {
        id: row.get(0)?,
        product_id: row.get(1)?,
        product_name: row.get(2)?,
        movement_type: row.get(3)?,
        quantity: row.get(4)?,
        previous_stock: row.get(5)?,
        new_stock: row.get(6)?,
        operator_name: row.get(7)?,
        note: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn validate_product(input: ProductInput) -> AppResult<ProductInput> {
    let name = input.name.trim().to_string();
    let barcode = input
        .barcode
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let unit = input.unit.trim().to_uppercase();
    let description = input
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if name.is_empty() {
        return Err(AppError::InvalidInput(
            "Informe o nome do produto.".to_string(),
        ));
    }

    if name.len() > 120 {
        return Err(AppError::InvalidInput(
            "O nome do produto deve ter no maximo 120 caracteres.".to_string(),
        ));
    }

    if input.price_cents <= 0 {
        return Err(AppError::InvalidInput(
            "Informe um valor maior que zero para o produto.".to_string(),
        ));
    }

    if input.price_cents > 99_999_999 {
        return Err(AppError::InvalidInput(
            "O valor informado esta acima do limite permitido.".to_string(),
        ));
    }

    if input.cost_price_cents < 0 {
        return Err(AppError::InvalidInput(
            "O custo nao pode ser negativo.".to_string(),
        ));
    }

    if !matches!(unit.as_str(), "UN" | "KG" | "L" | "CX" | "PCT") {
        return Err(AppError::InvalidInput("Unidade invalida.".to_string()));
    }

    Ok(ProductInput {
        name,
        price_cents: input.price_cents,
        barcode,
        cost_price_cents: input.cost_price_cents,
        unit,
        category_id: input.category_id,
        stock: input.stock,
        reorder_level: input.reorder_level.max(0),
        description,
        validade: input.validade,
    })
}

// FASE 4: validacao rigida (equivale a regex ^[A-Za-z0-9]{1,20}$, sem nova crate)
fn validate_username(username: &str) -> AppResult<String> {
    let value = username.trim();
    if value.is_empty() || value.chars().count() > 20 {
        return Err(AppError::InvalidInput(
            "O usuario deve ter de 1 a 20 caracteres.".to_string(),
        ));
    }
    if !value.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AppError::InvalidInput(
            "O usuario deve conter apenas letras e numeros (sem espacos ou simbolos).".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn validate_password(password: &str) -> AppResult<()> {
    let len = password.chars().count();
    if len < 4 || len > 30 {
        return Err(AppError::InvalidInput(
            "A senha deve ter de 4 a 30 caracteres.".to_string(),
        ));
    }
    Ok(())
}

fn validate_config(input: AppConfigInput) -> AppResult<AppConfigInput> {
    let company_name = input.company_name.trim().to_string();
    let tax_id = input.tax_id.trim().to_string();
    let thank_you_message = input
        .thank_you_message
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let printer_name = input
        .printer_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if company_name.is_empty() {
        return Err(AppError::InvalidInput(
            "Informe o nome da empresa.".to_string(),
        ));
    }

    if company_name.len() > 100 {
        return Err(AppError::InvalidInput(
            "O nome da empresa deve ter no maximo 100 caracteres.".to_string(),
        ));
    }

    if tax_id.is_empty() {
        return Err(AppError::InvalidInput("Informe o CPF ou CNPJ.".to_string()));
    }

    if input.validity_days < 1 || input.validity_days > 3650 {
        return Err(AppError::InvalidInput(
            "A validade deve ficar entre 1 e 3650 dias.".to_string(),
        ));
    }

    if input.theme != "light" && input.theme != "dark" {
        return Err(AppError::InvalidInput("Tema invalido.".to_string()));
    }

    if input.print_width_chars < 32 || input.print_width_chars > 64 {
        return Err(AppError::InvalidInput(
            "A largura de impressao deve ficar entre 32 e 64 caracteres.".to_string(),
        ));
    }

    if input.table_count < 1 || input.table_count > 100 {
        return Err(AppError::InvalidInput(
            "A quantidade de mesas deve ficar entre 1 e 100.".to_string(),
        ));
    }

    Ok(AppConfigInput {
        company_name,
        tax_id,
        thank_you_message,
        validity_days: input.validity_days,
        theme: input.theme,
        printer_name,
        print_width_chars: input.print_width_chars,
        setup_completed: input.setup_completed,
        table_count: input.table_count,
        backup_time: input.backup_time,
    })
}

fn validate_mesa_product_input(input: &MesaProdutoInput) -> AppResult<()> {
    if input.id_mesa <= 0 {
        return Err(AppError::InvalidInput("Mesa invalida.".to_string()));
    }

    if input.id_produto <= 0 {
        return Err(AppError::InvalidInput("Produto invalido.".to_string()));
    }

    if input.quantidade <= 0 || input.quantidade > 999 {
        return Err(AppError::InvalidInput(
            "A quantidade deve ficar entre 1 e 999.".to_string(),
        ));
    }

    Ok(())
}

fn normalize_sale_items(items: &[SaleCartItemInput]) -> AppResult<Vec<(i64, i64)>> {
    if items.is_empty() {
        return Err(AppError::InvalidInput(
            "Adicione produtos ao carrinho.".to_string(),
        ));
    }

    let mut aggregated = BTreeMap::new();
    for item in items {
        if item.product_id <= 0 {
            return Err(AppError::InvalidInput("Produto invalido.".to_string()));
        }

        if item.quantidade <= 0 || item.quantidade > 999 {
            return Err(AppError::InvalidInput(
                "A quantidade deve ficar entre 1 e 999.".to_string(),
            ));
        }

        *aggregated.entry(item.product_id).or_insert(0) += item.quantidade;
    }

    Ok(aggregated.into_iter().collect())
}

fn ensure_permission(
    role: Option<&str>,
    permissions: Option<&[String]>,
    required_permission: &str,
) -> AppResult<()> {
    if role == Some("admin") {
        return Ok(());
    }

    if permissions
        .unwrap_or(&[])
        .iter()
        .any(|permission| permission == required_permission)
    {
        return Ok(());
    }

    Err(AppError::InvalidInput(
        "Usuario sem permissao para executar esta acao.".to_string(),
    ))
}

fn normalize_permissions_json(permissions: Option<Vec<String>>, role: &str) -> AppResult<String> {
    let raw_permissions = if role == "admin" {
        allowed_permissions()
            .iter()
            .map(|permission| permission.to_string())
            .collect::<Vec<_>>()
    } else {
        permissions.unwrap_or_else(|| {
            serde_json::from_str::<Vec<String>>(DEFAULT_OPERATOR_PERMISSIONS_JSON)
                .unwrap_or_default()
        })
    };

    let mut normalized = Vec::new();
    for permission in raw_permissions {
        if !allowed_permissions().contains(&permission.as_str()) {
            return Err(AppError::InvalidInput(
                "Permissao de usuario invalida.".to_string(),
            ));
        }

        if !normalized.iter().any(|existing| existing == &permission) {
            normalized.push(permission);
        }
    }

    serde_json::to_string(&normalized)
        .map_err(|error| AppError::Database(format!("Nao foi possivel salvar permissoes: {error}")))
}

fn allowed_permissions() -> &'static [&'static str] {
    &[
        "addTableProducts",
        "removeTableProducts",
        "closeTable",
        MANAGE_PRODUCTS_PERMISSION,
        MANAGE_USERS_PERMISSION,
        "manageTickets",
        "viewLogsReports",
        "manageCompanyInfo",
        "manageTicketValidity",
        "manageTableCount",
        "manageBackupTime",
        "configurePrinters",
        "manageCash",
        "manageCashMovements",
    ]
}

struct SalesReportPeriodBounds {
    period: String,
    period_label: String,
    previous_reference_label: String,
    current_start: i64,
    current_end: i64,
    previous_start: i64,
    previous_end: i64,
}

fn sales_report_period_bounds(period: &str) -> AppResult<SalesReportPeriodBounds> {
    let today = Local::now().date_naive();
    let normalized = period.trim().to_lowercase();

    match normalized.as_str() {
        "day" | "dia" | "daily" => {
            let previous_day = today.checked_sub_days(Days::new(1)).ok_or_else(|| {
                AppError::InvalidInput("Nao foi possivel calcular o dia anterior.".to_string())
            })?;
            let next_day = today.checked_add_days(Days::new(1)).ok_or_else(|| {
                AppError::InvalidInput("Nao foi possivel calcular o proximo dia.".to_string())
            })?;

            Ok(SalesReportPeriodBounds {
                period: "day".to_string(),
                period_label: "Vendas do dia".to_string(),
                previous_reference_label: "dia anterior".to_string(),
                current_start: local_start_of_day_millis(today)?,
                current_end: local_start_of_day_millis(next_day)?,
                previous_start: local_start_of_day_millis(previous_day)?,
                previous_end: local_start_of_day_millis(today)?,
            })
        }
        "month" | "mes" | "mensal" | "monthly" => {
            let first_day = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                .ok_or_else(|| AppError::InvalidInput("Mes invalido.".to_string()))?;
            let next_month = if today.month() == 12 {
                NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
            }
            .ok_or_else(|| AppError::InvalidInput("Proximo mes invalido.".to_string()))?;
            let previous_month = if today.month() == 1 {
                NaiveDate::from_ymd_opt(today.year() - 1, 12, 1)
            } else {
                NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1)
            }
            .ok_or_else(|| AppError::InvalidInput("Mes anterior invalido.".to_string()))?;

            Ok(SalesReportPeriodBounds {
                period: "month".to_string(),
                period_label: "Vendas do mes".to_string(),
                previous_reference_label: "mes anterior".to_string(),
                current_start: local_start_of_day_millis(first_day)?,
                current_end: local_start_of_day_millis(next_month)?,
                previous_start: local_start_of_day_millis(previous_month)?,
                previous_end: local_start_of_day_millis(first_day)?,
            })
        }
        _ => Err(AppError::InvalidInput(
            "Periodo de relatorio invalido.".to_string(),
        )),
    }
}

fn local_start_of_day_millis(date: NaiveDate) -> AppResult<i64> {
    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::InvalidInput("Data de relatorio invalida.".to_string()))?;
    local_datetime_millis(datetime)
}

fn local_datetime_millis(datetime: NaiveDateTime) -> AppResult<i64> {
    Local
        .from_local_datetime(&datetime)
        .earliest()
        .or_else(|| Local.from_local_datetime(&datetime).latest())
        .map(|value| value.timestamp_millis())
        .ok_or_else(|| AppError::InvalidInput("Data local invalida.".to_string()))
}

fn comparison_summary(current: i64, previous: i64, reference_label: &str) -> (Option<f64>, String) {
    if previous == 0 {
        if current == 0 {
            return (
                None,
                format!("Sem vendas no periodo atual e no {reference_label}."),
            );
        }

        return (
            Some(100.0),
            format!("Aumento de 100% no total de vendas comparado com o {reference_label}."),
        );
    }

    let percent = (((current - previous) as f64 / previous as f64) * 100.0 * 10.0).round() / 10.0;

    if percent > 0.0 {
        (
            Some(percent),
            format!(
                "Aumento de {} no total de vendas comparado com o {reference_label}.",
                format_percent(percent)
            ),
        )
    } else if percent < 0.0 {
        (
            Some(percent),
            format!(
                "Queda de {} no total de vendas comparado com o {reference_label}.",
                format_percent(percent.abs())
            ),
        )
    } else {
        (
            Some(0.0),
            format!("Sem variacao no total de vendas comparado com o {reference_label}."),
        )
    }
}

fn format_percent(value: f64) -> String {
    if (value.fract()).abs() < 0.05 {
        format!("{:.0}%", value)
    } else {
        format!("{:.1}%", value).replace('.', ",")
    }
}

fn validate_payment_method(value: &str) -> AppResult<String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "pix" | "dinheiro" | "debito" | "credito" => Ok(normalized),
        _ => Err(AppError::InvalidInput(
            "Forma de pagamento invalida.".to_string(),
        )),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn format_duration(duration_millis: i64) -> String {
    let total_seconds = (duration_millis / 1000).max(0);
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn generate_short_id(seed: i64, sequence: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut value = (seed as u64)
        .wrapping_mul(1_103_515_245)
        .wrapping_add(sequence.wrapping_mul(12_345))
        .wrapping_add(TICKET_COUNTER.fetch_add(1, Ordering::Relaxed));
    let mut id = String::with_capacity(6);

    for _ in 0..6 {
        let index = (value % 36) as usize;
        id.push(ALPHABET[index] as char);
        value = value / 36 + 17;
    }

    id
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_database() -> (Database, PathBuf) {
        let unique = TICKET_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "portex_pdv_test_{}_{}.sqlite",
            std::process::id(),
            unique
        ));
        let _ = std::fs::remove_file(&path);
        (
            Database::initialize(path.clone()).expect("database should initialize"),
            path,
        )
    }

    fn sample_product(name: &str, stock: i64) -> ProductInput {
        ProductInput {
            name: name.to_string(),
            price_cents: 1_000,
            barcode: None,
            cost_price_cents: 400,
            unit: "UN".to_string(),
            category_id: None,
            stock,
            reorder_level: 2,
            description: None,
            validade: None,
        }
    }

    #[test]
    fn simulacao_noite_de_bar_dinheiro_entre_turnos() {
        let (database, path) = test_database();

        // Produtos do bar (preco 1000, custo 400 cada).
        let chopp = database
            .create_product(sample_product("Chopp", 100))
            .expect("produto chopp");
        let porcao = database
            .create_product(sample_product("Porcao", 100))
            .expect("produto porcao");

        // ===================== TURNO 1: Ana, fundo R$200 =====================
        let t1 = database
            .abrir_turno(AbrirTurnoInput {
                operador: "Ana".into(),
                caixa_id: None,
                saldo_inicial_cents: 20_000,
            })
            .expect("abrir turno 1");

        // Venda direta em DINHEIRO: 3x Chopp = 3000 (entra na gaveta).
        database
            .fechar_venda_caixa(FecharVendaCaixaInput {
                forma_pagamento: "dinheiro".into(),
                valor_pago_cents: Some(3_000),
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("Ana".into()),
                items: vec![SaleCartItemInput {
                    product_id: chopp.id,
                    quantidade: 3,
                }],
            })
            .expect("venda dinheiro");

        // Venda direta no CREDITO: 2x Porcao (NAO entra na gaveta de dinheiro).
        database
            .fechar_venda_caixa(FecharVendaCaixaInput {
                forma_pagamento: "credito".into(),
                valor_pago_cents: None,
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("Ana".into()),
                items: vec![SaleCartItemInput {
                    product_id: porcao.id,
                    quantidade: 2,
                }],
            })
            .expect("venda credito");

        // Mesa 1 quitada integralmente em DINHEIRO: 5x Chopp = 5000.
        let mesa1 = database
            .get_all_mesas()
            .unwrap()
            .into_iter()
            .find(|m| m.numero == 1)
            .unwrap();
        database
            .replace_mesa_produtos(
                mesa1.id,
                Some("Joao".into()),
                vec![MesaProdutoInput {
                    id_mesa: mesa1.id,
                    id_produto: chopp.id,
                    quantidade: 5,
                }],
            )
            .expect("itens mesa 1");
        let pay1 = database
            .registrar_pagamento_mesa(RegistrarPagamentoMesaInput {
                id_mesa: mesa1.id,
                forma_pagamento: "dinheiro".into(),
                valor_cents: 5_000,
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("Ana".into()),
            })
            .expect("pagamento mesa 1");
        assert!(pay1.finalizada);

        // Mesa 2: PARCIAL em dinheiro durante o turno da Ana. Consome 10000,
        // a cliente Maria adianta 4000 e deixa 6000 em aberto (sai sem fechar).
        let mesa2 = database
            .get_all_mesas()
            .unwrap()
            .into_iter()
            .find(|m| m.numero == 2)
            .unwrap();
        database
            .replace_mesa_produtos(
                mesa2.id,
                Some("Maria".into()),
                vec![MesaProdutoInput {
                    id_mesa: mesa2.id,
                    id_produto: porcao.id,
                    quantidade: 10,
                }],
            )
            .expect("itens mesa 2");
        let parcial = database
            .registrar_pagamento_mesa(RegistrarPagamentoMesaInput {
                id_mesa: mesa2.id,
                forma_pagamento: "dinheiro".into(),
                valor_cents: 4_000,
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("Ana".into()),
            })
            .expect("pagamento parcial mesa 2");
        assert!(!parcial.finalizada);
        assert_eq!(parcial.saldo_restante_cents, 6_000);

        // Sangria de R$10 (retirada da gaveta).
        database
            .add_cash_movement(CashMovementInput {
                movement_type: "sangria".into(),
                amount_cents: 1_000,
                note: None,
                operator_name: "Ana".into(),
            })
            .expect("sangria");

        // Dinheiro FISICO real na gaveta da Ana ao final do turno:
        //   fundo 20000 + venda direta 3000 + mesa1 5000 + mesa2 parcial 4000
        //   - sangria 1000 = 31000
        let gaveta_fisica_ana = 20_000 + 3_000 + 5_000 + 4_000 - 1_000;
        assert_eq!(gaveta_fisica_ana, 31_000);

        let esperado_ana = database
            .get_cashier_status()
            .unwrap()
            .esperado_atual_cents
            .unwrap();

        // CORRECAO B: o parcial de 4000 e atribuido ao turno da Ana no instante
        // do pagamento (sale_payments.turno_operacional_id), entao ENTRA no
        // esperado dela mesmo com a conta ainda aberta. Esperado bate com a
        // gaveta fisica.
        assert_eq!(esperado_ana, 31_000);
        let diferenca_ana = gaveta_fisica_ana - esperado_ana;
        assert_eq!(diferenca_ana, 0); // sem sobra fantasma

        let t1_fechado = database
            .fechar_turno(FecharTurnoInput {
                turno_id: t1.id,
                valor_fisico_cents: gaveta_fisica_ana,
                observacoes: None,
            })
            .expect("fechar turno 1");
        assert_eq!(t1_fechado.valor_esperado_cents, 31_000);
        assert_eq!(t1_fechado.diferenca_cents, Some(0));

        // ===================== TURNO 2: Bruno, fundo R$150 =====================
        let t2 = database
            .abrir_turno(AbrirTurnoInput {
                operador: "Bruno".into(),
                caixa_id: None,
                saldo_inicial_cents: 15_000,
            })
            .expect("abrir turno 2");

        // Maria volta e quita a mesa2: 6000 em dinheiro -> finaliza a venda.
        let quita = database
            .registrar_pagamento_mesa(RegistrarPagamentoMesaInput {
                id_mesa: mesa2.id,
                forma_pagamento: "dinheiro".into(),
                valor_cents: 6_000,
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("Bruno".into()),
            })
            .expect("quitar mesa 2");
        assert!(quita.finalizada);

        // Dinheiro FISICO real na gaveta do Bruno: fundo 15000 + 6000 = 21000.
        let gaveta_fisica_bruno = 15_000 + 6_000;
        assert_eq!(gaveta_fisica_bruno, 21_000);

        let esperado_bruno = database
            .get_cashier_status()
            .unwrap()
            .esperado_atual_cents
            .unwrap();

        // CORRECAO B: ao quitar a mesa2 agora, SO os 6000 recebidos pelo Bruno
        // sao carimbados no turno dele. Os 4000 ja ficaram com a Ana. O esperado
        // do Bruno bate com a gaveta fisica dele.
        assert_eq!(esperado_bruno, 21_000); // 15000 + 6000
        let diferenca_bruno = gaveta_fisica_bruno - esperado_bruno;
        assert_eq!(diferenca_bruno, 0); // sem falta fantasma

        let t2_fechado = database
            .fechar_turno(FecharTurnoInput {
                turno_id: t2.id,
                valor_fisico_cents: gaveta_fisica_bruno,
                observacoes: None,
            })
            .expect("fechar turno 2");
        assert_eq!(t2_fechado.diferenca_cents, Some(0));

        // ===================== PERIODO =====================
        let periodo = database
            .consolidar_periodo(ConsolidarPeriodoInput {
                data: None,
                usuario: "admin".into(),
            })
            .expect("consolidar periodo");

        // CORRECAO B: agora cada operador fecha certo (Ana e Bruno com diferenca
        // 0) E o periodo continua batendo. 31000 + 21000 dos dois lados.
        assert_eq!(periodo.total_esperado_cents, 52_000); // 31000 + 21000
        assert_eq!(periodo.total_real_cents, 52_000); // 31000 + 21000

        // PROBLEMA C (de UX, tratado na tela): o total do periodo reflete SO a
        // gaveta de dinheiro. A receita real do dia (relatorio) inclui credito/
        // pix e e um numero diferente — por isso a tela rotula os dois separados.
        let rel = database.get_sales_report("day").expect("relatorio do dia");
        println!(
            "PERIODO esperado={} real={} | RELATORIO faturamento={} (inclui cartao/pix)",
            periodo.total_esperado_cents, periodo.total_real_cents, rel.total_sales_cents
        );
        assert!(
            rel.total_sales_cents != periodo.total_esperado_cents,
            "faturamento e gaveta de dinheiro nao sao a mesma coisa"
        );

        // Gatekeeper: sem turno aberto (ambos fechados) a venda e bloqueada.
        let bloqueio = database.fechar_venda_caixa(FecharVendaCaixaInput {
            forma_pagamento: "dinheiro".into(),
            valor_pago_cents: Some(1_000),
            aplicar_acrescimo: None,
            aplicar_garcom: None,
            operator_name: Some("Ninguem".into()),
            items: vec![SaleCartItemInput {
                product_id: chopp.id,
                quantidade: 1,
            }],
        });
        assert!(bloqueio.is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn turno_drawer_tracks_difference() {
        let (database, path) = test_database();

        let turno = database
            .abrir_turno(AbrirTurnoInput {
                operador: "admin".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 10_000,
            })
            .expect("turno should open");
        assert_eq!(turno.saldo_inicial_cents, 10_000);

        database
            .add_cash_movement(CashMovementInput {
                movement_type: "suprimento".to_string(),
                amount_cents: 5_000,
                note: None,
                operator_name: "admin".to_string(),
            })
            .expect("cash injection should be recorded");
        database
            .add_cash_movement(CashMovementInput {
                movement_type: "sangria".to_string(),
                amount_cents: 2_000,
                note: None,
                operator_name: "admin".to_string(),
            })
            .expect("cash withdrawal should be recorded");

        // Gaveta ao vivo = 10000 + 5000 - 2000 = 13000.
        let status = database
            .get_cashier_status()
            .expect("cashier status should load");
        assert_eq!(status.esperado_atual_cents, Some(13_000));

        let closed = database
            .fechar_turno(FecharTurnoInput {
                turno_id: turno.id,
                valor_fisico_cents: 12_900,
                observacoes: None,
            })
            .expect("turno should close");

        assert_eq!(closed.valor_esperado_cents, 13_000);
        assert_eq!(closed.diferenca_cents, Some(-100));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn cash_movement_requires_open_turno() {
        let (database, path) = test_database();

        let error = database
            .add_cash_movement(CashMovementInput {
                movement_type: "sangria".to_string(),
                amount_cents: 1_000,
                note: None,
                operator_name: "admin".to_string(),
            })
            .expect_err("cash movement without an open turno should be blocked");

        assert!(format!("{error}").contains("Abra um turno"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sale_decrements_stock_and_updates_reports() {
        let (database, path) = test_database();
        database
            .abrir_turno(AbrirTurnoInput {
                operador: "admin".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 0,
            })
            .expect("turno should open");
        let product = database
            .create_product(sample_product("Cafe", 3))
            .expect("product should be created");
        let mesa = database
            .get_all_mesas()
            .expect("tables should load")
            .into_iter()
            .find(|mesa| mesa.numero == 1)
            .expect("table one should exist");

        database
            .replace_mesa_produtos(
                mesa.id,
                Some("Cliente teste".to_string()),
                vec![MesaProdutoInput {
                    id_mesa: mesa.id,
                    id_produto: product.id,
                    quantidade: 5,
                }],
            )
            .expect("table items should be saved");

        let ticket = database
            .fechar_mesa(FecharMesaInput {
                id_mesa: mesa.id,
                forma_pagamento: "pix".to_string(),
                valor_pago_cents: None,
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("admin".to_string()),
            })
            .expect("table should close");

        assert_eq!(ticket.total_cents, 5_000);
        let updated = database
            .get_product(product.id)
            .expect("product should still exist");
        assert_eq!(updated.stock, -2);

        let reports = database.get_reports().expect("reports should load");
        assert_eq!(reports.total_revenue_cents, 5_000);
        assert_eq!(reports.estimated_profit_cents, 3_000);
        assert!(reports
            .low_stock_products
            .iter()
            .any(|item| item.id == product.id));

        let details = database
            .get_mesa_details(mesa.id)
            .expect("table details should load");
        assert!(details.produtos.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn table_products_require_open_turno() {
        let (database, path) = test_database();
        let product = database
            .create_product(sample_product("Suco", 4))
            .expect("product should be created");
        let mesa = database
            .get_all_mesas()
            .expect("tables should load")
            .into_iter()
            .find(|mesa| mesa.numero == 1)
            .expect("table one should exist");

        let error = database
            .replace_mesa_produtos(
                mesa.id,
                None,
                vec![MesaProdutoInput {
                    id_mesa: mesa.id,
                    id_produto: product.id,
                    quantidade: 1,
                }],
            )
            .expect_err("closed turno should block adding products");

        assert!(format!("{error}").contains("Abra um turno"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn ticket_printing_requires_open_turno() {
        let (database, path) = test_database();

        let error = database
            .ensure_tickets_can_be_printed()
            .expect_err("closed turno should block ticket printing");

        assert!(format!("{error}").contains("Abra um turno"));

        database
            .abrir_turno(AbrirTurnoInput {
                operador: "admin".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 0,
            })
            .expect("turno should open");

        database
            .ensure_tickets_can_be_printed()
            .expect("tickets can be printed with an open turno");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn daily_sales_report_splits_sources_and_compares_previous_day() {
        let (database, path) = test_database();
        let connection = database.connection().expect("test connection should open");
        let now = now_millis();
        let yesterday = now - 86_400_000;

        connection
            .execute(
                "INSERT INTO sales (
                    mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                    surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                    created_at, nfe_status
                 ) VALUES (NULL, 'caixa', 'admin', 10000, 0, 0, 10000, 3000, 'pix', ?1, 'placeholder')",
                params![now],
            )
            .expect("cash sale should insert");
        connection
            .execute(
                "INSERT INTO sales (
                    mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                    surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                    created_at, nfe_status
                 ) VALUES (1, 'mesa', 'admin', 5000, 0, 0, 5000, 2000, 'dinheiro', ?1, 'placeholder')",
                params![now],
            )
            .expect("table sale should insert");
        connection
            .execute(
                "INSERT INTO sales (
                    mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
                    surcharge_cents, total_cents, estimated_profit_cents, payment_method,
                    created_at, nfe_status
                 ) VALUES (NULL, 'caixa', 'admin', 10000, 0, 0, 10000, 2500, 'pix', ?1, 'placeholder')",
                params![yesterday],
            )
            .expect("previous sale should insert");

        let report = database
            .get_sales_report("day")
            .expect("daily report should load");

        assert_eq!(report.direct_sales_cents, 10_000);
        assert_eq!(report.table_sales_cents, 5_000);
        assert_eq!(report.total_sales_cents, 15_000);
        assert_eq!(report.estimated_profit_cents, 5_000);
        assert_eq!(report.previous_total_sales_cents, 10_000);
        assert_eq!(report.comparison_percent, Some(50.0));
        assert!(report.comparison_text.contains("Aumento de 50%"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn generated_tickets_are_counted_as_ticket_sales_in_reports() {
        let (database, path) = test_database();
        let product = database
            .create_product(sample_product("Ticket Almoco", 10))
            .expect("product should be created");

        database
            .record_ticket_sale(&product, 3)
            .expect("ticket sale should be recorded");

        let report = database
            .get_sales_report("day")
            .expect("daily report should load");

        assert_eq!(report.ticket_sales_cents, 3_000);
        assert_eq!(report.total_sales_cents, 3_000);
        assert_eq!(report.estimated_profit_cents, 1_800);
        assert!(database
            .get_reports()
            .expect("reports should load")
            .top_products
            .iter()
            .any(|item| item.product_name == "Ticket Almoco" && item.quantity == 3));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reset_sales_requires_current_admin_password() {
        let (database, path) = test_database();
        database
            .create_user(CreateUserInput {
                username: "admin".to_string(),
                password: "senha123".to_string(),
                role: "admin".to_string(),
                permissions: None,
                requester_role: None,
                requester_permissions: None,
            })
            .expect("admin should be created");
        let product = database
            .create_product(sample_product("Combo", 10))
            .expect("product should be created");
        database
            .record_ticket_sale(&product, 2)
            .expect("sale should be recorded");

        let error = database
            .reset_sales("admin", "senha-errada")
            .expect_err("wrong admin password should be rejected");
        assert!(format!("{error}").contains("Senha de administrador invalida"));

        database
            .reset_sales("admin", "senha123")
            .expect("admin password should reset sales");
        let reports = database.get_reports().expect("reports should load");
        assert_eq!(reports.total_revenue_cents, 0);
        assert_eq!(reports.estimated_profit_cents, 0);
        assert!(reports.top_products.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn app_data_export_and_import_replaces_products_categories_and_basic_config() {
        let (source, source_path) = test_database();
        let (target, target_path) = test_database();
        source
            .save_config(AppConfigInput {
                company_name: "GPC Comercio".to_string(),
                tax_id: "12.345.678/0001-90".to_string(),
                thank_you_message: Some("Volte sempre".to_string()),
                validity_days: 30,
                theme: "light".to_string(),
                printer_name: None,
                print_width_chars: 42,
                setup_completed: true,
                table_count: 40,
                backup_time: None,
            })
            .expect("source config should be saved");
        let category = source
            .create_category(
                CategoryInput {
                    name: "Bebidas".to_string(),
                },
                Some("admin".to_string()),
                Some("admin".to_string()),
                None,
            )
            .expect("category should be created");
        source
            .create_product(ProductInput {
                name: "Agua Mineral".to_string(),
                price_cents: 500,
                barcode: Some("789000000001".to_string()),
                cost_price_cents: 200,
                unit: "UN".to_string(),
                category_id: Some(category.id),
                stock: 12,
                reorder_level: 3,
                description: Some("Sem gas".to_string()),
                validade: None,
            })
            .expect("product should be created");

        let exported = source.export_app_data().expect("app data should export");
        target
            .import_app_data(exported)
            .expect("app data should import");

        let imported_config = target.get_config().expect("target config should load");
        assert_eq!(imported_config.company_name, "GPC Comercio");
        assert_eq!(imported_config.tax_id, "12.345.678/0001-90");
        assert_eq!(imported_config.print_width_chars, 42);
        let categories = target
            .list_categories()
            .expect("categories should load after import");
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Bebidas");
        let products = target
            .list_products()
            .expect("products should load after import");
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].name, "Agua Mineral");
        assert_eq!(products[0].category_name.as_deref(), Some("Bebidas"));
        assert_eq!(products[0].stock, 12);

        let _ = std::fs::remove_file(source_path);
        let _ = std::fs::remove_file(target_path);
    }

    #[test]
    fn direct_cashier_sale_decrements_stock_and_updates_cash_balance() {
        let (database, path) = test_database();
        database
            .abrir_turno(AbrirTurnoInput {
                operador: "admin".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 2_000,
            })
            .expect("turno should open");
        let product = database
            .create_product(sample_product("Agua", 10))
            .expect("product should be created");

        let ticket = database
            .fechar_venda_caixa(FecharVendaCaixaInput {
                forma_pagamento: "dinheiro".to_string(),
                valor_pago_cents: Some(2_500),
                aplicar_acrescimo: None,
                aplicar_garcom: None,
                operator_name: Some("admin".to_string()),
                items: vec![SaleCartItemInput {
                    product_id: product.id,
                    quantidade: 2,
                }],
            })
            .expect("direct sale should be closed");

        assert_eq!(ticket.total_cents, 2_000);
        assert_eq!(ticket.troco_cents, Some(500));
        let updated = database
            .get_product(product.id)
            .expect("product should still exist");
        assert_eq!(updated.stock, 8);
        assert_eq!(updated.sold_quantity, 2);
        // Gaveta = fundo 2000 + venda em dinheiro 2000 = 4000.
        let status = database
            .get_cashier_status()
            .expect("cashier status should load");
        assert_eq!(status.esperado_atual_cents, Some(4_000));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn local_users_and_categories_are_persisted() {
        let (database, path) = test_database();

        assert!(!database
            .has_configured_users()
            .expect("configured user check should work"));

        let admin = database
            .create_user(CreateUserInput {
                username: "admin".to_string(),
                password: "senha123".to_string(),
                role: "admin".to_string(),
                permissions: None,
                requester_role: None,
                requester_permissions: None,
            })
            .expect("first admin should be created");
        assert_eq!(admin.role, "admin");
        assert!(admin
            .permissions
            .contains(&MANAGE_USERS_PERMISSION.to_string()));

        assert!(database
            .has_configured_users()
            .expect("configured user check should work"));

        let auth = database
            .login(LoginInput {
                username: "admin".to_string(),
                password: "senha123".to_string(),
            })
            .expect("created admin should login");
        assert_eq!(auth.user.role, "admin");

        let operator = database
            .create_user(CreateUserInput {
                username: "caixa".to_string(),
                password: "1234".to_string(),
                role: "operator".to_string(),
                permissions: Some(vec!["manageProducts".to_string()]),
                requester_role: Some("admin".to_string()),
                requester_permissions: None,
            })
            .expect("operator should be created");
        assert_eq!(operator.role, "operator");
        assert_eq!(operator.permissions, vec!["manageProducts".to_string()]);

        let category = database
            .create_category(
                CategoryInput {
                    name: "Bebidas".to_string(),
                },
                Some("admin".to_string()),
                Some("admin".to_string()),
                None,
            )
            .expect("category should be created");
        assert_eq!(category.name, "Bebidas");
        assert!(database
            .list_categories()
            .expect("categories should load")
            .iter()
            .any(|item| item.id == category.id));

        let updated = database
            .update_user(UpdateUserInput {
                id: operator.id,
                username: "caixa2".to_string(),
                password: None,
                role: "operator".to_string(),
                permissions: Some(vec![
                    "addTableProducts".to_string(),
                    "closeTable".to_string(),
                ]),
                requester_role: Some("admin".to_string()),
                requester_permissions: None,
            })
            .expect("operator permissions should update");
        assert_eq!(
            updated.permissions,
            vec!["addTableProducts".to_string(), "closeTable".to_string()]
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn turno_e_periodo_fecham_em_cascata() {
        let (database, path) = test_database();
        let product = database
            .create_product(sample_product("Cerveja", 100))
            .expect("product should be created");

        // Abre um turno operacional com fundo de troco de 10000.
        let turno = database
            .abrir_turno(AbrirTurnoInput {
                operador: "caixa1".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 10_000,
            })
            .expect("turno should open");
        assert_eq!(turno.status, TurnoStatus::Aberto);
        assert_eq!(turno.saldo_inicial_cents, 10_000);

        // Nao pode haver dois turnos abertos simultaneamente.
        assert!(database
            .abrir_turno(AbrirTurnoInput {
                operador: "caixa2".to_string(),
                caixa_id: None,
                saldo_inicial_cents: 0,
            })
            .is_err());

        // Vendas em ticket (nao-dinheiro) nao entram na gaveta.
        database
            .record_ticket_sale(&product, 2)
            .expect("sale should record");
        database
            .record_ticket_sale(&product, 3)
            .expect("sale should record");

        // Movimentos de gaveta atrelados ao turno ativo: +5000 -2000.
        database
            .add_cash_movement(CashMovementInput {
                movement_type: "suprimento".to_string(),
                amount_cents: 5_000,
                note: None,
                operator_name: "caixa1".to_string(),
            })
            .expect("suprimento should record");
        database
            .add_cash_movement(CashMovementInput {
                movement_type: "sangria".to_string(),
                amount_cents: 2_000,
                note: None,
                operator_name: "caixa1".to_string(),
            })
            .expect("sangria should record");

        // Consolidar com turno ainda aberto deve falhar.
        assert!(database
            .consolidar_periodo(ConsolidarPeriodoInput {
                data: None,
                usuario: "gerente".to_string(),
            })
            .is_err());

        // Gaveta esperada = 10000 + 0 (sem dinheiro) + 5000 - 2000 = 13000.
        // Contando 12900 -> diferenca -100.
        let fechado = database
            .fechar_turno(FecharTurnoInput {
                turno_id: turno.id,
                valor_fisico_cents: 12_900,
                observacoes: Some("diferenca de caixa".to_string()),
            })
            .expect("turno should close");
        assert_eq!(fechado.status, TurnoStatus::Fechado);
        assert_eq!(fechado.valor_esperado_cents, 13_000);
        assert_eq!(fechado.diferenca_cents, Some(-100));

        // Consolida o periodo do dia contabil.
        let periodo = database
            .consolidar_periodo(ConsolidarPeriodoInput {
                data: None,
                usuario: "gerente".to_string(),
            })
            .expect("periodo should consolidate");
        assert_eq!(periodo.status, PeriodoStatus::Fechado);
        assert_eq!(periodo.total_esperado_cents, 13_000);
        assert_eq!(periodo.total_real_cents, 12_900);

        // Status do caixa reflete a consolidacao.
        let status = database.get_cashier_status().expect("status should load");
        assert!(status.turno_ativo.is_none());
        assert_eq!(status.turnos_do_dia.len(), 1);
        assert_eq!(status.turnos_do_dia[0].status, TurnoStatus::Reconciliado);

        // Bloqueia o periodo, tornando-o imutavel.
        let bloqueado = database
            .bloquear_periodo(BloquearPeriodoInput {
                periodo_id: periodo.id,
                usuario: "gerente".to_string(),
            })
            .expect("periodo should lock");
        assert_eq!(bloqueado.status, PeriodoStatus::Bloqueado);

        // Uma venda do periodo bloqueado nao pode ser editada.
        let connection = database.connection().expect("connection should open");
        let sale_id: i64 = connection
            .query_row(
                "SELECT id FROM sales WHERE periodo_contabil_id = ?1 LIMIT 1",
                params![periodo.id],
                |row| row.get(0),
            )
            .expect("sale should be linked to periodo");
        assert!(!database
            .pode_editar_venda(sale_id)
            .expect("edit check should run"));
        assert!(database
            .editar_venda(EditarVendaInput {
                sale_id,
                novo_total_cents: 1,
                motivo: "tentativa".to_string(),
                usuario: "gerente".to_string(),
            })
            .is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn data_contabil_respeita_dia_fiscal_deslocado() {
        // Dia fiscal comecando as 06:00: vendas de madrugada caem no dia anterior.
        let offset = 6 * 60; // 06:00 em minutos
        // 2026-06-04 02:00 local -> deve pertencer a 2026-06-03.
        let madrugada = Local
            .with_ymd_and_hms(2026, 6, 4, 2, 0, 0)
            .single()
            .expect("valid datetime")
            .timestamp_millis();
        assert_eq!(business_date(madrugada, offset).unwrap(), "2026-06-03");

        // 2026-06-04 22:00 local -> mesmo dia 2026-06-04.
        let noite = Local
            .with_ymd_and_hms(2026, 6, 4, 22, 0, 0)
            .single()
            .expect("valid datetime")
            .timestamp_millis();
        assert_eq!(business_date(noite, offset).unwrap(), "2026-06-04");

        // Sem deslocamento, segue o dia civil.
        assert_eq!(business_date(madrugada, 0).unwrap(), "2026-06-04");
    }
}
