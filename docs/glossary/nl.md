# Dutch (nl) terms

How `i18n/nl/triib.ftl` renders the terms in `docs/GLOSSARY.md`.

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lower case mid-sentence; plural talkers |
| listener | listener | Loanword, lower case mid-sentence; plural listeners |
| grandmaster | grandmaster | Kept in Latin script, lower case mid-sentence |
| entity | entiteit | Plural entiteiten; pronoun haar; not apparaat, since an entity can be software |
| entity model | entiteitsmodel | |
| controller | controller | |
| stream | stream | Plural streams |
| stream input | stream-ingang | Hyphen for readability (streamingang misreads) |
| stream output | stream-uitgang | |
| connection | verbinding | |
| connect | verbinden | Disconnect: loskoppelen |
| bind, bound | koppelen, gekoppeld | "Gekoppeld, wacht op de stream van de talker" |
| media clock | mediaklok | |
| clock domain | klokdomein | |
| clock source | klokbron | |
| sampling rate | samplefrequentie | |
| bridge | switch | Plural switches; AVB-switch where English says AVB bridge |
| reservation | reservering | |
| egress | egress | egress-poort; column heading Egress |
| link | link | linkvertraging, linkonderbrekingen |
| peer delay | peer delay | peer-delay-verzoek |
| offset | offset | |
| hop | hop | Plural hops |
| unicast | unicast | |
| multicast | multicast | |
| fan-out | fan-out | In the interface as "Tot N listeners per stream, daarna multicast" |
| descriptor | descriptor | Plural descriptors |
| cluster | cluster | Plural clusters |
| stream port | streampoort | "poort N" in the mapping list |
| channel mapping | kanaaltoewijzing | Map: toewijzen; unmap: toewijzing opheffen; a map: toewijzing |
| control | regelaar | As in volumeregelaar; plural regelaars |
| preset | preset | Recall: oproepen |
| identify | identificeren | |
| counter | teller | |
| locked | vergrendeld | As in fasevergrendelde lus (PLL) |
| lost lock | vergrendeling verloren | |
| interrupted | onderbroken | |
| timestamp | tijdstempel | |
| advertise, advertised | aankondigen, aangekondigd | Also for "announce itself": zich aankondigen |
| interface | interface | netwerkinterface, AVB-interface |
| hardware clock | hardwareklok | |
| virtual (interface) | virtueel | |

Discovery is detectie (as in Windows' netwerkdetectie): "Detectie actief",
"om entiteiten te detecteren".

## Choices a native speaker should check

- **regelaar** for control. It is short enough for a tab, but a switch such
  as mute is not really a regelaar; bedieningselement is more exact and
  much longer.
- **gekoppeld** for bound, with loskoppelen for disconnect, rather than
  gebonden.
- **samplefrequentie** rather than samplerate or bemonsteringsfrequentie.
- **detectie / detecteren** for discovery.
- **stream-ingang / stream-uitgang** with a hyphen, and closed compounds
  such as talkerstreams and mediaklokstreams.
