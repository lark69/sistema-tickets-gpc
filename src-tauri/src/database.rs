use crate::error::{AppError, AppResult};
use crate::models::{
    AppConfig, AppConfigInput, AuthPayload, CashMovement, CashMovementInput,
    CashRegister, Category, CategoryInput, CreateMesaInput, CreateUserInput, FecharMesaInput,
    IssuedTicket, LocalUser, LogEntry, LogFiltros, LoginInput, Mesa, MesaDetailed,
    MesaProdutoDetalhado, MesaProdutoInput, MesaSessao, OpenCashRegisterInput, Product,
    ProductInput, ProductUpdateInput, ReportsPayload, SalesByDay, StockAdjustInput,
    StockMovement, TicketData, TicketProduto, TopProductReport, CloseCashRegisterInput,
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

    pub fn list_products(&self) -> AppResult<Vec<Product>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT p.id, p.name, p.price_cents, p.barcode, p.cost_price_cents, p.unit,
                    p.category_id, c.name, p.stock, p.reorder_level, p.description,
                    p.created_at, p.updated_at
             FROM products p
             LEFT JOIN categories c ON c.id = p.category_id
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
                        p.created_at, p.updated_at
                 FROM products p
                 LEFT JOIN categories c ON c.id = p.category_id
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
                stock, reorder_level, description, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
        })?;
        let now = now_millis();
        let connection = self.connection()?;

        let affected = connection.execute(
            "UPDATE products
             SET name = ?1, price_cents = ?2, barcode = ?3, cost_price_cents = ?4,
                 unit = ?5, category_id = ?6, stock = ?7, reorder_level = ?8,
                 description = ?9, updated_at = ?10
             WHERE id = ?11",
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
                now,
                input.id
            ],
        )?;

        if affected == 0 {
            return Err(AppError::InvalidInput("Produto nao encontrado.".to_string()));
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
        let mut statement =
            connection.prepare("SELECT id, name, created_at FROM categories ORDER BY name COLLATE NOCASE")?;
        let categories = statement
            .query_map([], map_category)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(categories)
    }

    pub fn create_category(&self, input: CategoryInput, operator_name: Option<String>) -> AppResult<Category> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::InvalidInput("Informe o nome da categoria.".to_string()));
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
            Some(format!("[{{\"categoria\":\"{}\"}}]", category.name.replace('"', "'"))),
            None,
            None,
        );
        Ok(category)
    }

    pub fn adjust_stock(&self, input: StockAdjustInput) -> AppResult<StockMovement> {
        let movement_type = input.movement_type.trim().to_lowercase();
        if !matches!(movement_type.as_str(), "entrada" | "ajuste" | "saida") {
            return Err(AppError::InvalidInput("Tipo de movimento de estoque invalido.".to_string()));
        }
        if input.quantity == 0 {
            return Err(AppError::InvalidInput("Informe uma quantidade diferente de zero.".to_string()));
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
                    ms.tempo_inicio
             FROM mesas m
             LEFT JOIN mesa_produtos mp ON mp.id_mesa = m.id
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
        let operator_name =
            normalize_optional_text(input.operator_name.clone()).unwrap_or_else(|| "caixa".to_string());
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
        let estimated_profit_cents = details
            .produtos
            .iter()
            .map(|item| (item.produto.price_cents - item.produto.cost_price_cents) * item.quantidade)
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
                "SELECT id, username, role, active, created_at
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
        let mut statement =
            connection.prepare("SELECT id, username, role, active, created_at FROM users ORDER BY username")?;
        let users = statement
            .query_map([], map_local_user)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(users)
    }

    pub fn create_user(&self, input: CreateUserInput) -> AppResult<LocalUser> {
        let username = input.username.trim().to_string();
        let role = input.role.trim().to_lowercase();
        if username.is_empty() || input.password.len() < 4 {
            return Err(AppError::InvalidInput(
                "Informe usuario e senha com pelo menos 4 caracteres.".to_string(),
            ));
        }
        if role != "admin" && role != "operator" {
            return Err(AppError::InvalidInput("Perfil de usuario invalido.".to_string()));
        }
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO users (username, password_hash, role, active, created_at)
             VALUES (?1, ?2, ?3, 1, ?4)",
            params![username, hash_password(&input.password), role, now],
        )?;
        connection
            .query_row(
                "SELECT id, username, role, active, created_at FROM users WHERE id = ?1",
                params![connection.last_insert_rowid()],
                map_local_user,
            )
            .map_err(AppError::from)
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
        register.map(|register| self.enrich_cash_register(register)).transpose()
    }

    pub fn open_cash_register(&self, input: OpenCashRegisterInput) -> AppResult<CashRegister> {
        if self.get_current_cash_register()?.is_some() {
            return Err(AppError::InvalidInput("Ja existe um caixa aberto.".to_string()));
        }
        if input.initial_balance_cents < 0 {
            return Err(AppError::InvalidInput("Saldo inicial invalido.".to_string()));
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
        let current = self
            .get_current_cash_register()?
            .ok_or_else(|| AppError::InvalidInput("Abra o caixa antes de registrar movimentos.".to_string()))?;
        let movement_type = input.movement_type.trim().to_lowercase();
        if movement_type != "sangria" && movement_type != "suprimento" {
            return Err(AppError::InvalidInput("Movimento de caixa invalido.".to_string()));
        }
        if input.amount_cents <= 0 {
            return Err(AppError::InvalidInput("Informe um valor maior que zero.".to_string()));
        }
        let now = now_millis();
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO cash_movements (
                cash_register_id, movement_type, amount_cents, note, operator_name, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                current.id,
                movement_type,
                input.amount_cents,
                input.note,
                input.operator_name,
                now
            ],
        )?;
        Ok(CashMovement {
            id: connection.last_insert_rowid(),
            cash_register_id: current.id,
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
            "SELECT id, cash_register_id, movement_type, amount_cents, note, operator_name, created_at
             FROM cash_movements
             ORDER BY created_at DESC
             LIMIT 300",
        )?;
        let movements = statement
            .query_map([], map_cash_movement)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(movements)
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

        add_column_if_missing(&connection, "app_config", "table_count", "INTEGER NOT NULL DEFAULT 40")?;
        add_column_if_missing(&connection, "app_config", "backup_time", "TEXT")?;
        add_column_if_missing(&connection, "products", "barcode", "TEXT")?;
        add_column_if_missing(&connection, "products", "cost_price_cents", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&connection, "products", "unit", "TEXT NOT NULL DEFAULT 'UN'")?;
        add_column_if_missing(&connection, "products", "category_id", "INTEGER")?;
        add_column_if_missing(&connection, "products", "stock", "INTEGER NOT NULL DEFAULT 0")?;
        add_column_if_missing(&connection, "products", "reorder_level", "INTEGER NOT NULL DEFAULT 0")?;

        connection.execute(
            "INSERT OR IGNORE INTO app_config (
                id, company_name, tax_id, thank_you_message, validity_days, theme,
                printer_name, print_width_chars, onboarding_completed, setup_completed,
                table_count, backup_time, updated_at
             ) VALUES (1, '', '', NULL, 30, 'light', NULL, 48, 0, 0, 40, NULL, ?1)",
            params![now],
        )?;

        connection.execute(
            "INSERT OR IGNORE INTO users (username, password_hash, role, active, created_at)
             VALUES ('admin', ?1, 'admin', 1, ?2)",
            params![hash_password("admin"), now],
        )?;

        self.ensure_default_mesas()?;

        Ok(())
    }

    fn ensure_default_mesas(&self) -> AppResult<()> {
        let connection = self.connection()?;
        let now = now_millis();

        let table_count = connection
            .query_row("SELECT table_count FROM app_config WHERE id = 1", [], |row| {
                row.get::<_, i64>(0)
            })
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

    fn enrich_cash_register(&self, mut register: CashRegister) -> AppResult<CashRegister> {
        let connection = self.connection()?;
        let end = register.closed_at.unwrap_or_else(now_millis);
        let cash_sales = connection.query_row(
            "SELECT COALESCE(SUM(total_cents), 0)
             FROM sales
             WHERE payment_method = 'dinheiro' AND created_at >= ?1 AND created_at <= ?2",
            params![register.opened_at, end],
            |row| row.get::<_, i64>(0),
        )?;
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
        return Err(AppError::InvalidInput("Informe o ID do ticket.".to_string()));
    }

    if normalized.len() > 80 {
        return Err(AppError::InvalidInput("ID do ticket muito longo.".to_string()));
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
        connection.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"), [])?;
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
        description: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
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
            barcode: row.get(8)?,
            cost_price_cents: row.get(9)?,
            unit: row.get(10)?,
            category_id: row.get(11)?,
            category_name: row.get(12)?,
            stock: row.get(13)?,
            reorder_level: row.get(14)?,
            description: row.get(15)?,
            created_at: row.get(16)?,
            updated_at: row.get(17)?,
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
    Ok(LocalUser {
        id: row.get(0)?,
        username: row.get(1)?,
        role: row.get(2)?,
        active: row.get::<_, i64>(3)? == 1,
        created_at: row.get(4)?,
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
        movement_type: row.get(2)?,
        amount_cents: row.get(3)?,
        note: row.get(4)?,
        operator_name: row.get(5)?,
        created_at: row.get(6)?,
    })
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

    if input.cost_price_cents < 0 {
        return Err(AppError::InvalidInput("O custo nao pode ser negativo.".to_string()));
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
        (Database::initialize(path.clone()).expect("database should initialize"), path)
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
        }
    }

    #[test]
    fn cash_register_open_close_tracks_difference() {
        let (database, path) = test_database();

        let opened = database
            .open_cash_register(OpenCashRegisterInput {
                initial_balance_cents: 10_000,
                operator_name: "admin".to_string(),
            })
            .expect("cash register should open");

        assert_eq!(opened.initial_balance_cents, 10_000);

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

        let current = database
            .get_current_cash_register()
            .expect("current cash register query should work")
            .expect("cash register should be open");
        assert_eq!(current.expected_balance_cents, 13_000);

        let closed = database
            .close_cash_register(CloseCashRegisterInput {
                final_counted_cents: 12_900,
                operator_name: "admin".to_string(),
            })
            .expect("cash register should close");

        assert_eq!(closed.expected_balance_cents, 13_000);
        assert_eq!(closed.difference_cents, Some(-100));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sale_decrements_stock_and_updates_reports() {
        let (database, path) = test_database();
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
        assert!(reports.low_stock_products.iter().any(|item| item.id == product.id));

        let details = database
            .get_mesa_details(mesa.id)
            .expect("table details should load");
        assert!(details.produtos.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn local_users_and_categories_are_persisted() {
        let (database, path) = test_database();

        let admin = database
            .login(LoginInput {
                username: "admin".to_string(),
                password: "admin".to_string(),
            })
            .expect("default admin should login");
        assert_eq!(admin.user.role, "admin");

        let operator = database
            .create_user(CreateUserInput {
                username: "caixa".to_string(),
                password: "1234".to_string(),
                role: "operator".to_string(),
            })
            .expect("operator should be created");
        assert_eq!(operator.role, "operator");

        let category = database
            .create_category(
                CategoryInput {
                    name: "Bebidas".to_string(),
                },
                Some("admin".to_string()),
            )
            .expect("category should be created");
        assert_eq!(category.name, "Bebidas");
        assert!(database
            .list_categories()
            .expect("categories should load")
            .iter()
            .any(|item| item.id == category.id));
        let _ = std::fs::remove_file(path);
    }
}
