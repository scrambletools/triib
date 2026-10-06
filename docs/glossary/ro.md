# Romanian (ro) term choices

How `i18n/ro/triib.ftl` renders the terms in `docs/GLOSSARY.md`. Buttons
use the imperative ("Salvează", "Încarcă") and the text addresses the user
as *tu*. Quotation marks are „ ”, ș and ț take the comma below, the
thousands separator is `.` and `%` follows a no-break space. Counts take
"de" from 20 on (CLDR *other*: "21 de fluxuri", "2 fluxuri").

| English | Rendering | Note |
|---|---|---|
| talker | talker (n.), talkerul, talkere | Loanword, lower case mid-sentence: „se așteaptă talkerul”, „Ieșiri de talker”. |
| listener | listener (n.), listenerul, listenere | Loanword, as for talker: „niciun listener pregătit”. |
| grandmaster | grandmaster, grandmasterul | Kept in Latin script; never a word built on „master” or „stăpân”. |
| entity | entitate | Not „dispozitiv”, which is kept for *device* in the network view. |
| entity model | model de entitate | |
| controller | controler | |
| stream | flux, fluxuri | |
| stream input, stream output | intrare de flux, ieșire de flux | Matrix corner: „Ieșiri de talker”, „Intrări de listener”. |
| connection, connect, bind | conexiune, a conecta; a asocia | A bound input is „asociată” (intrarea); „legat” is avoided because the link is „legătură”. |
| media clock | ceas media | Also „VLAN media”, „resetare media”. |
| clock domain | domeniu de ceas | |
| clock source | sursă de ceas | |
| sampling rate | frecvență de eșantionare | |
| bridge | switch, switch-ul, switch-uri | What Romanian practitioners call an AVB bridge. |
| reservation | rezervare | |
| egress | de ieșire (trafic de ieșire, port de ieșire) | |
| link | legătură | „Legătură activă”, „Întârziere pe legătură”. |
| peer delay | peer delay | Kept in English, as PTP practitioners say it. |
| offset | decalaj | „Decalaj PTP de −72 µs”. |
| hop | salt, salturi | „La 2 salturi de grandmaster”. |
| unicast, multicast | unicast, multicast | Kept, as practitioners say. |
| fan-out | phrase: „Până la N listenere per flux, apoi multicast” | The MSRP *fan-in* stays „fan-in”. |
| descriptor | descriptor, descriptori | |
| cluster | cluster, clustere | Not „grup”, which is the entity group. |
| stream port | port de flux | The mappings show it as „port N”. |
| channel mapping | maparea canalelor; a mapa, a elimina maparea | „Nemapat”; AUDIO_MAP descriptors are „mapări”. |
| control | control, controale | |
| preset | preset, preseturi | Recall is „a încărca”. |
| identify | a identifica | Button: „Identifică”. |
| counter | contor, contoare | |
| locked, lost lock | calat, calare pierdută | The PLL term; „calat pe ceasul media” for media lock. |
| interrupted | întrerupt | |
| timestamp | marcaj temporal | |
| advertise, advertised | a anunța, anunțat | „a raporta” is kept for *report*. |
| interface | interfață | Up is „activă”. |
| hardware clock | ceas hardware | |
| virtual (interface) | virtuală | |

## Choices a native speaker should check

- **locked → calat, calare.** The least certain choice. It keeps
  „sincronizat” free for *synced* (gPTP); „blocat” would read as *stuck*.
- **bridge → switch** (neuter, „switch-uri”) rather than „comutator”.
- **link → legătură.** Many engineers just say „link”.
- **talker, listener as neuter nouns** (plural „talkere”, „listenere”), like
  other device nouns („playere”, „controlere”).
- **recall → „Încarcă”** (load), as Romanian software says for presets.
- **frame → cadru**, rather than the English „frame”.
