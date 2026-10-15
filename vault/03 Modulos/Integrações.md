---
tags: [modulo]
status: planejamento
marco: M1
spec: docs/modulos/integracoes.md
---
# Integrações (clima e calendário)

- **Clima** (M1): condição atual, umidade, vento, sensação, previsão horária/7 dias. Open-Meteo padrão, MET Norway alternativa.
- **Calendário** (M3): semana seg→dom. ICS e Google primeiro; depois Outlook (Graph) e CalDAV.

Chamadas sempre **do PC direto ao provedor** — a nuvem do CodatOS nunca vê. Tokens OAuth no [[Cofre]].

## Fios soltos
- OAuth desktop: PKCE + redirect loopback.
- Google `calendar.readonly` é escopo sensível → verificação do app; ICS privado como atalho.
- Licença do Open-Meteo para uso não comercial → [[Provedores de clima]]
