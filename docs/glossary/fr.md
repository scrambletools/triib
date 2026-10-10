# French (fr) term choices

How `i18n/fr/triib.ftl` renders the terms in `docs/GLOSSARY.md`. Buttons
use the infinitive ("Enregistrer"), instructions address the user as
*vous*. Typography: a no-break space before `:` and inside « », a narrow
no-break space (U+202F) before `;`, before `%` and as the thousands
separator.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m.), talkers | Loanword, lower case mid-sentence: « en attente du talker », « Flux talker ». |
| listener | listener (m.), listeners | Loanword, as for talker: « aucun listener prêt ». |
| grandmaster | grandmaster (m.) | Kept in Latin script; never « maître ». |
| entity | entité | Not « appareil », which is kept for *device* in the network view. |
| entity model | modèle d’entité | |
| controller | contrôleur | |
| stream | flux (m., invariable) | |
| stream input, stream output | entrée de flux, sortie de flux | Matrix corner: « Sorties des talkers », « Entrées des listeners ». |
| connection, connect, bind | connexion, connecter; lier | A bound input is « liée » (l’entrée); in the map, « 3 liés » (flux). |
| media clock | horloge média | Also « VLAN média », « réinitialisation média ». |
| clock domain | domaine d’horloge | |
| clock source | source d’horloge | |
| sampling rate | fréquence d’échantillonnage | |
| bridge | commutateur | The French word for an AVB switch (« commutateur AVB »); not « pont ». |
| reservation | réservation | |
| egress | sortant (trafic sortant, port sortant) | Avoids « sortie », which is kept for *output*. |
| link | liaison | « Liaison active », « Délai de liaison », « Coupures de liaison ». |
| peer delay | peer delay (m.) | Kept in English, as PTP practitioners say it. |
| offset | décalage | « Décalage PTP de −72 µs ». |
| hop | saut | « À 2 sauts du grandmaster ». |
| unicast, multicast | unicast, multicast | Kept, as practitioners say; not « monodiffusion », « multidiffusion ». |
| fan-out | phrase: « jusqu’à N listeners par flux, puis multicast » | The MSRP *fan-in* stays « fan-in ». |
| descriptor | descripteur | |
| cluster | cluster (m.) | Not « groupe », which is the entity group. |
| stream port | port de flux | The mappings show it as « port N ». |
| channel mapping | mappage des canaux; mapper, supprimer le mappage | « Non mappé »; AUDIO_MAP descriptors are « mappages ». |
| control | contrôle | Not « commande », which is kept for the shell command. |
| preset | preset (m.), presets | Recall is « rappeler », as for scenes on a desk. |
| identify | identifier | |
| counter | compteur | |
| locked, lost lock | verrouillé, verrouillage perdu | « verrouillé sur l’horloge média » for media lock. |
| holding over | en holdover | Kept in English, as synchronization practitioners say it; ITU-T French writes « mode de maintien ». |
| station | station (f.) | The IEEE 802.11 term: « Station, verrouillée ». |
| access point | point d’accès (m.) | |
| beacon | balise (f.) | « Mode B, par les balises ». |
| signal | signal | The inspector label, in dBm. |
| interrupted | interrompu | |
| timestamp | horodatage | |
| advertise, advertised | annoncer, annoncé | « signaler » is kept for *report*. |
| interface | interface (f.) | Up is « active ». |
| hardware clock | horloge matérielle | |
| virtual (interface) | virtuelle | |

## Choices a native speaker should check

- **bridge → commutateur.** Live-sound engineers often say « switch AVB »;
  « commutateur » reads more like a manual. Change both together if
  « switch » fits better.
- **preset** kept, against « préréglage » (used in some French software).
- **peer delay** kept in English rather than « délai entre pairs ».
- **bind → lier, « liée ».** « associée » was the alternative. « liaison »
  is the *link*, from the same root.
- **channel mapping → mappage, mapper.** Common in French software;
  « assignation des canaux » is the more purist audio term.
- **egress → « port sortant »**, chosen so one word covers the column
  (« Trafic sortant ») and the MSRP codes; « port de sortie » is also common.
- **Firmware** kept rather than « micrologiciel ».
- **holding over → « en holdover »**, rather than « en maintien » (ITU-T).
- **beacon → « balise »**; many Wi-Fi engineers say « beacon ».
- **Time → « Temps »** for how a station gets its time, and « Aucune
  synchronisation » for the mode « No time ».
