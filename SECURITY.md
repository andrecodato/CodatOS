# Política de segurança

Segurança é o centro do CodatOS: ele guarda chaves SSH, senhas de servidores e dados pessoais. Levamos qualquer relato a sério.

## Como reportar uma vulnerabilidade

**Não abra issue pública.** Use o **reporte privado de vulnerabilidades do GitHub**: aba **Security** → **Report a vulnerability** neste repositório.

Inclua, se possível:

- componente afetado (app desktop, núcleo, UI, nuvem, relay, site);
- versão e sistema operacional;
- passos para reproduzir ou prova de conceito;
- impacto que você enxerga.

**Nunca** inclua segredos reais (seus ou de terceiros) no relato.

## O que esperar

| Etapa | Prazo-alvo |
|---|---|
| Confirmação de recebimento | até 72 horas |
| Avaliação inicial e severidade | até 7 dias |
| Correção de falhas críticas | o mais rápido possível, com release dedicada |
| Divulgação | coordenada com você, após a correção |

Pesquisadores serão creditados nas notas da versão, se desejarem.

## Escopo

**Dentro:** tudo neste repositório — app, núcleo, criptografia, sincronização, relay, API da nuvem e site.

Interesse especial em:

- qualquer forma de o servidor ou o relay lerem dados do usuário (quebra do modelo zero-knowledge);
- acesso à API local por sites ou processos não autorizados;
- vazamento de segredos em disco, memória, logs ou UI;
- escalada de permissões entre membros de um tenant.

**Fora:** ataques que exigem o sistema operacional já comprometido (malware com privilégios), engenharia social contra mantenedores, negação de serviço volumétrica.

## Versões suportadas

O projeto ainda não tem versões publicadas. Após a primeira release, apenas a versão estável mais recente receberá correções de segurança.

## Porto seguro

Pesquisas feitas de boa-fé, sem acessar dados de terceiros, sem degradar o serviço e respeitando esta política não serão alvo de ação por parte do projeto.

Detalhes do modelo de segurança em [docs/04-seguranca.md](docs/04-seguranca.md).
