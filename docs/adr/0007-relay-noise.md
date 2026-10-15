# ADR-0007 · Acesso remoto via relay com canal Noise ponta a ponta

- **Status:** Proposto
- **Data:** 2026-10-14
- **Relacionados:** [Acesso remoto](../modulos/acesso-remoto.md) · [APIs → relay](../06-api-e-protocolos.md#protocolo-do-relay) · [Segurança §9](../04-seguranca.md#9-relay-e-acesso-remoto)

## Contexto

Assinantes querem controlar o app do PC pelo celular, de qualquer rede. PCs domésticos ficam atrás de NAT/CGNAT; pedir para abrir portas no roteador é inviável e inseguro. O servidor não pode ver o que trafega.

## Decisão

- O PC mantém **uma conexão WebSocket de saída** com o relay quando o acesso remoto está ligado.
- O cliente (PWA) conecta ao relay, que **apenas pareia e encaminha** bytes entre os dois.
- Dentro do túnel, cliente e PC fazem handshake **Noise XX** (X25519, ChaCha20-Poly1305, BLAKE2s). A chave do cliente precisa estar **assinada pela identidade Ed25519 do usuário**, que só existe decifrada nos dispositivos dele.
- O protocolo dentro do túnel é o mesmo da API local ([ADR-0003](0003-agente-local-api-unica.md)).

## Alternativas consideradas

- **TLS terminando no relay** — o relay veria todo o conteúdo.
- **WebRTC P2P desde o início** — menor latência e custo de banda, mas NAT traversal é complexo (TURN ainda é necessário em muitas redes). Fica como otimização futura, com o relay fazendo a sinalização.
- **VPN (Tailscale/WireGuard)** — excelente para self-hosters (documentado como alternativa), mas exige instalação extra no celular e não serve como produto integrado.

## Consequências

- ✅ Funciona atrás de qualquer NAT, sem configuração.
- ✅ Nem o operador do relay consegue ler ou injetar comandos.
- ⚠️ Custo de banda no relay (terminal/SFTP via relay) — limites por plano.
- ⚠️ Latência adicional de um salto; meta < 150 ms p95 na mesma região.
