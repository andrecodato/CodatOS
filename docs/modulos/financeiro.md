# Módulo · Financeiro

> **Prefixo:** `RF-FIN` · **Marco principal:** M3 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [Segurança](../04-seguranca.md) · [Modelo de dados](../05-modelo-de-dados.md#banco-local-sqlite)

## Objetivo

Controle financeiro pessoal simples e bonito: para onde vai o dinheiro, quanto sobrou, o que vence. Os dados financeiros são **dados**, não configuração — por isso ficam **no seu dispositivo** e nunca são armazenados pelo servidor em texto claro (ver [Regras de negócio](#regras-de-negócio)).

## Telas (mockup v0)

Abas **Visão geral · Transações · Categorias · Metas** e seletor de mês.

- Cartões: Receitas, Despesas e Saldo do mês, com variação vs. mês anterior.
- **Fluxo de caixa**: barras diárias de receitas (verde) e despesas (vermelho).
- **Categorias de despesas**: gráfico de rosca (Moradia, Alimentação, Transporte, Lazer, Saúde, Outros).
- **Últimas transações**: data, descrição, categoria, valor; "Ver todas".

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-FIN-001 | Contas: corrente, poupança, cartão de crédito, dinheiro, investimento (só saldo), com saldo inicial, moeda, cor e ícone. | Deve | M3 |
| RF-FIN-002 | Transações: receita, despesa e transferência entre contas; data, descrição, valor, categoria, conta, tags, observação. | Deve | M3 |
| RF-FIN-003 | Recorrências (mensal, semanal, anual) e parcelamentos (ex.: 3/10). | Deve | M3 |
| RF-FIN-004 | Importação de extratos **OFX e CSV** com mapeamento de colunas, pré-visualização e deduplicação. | Deve | M3 |
| RF-FIN-005 | Regras de categorização automática ("descrição contém UBER → Transporte"), aprendidas a partir de correções. | Deveria | M3 |
| RF-FIN-006 | Visão geral do mês conforme o mockup (cartões, fluxo de caixa, rosca por categoria, últimas transações). | Deve | M3 |
| RF-FIN-007 | Categorias e subcategorias editáveis, com cor e ícone; conjunto padrão em pt-BR. | Deve | M3 |
| RF-FIN-008 | Metas: orçamento mensal por categoria e metas de economia, com barra de progresso e alerta ao ultrapassar. | Deveria | M3 |
| RF-FIN-009 | Cartão de crédito: faturas por data de fechamento/vencimento, limite disponível. | Deveria | M3 |
| RF-FIN-010 | Contas a pagar/receber com lembretes (notificação X dias antes). | Deveria | M3 |
| RF-FIN-011 | **Modo privacidade**: ocultar valores com um atalho (útil no celular e ao compartilhar a tela). | Deve | M3 |
| RF-FIN-012 | Widgets do dashboard: Saldo total (com gráfico), Fluxo do mês, Próximos vencimentos. | Deve | M3 |
| RF-FIN-013 | Busca e filtros de transações (período, conta, categoria, valor, tag); exportação CSV. | Deve | M3 |
| RF-FIN-014 | Relatórios: evolução mensal (12 meses), comparativo por categoria. | Poderia | Futuro |
| RF-FIN-015 | Multimoeda com conversão por cotação diária. | Poderia | Futuro |
| RF-FIN-016 | Open Finance (agregadores como Pluggy/Belvo) para importar automaticamente. | Não agora | Futuro |

## Regras de negócio

- **Armazenamento**: banco local (SQLite cifrado — ver [Segurança](../04-seguranca.md#7-segurança-local)). O servidor nunca recebe dados financeiros em claro.
- **Acesso remoto**: o celular vê o financeiro **lendo do PC** pelo relay, sem passar pela nuvem como armazenamento.
- **Sincronização entre dispositivos**: desligada por padrão. Se habilitada no futuro, seria cifrada de ponta a ponta como qualquer item do cofre — decisão pendente na [Q-07](../10-perguntas-em-aberto.md#financeiro).
- Valores monetários armazenados como inteiros em centavos (sem ponto flutuante).
- Tenant: cada tenant tem suas próprias contas e transações (ex.: "Pessoal" × "Empresa").

## Critérios de aceite

- **Dado** um arquivo OFX exportado do banco, **quando** é importado duas vezes, **então** nenhuma transação é duplicada.
- **Dado** o modo privacidade ativado, **quando** qualquer tela ou widget financeiro é exibido, **então** os valores aparecem como `R$ •••••`.
- **Dado** transações de abril, **quando** a visão geral de abril é aberta, **então** Receitas − Despesas = Saldo do mês e a rosca soma 100%.

## Perguntas em aberto

- De onde vêm os dados (manual, OFX/CSV, Open Finance, app atual)? → [Q-06](../10-perguntas-em-aberto.md#financeiro)
- Os dados financeiros podem sincronizar (cifrados) ou ficam só no PC? → [Q-07](../10-perguntas-em-aberto.md#financeiro)
- Cartão de crédito com faturas, investimentos, multimoeda? → [Q-08](../10-perguntas-em-aberto.md#financeiro)
