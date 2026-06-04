# Portex PDV

Aplicativo desktop profissional para Windows, desenvolvido como um PDV básico para controle de mesas, cadastro de produtos, fechamento de vendas, impressão e validação de tickets térmicos para comércios.

O sistema foi criado com foco em desempenho, organização, segurança local e facilidade de uso. Ele permite que pequenos negócios controlem até 100 mesas, adicionem produtos ao consumo, fechem pagamentos por PIX, dinheiro, débito ou crédito, controlem caixa e estoque, imprimam tickets térmicos compatíveis com a impressora **Elgin i8** e mantenham logs locais de auditoria.

## Funcionalidades

- Login local com perfis Admin e Operador/Caixa
- Caixa unificado por **turno operacional**: o turno é a fonte única da gaveta
- Abertura de turno com **fundo de troco** e fechamento com contagem da gaveta
- Fechamento em cascata: **Turno → Período Contábil** (consolidar e bloquear o dia)
- Dia fiscal deslocado configurável (ex.: bar que vira a madrugada fecha às 06:00)
- Registro de sangria e suprimento vinculados ao turno
- Pagamento parcial de mesa, com o dinheiro atribuído ao turno em que foi recebido
- Guia rápido em tela explicando todo o ciclo do caixa para o usuário final
- Cadastro de produtos
- Edição de produtos
- Exclusão de produtos
- Categorias de produtos
- Código de barras, custo, unidade e estoque por produto
- Cálculo de preço por custo + markup
- Entrada, saída e ajuste manual de estoque
- Alerta de estoque baixo e estoque negativo
- Pesquisa de produtos em tempo real
- Dashboard com lista moderna de produtos
- Dashboard PDV com quantidade configurável de mesas
- Modal de mesa com produtos adicionados e catálogo
- Cronômetro de permanência por mesa
- Checkout com PIX, dinheiro, débito e crédito
- Cálculo de troco para dinheiro
- Acréscimo automático de 5% para crédito
- Relatórios de vendas, lucro estimado, produtos mais vendidos e estoque baixo
- Logs de tickets, mesas fechadas, categorias e produtos
- Exportação CSV de logs
- Backup manual e backup automático enquanto o app estiver aberto
- Impressão térmica compatível com Elgin i8
- Configuração dos dados da empresa
- Tema claro e tema escuro
- Seleção de impressora instalada no Windows
- Configuração de largura de impressão
- Onboarding na primeira execução
- Geração automática de ID único para cada ticket
- Verificação de autenticidade do ticket pelo ID
- Exclusão automática de tickets vencidos
- Banco de dados local com SQLite
- Geração de instalador `.exe` para Windows

## Validação e Segurança

Cada ticket impresso recebe um ID alfanumérico único de 6 caracteres, por exemplo:

```text
A7K92B
```

Esse ID é salvo localmente no banco SQLite junto com a data de validade configurada pelo usuário.

Na tela **Verificar**, o usuário pode digitar o ID impresso no ticket. O sistema consulta o banco local e retorna:

```text
Este ticket é válido e foi impresso usando o Portex PDV
```

ou:

```text
Este ticket é inválido, ou passou da válidade.
```

Tickets vencidos são removidos automaticamente quando o aplicativo inicia, quando novos tickets são impressos ou quando uma verificação é feita.

## Fechamento de Caixa em Cascata (Turno → Período)

O controle de caixa gira em torno do **turno operacional**, que é a fonte única
da gaveta de dinheiro. O fluxo do dia é:

```text
Turno aberto  →  Turno fechado  →  Período consolidado  →  Período bloqueado
 (vendendo)      (gaveta contada)    (dia somado)            (selado p/ auditoria)
```

1. **Abrir o turno** (em **Fechamento**): informe o fundo de troco. Sem turno
   aberto o caixa bloqueia vendas, adição de produtos na mesa e impressão de tickets.
2. **Vender**: na tela **Caixa** (venda direta) e em **Mesas** (comandas).
3. **Sangria/Suprimento**: tirar ou colocar dinheiro na gaveta.
4. **Fechar o turno**: conte o dinheiro físico; o sistema compara com o esperado.
5. **Consolidar o período**: com todos os turnos do dia fechados, fecha o dia.
6. **Bloquear o período** (Admin): sela o dia; nenhuma venda daquele dia pode mais
   ser alterada.

O **esperado na gaveta** de cada turno é calculado assim:

```text
Fundo de troco + Vendas em dinheiro + Suprimentos − Sangrias
```

Cartão e PIX **não** entram na gaveta de dinheiro. O total do período reflete a
conferência da **gaveta (dinheiro)**, não o faturamento — a receita total
(incluindo cartão/PIX) fica em **Relatórios**.

**Dinheiro de mesa entre turnos:** o dinheiro de uma comanda entra na gaveta no
**instante do pagamento**, não no fechamento da conta. Cada pagamento carimba o
turno ativo (`sale_payments.turno_operacional_id`), de modo que um pagamento
parcial recebido em um turno e a quitação em outro são atribuídos corretamente a
cada operador — sem sobras nem faltas "fantasma" na virada de turno.

## Tecnologias Utilizadas

- **Tauri**: empacotamento desktop nativo e leve para Windows
- **React**: construção da interface
- **TypeScript**: tipagem e organização do frontend
- **Rust**: backend nativo, persistência e impressão
- **SQLite**: banco de dados local
- **ESC/POS**: comandos de impressão térmica
- **Vite**: build e desenvolvimento frontend

