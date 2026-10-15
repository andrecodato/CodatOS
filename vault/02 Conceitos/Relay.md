---
tags: [conceito, arquitetura, seguranca]
---
# Relay

Serviço da nuvem que **pareia** o celular com o PC e **encaminha bytes** entre eles — sem conseguir lê-los.

1. PC abre WSS **de saída** (nada de abrir porta no roteador) e registra presença.
2. Celular pede túnel para o PC; relay confere dono/permissão `remote.access`.
3. Handshake **Noise XX** ponta a ponta; o PC só aceita chaves assinadas pela identidade do usuário → [[Hierarquia de chaves]].
4. Dentro do túnel: o mesmo protocolo da API local → [[Agente local]].

**Vê:** quem conectou em qual dispositivo, quando, quantos bytes. **Não vê:** nada do conteúdo.
**Futuro:** WebRTC P2P com o relay só na sinalização (custo de banda).

📄 `docs/adr/0007-relay-noise.md` · Módulo: [[Acesso remoto]]
