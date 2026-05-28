# Guia Elgin i8

## Configuracao recomendada

- Papel: 80mm.
- Largura do ticket no app: 48 caracteres.
- Modo de envio: RAW ESC/POS.
- Driver: driver oficial Elgin instalado no Windows.

## Fluxo de impressao

O aplicativo gera um ticket com:

1. Nome da empresa centralizado.
2. CPF/CNPJ.
3. Nome do produto.
4. Valor em reais.
5. Validade calculada em dias.
6. Frase de agradecimento quando configurada.

O backend envia o payload ESC/POS diretamente para a impressora selecionada ou para a impressora padrao do Windows.

## Solucao de problemas

- Se a impressora nao aparecer, reinstale o driver e clique em **Atualizar** na tela de configuracoes.
- Se o ticket sair desalinhado, ajuste a largura entre 42 e 48 caracteres.
- Se caracteres acentuados sairem incorretos, configure a pagina de codigo no utilitario da Elgin ou evite acentos nos campos impressos.
- Se nada imprimir, verifique se a Elgin i8 imprime uma pagina de teste pelo Windows.
- Se o Windows exibir erro de spooler, reinicie o servico "Spooler de Impressao".
