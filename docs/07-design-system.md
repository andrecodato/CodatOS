# 07 · Design system

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [APIs](06-api-e-protocolos.md) · [Índice](README.md) · Próximo: [Site](08-site.md) →
> Referência visual: **mockup v0** (salvar a imagem em `docs/assets/mockup-v0.png`). O nome "Nexora" que aparece no mockup foi substituído por **CodatOS** ([Q-01](10-perguntas-em-aberto.md#produto-e-marca)).

## Linguagem visual

- **Escuro e profundo**: azul-marinho quase preto, com cartões levemente mais claros e bordas finas — nada de cinza neutro.
- **Um azul protagonista** para ação e foco, com um **gradiente violeta → azul** reservado para a marca e o botão principal do login.
- **Dados em destaque**: números grandes com algarismos tabulares, gráficos finos e coloridos, rótulos discretos.
- **Densidade de dashboard**: muita informação, mas com respiro (16 px entre cartões, 20 px de padding).
- **Movimento sutil**: transições curtas, nada que distraia do dado.

## Tokens

Valores estimados a partir do mockup; serão refinados no protótipo.

```css
:root {
  /* Superfícies */
  --bg-app:        #060A14;   /* atrás da janela / site */
  --bg-base:       #0A0F1E;   /* fundo da janela */
  --bg-sidebar:    #0B1122;
  --bg-card:       #0F1629;
  --bg-card-hover: #131B31;
  --bg-input:      #0C1324;
  --border:        #1C2640;
  --border-strong: #26324F;

  /* Texto */
  --text:          #E6EAF2;
  --text-muted:    #8B95AD;
  --text-subtle:   #5C6680;

  /* Marca e ação */
  --primary:        #3B82F6;
  --primary-hover:  #2563EB;
  --primary-soft:   rgb(59 130 246 / 0.14);   /* item ativo do menu, chips */
  --primary-glow:   0 0 0 1px rgb(59 130 246 / 0.4), 0 8px 24px rgb(59 130 246 / 0.25);
  --accent:         #7C5CFF;
  --gradient-brand: linear-gradient(135deg, #7C5CFF 0%, #3B82F6 100%);

  /* Semânticas */
  --success: #22C55E;   /* receitas, online, concluído */
  --warning: #F59E0B;   /* prioridade média, alerta */
  --danger:  #F43F5E;   /* despesas, prioridade alta, erro */
  --info:    #38BDF8;

  /* Séries de gráficos (ordem fixa) */
  --chart-1: #3B82F6;  --chart-2: #F43F5E;  --chart-3: #F59E0B;
  --chart-4: #22C55E;  --chart-5: #A855F7;  --chart-6: #06B6D4;

  /* Forma */
  --radius-sm: 6px;     /* badges */
  --radius-md: 8px;     /* inputs, botões */
  --radius-lg: 12px;    /* cartões */
  --radius-xl: 16px;    /* modais, janela */
  --shadow-card: 0 1px 0 rgb(255 255 255 / 0.03) inset, 0 8px 24px rgb(0 0 0 / 0.35);

  /* Espaço (base 4 px) */
  --space-1: 4px; --space-2: 8px; --space-3: 12px; --space-4: 16px;
  --space-5: 20px; --space-6: 24px; --space-8: 32px;

  /* Movimento */
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --dur-fast: 120ms; --dur-base: 200ms; --dur-slow: 320ms;
}
```

O tema claro (Deveria) redefine apenas superfícies e texto; cores semânticas e de gráfico se mantêm.

## Tipografia

| Estilo | Fonte | Tamanho/altura | Peso | Uso |
|---|---|---|---|---|
| Display | Inter | 26/32 | 600 | Saudação "Olá, Andre 👋" |
| Título de página | Inter | 22/28 | 600 | "Financeiro", "TaskHub", "Conexões" |
| Título de cartão | Inter | 14/20 | 600 | "Saldo total", "Discos" |
| Métrica | Inter (tabular-nums) | 28/32 | 600 | R$ 12.480,32 · 22 °C · 12 |
| Corpo | Inter | 14/20 | 400 | Textos e listas |
| Legenda | Inter | 12/16 | 400–500 | "Último acesso: hoje", eixos |
| Mono | JetBrains Mono | 13/20 | 400 | Terminal, IPs, comandos |

Todas as fontes embutidas no bundle (nada de CDN — RNF de privacidade e funcionamento offline).

## Componentes

| Componente | Notas do mockup |
|---|---|
| **Janela / barra de título** | Sem moldura nativa; semáforo no macOS, controles à direita no Windows/Linux; região arrastável |
| **Sidebar** | ~220 px; logo no topo; item ativo com fundo `--primary-soft` e texto claro; ícones Lucide 18 px; rodapé com **seletor de tenant** ("Pessoal / Tenant") e **cartão do usuário** (avatar, nome, papel) |
| **Topbar** | Busca com dica "(Ctrl + K)", sino com badge, ícone de alertas, data e hora |
| **Cartão / widget** | Título + ação ("Ver todos", menu `⋯`, ícone); corpo; estados de carregando/vazio/erro |
| **Stat** | Rótulo, valor grande, variação com seta (verde/vermelho) e "vs mês anterior" |
| **Anel (gauge)** | CPU/RAM/GPU: anel com percentual no centro, rótulo e temperatura abaixo |
| **Barra de progresso** | Discos e tarefas: trilho `--border`, preenchimento azul; vermelho acima do limite |
| **Gráfico de área** | Saldo: linha + gradiente para transparente |
| **Barras** | Fluxo de caixa: pares receita/despesa por dia |
| **Rosca** | Categorias com legenda e percentuais |
| **Lista de eventos** | Bloco de dia (abreviação + número) + título + horário |
| **Badge** | Prioridade Alta (vermelho), Média (âmbar), Baixa (verde); status "Ativo", "Online" |
| **Status dot** | Ponto verde "Online"; cinza offline; âmbar degradado |
| **Toggle** | WSL em execução, ativar sincronização |
| **Abas** | Sublinhado azul na aba ativa (Visão geral / Transações…) |
| **Tabela** | Cabeçalho discreto, linhas com divisor fino, valores alinhados à direita |
| **Input** | Fundo `--bg-input`, borda `--border`, foco com anel azul; ícone de olho em senhas |
| **Botões** | Primário (azul sólido; no login, gradiente da marca), secundário (contorno), fantasma, perigo |
| **Terminal** | Painel escuro com JetBrains Mono, prompt verde, abas acima |
| **Paleta de comandos** | Modal central com busca e resultados agrupados |
| **Toast / notificação** | Canto inferior direito; ícone semântico |
| **Skeleton** | Blocos com brilho suave enquanto carrega |

Gráficos são componentes SVG próprios ([Stack](03-stack.md#ui-rust--wasm)) para manter exatamente este visual.

## Inventário de telas (mockup v0)

| Tela | Módulo | Documento |
|---|---|---|
| Login / Cadastro | Contas | [contas-e-sync.md](modulos/contas-e-sync.md#telas-mockup-v0) |
| Dashboard | Dashboard | [dashboard.md](modulos/dashboard.md#telas-mockup-v0) |
| Financeiro — Visão geral | Financeiro | [financeiro.md](modulos/financeiro.md#telas-mockup-v0) |
| TaskHub — Minhas tarefas | TaskHub | [taskhub.md](modulos/taskhub.md#telas-mockup-v0) |
| Conexões — lista | Conexões | [conexoes.md](modulos/conexoes.md#telas-mockup-v0) |
| Conexão — detalhe + terminal | Conexões | [conexoes.md](modulos/conexoes.md#telas-mockup-v0) |
| Tenancy e Permissões | Multitenancy | [multitenancy.md](modulos/multitenancy.md#telas-mockup-v0) |
| Sincronização | Contas e sync | [contas-e-sync.md](modulos/contas-e-sync.md#telas-mockup-v0) |

Telas ainda **não desenhadas**: Sistema (detalhe de PC/WSL/Docker), Projetos, Configurações, Onboarding, Paleta de comandos, SFTP, Modo edição do dashboard, PWA no celular, estados vazios e de erro.

## Movimento

- Hover/foco: `--dur-fast`; abrir painéis e modais: `--dur-base`; troca de página: fade + 4 px de deslocamento.
- Métricas animam a transição entre valores (tween de 300 ms); anéis e barras animam o preenchimento.
- `prefers-reduced-motion`: sem tweens, sem shimmer, transições instantâneas.

## Acessibilidade

- Contraste mínimo AA (texto `--text-muted` sobre `--bg-card` verificado ≥ 4.5:1).
- Foco sempre visível (anel azul de 2 px).
- Cor nunca é o único sinal: prioridades e variações também têm texto/ícone.
- Gráficos com resumo textual e tabela acessível alternativa.

## Marca

- Logo: monograma **C** com gradiente violeta → azul, o mesmo usado no [protótipo do site](../apps/site/prototypes/mosaico.html); a versão final ainda será refinada.
- Slogan do mockup: **"Seu universo, em um só lugar."**
- A imagem de paisagem do login deve ter licença livre (foto própria, CC0 ou licença Unsplash) e receber sobreposição em gradiente escuro.
