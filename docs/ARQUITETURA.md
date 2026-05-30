# Arquitetura

O Portex PDV foi organizado em camadas para facilitar manutencao, testes futuros e evolucao.

## Frontend

O frontend usa React com TypeScript e fica em `src/`.

- `pages`: telas completas.
- `components`: blocos reutilizaveis.
- `services`: comunicacao com comandos Tauri.
- `hooks`: estados compartilhados e comportamento de interface.
- `utils`: funcoes puras de formatacao, validacao e erro.
- `styles`: tema, layout e responsividade.

## Backend nativo

O backend fica em `src-tauri/src/` e usa Rust.

- `commands.rs`: API exposta ao frontend.
- `database.rs`: conexao SQLite, migracao e operacoes de dados.
- `printer.rs`: formato ESC/POS e envio ao spooler do Windows.
- `models.rs`: DTOs e modelos serializados.
- `error.rs`: erros tipados retornados ao frontend.

## Persistencia

O SQLite e criado na pasta local do aplicativo. A aplicacao abre conexoes curtas por operacao, reduzindo risco de bloqueio e simplificando concorrencia.

## Impressao

A impressao usa ESC/POS RAW:

1. O frontend envia `productId` e `quantity`.
2. O backend busca produto e configuracao no SQLite.
3. `printer.rs` gera bytes ESC/POS.
4. No Windows, os bytes sao enviados ao spooler via Winspool.

## Evolucao recomendada

- Adicionar testes unitarios para validadores e formatadores.
- Adicionar exportacao/importacao de produtos via CSV.
- Criar tela de historico de impressoes.
- Assinar o instalador Windows para reduzir alertas de seguranca.
