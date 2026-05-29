use crate::error::{AppError, AppResult};
use crate::models::{AppConfig, IssuedTicket, PrintResult, PrinterInfo, Product, TicketData};
use chrono::{Duration, Local};
use serde::Deserialize;

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

pub fn print_tickets(
    config: &AppConfig,
    product: &Product,
    issued_tickets: &[IssuedTicket],
) -> AppResult<PrintResult> {
    if issued_tickets.is_empty() || issued_tickets.len() > 999 {
        return Err(AppError::InvalidInput(
            "A quantidade deve ficar entre 1 e 999 tickets.".to_string(),
        ));
    }

    let printer_name = match config.printer_name.as_ref().filter(|value| !value.is_empty()) {
        Some(name) => name.clone(),
        None => default_printer_name()?.ok_or_else(|| {
            AppError::Printer(
                "Nenhuma impressora configurada. Selecione uma impressora nas configuracoes."
                    .to_string(),
            )
        })?,
    };

    let mut payload = Vec::new();
    for ticket in issued_tickets {
        payload.extend(build_ticket_payload(config, product, ticket));
    }

    send_raw_to_printer(&printer_name, &payload)?;

    Ok(PrintResult {
        printed: issued_tickets.len() as i64,
        printer_name,
        ticket_ids: issued_tickets
            .iter()
            .map(|ticket| ticket.ticket_id.clone())
            .collect(),
    })
}

pub fn list_printers() -> AppResult<Vec<PrinterInfo>> {
    platform_list_printers()
}

pub fn print_pdv_ticket(config: &AppConfig, ticket: &TicketData) -> AppResult<()> {
    let printer_name = match config.printer_name.as_ref().filter(|value| !value.is_empty()) {
        Some(name) => name.clone(),
        None => default_printer_name()?.ok_or_else(|| {
            AppError::Printer(
                "Nenhuma impressora configurada. Selecione uma impressora nas configuracoes."
                    .to_string(),
            )
        })?,
    };
    let payload = build_pdv_ticket_payload(config, ticket);
    send_raw_to_printer(&printer_name, &payload)
}

fn build_ticket_payload(config: &AppConfig, product: &Product, ticket: &IssuedTicket) -> Vec<u8> {
    let width = config.print_width_chars.clamp(32, 64) as usize;
    let validity_label = timestamp_to_label(ticket.expires_at);

    let mut bytes = Vec::new();
    bytes.extend([ESC, b'@']);
    bytes.extend([ESC, b'a', 1]);
    bytes.extend([ESC, b'E', 1]);
    push_line(&mut bytes, &config.company_name);
    bytes.extend([ESC, b'E', 0]);
    push_line(&mut bytes, &config.tax_id);
    push_line(&mut bytes, "");

    bytes.extend([ESC, b'a', 0]);
    push_line(&mut bytes, &"-".repeat(width));
    push_wrapped_line(&mut bytes, &format!("Produto: {}", product.name), width);
    push_line(&mut bytes, &format!("Valor: {}", format_currency(product.price_cents)));
    push_line(&mut bytes, &format!("Validade: {}", validity_label));
    push_wrapped_line(&mut bytes, &format!("ID: {}", ticket.ticket_id), width);
    push_line(&mut bytes, &"-".repeat(width));

    if let Some(message) = &config.thank_you_message {
        bytes.extend([ESC, b'a', 1]);
        push_wrapped_line(&mut bytes, message, width);
    }

    push_line(&mut bytes, "");
    push_line(&mut bytes, "");
    bytes.extend([GS, b'V', 66, 0]);
    bytes
}

