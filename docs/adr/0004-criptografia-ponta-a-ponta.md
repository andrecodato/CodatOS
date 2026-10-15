# ADR-0004 · Criptografia ponta a ponta (zero-knowledge) em vez de hash para segredos

- **Status:** Proposto
- **Data:** 2026-10-14
- **Relacionados:** [Segurança](../04-seguranca.md) · [Requisitos R-02…R-04](../01-requisitos.md#4-restrições-invioláveis) · [Contas e sync](../modulos/contas-e-sync.md)

## Contexto

O briefing pede que "senhas, chaves SSH, tokens e dados nocivos sejam escondidos com métodos de hash" e que o banco guarde apenas configurações. Porém senhas de servidores, chaves privadas e tokens precisam ser **usados** depois — e hash é irreversível. Além disso, o sync deve funcionar entre dispositivos e com membros de tenants.

## Decisão

Adotar um modelo **zero-knowledge**, no estilo Bitwarden/1Password:

- **Senha de login**: nunca enviada. O cliente deriva uma chave com **Argon2id**; o servidor recebe um derivado (`authKey`) e guarda apenas o **hash Argon2id** dele.
- **Segredos e configurações**: cifrados no dispositivo com **XChaCha20-Poly1305**, usando chaves derivadas da senha mestra (hierarquia em [Segurança §3](../04-seguranca.md#3-hierarquia-de-chaves)). O servidor guarda só texto cifrado.
- **Compartilhamento**: cada cofre tem sua chave, selada (X25519) para a chave pública de cada membro.
- **Recuperação**: chave de recuperação gerada no cadastro; sem ela e sem a senha, os dados da nuvem são irrecuperáveis.

## Alternativas consideradas

- **Hash de tudo** — impossível usar os segredos depois.
- **Criptografia só no servidor (chave do servidor/KMS)** — o servidor (e quem o invadir) conseguiria ler tudo; contraria o objetivo do pedido.
- **OPAQUE (PAKE)** em vez de `authKey` + Argon2id — mais elegante, mas com menos bibliotecas maduras em Rust e Node. Pode substituir o esquema numa versão futura sem mudar o restante da hierarquia.

## Consequências

- ✅ Vazamento do banco não expõe segredos nem configurações.
- ✅ Atende ao espírito do pedido com garantia matemática, não só política.
- ⚠️ Esquecer senha + chave de recuperação = perder os dados sincronizados. Precisa ser comunicado com clareza.
- ⚠️ Busca, merge e validação de conteúdo acontecem só no cliente.
- ⚠️ Login social (GitHub) autentica, mas não desbloqueia: o cofre continua exigindo senha mestra ou biometria.
- ⚠️ Exige revisão cuidadosa e vetores de teste; nenhuma primitiva própria.
