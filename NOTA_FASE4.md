# Portex PDV — FASE 4 (completa)

## Rodar (teste)
```
cd src-tauri && cargo check      # valida o Rust
cd .. && npm install             # deps (inclui recharts, novo)
npm run tauri dev                # sobe o app
```
> Gerado sem `cargo`/rede no ambiente de origem: trate como pronto-para-build. Rode os comandos
> acima e, se houver erro de compilação, me mande a saída.

## Entregue
- **Pagamento parcial de mesas** — livro-razão `sale_payments`; parciais abatem o saldo e a mesa
  segue aberta até zerar; fechamento atômico (venda + estoque + limpa mesa em 1 transação).
- **Total da mesa** no payload raso (cards exibem valor sem abrir; lazy loading mantido).
- **Validade de produtos** — campo no cadastro + toast no boot p/ itens vencendo em 7 dias.
- **Validação de usuário** (Rust + React) — só letras/números, usuário ≤20, senha 4–30.
- **Caixa** — lista com rolagem (`max-height:60vh; overflow-y:auto`).
- **Relatórios** — gráfico de barras (recharts) das vendas dos últimos 7 dias.
- **Impressora** — WPC1252 (`ESC t 16`) + `encoding_rs` (corrige acentos/mojibake).
- **Distribuição** — NSIS aplicado no `tauri.conf.json` (perMachine, pt-BR).
- **Gaveta** — `enrich_cash_register` corrigido p/ pagamentos mistos.
- `reset_sales` agora também limpa `sale_payments`.

## Updater (estrutura pronta — ativar quando tiver servidor + chaves)
1. `npm run tauri signer generate -- -w ~/.tauri/portex.key` (guarde a privada; copie a pública).
2. `cargo add tauri-plugin-updater` e em `lib.rs`:
   `.plugin(tauri_plugin_updater::Builder::new().build())`.
3. `npm i @tauri-apps/plugin-updater @tauri-apps/plugin-process`.
4. Em `src-tauri/capabilities/default.json` adicione `"updater:default"`.
5. Em `tauri.conf.json`, dentro do objeto raiz, adicione:
   ```json
   "plugins": {
     "updater": {
       "pubkey": "SUA_CHAVE_PUBLICA",
       "endpoints": ["https://SEU-SERVIDOR/portex/{{target}}/{{arch}}/{{current_version}}"]
     }
   }
   ```
6. No build: setar `TAURI_SIGNING_PRIVATE_KEY` (e senha) e publicar `latest.json` + binários assinados.

## Ainda pendente (fora da FASE 4)
- **FASE 1**: Argon2 + sessão real (substituir hash FNV e o RBAC client-side). Bloqueante p/ produção.