fn build_pdv_ticket_payload(config: &AppConfig, ticket: &TicketData) -> Vec<u8> {
    let width = config.print_width_chars.clamp(32, 64) as usize;
    let mut bytes = Vec::new();

    bytes.extend([ESC, b'@']);
    bytes.extend([ESC, b'a', 1]);
    bytes.extend([ESC, b'E', 1]);
    push_line(&mut bytes, &config.company_name);
    bytes.extend([ESC, b'E', 0]);
    push_line(&mut bytes, &config.tax_id);
    push_line(&mut bytes, "");
    push_line(&mut bytes, &format!("Mesa {:02}", ticket.numero_mesa));
    push_line(&mut bytes, &format!("ID: {}", ticket.id_unico));

    if let Some(cliente) = &ticket.nome_cliente {
        push_wrapped_line(&mut bytes, &format!("Cliente: {cliente}"), width);
    }

    push_line(&mut bytes, &format!("Tempo: {}", ticket.tempo_permanencia));
    push_line(&mut bytes, "");
    bytes.extend([ESC, b'a', 0]);
    push_line(&mut bytes, &"-".repeat(width));

    for item in &ticket.produtos {
        push_wrapped_line(&mut bytes, &item.nome, width);
        push_line(
            &mut bytes,
            &format!(
                "x{}  {}  {}",
                item.quantidade,
                format_currency(item.preco_unit_cents),
                format_currency(item.subtotal_cents)
            ),
        );
    }

    push_line(&mut bytes, &"-".repeat(width));
    push_line(&mut bytes, &format!("Subtotal: {}", format_currency(ticket.subtotal_cents)));

    if ticket.acrescimo_cents > 0 {
        push_line(
            &mut bytes,
            &format!("Acrescimo: {}", format_currency(ticket.acrescimo_cents)),
        );
    }

    bytes.extend([ESC, b'E', 1]);
    push_line(&mut bytes, &format!("TOTAL: {}", format_currency(ticket.total_cents)));
    bytes.extend([ESC, b'E', 0]);
    push_line(&mut bytes, &format!("Pagamento: {}", payment_label(&ticket.forma_pagamento)));

    if let Some(valor_pago) = ticket.valor_pago_cents {
        push_line(&mut bytes, &format!("Valor pago: {}", format_currency(valor_pago)));
    }

    if let Some(troco) = ticket.troco_cents {
        push_line(&mut bytes, &format!("Troco: {}", format_currency(troco)));
    }

    if let Some(message) = &config.thank_you_message {
        push_line(&mut bytes, "");
        bytes.extend([ESC, b'a', 1]);
        push_wrapped_line(&mut bytes, message, width);
    }

    push_line(&mut bytes, "");
    push_line(&mut bytes, "");
    bytes.extend([GS, b'V', 66, 0]);
    bytes
}

fn payment_label(value: &str) -> &'static str {
    match value {
        "pix" => "PIX",
        "dinheiro" => "Dinheiro",
        "debito" => "Debito",
        "credito" => "Credito",
        _ => "Pagamento",
    }
}

fn timestamp_to_label(timestamp_millis: i64) -> String {
    let seconds = timestamp_millis.div_euclid(1000);
    match chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0) {
        Some(date_time) => date_time
            .with_timezone(&Local)
            .format("%d/%m/%Y")
            .to_string(),
        None => (Local::now() + Duration::days(0))
            .format("%d/%m/%Y")
            .to_string(),
    }
}

fn push_wrapped_line(bytes: &mut Vec<u8>, value: &str, width: usize) {
    let mut current = String::new();

    for word in value.split_whitespace() {
        let next_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };

        if next_len > width && !current.is_empty() {
            push_line(bytes, &current);
            current.clear();
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        push_line(bytes, &current);
    }
}

fn push_line(bytes: &mut Vec<u8>, value: &str) {
    // A Elgin i8 aceita ESC/POS em modo RAW; caracteres fora de ASCII dependem da pagina de codigo da impressora.
    bytes.extend(value.as_bytes());
    bytes.push(b'\n');
}

fn format_currency(price_cents: i64) -> String {
    let reais = price_cents / 100;
    let cents = price_cents.abs() % 100;
    format!("R$ {reais},{cents:02}")
}

