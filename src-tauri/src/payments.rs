use crate::error::{AppError, AppResult};
use crate::models::{ContaMesa, PagamentoMesa, PagamentoMesaResult, TicketData, TicketProduto};
use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};

const CREDIT_SURCHARGE_BPS: i64 = 500; // 5,00%

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn format_duration(ms: i64) -> String {
    let s = (ms / 1000).max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

fn normalize_payment(value: &str) -> AppResult<String> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "pix" | "dinheiro" | "debito" | "credito" => Ok(v),
        _ => Err(AppError::InvalidInput("Forma de pagamento invalida.".into())),
    }
}

fn open_session_id(conn: &Connection, id_mesa: i64) -> AppResult<i64> {
    conn.query_row(
        "SELECT id FROM mesa_sessao WHERE id_mesa = ?1 AND tempo_fim IS NULL",
        params![id_mesa],
        |r| r.get::<_, i64>(0),
    )
    .optional()?
    .ok_or_else(|| AppError::InvalidInput("Mesa sem sessao aberta.".into()))
}

fn mesa_total_cents(conn: &Connection, id_mesa: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(mp.quantidade * p.price_cents), 0)
         FROM mesa_produtos mp JOIN products p ON p.id = mp.id_produto
         WHERE mp.id_mesa = ?1",
        params![id_mesa],
        |r| r.get(0),
    )?)
}

fn pago_cents(conn: &Connection, id_sessao: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(valor_cents), 0) FROM sale_payments
         WHERE id_sessao = ?1 AND sale_id IS NULL",
        params![id_sessao],
        |r| r.get(0),
    )?)
}

