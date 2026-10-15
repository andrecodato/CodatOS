---
tags: [pesquisa]
status: aberta
relacionado: Q-30
---
# Provedores de clima

| Provedor | Chave? | Uso comercial | Observações |
|---|---|---|---|
| **Open-Meteo** | Não | API gratuita é para uso **não comercial**; há planos pagos | Tem geocoding; dados com atribuição |
| **MET Norway** (api.met.no) | Não | Permitido, com atribuição (CC BY 4.0) | Exige `User-Agent` identificando o app; respeitar cache/limites |
| OpenWeatherMap / WeatherAPI | Sim | Planos variados | Exigiria cada usuário criar chave — fricção |

## Questão
Cada usuário chama a API **do próprio PC**, num app open source gratuito → tende a ser uso pessoal. Mas, com plano pago existindo, convém **confirmar com o Open-Meteo** ou usar MET Norway como padrão. _(verificar termos atuais antes do M4)_

## Implementação
- Cache de 30 min por localização; ícones próprios mapeados dos códigos WMO.
- Geocoding para busca de cidade; IP-geolocalização só com opt-in.

[[Integrações]]
