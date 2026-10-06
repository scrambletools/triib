# Catalan (ca) term choices

How `i18n/ca/triib.ftl` renders the terms in `docs/GLOSSARY.md`. As in
Softcatalà's style, buttons and menu items use the second person singular
imperative ("Desa", "Recupera") and instructions address the user as
*vós* ("Feu clic"). Quotation marks are « », the thousands separator is
`.` and `%` follows a no-break space.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m.), talkers | Loanword, lower case mid-sentence: «esperant el talker», «Sortides dels talkers». |
| listener | listener (m.), listeners | Loanword, as for talker: «cap listener a punt». |
| grandmaster | grandmaster (m.) | Kept in Latin script; never «mestre». |
| entity | entitat | Not «dispositiu», which is kept for *device* in the network view. |
| entity model | model d’entitat | |
| controller | controlador | |
| stream | flux (m.), fluxos | |
| stream input, stream output | entrada de flux, sortida de flux | Matrix corner: «Sortides dels talkers», «Entrades dels listeners». |
| connection, connect, bind | connexió, connectar; vincular | A bound input is «vinculada» (l’entrada). |
| media clock | rellotge de mitjans | Also «VLAN de mitjans», «reinici de mitjans». |
| clock domain | domini de rellotge | |
| clock source | font de rellotge | |
| sampling rate | freqüència de mostreig | |
| bridge | commutador | The Catalan word for a switch: «commutador AVB». |
| reservation | reserva | |
| egress | de sortida (trànsit de sortida, port de sortida) | |
| link | enllaç | «Enllaç actiu», «Retard de l’enllaç». |
| peer delay | peer delay (m.) | Kept in English, as PTP practitioners say it. |
| offset | decalatge | As in «decalatge horari». |
| hop | salt | «A 2 salts del grandmaster». |
| unicast, multicast | unicast, multicast | Kept, as practitioners say. |
| fan-out | phrase: «Fins a N listeners per flux, després multicast» | The MSRP *fan-in* stays «fan-in». |
| descriptor | descriptor | |
| cluster | clúster | Not «grup», which is the entity group. |
| stream port | port de flux | The mappings show it as «port N». |
| channel mapping | mapatge de canals; mapar, treure el mapatge | «No mapat»; AUDIO_MAP descriptors are «mapatges». |
| control | control | |
| preset | preset (m.), presets | Recall is «recuperar». |
| identify | identificar | Button: «Identifica». |
| counter | comptador | |
| locked, lost lock | enganxat, enganxament perdut | «enganxat al rellotge de mitjans» for media lock. |
| interrupted | interromput | |
| timestamp | marca de temps | |
| advertise, advertised | anunciar, anunciat | «notificar» is kept for *report*. |
| interface | interfície | Up is «activa». |
| hardware clock | rellotge de maquinari | |
| virtual (interface) | virtual | |

## Choices a native speaker should check

- **locked → enganxat, enganxament.** The least certain choice. It keeps
  «sincronitzat» free for *synced* (gPTP); «bloquejat» would read as
  *blocked*.
- **media clock → rellotge de mitjans.** «rellotge multimèdia» is the
  alternative; engineers may just say «media clock».
- **offset → decalatge.** «desfasament» or the English «offset» are the
  alternatives.
- **preset** and **firmware** kept as practitioners say, rather than
  «preconfiguració» and «microprogramari».
- **report → notificar** («Camí no notificat»).
