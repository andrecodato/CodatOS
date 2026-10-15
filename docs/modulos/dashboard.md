# Módulo · Dashboard

> **Prefixo:** `RF-DASH` · **Marco principal:** M1 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Fontes de dados: [Sistema](sistema.md) · [Projetos](projetos.md) · [Financeiro](financeiro.md) · [TaskHub](taskhub.md) · [Integrações](integracoes.md) · [Conexões](conexoes.md)

## Objetivo

A primeira tela do dia: um resumo de tudo que importa, montado pelo usuário com widgets arrastáveis. O dashboard não tem dados próprios — ele **consome** os módulos e oferece atalhos para cada um ("Ver todos").

## Telas (mockup v0)

Cabeçalho "Olá, Andre 👋 — Aqui está um resumo do seu dia", busca (Ctrl+K), notificações, data e hora. Layout padrão em duas linhas:

| Linha 1 | Saldo total | Tarefas | Calendário da semana | Tempo |
|---|---|---|---|---|
| **Linha 2** | **Uso do PC** (CPU/RAM/GPU + temperaturas) | **Discos** | **Projetos recentes** | **WSL em execução** + instâncias + Docker no WSL |

## Catálogo de widgets

| Widget | Módulo | Conteúdo | Configurações | Marco |
|---|---|---|---|---|
| Saldo total | FIN | Saldo consolidado, variação no mês, gráfico de área, receitas × despesas | Contas incluídas, período, ocultar valores | M3 |
| Fluxo do mês | FIN | Barras de receitas/despesas por dia | Período | M3 |
| Próximos vencimentos | FIN | Contas a pagar/receber dos próximos dias | Horizonte (dias) | M3 |
| Tarefas | TASK | Total, em andamento, concluídas, barra de progresso, próximas tarefas com vencimento relativo | Projeto, filtro, quantidade | M3 |
| Calendário da semana | INT | Eventos de seg→dom com horário | Calendários, dias exibidos | M3 |
| Tempo | INT | Temperatura, condição, umidade, vento, sensação térmica | Cidade, unidades | M1 |
| Uso do PC | SYS | Anéis de CPU/RAM/GPU com temperaturas | Métricas exibidas, intervalo, °C/°F | M1 |
| Histórico de uso | SYS | Gráfico de linha da última hora | Métricas, janela | M1 |
| Discos | SYS | Volumes com barra de uso, tipo (SSD/HDD), usado/total | Volumes, limite de alerta | M1 |
| Rede | SYS | Upload/download atuais, IP local | Interface | M1 |
| Energia | SYS | Potência estimada (W), bateria | Fontes | M3 |
| WSL | SYS | Distros em execução, tempo ligado, CPU/RAM, tamanho; toggle liga/desliga | Distros exibidas | M1 |
| Docker | SYS | Containers em execução por engine (host e cada distro WSL) | Engines, mostrar parados | M1 |
| Projetos recentes | PROJ | Últimos projetos com último acesso e status git | Quantidade, pastas | M1 |
| Conexões favoritas | CON | Hosts fixados com status online e botão Conectar | Grupo/tag | M2 |
| Túneis ativos | CON | Port forwards ligados | — | M2 |
| Relógio / fusos | APP | Horas em várias cidades | Fusos | M1 |
| Links rápidos | APP | Atalhos para URLs, pastas e comandos | Itens | M1 |
| GitHub | PROJ | PRs e issues atribuídos | Conta, repositórios | Futuro |

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-DASH-001 | Dashboard padrão (layout do mockup v0) criado após o onboarding, mostrando apenas widgets com fonte de dados disponível. | Deve | M1 |
| RF-DASH-002 | Modo edição: adicionar, remover, mover (arrastar) e redimensionar widgets numa grade de 12 colunas; desfazer/refazer; sair salva. | Deve | M1 |
| RF-DASH-003 | Catálogo de widgets com busca, descrição e pré-visualização. | Deve | M1 |
| RF-DASH-004 | Configurações por instância de widget (cada widget pode aparecer várias vezes com configurações diferentes). | Deve | M1 |
| RF-DASH-005 | Múltiplos dashboards nomeados (ex.: "Dia a dia", "Homelab"), troca por abas ou atalho. | Deveria | M3 |
| RF-DASH-006 | Layout por breakpoint (desktop, tablet, celular) com reorganização automática e ajuste manual opcional. | Deve | M1 |
| RF-DASH-007 | Atualização em tempo real por streaming (WebSocket), somente para widgets visíveis; pausa quando a janela/aba está oculta. | Deve | M1 |
| RF-DASH-008 | Cabeçalho com saudação contextual (bom dia/boa tarde/boa noite + nome), busca, notificações, data e hora. | Deve | M1 |
| RF-DASH-009 | Isolamento de falhas: widget com erro mostra a mensagem e "tentar de novo" sem afetar os demais. | Deve | M1 |
| RF-DASH-010 | Layouts salvos localmente e sincronizados para assinantes (configuração global vs. por dispositivo). | Deve | M1 / M4 |
| RF-DASH-011 | Dashboards por tenant (cada tenant tem seus próprios dashboards). | Deveria | M3 |
| RF-DASH-012 | Exportar/importar layout como JSON para compartilhar com a comunidade (sem dados, só estrutura). | Poderia | M3 |
| RF-DASH-013 | Limites e alertas por widget (ex.: disco > 90%, CPU > 85 °C por 2 min) gerando notificações. | Deveria | M3 |
| RF-DASH-014 | Estados padronizados: carregando (skeleton), vazio (com ação sugerida), erro, sem permissão, desatualizado (sem conexão com o PC). | Deve | M1 |
| RF-DASH-015 | SDK de widgets de terceiros (plugins WASM em sandbox). | Não agora | Futuro |

## Regras de negócio

- Um widget só existe se o módulo de origem estiver disponível na plataforma (ex.: WSL só no Windows); em outras plataformas, ele some do catálogo.
- Widgets que exibem valores financeiros respeitam o **modo privacidade** (RNF-PRI-05).
- No acesso remoto, o dashboard mostra os dados do PC conectado, nunca dados vindos da nuvem.
- O layout é configuração (sincroniza); os dados exibidos não são (ficam no PC).

## Critérios de aceite

- **Dado** o modo edição, **quando** o usuário arrasta o widget "Discos" para outra posição e sai do modo edição, **então** o layout persiste após reiniciar o app.
- **Dado** que o coletor de Docker falha (engine desligado), **quando** o dashboard carrega, **então** apenas o widget Docker mostra "Docker não está em execução" e os demais atualizam normalmente.
- **Dado** um celular com tela de 390 px, **quando** o dashboard é aberto, **então** os widgets aparecem em coluna única, sem rolagem horizontal.

## Perguntas em aberto

- Quais widgets extras você quer no lançamento? → [Ideias para enriquecer](../10-perguntas-em-aberto.md#ideias-para-enriquecer-o-produto)
