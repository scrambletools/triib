# Finnish (fi) term choices

How `i18n/fi/triib.ftl` renders the terms in `docs/GLOSSARY.md`. The text
addresses the user in the singular imperative (Valitse, Napsauta) and uses
” ” for quotation marks. Numbers use a no-break space for thousands and a
comma for decimals, with a no-break space before the percent sign
(80 %). After numbers other than one, nouns take the partitive
(5 entiteettiä), so the plural variants differ.

Names that triib fills in are kept in the nominative by giving them a
head noun that takes the case instead: «Entiteettiä X ei voitu tunnistaa»,
«Liitännän X entiteetit», «Lähetetään osoitteeseen X».

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lower case mid-sentence; inflected with -i-: «talkerin», «talkeria». Compounds: «Talker-striimit», «Talker-lähdöt». |
| listener | listener | As for talker: «listeneriä», «Listener-tulot». |
| grandmaster | grandmaster | Kept; never a word built on «master» or «isäntä». «grandmasterista», «grandmasteriin». |
| entity | entiteetti | Not «laite», which is kept for *device* in the network view. |
| entity model | entiteettimalli | |
| controller | ohjain | |
| stream | striimi | The word engineers use; «virta» would read as electric current next to power inputs. |
| stream input, stream output | striimitulo, striimilähtö | Matrix corner: «Talker-lähdöt», «Listener-tulot». |
| connection, connect, bind | yhteys, yhdistää; sidottu | Disconnect: «katkaista yhteys». A bound input is «sidottu», kept apart from «yhdistetty» (connected and receiving). |
| media clock | mediakello | Also «mediakellostriimit», «Media-VLAN». |
| clock domain | kelloalue | The gPTP domain is «gPTP-toimialue». |
| clock source | kellolähde | |
| sampling rate | näytteenottotaajuus | |
| bridge | silta | «AVB-silta», «Sillan portti»; the one mention of the switch itself is «kytkin». |
| reservation | varaus | As in «kaistanvaraus». |
| egress | lähtevä liikenne; lähtöportti | Column and alarm: «Lähtevä liikenne»; egress port: «lähtöportti». |
| link | linkki | «Linkki muodostui», «Linkki katkesi», «Linkin katkokset», «Linkin viive». |
| peer delay | peer delay | Kept, as PTP writing does: «peer delay -pyyntö». |
| offset | poikkeama | «PTP-poikkeama». |
| hop | hyppy | «Hypyt grandmasterista». |
| unicast, multicast | unicast, multicast | Kept, as practitioners say them, rather than «täsmälähetys», «monilähetys». |
| fan-out | (phrase) | Not shown as a word; the inspector says «Enintään N listeneriä striimiä kohden, sitten multicast». |
| descriptor | kuvaaja | As in «tiedostokuvaaja». |
| cluster | klusteri | A technical grouping; «ryhmä» stays for entity groups. |
| stream port | striimiportti | |
| channel mapping | kanavakartoitus | Verbs: «kartoittaa», «poistaa kartoitus»; «Ei kartoitettu». |
| control | säädin | «Säätimet» for the tab. |
| preset | esiasetus | «Palauta» for recall. |
| identify | tunnistaa | The button is «Tunnista». |
| counter | laskuri | |
| locked, lost lock | lukittui, menetti lukituksen | Media lock: «lukittui mediakelloon», «mediakellon lukitus». |
| interrupted | keskeytyi | |
| timestamp | aikaleima | |
| advertise, advertised | mainostaa, mainostettu | Also for an entity announcing itself: «mainostaa itseään». The ADP section is «Mainostus». |
| interface | liitäntä | «Verkkoliitäntä», «AVB-liitäntä». |
| hardware clock | laitteistokello | |
| virtual (interface) | virtuaalinen | |

Other recurring words: frame «kehys», discovery «etsintä» (Etsitään),
heard «havaittu» (in the map) and «Vastaanotettu» (in the log), format
«muoto», inspector «Tarkastelu», log «Loki».

## Choices a native speaker should check

- «striimi» for *stream*: the alternatives are «virta» (ambiguous with
  electric current) and English «stream».
- «kanavakartoitus» for *channel mapping*; «kanavien kohdistus» is
  another option.
- «sidottu» for *bound*.
- «kelloalue» for *clock domain* and «toimialue» for the gPTP domain.
- «Tarkastelu» for the Inspector panel and «etsintä» for ADP discovery.
