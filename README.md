# Sistema de Tickets GPC

Aplicativo desktop profissional para Windows, desenvolvido como um PDV básico para controle de mesas, cadastro de produtos, fechamento de vendas, impressão e validação de tickets térmicos para comércios.

O sistema foi criado com foco em desempenho, organização, segurança local e facilidade de uso. Ele permite que pequenos negócios controlem até 40 mesas, adicionem produtos ao consumo, fechem pagamentos por PIX, dinheiro, débito ou crédito, imprimam tickets térmicos compatíveis com a impressora **Elgin i8** e mantenham logs locais de auditoria.

## Funcionalidades

- Cadastro de produtos
- Edição de produtos
- Exclusão de produtos
- Pesquisa de produtos em tempo real
- Dashboard com lista moderna de produtos
- Dashboard PDV com 40 mesas
- Modal de mesa com produtos adicionados e catálogo
- Cronômetro de permanência por mesa
- Checkout com PIX, dinheiro, débito e crédito
- Cálculo de troco para dinheiro
- Acréscimo automático de 5% para crédito
- Logs de tickets, mesas fechadas e produtos criados
- Exportação CSV de logs
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
Este ticket é válido e foi impresso usano o Sistema de Tickets GPC
```

ou:

```text
Este ticket é inválido, ou passou da válidade.
```

Tickets vencidos são removidos automaticamente quando o aplicativo inicia, quando novos tickets são impressos ou quando uma verificação é feita.

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
    error.rs       Tratamento de erros
    lib.rs         Inicialização do Tauri
    models.rs      Modelos do backend
    printer.rs     Impressão térmica ESC/POS
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
src-tauri/target/release/sistema_tickets_gpc.exe
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
Sistema de Tickets GPC_1.0.0_x64-setup.exe
```

3. Siga as etapas do instalador.
4. Abra o aplicativo pelo Menu Iniciar ou pelo atalho criado no Windows.

O usuário final não precisa instalar Node.js, Rust ou abrir PowerShell.

## Configurar Impressora Elgin i8

1. Instale o driver oficial da Elgin i8 no Windows.
2. Conecte a impressora por USB ou rede.
3. Abra o Sistema de Tickets GPC.
4. Vá em **Configurações**.
5. Na área **Impressora**, clique em **Atualizar**.
6. Selecione a Elgin i8.
7. Mantenha a largura em 48 caracteres para papel 80mm.
8. Salve as configurações.
9. Imprima um ticket de teste.

## Banco de Dados Local

O aplicativo cria automaticamente um banco SQLite local com as tabelas:

- `app_config`: configurações da empresa, tema, impressora e validade
- `products`: produtos cadastrados
- `issued_tickets`: tickets emitidos e validade dos IDs
- `mesas`: cadastro local das 40 mesas
- `mesa_produtos`: produtos adicionados em cada mesa
- `mesa_sessao`: sessão ativa/fechada da mesa, cliente, pagamento e ID único
- `logs`: auditoria de tickets, mesas fechadas e produtos criados

## Autor

Criado por **Gabriel Portela Carmo**.

Portfolio: [https://lark69.github.io/Gabriel-Portela-Portfolio/](https://lark69.github.io/Gabriel-Portela-Portfolio/)
