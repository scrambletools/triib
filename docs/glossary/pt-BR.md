# Brazilian Portuguese (pt-BR) term choices

How `i18n/pt-BR/triib.ftl` renders the terms in `docs/GLOSSARY.md`. The text
uses “ ” for quotation marks, gerunds for ongoing states («Enviando»), no
space before `%` and the point as the thousands separator. Selectors add a
`[0]` variant in the plural, since CLDR puts 0 in the `one` category for
pt.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m.), talkers | Loanword, lower case mid-sentence: «Fluxos de talker», «aguardando o talker». |
| listener | listener (m.), listeners | Loanword, as for talker. |
| grandmaster | grandmaster (m.) | Kept in Latin script; never «mestre». |
| entity | entidade | Not «dispositivo», which is kept for *device* in the network view. |
| entity model | modelo de entidade | |
| controller | controlador | |
| stream | fluxo | |
| stream input, stream output | entrada de fluxo, saída de fluxo | Matrix: «Saídas de talker», «Entradas de listener». |
| connection, connect, bind | conexão, conectar; vincular | A bound input is «vinculada» (a entrada). |
| media clock | relógio de mídia | Also in «VLAN de mídia», «lock de mídia». |
| clock domain | domínio de relógio | |
| clock source | fonte de relógio | |
| sampling rate | taxa de amostragem | |
| bridge | switch (m.), switches | Practitioners say switch. «switch AVB» where the standard says AVB bridge. |
| reservation | reserva | |
| egress | saída | «porta de saída»; the column and the alarm say «tráfego de saída» so it does not read as a stream output. |
| link | link (m.) | «Link ativo», «link inativo», «Quedas do link». |
| peer delay | peer delay | Kept, as PTP writing in Brazil does. |
| offset | offset (m.) | Kept, as PTP tools show it. |
| hop | salto | |
| unicast, multicast | unicast, multicast | Kept. |
| fan-out | fan-out | Not shown as a word; the inspector says «Até N listeners por fluxo, depois multicast». |
| descriptor | descritor | |
| cluster | cluster, clusters | |
| stream port | porta de fluxo | «porta» (f.) for every port. |
| channel mapping | mapeamento de canais | Verbs: «mapear», «remover o mapeamento». |
| control | controle | |
| preset | preset (m.), presets | «Recuperar» for recall. |
| identify | identificar | |
| counter | contador | |
| locked, lost lock | entrar em lock, perder o lock | «travar» reads as a frozen program in Brazilian computing. |
| interrupted | interrompido | |
| timestamp | timestamp (m.) | |
| advertise, advertised | anunciar, anunciado | The ADP section is «Anúncio». |
| interface | interface (f.) | «ativa», «sem fio» agree with it. |
| hardware clock | relógio de hardware | |
| virtual (interface) | virtual | |

Other recurring words: frame «quadro», reported «informado», heard
«detectado» (in the map) and «Recebido» (in the log), untagged «sem tag»,
Settings «Configurações», Save «Salvar», Delete «Excluir», log «Log».

## Choices a native speaker should check

- «lock» («entrou em lock», «perdeu o lock») for *locked*, rather than
  «travado» or «sincronizado».
- «fluxo» for *stream*: many engineers say «stream».
- «timestamp» kept, rather than «carimbo de tempo» or «marca temporal».
- «relógio de mídia» for *media clock*.
- «sem tag» and «tags VLAN» for VLAN tagging, rather than «não marcado».
- «substituído por um fluxo de rank superior» for MSRP's pre-emption.
