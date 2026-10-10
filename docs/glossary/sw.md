# Swahili (sw) terms

How `i18n/sw/triib.ftl` renders the glossary's roles and terms of art. It
uses the Swahili computing vocabulary of current software (mtandao,
kiolesura, maunzi, mipangilio) and keeps English loanwords for AVB terms
that have no settled Swahili word. Loanwords take class 9/10 agreement.
Counts set thousands apart with a comma (1,204,331).

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lower case mid-sentence, class 9/10: "talker nyingine", "talker inatuma". |
| listener | listener | Loanword, as for talker: "hakuna listener iliyo tayari". |
| grandmaster | grandmaster | Loanword, lower case mid-sentence: "kutoka kwa grandmaster". Never "bwana" or "mkuu". |
| entity | huluki | Class 9/10; not kifaa, which is kept for device. |
| entity model | modeli ya huluki | |
| controller | controller | Kept as the ATDECC role beside talker and listener; kidhibiti is taken by control. |
| stream | mtiririko, mitiririko | Class 3/4; plural selectors pick mtiririko/mitiririko. |
| stream input, stream output | ingizo la mtiririko, towe la mtiririko | Plurals maingizo, matowe. Matrix: "Matowe ya talker", "Maingizo ya listener". |
| connection, connect, bind | muunganisho, kuunganisha; kuambatisha | Disconnect kutenganisha; a bound input is "Imeambatishwa". |
| media clock | saa ya media | |
| clock domain | kikoa cha saa | |
| clock source | chanzo cha saa | |
| sampling rate | kiwango cha sampuli | |
| bridge | swichi | What engineers say; "daraja" would be the lay word. "AVB bridge" is "swichi ya AVB"; bridge port "mlango wa swichi". |
| reservation | uhifadhi | "uhifadhi wa talker umeshindwa". |
| egress | egress | "mlango wa egress"; the column is "Egress". |
| link | kiungo, viungo | "Kiungo kiko hai", "Kiungo kimekatika"; drops "Kukatika kwa kiungo". |
| peer delay | peer delay | Kept. |
| offset | offset | Kept. |
| hop | hop | "hop 3 kutoka kwa grandmaster". |
| unicast, multicast | unicast, multicast | Kept. |
| fan-out | fan-out | Not shown as a word; the inspector says "Hadi listener N kwa kila mtiririko, kisha multicast". "fan-in" is kept in the MSRP failure. |
| descriptor | descriptor | Loanword; the descriptor types themselves stay English. |
| cluster | cluster | Loanword; kikundi is kept for (entity) group. |
| stream port | mlango wa mtiririko | Port is mlango; the mapping view shows "mlango N". |
| channel mapping | ramani za chaneli | "kuweka ramani", "kuondoa ramani"; "Haina ramani" for not mapped. |
| control | kidhibiti, vidhibiti | |
| preset | preset | Loanword; "Rejesha" for recall. |
| identify | kutambua | The button is "Tambua". |
| counter | kihesabu, vihesabu | |
| locked, lost lock | kupata lock, kupoteza lock | "ilipata lock mara 3", "ilipoteza lock ya media mara moja". |
| holding over | holdover | Loanword, as telecom engineers say it: "Iko kwenye holdover". |
| station | station | Loanword, class 9/10, as for talker: "station 3". "kituo" would clash with access point, which Swahili software renders "kituo cha ufikiaji". |
| access point | access point | Loanword, class 9/10: "kutoka kwa access point". Kept with station so the two roles read apart. |
| beacon | beacon | Loanword: "Mode B, kutoka kwenye beacon". |
| signal | mawimbi | As phones show signal strength (nguvu ya mawimbi). |
| interrupted | kukatizwa | |
| timestamp | muhuri wa muda | Plural mihuri ya muda. |
| advertise, advertised | kutangaza, imetangazwa | The ADP section is "Utangazaji". |
| interface | kiolesura, violesura | |
| hardware clock | saa ya maunzi | |
| virtual (interface) | pepe | |

Other recurring words: frame "fremu", device "kifaa", network "mtandao",
path "njia", tree "mti", latency and delay "ucheleweshaji", bandwidth
"kipimo data", SR and traffic class "daraja", Log "Kumbukumbu", alarm
"Tahadhari".

## Choices a native speaker should check

- "huluki" for entity and "mtiririko" for stream, rather than keeping the
  English words.
- "ingizo" and "towe" for input and output, from software localization;
  some readers may know only "input" and "output".
- "lock" kept as a loanword ("ilipata lock"), since "kufunga" reads as
  closing.
- "swichi" for bridge, and "controller", "descriptor", "cluster" and
  "preset" kept as loanwords.
- "uhifadhi" for reservation, and "daraja" for SR class.
- "ramani" for channel mapping and "mlango" for port.
