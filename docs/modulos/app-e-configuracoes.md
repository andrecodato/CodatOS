# Módulo · App desktop e configurações

> **Prefixo:** `RF-APP` · **Marco principal:** M0–M1 · **Status:** Rascunho
> ↑ [Requisitos](../01-requisitos.md) · Relacionados: [Dashboard](dashboard.md) · [Acesso remoto](acesso-remoto.md) · [Arquitetura](../02-arquitetura.md) · [Design system](../07-design-system.md)

## Objetivo

O "casco" do produto: o app que inicia com o sistema, vive na bandeja, abre a janela principal com o menu lateral e concentra as configurações gerais. Tudo aqui deve ser leve e discreto — o CodatOS nunca pode ser o motivo de o PC demorar a ligar.

## Telas (mockup v0)

- **Janela principal**: barra de título customizada, menu lateral (Dashboard, Financeiro, TaskHub, Conexões, Projetos, Configurações), seletor de tenant ("Pessoal") e cartão do usuário no rodapé.
- **Cabeçalho**: busca global (Ctrl+K), notificações, data e hora.
- **Configurações**: agrupa Geral, Aparência, Acesso via navegador, Coletores, Integrações, Segurança, Conta e sync, Tenancy e permissões, Sobre.

## Requisitos funcionais

| ID | Requisito | Prioridade | Marco |
|---|---|---|---|
| RF-APP-001 | Iniciar junto com o SO (Windows: chave `Run` do usuário; macOS: Login Item; Linux: XDG autostart). Configurável; sugerido ligado no onboarding. | Deve | M0 |
| RF-APP-002 | Ao iniciar com o SO, abrir **minimizado na bandeja** ou **com a janela do dashboard** (configurável), sem roubar o foco. Atraso de início configurável (0–60 s). | Deve | M0 |
| RF-APP-003 | Ícone na bandeja com menu: Abrir, Abrir no navegador, Conexões recentes, Pausar coleta, Bloquear cofre, Sair. Indicador visual quando há sessão remota ativa. | Deve | M0 |
| RF-APP-004 | Instância única: executar o app de novo foca a janela existente (e repassa deep links `codatos://`). | Deve | M0 |
| RF-APP-005 | Janela sem moldura nativa com barra de título customizada que respeita o SO (semáforo à esquerda no macOS; minimizar/maximizar/fechar à direita no Windows/Linux). Lembra posição e tamanho. | Deveria | M0 |
| RF-APP-006 | Menu lateral com Dashboard, Financeiro, TaskHub, Conexões, Projetos e Configurações, mais "Sistema" (opcional). Itens podem ser ocultados e reordenados; menu recolhível; seletor de tenant e cartão do usuário no rodapé. | Deve | M1 |
| RF-APP-007 | Paleta de comandos (Ctrl/Cmd+K): busca hosts, projetos, tarefas, telas e ações ("conectar em produção", "parar container X", "nova transação"). | Deveria | M1 |
| RF-APP-008 | Atalho global configurável para mostrar/ocultar a janela (padrão: Ctrl+Alt+Espaço). | Poderia | M1 |
| RF-APP-009 | Atualização automática assinada, com canais estável e beta, notas da versão e opção de adiar. | Deve | M1 |
| RF-APP-010 | Central de notificações (sino) + notificações nativas do SO para alertas de sistema, tarefas, contas a vencer e eventos. Regras de silêncio (modo foco). | Deveria | M3 |
| RF-APP-011 | Tela de Configurações com as seções listadas acima, busca nas configurações e botão "restaurar padrão" por seção. | Deve | M1 |
| RF-APP-012 | Onboarding na primeira execução: idioma, iniciar com o SO, pastas de projetos, cidade para o clima, proteção do cofre local (senha/PIN/biometria), conta opcional. Pode ser pulado. | Deve | M1 |
| RF-APP-013 | Exportar/importar todas as configurações para um arquivo local cifrado com senha (backup do próprio usuário; nunca vai para a nuvem). | Deveria | M2 |
| RF-APP-014 | Modo painel: dashboard em tela cheia, sem menu, ideal para um monitor secundário. | Poderia | M3 |
| RF-APP-015 | Diagnóstico em Configurações → Sobre: versão, coletores ativos, logs, botão "copiar diagnóstico" (sem dados sensíveis — R-06). | Deveria | M1 |
| RF-APP-016 | O app funciona completo sem conta (modo local). Recursos que exigem conta exibem explicação e CTA, nunca bloqueiam o resto. | Deve | M1 |
| RF-APP-017 | Fechar a janela mantém o app na bandeja e **destrói a WebView** para liberar memória; reabrir recria a janela em < 500 ms. | Deve | M0 |
| RF-APP-018 | Tema escuro (padrão) e claro; cor de destaque configurável; densidade (confortável/compacta). | Deveria | M1 |

## Regras de negócio

- O autostart é por usuário, nunca por máquina (sem privilégios de administrador).
- O app não exige privilégios elevados para funcionar. Recursos que precisam de privilégio (ex.: temperatura no Windows) são opcionais e explicados — ver [Monitor do sistema](sistema.md#temperatura-e-energia-no-windows).
- "Sair" pela bandeja encerra o núcleo; isso derruba sessões SSH, túneis e acesso pelo navegador — pedir confirmação se houver sessões ativas.

## Critérios de aceite

- **Dado** que o autostart está ligado e configurado para abrir na bandeja, **quando** o usuário faz logon no Windows, **então** o CodatOS inicia em até 10 s após a área de trabalho carregar, aparece apenas na bandeja e não rouba o foco.
- **Dado** que o app já está aberto, **quando** o usuário executa o atalho do menu Iniciar, **então** nenhum segundo processo permanece e a janela existente vem para frente.
- **Dado** que a janela foi fechada, **quando** se passam 30 s, **então** o consumo de RAM do app fica abaixo de 80 MB (RNF-DES-01).

## Perguntas em aberto

- Nome e identidade visual do produto → [Q-01](../10-perguntas-em-aberto.md#produto-e-marca)
- Assinatura de código e orçamento → [Q-29](../10-perguntas-em-aberto.md#infraestrutura-e-custos)
