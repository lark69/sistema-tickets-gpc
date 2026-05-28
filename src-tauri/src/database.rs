use crate::error::{AppError, AppResult};
use crate::models::{AppConfig, AppConfigInput, IssuedTicket, Product, ProductInput, ProductUpdateInput};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TICKET_COUNTER: AtomicU64 = AtomicU64::new(1);

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
                    printer_name, print_width_chars, onboarding_completed, setup_completed, updated_at
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
                    updated_at: row.get(9)?,
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
                 updated_at = ?9
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
                now
            ],
        )?;

        self.get_config()
    }

    pub fn list_products(&self) -> AppResult<Vec<Product>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, name, price_cents, description, created_at, updated_at
             FROM products
             ORDER BY name COLLATE NOCASE ASC",
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
                "SELECT id, name, price_cents, description, created_at, updated_at
                 FROM products WHERE id = ?1",
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
            "INSERT INTO products (name, price_cents, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                normalized.name,
                normalized.price_cents,
                normalized.description,
                now,
                now
            ],
        )?;

        self.get_product(connection.last_insert_rowid())
    }

    pub fn update_product(&self, input: ProductUpdateInput) -> AppResult<Product> {
        if input.id <= 0 {
            return Err(AppError::InvalidInput("Produto invalido.".to_string()));
        }

        let normalized = validate_product(ProductInput {
            name: input.name,
            price_cents: input.price_cents,
            description: input.description,
        })?;
        let now = now_millis();
        let connection = self.connection()?;

        let affected = connection.execute(
            "UPDATE products
             SET name = ?1, price_cents = ?2, description = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                normalized.name,
                normalized.price_cents,
                normalized.description,
                now,
                input.id
            ],
        )?;

        if affected == 0 {
            return Err(AppError::InvalidInput("Produto nao encontrado.".to_string()));
        }

        self.get_product(input.id)
    }

    pub fn delete_product(&self, product_id: i64) -> AppResult<()> {
        if product_id <= 0 {
            return Err(AppError::InvalidInput("Produto invalido.".to_string()));
        }

        let connection = self.connection()?;
        let affected = connection.execute("DELETE FROM products WHERE id = ?1", params![product_id])?;

        if affected == 0 {
            return Err(AppError::InvalidInput("Produto nao encontrado.".to_string()));
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

    pub fn cleanup_expired_tickets(&self) -> AppResult<()> {
        let connection = self.connection()?;
        connection.execute(
            "DELETE FROM issued_tickets WHERE expires_at < ?1",
            params![now_millis()],
        )?;
        Ok(())
    }

    fn connection(&self) -> AppResult<Connection> {
        Connection::open(&self.path).map_err(AppError::from)
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
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS products (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                price_cents INTEGER NOT NULL,
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

            CREATE INDEX IF NOT EXISTS idx_products_name ON products(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_issued_tickets_expires_at ON issued_tickets(expires_at);",
        )?;

        connection.execute(
            "INSERT OR IGNORE INTO app_config (
                id, company_name, tax_id, thank_you_message, validity_days, theme,
                printer_name, print_width_chars, onboarding_completed, setup_completed, updated_at
             ) VALUES (1, '', '', NULL, 30, 'light', NULL, 48, 0, 0, ?1)",
            params![now],
        )?;

        Ok(())
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
        return Err(AppError::InvalidInput("Informe o ID do ticket.".to_string()));
    }

    if normalized.len() > 80 {
        return Err(AppError::InvalidInput("ID do ticket muito longo.".to_string()));
    }

    Ok(normalized)
}

fn map_product(row: &rusqlite::Row<'_>) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get(0)?,
        name: row.get(1)?,
        price_cents: row.get(2)?,
        description: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn validate_product(input: ProductInput) -> AppResult<ProductInput> {
    let name = input.name.trim().to_string();
    let description = input
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if name.is_empty() {
        return Err(AppError::InvalidInput("Informe o nome do produto.".to_string()));
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

    Ok(ProductInput {
        name,
        price_cents: input.price_cents,
        description,
    })
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
        return Err(AppError::InvalidInput("Informe o nome da empresa.".to_string()));
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

    Ok(AppConfigInput {
        company_name,
        tax_id,
        thank_you_message,
        validity_days: input.validity_days,
        theme: input.theme,
        printer_name,
        print_width_chars: input.print_width_chars,
        setup_completed: input.setup_completed,
    })
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
