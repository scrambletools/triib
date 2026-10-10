# Norwegian Bokmål (nb) terms

How `i18n/nb/triib.ftl` renders the terms in `docs/GLOSSARY.md`.

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword; talkeren, plural talkere |
| listener | listener | Loanword; listeneren, plural listenere |
| grandmaster | grandmaster | Kept in Latin script; grandmasteren |
| entity | entitet | Plural entiteter; not enhet, which renders device |
| entity model | entitetsmodell | |
| controller | controller | Kept in English so it never clashes with kontroller, the plural of kontroll (control) |
| stream | stream | en stream, streamen, plural streamer; not strøm, which reads as power |
| stream input | stream-inngang | |
| stream output | stream-utgang | |
| connection | tilkobling | |
| connect | koble til | Disconnect: koble fra |
| bind, bound | binde, bundet | |
| media clock | medieklokke | |
| clock domain | klokkedomene | |
| clock source | klokkekilde | |
| sampling rate | samplingsfrekvens | |
| bridge | switch | Plural switcher; AVB-switch where English says AVB bridge |
| reservation | reservasjon | |
| egress | egress | egress-porten; column heading Egress |
| link | link | en link; linkforsinkelse, linkbrudd |
| peer delay | peer delay | peer delay-forespørsel |
| offset | offset | |
| hop | hopp | Plural hopp |
| unicast | unicast | |
| multicast | multicast | |
| fan-out | fan-out | In the interface as "Opptil N listenere per stream, deretter multicast" |
| descriptor | deskriptor | Plural deskriptorer |
| cluster | klynge | Plural klynger, as in dataklynge |
| stream port | streamport | "port N" in the mapping list |
| channel mapping | kanaltilordning | Map: tilordne; unmap: fjerne tilordningen; a map: tilordning |
| control | kontroll | As in volumkontroll; plural kontroller |
| preset | preset | Plural presets; recall: hente |
| identify | identifisere | Button: Identifiser |
| counter | teller | |
| locked | låst | |
| lost lock | mistet låsing | |
| holding over | i holdover | Kept, as telecom writing does: Wi-Fi-tid i holdover |
| station | stasjon | Plural stasjoner; the IEEE 802.11 term |
| access point | tilgangspunkt | tilgangspunktet, as on router settings pages |
| beacon | beacon | beacon-rammer; burst is burst, plural burster |
| signal | signal | As in signalstyrke |
| interrupted | avbrutt | |
| timestamp | tidsstempel | |
| advertise, advertised | annonsere, annonsert | Also for "announce itself": annonsere seg |
| interface | grensesnitt | nettverksgrensesnitt, AVB-grensesnitt |
| hardware clock | maskinvareklokke | |
| virtual (interface) | virtuell | virtuelt with grensesnitt |

Frames are rammer; discovery is oppdage ("Oppdager som …").

## Choices a native speaker should check

- **switch** rather than the Språkrådet spelling svitsj.
- **klynge** for cluster rather than the loan cluster.
- **controller** kept in English, with **kontroll** for control.
- **maskinvareklokke** rather than hardwareklokke.
- **link** rather than lenke for a network link.
- **låst / mistet låsing** for locked and lost lock.
- **tilgangspunkt** rather than aksesspunkt, **stasjon** for a Wi-Fi
  station rather than klient, and **i holdover** kept in English.
