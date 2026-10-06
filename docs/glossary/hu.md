# Hungarian (hu) term choices

How `i18n/hu/triib.ftl` renders the terms in `docs/GLOSSARY.md`. The text
addresses the user formally (Válasszon, Kattintson) and uses „ ” for
quotation marks. Numbers use a no-break space for thousands and a comma
for decimals, and the percent sign follows the number with no space
(80%). Nouns stay singular after a number, so most plural variants read
the same.

Hungarian puts *a* or *az* before a noun depending on its first sound, and
inflects names. Messages therefore keep names that triib fills in (entity,
stream, interface, file names) out of the inflected positions, after a
colon or in parentheses: «Megnyitás: enp6s0.», «A csatlakoztatás nem
sikerült (X → Y): ok.»

| English | Rendering | Note |
|---|---|---|
| talker | talker, talkerek | Loanword, lower case mid-sentence; takes suffixes directly: «a talkerre», «a talker foglalása», «Talker-streamek». |
| listener | listener, listenerek | As for talker: «nincs kész listener», «Listener-bemenetek». |
| grandmaster | grandmaster | Kept; never a word built on «mester». «a grandmastertől», «Grandmaster-váltások». |
| entity | entitás | Not «eszköz», which is kept for *device* in the network view. |
| entity model | entitásmodell | «Entitásmodell-ID». |
| controller | vezérlő | |
| stream | stream, streamek | The word Hungarian engineers use; «adatfolyam» is the formal alternative. |
| stream input, stream output | streambemenet, streamkimenet | Written as one word, as the spelling rules give for a foreign word ending in a pronounced letter. |
| connection, connect, bind | kapcsolat, csatlakoztat; hozzárendelve | Disconnect: «leválaszt». A bound input is «hozzárendelve», kept apart from «csatlakoztatva» (connected and receiving). |
| media clock | médiaórajel | Audio clocks are «órajel»; the PTP clocks of devices (hardware clock, clock identity) are «óra». |
| clock domain | órajeltartomány | The gPTP domain is «gPTP-tartomány». |
| clock source | órajelforrás | |
| sampling rate | mintavételi frekvencia | |
| bridge | híd | «AVB-híd», «Hídport»; the one mention of the switch itself is «switch». |
| reservation | foglalás | As in «sávszélesség-foglalás». |
| egress | kimenő (forgalom, port) | Column and alarm: «Kimenő forgalom»; egress port: «kimenő port». |
| link | link | The word network engineers use; «kapcsolat» stays for *connection*. «Link létrejött», «Link megszakadt», «Linkvesztések». |
| peer delay | peer delay | Kept, as PTP writing does: «peer delay kérés». |
| offset | eltérés | «PTP-eltérés». |
| hop | ugrás | «Ugrások a grandmastertől». |
| unicast, multicast | unicast, multicast | Kept, as practitioners say them. |
| fan-out | (phrase) | Not shown as a word; the inspector says «Streamenként legfeljebb N listener, utána multicast». |
| descriptor | leíró | As in «fájlleíró». |
| cluster | klaszter | A technical grouping; «csoport» stays for entity groups. |
| stream port | streamport | |
| channel mapping | csatornakiosztás | Verbs: «kioszt», «a kiosztás megszüntetése»; «Nincs kiosztva». |
| control | vezérlőelem | Kept apart from «vezérlő» (controller). |
| preset | preset, presetek | The word desks and their manuals use; «Visszahívás» for recall. |
| identify | azonosítás | |
| counter | számláló | |
| locked, lost lock | zárolódott, elvesztette a zárolást | Media lock: «zárolódott a médiaórajelre», «médiaórajel-zárolás». |
| interrupted | megszakadt | |
| timestamp | időbélyeg | |
| advertise, advertised | meghirdet, meghirdetve | Also for an entity announcing itself: «hirdesse meg magát». The ADP section is «Hirdetés». |
| interface | interfész | «AVB-interfész», «Hálózati interfész». |
| hardware clock | hardveróra | |
| virtual (interface) | virtuális | |

Other recurring words: frame «keret», discovery «felderítés», heard
«észlelt» (in the map) and «Fogadott» (in the log), format «formátum»,
inspector «Vizsgáló», log «Napló».

## Choices a native speaker should check

- «zárolódott» / «zárolás» for clock *lock*: engineers may say «lockol»,
  and electronics texts sometimes use «befogás».
- «stream» kept, with closed compounds such as «streambemenet» and
  «streamport»; some writers would hyphenate them or use «adatfolyam».
- «hozzárendelve» for *bound* and «csatornakiosztás» for *channel
  mapping*, chosen so the two do not share a word.
- «link» rather than «összeköttetés» for *link*.
- «óra» (PTP clocks) versus «órajel» (audio clocks), as in «Óraazonosító»
  but «Órajelforrás» and «Órajelfa».
