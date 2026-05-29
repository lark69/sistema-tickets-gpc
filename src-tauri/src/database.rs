use crate::error::{AppError, AppResult};
use crate::models::{
    AppConfig, AppConfigInput, CreateMesaInput, FecharMesaInput, IssuedTicket, LogEntry, LogFiltros, Mesa,
    MesaDetailed, MesaProdutoDetalhado, MesaProdutoInput, MesaSessao, Product, ProductInput,
    ProductUpdateInput, TicketData, TicketProduto,
};
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

    pub fn get_all_mesas(&self) -> AppResult<Vec<Mesa>> {
        self.ensure_default_mesas()?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT m.id, m.numero, m.capacidade, m.criada_em,
                    CASE WHEN COALESCE(SUM(mp.quantidade), 0) > 0 THEN 'ativa' ELSE 'livre' END AS status,
                    ms.tempo_inicio
             FROM mesas m
             LEFT JOIN mesa_produtos mp ON mp.id_mesa = m.id
             LEFT JOIN mesa_sessao ms ON ms.id_mesa = m.id AND ms.tempo_fim IS NULL
             GROUP BY m.id, m.numero, m.capacidade, m.criada_em, ms.tempo_inicio
             ORDER BY m.numero ASC",
        )?;

        let mesas = statement
            .query_map([], map_mesa)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mesas)
    }

    pub fn create_mesa(&self, input: CreateMesaInput) -> AppResult<Mesa> {
        if input.numero < 1 || input.numero > 40 {
            return Err(AppError::InvalidInput(
                "O numero da mesa deve ficar entre 1 e 40.".to_string(),
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
                        ms.tempo_inicio
                 FROM mesas m
                 LEFT JOIN mesa_produtos mp ON mp.id_mesa = m.id
                 LEFT JOIN mesa_sessao ms ON ms.id_mesa = m.id AND ms.tempo_fim IS NULL
                 WHERE m.id = ?1
                 GROUP BY m.id, m.numero, m.capacidade, m.criada_em, ms.tempo_inicio",
                params![id_mesa],
                map_mesa,
            )
            .optional()?;

        mesa.ok_or_else(|| AppError::InvalidInput("Mesa nao encontrada.".to_string()))
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
                    p.id, p.name, p.price_cents, p.description, p.created_at, p.updated_at
             FROM mesa_produtos mp
             INNER JOIN products p ON p.id = mp.id_produto
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
        let transaction = connection.unchecked_transaction()?;

        transaction.execute("DELETE FROM mesa_produtos WHERE id_mesa = ?1", params![id_mesa])?;

        let has_items = !items.is_empty();
        for item in items {
            validate_mesa_product_input(&item)?;
            if item.id_mesa != id_mesa {
                return Err(AppError::InvalidInput("Produto enviado para mesa invalida.".to_string()));
            }
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
        let details = self.get_mesa_details(input.id_mesa)?;

        if details.produtos.is_empty() {
            return Err(AppError::InvalidInput("Adicione produtos antes de fechar a mesa.".to_string()));
        }

        let sessao = details
            .sessao
            .ok_or_else(|| AppError::InvalidInput("Mesa sem sessao aberta.".to_string()))?;
        let now = now_millis();
        let subtotal_cents = details.subtotal_cents;
        let acrescimo_cents = if forma_pagamento == "credito" {
            ((subtotal_cents as f64) * 0.05).round() as i64
        } else {
            0
        };
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

            CREATE INDEX IF NOT EXISTS idx_products_name ON products(name COLLATE NOCASE);
            CREATE INDEX IF NOT EXISTS idx_issued_tickets_expires_at ON issued_tickets(expires_at);
            CREATE INDEX IF NOT EXISTS idx_mesa_produtos_mesa ON mesa_produtos(id_mesa);
            CREATE INDEX IF NOT EXISTS idx_logs_data_hora ON logs(data_hora);
            CREATE INDEX IF NOT EXISTS idx_logs_tipo ON logs(tipo);",
        )?;

        connection.execute(
            "INSERT OR IGNORE INTO app_config (
                id, company_name, tax_id, thank_you_message, validity_days, theme,
                printer_name, print_width_chars, onboarding_completed, setup_completed, updated_at
             ) VALUES (1, '', '', NULL, 30, 'light', NULL, 48, 0, 0, ?1)",
            params![now],
        )?;

        self.ensure_default_mesas()?;

        Ok(())
    }

    fn ensure_default_mesas(&self) -> AppResult<()> {
        let connection = self.connection()?;
        let now = now_millis();

        for numero in 1..=40 {
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
                    params![now, nome_cliente, generate_short_id(now, id_mesa as u64), id_mesa],
                )?;
            }
            None => {
                connection.execute(
                    "INSERT INTO mesa_sessao (id_mesa, tempo_inicio, tempo_fim, nome_cliente,
                         forma_pagamento, valor_total_cents, id_unico)
                     VALUES (?1, ?2, NULL, ?3, NULL, NULL, ?4)",
                    params![id_mesa, now, nome_cliente, generate_short_id(now, id_mesa as u64)],
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

fn map_mesa(row: &rusqlite::Row<'_>) -> rusqlite::Result<Mesa> {
    Ok(Mesa {
        id: row.get(0)?,
        numero: row.get(1)?,
        capacidade: row.get(2)?,
        criada_em: row.get(3)?,
        status: row.get(4)?,
        tempo_inicio: row.get(5)?,
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
            description: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
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

fn validate_payment_method(value: &str) -> AppResult<String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "pix" | "dinheiro" | "debito" | "credito" => Ok(normalized),
        _ => Err(AppError::InvalidInput("Forma de pagamento invalida.".to_string())),
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