## Por Que Tauri?

O Tauri foi escolhido por ser mais leve que Electron, consumir menos memória e permitir integração nativa com recursos do Windows usando Rust. Isso torna o aplicativo mais rápido, profissional e adequado para distribuição como `.exe`.

## Estrutura do Projeto

```text
src/
  components/      Componentes reutilizáveis da interface
  database/        Contratos e valores padrão do frontend
  hooks/           Hooks React
  pages/           Telas principais do aplicativo
  services/        Comunicação entre frontend e backend Tauri
  styles/          Estilos globais e temas
  types/           Tipos TypeScript
  utils/           Utilitários de validação e formatação

src-tauri/
  src/
    commands.rs    Comandos expostos ao frontend
    database.rs    SQLite, migrações e regras de persistência
    payments.rs    Pagamento de mesa, razão sale_payments e fechamento atômico
    error.rs       Tratamento de erros
    lib.rs         Inicialização do Tauri
    models.rs      Modelos do backend
    printer.rs     Impressão térmica ESC/POS
```

Telas principais relacionadas ao caixa:

```text
src/pages/CashRegisterPage.tsx   Caixa (somente venda direta + sangria/suprimento)
src/pages/FecharCaixa.tsx        Abrir/fechar turno, consolidar e bloquear período
src/pages/GuiaCaixaPage.tsx      Guia rápido em tela do ciclo do caixa
src/services/cashierService.ts   Comunicação do fluxo de turno/período
```

## Instalação Para Desenvolvimento

### Pré-requisitos

Instale:

- [Node.js LTS](https://nodejs.org/)
- [Rust](https://rustup.rs/)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

No Visual Studio Build Tools, selecione:

```text
Desktop development with C++
```

Depois, confirme no PowerShell:

```powershell
node --version
npm --version
cargo --version
rustc --version
```

## Instalar Dependências

Na pasta do projeto:

```powershell
npm install
```

## Executar Em Desenvolvimento

```powershell
npm run dev
```

O aplicativo será aberto em uma janela desktop do Tauri.

Na primeira instalação com banco novo, o usuário local padrão é:

```text
Usuário: admin
Senha: admin
```

Depois de entrar, crie usuários definitivos em **Usuários**.

## Gerar Instalador .exe

```powershell
npm run build
```

O instalador será gerado em:

```text
src-tauri/target/release/bundle/nsis/
```

O executável direto será gerado em:

```text
src-tauri/target/release/portex_pdv.exe
```

Também é possível usar o arquivo:

```text
GERAR_EXE.bat
```

Basta dar dois cliques nele para iniciar o processo de build.

## Instalação Para Usuário Final

1. Baixe o instalador `.exe`.
2. Execute o arquivo:

```text
Portex PDV_1.0.0_x64-setup.exe
```

3. Siga as etapas do instalador.
4. Abra o aplicativo pelo Menu Iniciar ou pelo atalho criado no Windows.

O usuário final não precisa instalar Node.js, Rust ou abrir PowerShell.

## Backup Automático

O aplicativo possui backup manual em **Relatórios** e backup automático interno enquanto o Portex PDV estiver aberto.

Para criar um backup diário mesmo com o aplicativo fechado, execute:

```text
CRIAR_BACKUP_AGENDADO.bat
```

Por padrão, a tarefa roda às 23:00. Para escolher outro horário, execute pelo Prompt:

```bat
CRIAR_BACKUP_AGENDADO.bat 21:30
```

Os arquivos serão salvos em:

```text
Downloads\portex-pdv-backups
```

## Configurar Impressora Elgin i8

1. Instale o driver oficial da Elgin i8 no Windows.
2. Conecte a impressora por USB ou rede.
3. Abra o Portex PDV.
4. Vá em **Configurações**.
5. Na área **Impressora**, clique em **Atualizar**.
6. Selecione a Elgin i8.
7. Mantenha a largura em 48 caracteres para papel 80mm.
8. Salve as configurações.
9. Imprima um ticket de teste.

## Banco de Dados Local

O aplicativo cria automaticamente um banco SQLite local com as tabelas:

- `app_config`: configurações da empresa, tema, impressora, validade, mesas e backup
- `products`: produtos cadastrados, estoque, custo, categoria e código de barras
- `categories`: categorias dos produtos
- `issued_tickets`: tickets emitidos e validade dos IDs
- `mesas`: cadastro local das mesas
- `mesa_produtos`: produtos adicionados em cada mesa
- `mesa_sessao`: sessão ativa/fechada da mesa, cliente, pagamento e ID único
- `logs`: auditoria de tickets, mesas fechadas e produtos criados
- `cash_registers`: sessões de caixa (histórico legado, mantido para auditoria)
- `cash_movements`: sangrias e suprimentos (vinculados ao turno)
- `stock_movements`: histórico de ajustes de estoque
- `sales` e `sale_items`: vendas e itens vendidos
- `sale_payments`: razão de pagamentos de mesa (carimbado com o turno do pagamento)
- `turnos_operacionais`: turnos de caixa, fundo de troco, esperado e diferença
- `periodos_contabeis`: consolidação diária (período contábil) e bloqueio
- `sale_audit`: auditoria de edições de venda
- `users`: usuários locais e perfis

## Autor

Criado por **Gabriel Portela Carmo**.

Portfolio: [https://lark69.github.io/Gabriel-Portela-Portfolio/](https://lark69.github.io/Gabriel-Portela-Portfolio/)
