# 08 · Site de apresentação

> **Status:** Rascunho · **Versão:** 0.1 · **Atualizado:** 2026-10-14
> ← [Design system](07-design-system.md) · [Índice](README.md) · Próximo: [Roadmap](09-roadmap.md) →
> Decisão: [ADR-0008](adr/0008-site-astro.md)

## Objetivo

Apresentar o CodatOS, convencer em 10 segundos, permitir **baixar o app** para o sistema certo e levar ao **código no GitHub**. Visual escuro e moderno no estilo de Vercel, Render, claude.ai e ChatGPT, com um **mosaico interativo** que reage ao clique com uma explosão azul.

## Páginas

| Rota | Conteúdo |
|---|---|
| `/` | Landing (seções abaixo) |
| `/download` | Detecção de SO + todos os instaladores, checksums SHA-256, requisitos, instruções (winget, Homebrew, Flatpak) |
| `/pricing` | Free, Pro, Team e Self-host; FAQ de cobrança |
| `/security` | Como funciona o zero-knowledge, em linguagem simples, com link para [04 · Segurança](04-seguranca.md) |
| `/changelog` | Gerado a partir das releases do GitHub |
| `/docs` | Documentação pública (Starlight lendo `docs/`) — fase 2 |
| `/privacy`, `/terms` | Política de privacidade (LGPD) e termos |

## Landing — seções

1. **Navbar** fixa e translúcida (blur): logo, Recursos, Segurança, Preços, Docs, ícone do GitHub com contagem de estrelas, botão **Baixar**.
2. **Hero** sobre o **mosaico interativo**:
   - Título: "Seu universo, em um só lugar." (ou variação por nome final)
   - Subtítulo: "Finanças, tarefas, agenda, servidores e a saúde do seu PC num painel bonito, rápido e privado. Open source."
   - CTAs: **Baixar para Windows** (texto muda conforme o SO detectado; menu com outros SOs) · **Ver no GitHub**.
   - Selos: "Gratuito e open source" · "Windows · macOS · Linux" · "Criptografia ponta a ponta".
3. **Screenshot do produto** em perspectiva leve, com brilho azul por trás e borda em gradiente.
4. **Grade de recursos** (cartões tipo *bento*): Dashboard configurável · Monitor do PC, WSL e Docker · Conexões SSH/SFTP · Financeiro · TaskHub · Acesso pelo navegador e celular.
5. **Destaques com mini-animações**: arrastar widgets; terminal digitando; anel de CPU pulsando.
6. **Privacidade de verdade**: diagrama simples "seu dispositivo cifra → nuvem só vê cadeado"; frase-chave "Nem nós conseguimos ler suas senhas".
7. **Open source**: estrelas, contribuidores, licença, "como contribuir", "hospede seu próprio servidor".
8. **Preços** resumidos com link para `/pricing`.
9. **CTA final** sobre o mosaico de novo.
10. **Rodapé**: links, idioma (pt-BR / en), status da nuvem, redes.

## Mosaico interativo

> **Protótipo funcional:** [`apps/site/prototypes/mosaico.html`](../apps/site/prototypes/mosaico.html) — abra direto no navegador, sem build. O painel **Ajustes** (canto inferior direito) calibra os parâmetros abaixo ao vivo e copia o resultado em JSON. Medido no protótipo: 0,1–0,4 ms por quadro em uso normal e 1,1 ms com 14 ondas simultâneas (limite).

### Comportamento

- O fundo do hero é uma **grade de ladrilhos** escuros (≈ 36 px, espaçamento 2 px) com bordas quase invisíveis.
- **Ocioso**: alguns ladrilhos acendem e apagam aleatoriamente em azul bem fraco ("cintilar").
- **Hover**: ladrilhos próximos ao cursor ganham leve brilho (raio ~120 px).
- **Clique/toque**: uma **onda de choque azul** nasce no ponto do clique e se propaga em anel; cada ladrilho acende quando a frente da onda passa por ele (flash azul-claro → azul → apaga), com intensidade decaindo com a distância. Cliques múltiplos geram ondas que se somam.
- Acessibilidade: com `prefers-reduced-motion`, o mosaico fica estático com um gradiente suave e o clique gera apenas um brilho local curto.
- Desempenho: Canvas 2D com `devicePixelRatio`, laço via `requestAnimationFrame` **somente** enquanto houver animação ou o mouse estiver sobre o hero; pausa quando fora da tela (`IntersectionObserver`) ou com a aba oculta. Meta: < 2 ms por frame num notebook médio.

### Parâmetros iniciais

| Parâmetro | Valor |
|---|---|
| Tamanho do ladrilho | 36 px (28 px no celular) |
| Cor base | `#0B1122` com borda `#131B31` |
| Rampa de cor da onda | `#1D4ED8` → `#3B82F6` → `#93C5FD` (pico) |
| Velocidade da onda | 0,9 px/ms |
| Duração do pulso por ladrilho | 650 ms |
| Decaimento | `exp(-d / 650)` |
| Brilho | Halo aditivo nos ladrilhos com energia > 0,5 + anel e clarão no ponto do clique (sem `shadowBlur`, que é caro) |
| Ondas simultâneas | Máximo 14 (as mais antigas saem) |
| Cintilar ocioso | ~1,8% dos ladrilhos, 2,6 s cada, renderizado a 24 fps |

### Esboço do algoritmo

```ts
type Wave = { x: number; y: number; t0: number };
const waves: Wave[] = [];

canvas.addEventListener("pointerdown", (e) => {
  waves.push({ x: e.offsetX, y: e.offsetY, t0: performance.now() });
  start();
});

function frame(now: number) {
  for (const cell of cells) {
    let energy = cell.idle(now);                       // cintilar ocioso
    for (const w of waves) {
      const d = Math.hypot(cell.cx - w.x, cell.cy - w.y);
      const local = now - w.t0 - d / SPEED;            // tempo desde que a frente passou
      if (local >= 0 && local < PULSE) {
        const pulse = Math.sin((local / PULSE) * Math.PI); // sobe e desce
        energy = Math.max(energy, pulse * Math.exp(-d / FALLOFF));
      }
    }
    drawCell(cell, colorRamp(energy));
  }
  pruneFinishedWaves(now);
  if (waves.length || hovering) requestAnimationFrame(frame);
}
```

## Downloads

- No build, o site consulta `releases/latest` do GitHub e gera os links por plataforma/arquitetura.
- No cliente, detecta SO e arquitetura (`navigator.userAgentData` com *fallback*) para o botão principal.
- Cada instalador com **checksum SHA-256** e link para a assinatura.
- Canais: estável (padrão) e beta (link discreto).

## Requisitos não funcionais do site

| Métrica | Meta |
|---|---|
| Lighthouse (Performance, Acessibilidade, SEO, Boas práticas) | ≥ 95 |
| LCP | < 1,8 s em 4G |
| JS na landing | < 40 KB gzip (mosaico + menu) |
| Analytics | Nenhum por padrão; se houver, Plausible/Umami sem cookies |
| Idiomas | pt-BR e en (rota `/en`) |
| SEO | Open Graph com imagem gerada por página, sitemap, dados estruturados `SoftwareApplication` |

## Hospedagem

Site estático em Cloudflare Pages ou GitHub Pages, deploy automático a cada push em `main` que toque `apps/site/`. Domínio provisório `codatos.com`, ainda não comprado ([Q-01](10-perguntas-em-aberto.md#produto-e-marca)).
