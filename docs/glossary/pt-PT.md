# European Portuguese (pt-PT) term choices

How `i18n/pt-PT/triib.ftl` renders the terms in `docs/GLOSSARY.md`. The text
follows the 1990 spelling agreement as used in Portugal (aspeto, detetado,
registado), uses « » for quotation marks, «a + infinitive» for ongoing
states («A enviar»), no space before `%` and a no-break space as the
thousands separator.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m.), talkers | Loanword, lower case mid-sentence: «Fluxos de talker», «a aguardar o talker». |
| listener | listener (m.), listeners | Loanword, as for talker. |
| grandmaster | grandmaster (m.) | Kept in Latin script; never «mestre». |
| entity | entidade | Not «dispositivo», which is kept for *device* in the network view. |
| entity model | modelo de entidade | |
| controller | controlador | |
| stream | fluxo | |
| stream input, stream output | entrada de fluxo, saída de fluxo | Matrix: «Saídas de talker», «Entradas de listener». |
| connection, connect, bind | ligação, ligar; vincular | Disconnect is «desligar»; a bound input is «vinculada» (a entrada). |
| media clock | relógio de média | Also in «VLAN de média», «lock de média». |
| clock domain | domínio de relógio | |
| clock source | fonte de relógio | |
| sampling rate | frequência de amostragem | |
| bridge | switch (m.), switches | Practitioners say switch; «comutador» is rare. «switch AVB» where the standard says AVB bridge. |
| reservation | reserva | |
| egress | saída | «porta de saída»; the column and the alarm say «tráfego de saída» so it does not read as a stream output. |
| link | link (m.) | «Link ativo», «link em baixo». Not «ligação», which is taken by *connection*. |
| peer delay | peer delay | Kept. |
| offset | offset (m.) | Kept, as PTP tools show it. |
| hop | salto | «Saltos até ao grandmaster». |
| unicast, multicast | unicast, multicast | Kept. |
| fan-out | fan-out | Not shown as a word; the inspector says «Até N listeners por fluxo, depois multicast». |
| descriptor | descritor | |
| cluster | cluster, clusters | |
| stream port | porta de fluxo | «porta» (f.) for every port. |
| channel mapping | mapeamento de canais | Verbs: «mapear», «remover o mapeamento». |
| control | controlo | |
| preset | preset (m.), presets | «Recuperar» for recall. |
| identify | identificar | |
| counter | contador | |
| locked, lost lock | entrar em lock, perder o lock | |
| holding over | em holdover | Kept in English, as telecom and PTP writing does; pairs with «em lock». |
| station | estação | The IEEE 802.11 term: «Estação, em lock». |
| access point | ponto de acesso | |
| beacon | beacon (m.), beacons | «Mode B, a partir dos beacons». |
| signal | sinal | The inspector label, in dBm. |
| interrupted | interrompido | |
| timestamp | timestamp (m.) | |
| advertise, advertised | anunciar, anunciado | The ADP section is «Anúncio». |
| interface | interface (f.) | «ativa», «sem fios» agree with it. |
| hardware clock | relógio de hardware | |
| virtual (interface) | virtual | |

Other recurring words: frame «trama», reported «indicado», heard «detetado»
(in the map) and «Recebida» (in the log), untagged «sem etiqueta»,
Settings «Definições», Save «Guardar», Delete «Eliminar», log «Registo»,
desktop «ambiente de trabalho».

## Choices a native speaker should check

- «relógio de média» for *media clock*: «média» may read as *average*;
  the alternative is to keep «media clock».
- «ligação/ligar» for *connection* with «link» kept for *link*.
- «lock» («entrou em lock», «perdeu o lock») for *locked*.
- «frequência de amostragem» rather than «taxa de amostragem», which is
  shorter for the inspector's labels.
- «timestamp» kept, rather than «marca temporal».
- «sem etiqueta» and «etiquetagem VLAN» for VLAN tagging.
- «em holdover» for *holding over*, kept in English like «lock».
- «rajadas» for FTM *bursts*, rather than «bursts».
- «Sem sincronização» for the time mode «No time».