/// Estado da conta para a UI (saldo devedor + recebimentos).
pub fn conta_mesa(conn: &Connection, id_mesa: i64) -> AppResult<ContaMesa> {
    let total = mesa_total_cents(conn, id_mesa)?;
    let sessao = open_session_id(conn, id_mesa).ok();
    let (pago, pagamentos) = match sessao {
        Some(sid) => {
            let pago = pago_cents(conn, sid)?;
            let mut stmt = conn.prepare(
                "SELECT id, forma_pagamento, valor_cents, troco_cents, surcharge_cents, created_at
                 FROM sale_payments WHERE id_sessao = ?1 AND sale_id IS NULL
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt
                .query_map(params![sid], |r| {
                    Ok(PagamentoMesa {
                        id: r.get(0)?,
                        forma_pagamento: r.get(1)?,
                        valor_cents: r.get(2)?,
                        troco_cents: r.get(3)?,
                        surcharge_cents: r.get(4)?,
                        created_at: r.get(5)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            (pago, rows)
        }
        None => (0, Vec::new()),
    };
    Ok(ContaMesa {
        id_mesa,
        total_cents: total,
        pago_cents: pago,
        saldo_cents: (total - pago).max(0),
        pagamentos,
    })
}

struct MesaItem {
    id_produto: i64,
    name: String,
    quantidade: i64,
    price_cents: i64,
    cost_price_cents: i64,
    stock: i64,
}

fn mesa_items(conn: &Connection, id_mesa: i64) -> AppResult<Vec<MesaItem>> {
    let mut stmt = conn.prepare(
        "SELECT mp.id_produto, p.name, mp.quantidade, p.price_cents, p.cost_price_cents, p.stock
         FROM mesa_produtos mp JOIN products p ON p.id = mp.id_produto
         WHERE mp.id_mesa = ?1 ORDER BY mp.adicionado_em ASC",
    )?;
    let items = stmt
        .query_map(params![id_mesa], |r| {
            Ok(MesaItem {
                id_produto: r.get(0)?,
                name: r.get(1)?,
                quantidade: r.get(2)?,
                price_cents: r.get(3)?,
                cost_price_cents: r.get(4)?,
                stock: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(items)
}

/// Registra um recebimento. Se zerar o saldo, finaliza a mesa na MESMA transacao.
#[allow(clippy::too_many_arguments)]
pub fn registrar_pagamento_mesa(
    conn: &Connection,
    id_mesa: i64,
    forma: &str,
    valor_tendered: i64,
    operator: &str,
    aplicar_acrescimo: bool,
    turno_id: i64,
) -> AppResult<PagamentoMesaResult> {
    let forma = normalize_payment(forma)?;
    if valor_tendered <= 0 {
        return Err(AppError::InvalidInput(
            "Informe um valor de pagamento maior que zero.".into(),
        ));
    }

    let tx = conn.unchecked_transaction()?;
    let id_sessao = open_session_id(&tx, id_mesa)?;
    let total = mesa_total_cents(&tx, id_mesa)?;
    if total <= 0 {
        return Err(AppError::InvalidInput(
            "Adicione produtos antes de receber pagamento.".into(),
        ));
    }

    let saldo_before = (total - pago_cents(&tx, id_sessao)?).max(0);
    if saldo_before == 0 {
        return Err(AppError::InvalidInput("Esta conta ja esta quitada.".into()));
    }
    if forma != "dinheiro" && valor_tendered > saldo_before {
        return Err(AppError::InvalidInput(
            "O valor excede o saldo devedor da mesa.".into(),
        ));
    }

    let aplicado = valor_tendered.min(saldo_before);
    let troco = (valor_tendered - aplicado).max(0); // so dinheiro pode gerar troco
    let surcharge = if forma == "credito" && aplicar_acrescimo {
        (aplicado * CREDIT_SURCHARGE_BPS + 5_000) / 10_000 // 5% arredondado
    } else {
        0
    };
    let now = now_millis();

    tx.execute(
        "INSERT INTO sale_payments
            (id_mesa, id_sessao, sale_id, forma_pagamento, valor_cents, troco_cents, surcharge_cents, operator_name, created_at, turno_operacional_id)
         VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id_mesa, id_sessao, forma, aplicado, troco, surcharge, operator, now, turno_id],
    )?;

    let saldo_after = saldo_before - aplicado;
    if saldo_after > 0 {
        tx.commit()?;
        return Ok(PagamentoMesaResult {
            finalizada: false,
            saldo_restante_cents: saldo_after,
            troco_cents: troco,
            ticket: None,
        });
    }

    let ticket = finalizar_conta(&tx, id_mesa, id_sessao, operator)?;
    tx.commit()?;
    Ok(PagamentoMesaResult {
        finalizada: true,
        saldo_restante_cents: 0,
        troco_cents: troco,
        ticket: Some(ticket),
    })
}

/// Fechamento ATOMICO: cria venda + baixa estoque + vincula pagamentos + limpa mesa.
fn finalizar_conta(
    tx: &Connection,
    id_mesa: i64,
    id_sessao: i64,
    operator: &str,
) -> AppResult<TicketData> {
    let (numero_mesa, nome_cliente, tempo_inicio, id_unico): (i64, Option<String>, i64, String) = tx
        .query_row(
            "SELECT m.numero, s.nome_cliente, s.tempo_inicio, s.id_unico
             FROM mesa_sessao s JOIN mesas m ON m.id = s.id_mesa
             WHERE s.id = ?1",
            params![id_sessao],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;

    let items = mesa_items(tx, id_mesa)?;
    if items.is_empty() {
        return Err(AppError::InvalidInput(
            "Mesa sem produtos para finalizar.".into(),
        ));
    }
    let subtotal: i64 = items.iter().map(|i| i.price_cents * i.quantidade).sum();

    // agrega formas + acrescimo a partir do razao
    let (surcharge_total, payment_method) = {
        let mut stmt = tx.prepare(
            "SELECT forma_pagamento, surcharge_cents FROM sale_payments
             WHERE id_sessao = ?1 AND sale_id IS NULL",
        )?;
        let rows = stmt
            .query_map(params![id_sessao], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let surcharge_total: i64 = rows.iter().map(|(_, s)| *s).sum();
        let mut formas: Vec<String> = rows.into_iter().map(|(f, _)| f).collect();
        formas.sort();
        formas.dedup();
        let method = if formas.len() == 1 {
            formas.remove(0)
        } else {
            "multiplas".to_string()
        };
        (surcharge_total, method)
    };

    let total = subtotal + surcharge_total;
    let estimated_profit: i64 = items
        .iter()
        .map(|i| (i.price_cents - i.cost_price_cents) * i.quantidade)
        .sum();
    let now = now_millis();

    tx.execute(
        "INSERT INTO sales
            (mesa_numero, sale_type, operator_name, subtotal_cents, discount_cents,
             surcharge_cents, total_cents, estimated_profit_cents, payment_method, created_at, nfe_status)
         VALUES (?1, 'mesa', ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, 'placeholder')",
        params![
            numero_mesa,
            operator,
            subtotal,
            surcharge_total,
            total,
            estimated_profit,
            payment_method,
            now
        ],
    )?;
    let sale_id = tx.last_insert_rowid();

    let mut produtos = Vec::with_capacity(items.len());
    for i in &items {
        let sub = i.price_cents * i.quantidade;
        tx.execute(
            "INSERT INTO sale_items
                (sale_id, product_id, product_name, quantity, unit_price_cents, cost_price_cents, subtotal_cents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![sale_id, i.id_produto, i.name, i.quantidade, i.price_cents, i.cost_price_cents, sub],
        )?;
        let new_stock = i.stock - i.quantidade;
        tx.execute(
            "UPDATE products SET stock = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_stock, now, i.id_produto],
        )?;
        tx.execute(
            "INSERT INTO stock_movements
                (product_id, movement_type, quantity, previous_stock, new_stock, operator_name, note, created_at)
             VALUES (?1, 'venda', ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                i.id_produto,
                -i.quantidade,
                i.stock,
                new_stock,
                operator,
                if new_stock < 0 { Some("Estoque negativo apos venda") } else { None },
                now
            ],
        )?;
        produtos.push(TicketProduto {
            nome: i.name.clone(),
            quantidade: i.quantidade,
            preco_unit_cents: i.price_cents,
            subtotal_cents: sub,
        });
    }

    tx.execute(
        "UPDATE sale_payments SET sale_id = ?1 WHERE id_sessao = ?2 AND sale_id IS NULL",
        params![sale_id, id_sessao],
    )?;
    tx.execute(
        "UPDATE mesa_sessao SET tempo_fim = ?1, forma_pagamento = ?2, valor_total_cents = ?3 WHERE id = ?4",
        params![now, payment_method, total, id_sessao],
    )?;
    tx.execute(
        "DELETE FROM mesa_produtos WHERE id_mesa = ?1",
        params![id_mesa],
    )?;

    let tempo = format_duration(now - tempo_inicio);
    let produtos_json = serde_json::to_string(&produtos).ok();
    tx.execute(
        "INSERT INTO logs
            (tipo, numero_mesa, nome_cliente, valor_total_cents, forma_pagamento,
             tempo_permanencia, lista_produtos_json, data_hora, id_mesa_unico)
         VALUES ('mesa_fechada', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            numero_mesa,
            nome_cliente,
            total,
            payment_method,
            tempo,
            produtos_json,
            now,
            id_unico
        ],
    )?;

    Ok(TicketData {
        numero_mesa,
        nome_cliente,
        tempo_permanencia: tempo,
        id_unico,
        forma_pagamento: payment_method,
        subtotal_cents: subtotal,
        acrescimo_cents: surcharge_total,
        total_cents: total,
        valor_pago_cents: Some(total),
        troco_cents: Some(0),
        produtos,
    })
}
