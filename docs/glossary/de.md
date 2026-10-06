# German (de) terms

How `i18n/de/triib.ftl` renders the terms in `docs/GLOSSARY.md`.

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Loanword, capitalized as a noun; plural Talker |
| listener | Listener | Loanword, capitalized as a noun; plural Listener |
| grandmaster | Grandmaster | Kept as written |
| entity | Entität | Plural Entitäten; not Gerät, since an entity can be software |
| entity model | Entitätsmodell | |
| controller | Controller | Loanword, as for DAW and network controllers |
| stream | Stream | der Stream, plural Streams |
| stream input | Stream-Eingang | |
| stream output | Stream-Ausgang | |
| connection | Verbindung | |
| connect | verbinden | Disconnect: trennen |
| bind, bound | binden, gebunden | "Gebunden, wartet auf den Stream des Talkers" |
| media clock | Media-Clock | die Clock, as in Wordclock and Clock-Quelle |
| clock domain | Clock-Domäne | |
| clock source | Clock-Quelle | |
| sampling rate | Abtastrate | |
| bridge | Switch | der Switch, plural Switches; AVB-Switch where English says AVB bridge |
| reservation | Reservierung | |
| egress | Egress | Egress-Port; column heading Egress |
| link | Link | der Link; Link-Verzögerung, Link-Abbrüche |
| peer delay | Peer-Delay | Peer-Delay-Anfrage |
| offset | Offset | |
| hop | Hop | Plural Hops |
| unicast | Unicast | |
| multicast | Multicast | |
| fan-out | Fan-out | In the interface as "Bis zu N Listener je Stream, dann Multicast" |
| descriptor | Deskriptor | Plural Deskriptoren |
| cluster | Cluster | Plural Cluster |
| stream port | Stream-Port | "Port N" in the mapping list |
| channel mapping | Kanalzuordnung | Map: zuordnen; unmap: Zuordnung aufheben; a map: Zuordnung |
| control | Bedienelement | Covers knobs and switches alike (gain, mute, selectors) |
| preset | Preset | Recall: abrufen |
| identify | identifizieren | Button: Identifizieren |
| counter | Zähler | |
| locked | eingerastet | As a PLL rastet ein |
| lost lock | Einrastung verloren | |
| interrupted | unterbrochen | |
| timestamp | Zeitstempel | |
| advertise, advertised | ankündigen, angekündigt | Also for "announce itself": sich ankündigen |
| interface | Schnittstelle | Netzwerkschnittstelle, AVB-Schnittstelle |
| hardware clock | Hardware-Clock | |
| virtual (interface) | virtuell | |

Instructions avoid both Sie and du: buttons and hints use the infinitive
("Zum Trennen klicken", "Hintergrund anklicken, um …").

## Choices a native speaker should check

- **Entität** for entity. Some German AVB writing keeps "Entity"/"Entities".
- **Clock** loanword for the audio clock (Media-Clock, Clock-Quelle,
  Clock-Domäne, Clock-Baum) rather than Takt (Medientakt, Taktquelle,
  Taktdomäne).
- **Abtastrate** rather than Samplerate.
- **Switch** for bridge throughout, including counts and "Switch-Port".
- **Bedienelement** for control, rather than Regler or Steuerelement.
- **eingerastet / Einrastung verloren** for locked and lost lock.
- **Frame** (der Frame) rather than Rahmen.
