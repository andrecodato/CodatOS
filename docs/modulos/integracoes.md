# Módulo · Integrações (clima e calendário)

> **Prefixo:** `RF-INT` · **Marco principal:** M1 (clima) / M3 (calendário) · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [Segurança → tokens](../04-seguranca.md#3-hierarquia-de-chaves) · [TaskHub](taskhub.md)

## Objetivo

Trazer para o dashboard informações externas do dia: previsão do tempo e a agenda da semana. Toda chamada é feita **pelo PC do usuário direto ao provedor** — a nuvem do CodatOS nunca intermedeia nem vê esses dados.

## Clima

### Telas (mockup v0)

Widget **Tempo**: 22 °C, Ensolarado, São Paulo – SP, umidade, vento, sensação térmica; "Ver todos" abre a previsão completa.

### Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-INT-001 | Condições atuais: temperatura, condição com ícone, mínima/máxima, umidade, vento, sensação térmica, chance de chuva. | Deve | M1 |
| RF-INT-002 | Localização por busca de cidade (geocoding) ou automática por IP (opt-in); várias cidades salvas. | Deve | M1 |
| RF-INT-003 | Provedor configurável — **Open-Meteo** (padrão, sem chave) e **MET Norway** (alternativa); cache de 30 min; unidades métricas/imperiais. | Deve | M1 |
| RF-INT-004 | Tela expandida: previsão por hora (24 h) e 7 dias. | Deveria | M1 |
| RF-INT-005 | Alertas de tempo severo (chuva forte, calor extremo) como notificação. | Poderia | Futuro |

> [!WARNING]
> **Licença dos provedores**: a API gratuita do Open-Meteo é para uso não comercial. Como cada usuário chama a API do próprio PC num app open source, o uso tende a ser pessoal — mas é preciso confirmar antes do lançamento do plano pago. O MET Norway permite uso comercial com atribuição (CC BY 4.0) e exige `User-Agent` identificável. Ver [Q-30](../10-perguntas-em-aberto.md#infraestrutura-e-custos).

## Calendário

### Telas (mockup v0)

Widget **Calendário da semana** (Abr 28 – Mai 4): dias com eventos, horário e título; "Ver todos" abre a visão semanal completa.

### Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-INT-010 | Fontes: **URL ICS** e **Google Calendar** primeiro; depois **Microsoft 365/Outlook** (Graph) e **CalDAV** (iCloud, Nextcloud, Fastmail). | Deve | M3 |
| RF-INT-011 | OAuth 2.0 com PKCE e redirecionamento *loopback*; tokens guardados no cofre cifrado (R-02). | Deve | M3 |
| RF-INT-012 | Visão da semana (seg → dom) com eventos por dia, horário e cor por calendário; eventos de dia inteiro destacados. | Deve | M3 |
| RF-INT-013 | Escolher quais calendários de cada fonte aparecem. | Deve | M3 |
| RF-INT-014 | Notificação antes dos eventos (padrão: 10 min). | Deveria | M3 |
| RF-INT-015 | Somente leitura por padrão, com escopos mínimos (`calendar.readonly`). | Deve | M3 |
| RF-INT-016 | Mostrar vencimentos de tarefas e contas no calendário (camadas opcionais). | Poderia | M3 |
| RF-INT-017 | Criar evento rápido. | Poderia | Futuro |

## Regras de negócio

- Configurações de fontes (qual calendário, cores, cidade) **sincronizam** cifradas; tokens OAuth sincronizam **cifrados no cofre**, ou ficam só no dispositivo se o usuário preferir.
- Atualização do calendário a cada 15 min e ao abrir o app; offline mostra o último cache com indicação "desatualizado".

## Critérios de aceite

- **Dado** uma URL ICS pública válida, **quando** é adicionada, **então** os eventos da semana aparecem no widget em até 5 s.
- **Dado** que o token do Google expirou e a renovação falhou, **quando** o widget atualiza, **então** ele mostra "Reconectar Google" sem afetar outras fontes.

## Riscos

- O escopo `calendar.readonly` do Google é "sensível": o app OAuth precisa passar pela verificação do Google para uso público. Alternativa até lá: ICS privado do Google Agenda. Ver [Roadmap → riscos](../09-roadmap.md#riscos).

## Perguntas em aberto

- Quais calendários você usa hoje? → [Q-09](../10-perguntas-em-aberto.md#calendário-e-tarefas)