#[cfg(target_os = "windows")]
fn platform_list_printers() -> AppResult<Vec<PrinterInfo>> {
    use std::os::windows::process::CommandExt;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct WindowsPrinter {
        name: String,
        default: bool,
    }

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = "Get-CimInstance Win32_Printer | Select-Object Name, Default | ConvertTo-Json -Compress";
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    if !output.status.success() {
        return Err(AppError::Printer(
            "Nao foi possivel consultar as impressoras do Windows.".to_string(),
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|error| AppError::Printer(error.to_string()))?;

    let printers = if value.is_array() {
        serde_json::from_value::<Vec<WindowsPrinter>>(value)
            .map_err(|error| AppError::Printer(error.to_string()))?
    } else {
        vec![serde_json::from_value::<WindowsPrinter>(value)
            .map_err(|error| AppError::Printer(error.to_string()))?]
    };

    Ok(printers
        .into_iter()
        .map(|printer| PrinterInfo {
            name: printer.name,
            is_default: printer.default,
        })
        .collect())
}

#[cfg(not(target_os = "windows"))]
fn platform_list_printers() -> AppResult<Vec<PrinterInfo>> {
    Ok(Vec::new())
}

#[cfg(target_os = "windows")]
fn default_printer_name() -> AppResult<Option<String>> {
    Ok(platform_list_printers()?
        .into_iter()
        .find(|printer| printer.is_default)
        .map(|printer| printer.name))
}

#[cfg(not(target_os = "windows"))]
fn default_printer_name() -> AppResult<Option<String>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn send_raw_to_printer(printer_name: &str, payload: &[u8]) -> AppResult<()> {
    use std::ffi::c_void;

    type Handle = *mut c_void;

    #[repr(C)]
    struct DocInfo1W {
        p_doc_name: *mut u16,
        p_output_file: *mut u16,
        p_datatype: *mut u16,
    }

    #[link(name = "Winspool")]
    extern "system" {
        fn OpenPrinterW(
            p_printer_name: *mut u16,
            ph_printer: *mut Handle,
            p_default: *mut c_void,
        ) -> i32;
        fn ClosePrinter(h_printer: Handle) -> i32;
        fn StartDocPrinterW(h_printer: Handle, level: u32, p_doc_info: *const DocInfo1W) -> u32;
        fn EndDocPrinter(h_printer: Handle) -> i32;
        fn StartPagePrinter(h_printer: Handle) -> i32;
        fn EndPagePrinter(h_printer: Handle) -> i32;
        fn WritePrinter(
            h_printer: Handle,
            p_buf: *const c_void,
            cb_buf: u32,
            pc_written: *mut u32,
        ) -> i32;
    }

    let mut handle: Handle = std::ptr::null_mut();
    let mut printer_name_w = to_wide(printer_name);

    unsafe {
        if OpenPrinterW(printer_name_w.as_mut_ptr(), &mut handle, std::ptr::null_mut()) == 0 {
            return Err(AppError::Printer(format!(
                "Nao foi possivel abrir a impressora '{printer_name}'."
            )));
        }

        let mut doc_name = to_wide("Sistema de Tickets GPC");
        let mut data_type = to_wide("RAW");
        let doc_info = DocInfo1W {
            p_doc_name: doc_name.as_mut_ptr(),
            p_output_file: std::ptr::null_mut(),
            p_datatype: data_type.as_mut_ptr(),
        };

        let job_id = StartDocPrinterW(handle, 1, &doc_info);
        if job_id == 0 {
            ClosePrinter(handle);
            return Err(AppError::Printer("Nao foi possivel iniciar o job de impressao.".to_string()));
        }

        if StartPagePrinter(handle) == 0 {
            EndDocPrinter(handle);
            ClosePrinter(handle);
            return Err(AppError::Printer("Nao foi possivel iniciar a pagina de impressao.".to_string()));
        }

        let mut written = 0u32;
        let success = WritePrinter(
            handle,
            payload.as_ptr() as *const c_void,
            payload.len() as u32,
            &mut written,
        );

        EndPagePrinter(handle);
        EndDocPrinter(handle);
        ClosePrinter(handle);

        if success == 0 || written != payload.len() as u32 {
            return Err(AppError::Printer(
                "O Windows nao confirmou o envio completo do ticket para a impressora.".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn send_raw_to_printer(_printer_name: &str, _payload: &[u8]) -> AppResult<()> {
    Err(AppError::Printer(
        "A impressao termica RAW esta disponivel apenas no Windows.".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
