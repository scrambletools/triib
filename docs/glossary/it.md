# Italian (it) term choices

How `i18n/it/triib.ftl` renders the terms in `docs/GLOSSARY.md`. Buttons
use the imperative ("Salva", "Richiama"), the text addresses the user as
*tu*, quotation marks are “ ”, the thousands separator is `.` and `%`
follows the number with no space.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m., invariable) | Loanword, lower case mid-sentence: “in attesa del talker”, “Uscite dei talker”. |
| listener | listener (m., invariable) | Loanword, as for talker: “nessun listener pronto”. |
| grandmaster | grandmaster (m.) | Kept in Latin script; never a word built on “master” or “principale”. |
| entity | entità | Not “dispositivo”, which is kept for *device* in the network view. |
| entity model | modello di entità | |
| controller | controller (m.) | The word Italian audio and network practitioners use; not “controllore”. |
| stream | flusso | |
| stream input, stream output | ingresso di flusso, uscita di flusso | Matrix corner: “Uscite dei talker”, “Ingressi dei listener”. |
| connection, connect, bind | connessione, connettere; associare | A bound input is “associato” (l’ingresso). “collegamento” is kept for *link*. |
| media clock | media clock (m.) | Kept in English: “clock media” would read as “average clock”. Clock is “clock” throughout, as Italian audio engineers say. |
| clock domain | dominio di clock | |
| clock source | sorgente di clock | |
| sampling rate | frequenza di campionamento | |
| bridge | switch (m., invariable) | What Italian practitioners call an AVB bridge: “switch AVB”. |
| reservation | prenotazione | As in “prenotazione delle risorse”. |
| egress | in uscita (traffico in uscita, porta in uscita) | Avoids bare “uscita”, which is kept for *output*. |
| link | collegamento | “Collegamento attivo”, “Ritardo del collegamento”. |
| peer delay | peer delay (m.) | Kept in English, as PTP practitioners say it. |
| offset | offset (m.) | Kept, the usual Italian technical word. |
| hop | hop (m., invariable) | “A 2 hop dal grandmaster”. |
| unicast, multicast | unicast, multicast | Kept, as practitioners say. |
| fan-out | phrase: “Fino a N listener per flusso, poi multicast” | The MSRP *fan-in* stays “fan-in”. |
| descriptor | descrittore | |
| cluster | cluster (m., invariable) | Not “gruppo”, which is the entity group. |
| stream port | porta di flusso | The mappings show it as “porta N”. |
| channel mapping | mappatura dei canali; mappare, rimuovere la mappatura | “Non mappato”; AUDIO_MAP descriptors are “mappature”. |
| control | controllo | |
| preset | preset (m., invariable) | Recall is “richiamare”, as for scenes on a desk. |
| identify | identificare | Button: “Identifica”. |
| counter | contatore | |
| locked, lost lock | agganciato, aggancio perso | The PLL term; “agganciato al media clock” for media lock. |
| interrupted | interrotto | |
| timestamp | timestamp (m., invariable) | Rather than “marca temporale”. |
| advertise, advertised | annunciare, annunciato | “segnalare” is kept for *report*. |
| interface | interfaccia | Up is “attiva”. |
| hardware clock | clock hardware | |
| virtual (interface) | virtuale | |

## Choices a native speaker should check

- **media clock** kept in English, while clock domain and clock source are
  “dominio di clock”, “sorgente di clock”.
- **reservation → prenotazione.** “riserva (di banda)” is the alternative.
- **link → collegamento.** Many engineers just say “link”; if so, change
  the link messages together.
- **frame** (m.) for Ethernet frames, rather than the textbook “trama”.
- **Inspector** and **Log** kept as in Italian software, rather than
  “Ispettore” and “Registro”.
- **pre-empted → “prelazione da parte di un flusso di rango superiore”**
  (MSRP code 6).
