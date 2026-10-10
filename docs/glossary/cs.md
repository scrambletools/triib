# Czech (cs) terms

How `i18n/cs/triib.ftl` renders the glossary's roles and terms of art.

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lowercase in a sentence, inflected as an inanimate noun like server: talkeru, talkerem; plural talkery, talkerů. |
| listener | listener | As talker: listeneru, listenerem; plural listenery, listenerů. |
| grandmaster | grandmaster | Inflected: grandmasteru, grandmasterem. Never «master» or «nadřízené hodiny». |
| entity | entita | Not «zařízení»: an entity can be software. 1 entita, 2 entity, 5 entit. |
| entity model | model entity | |
| controller | kontrolér | |
| stream | stream | Streamu, streamy, streamů. «Audio stream», «stream mediálních hodin». |
| stream input, stream output | vstup streamu, výstup streamu | Plural «vstupy streamů», «výstupy streamů». |
| connection, connect, bind | připojení, připojit, přiřadit | "Bound" is «přiřazeno»; disconnect is «odpojit». |
| media clock | mediální hodiny | Media lock is «synchronizace s mediálními hodinami». |
| clock domain | hodinová doména | |
| clock source | zdroj hodin | |
| sampling rate | vzorkovací frekvence | |
| bridge | přepínač | «Přepínač AVB» where AVB is named. Chosen over «most». |
| reservation | rezervace | |
| egress | odchozí provoz | Egress port is «odchozí port». |
| link | linka | Link up / down: «linka aktivní» / «linka neaktivní». |
| peer delay | peer delay | Kept in English, as in PTP practice. Link delay is «zpoždění linky». |
| offset | odchylka | |
| hop | skok | «Skoky od grandmasteru». |
| unicast, multicast | unicast, multicast | As Czech networking writes them. |
| fan-out | (phrase) | «Až N listenerů na stream, pak multicast». Fan-in (MSRP code 15) is «vstupní porty (fan-in)». |
| descriptor | deskriptor | |
| cluster | klastr | Not «skupina», which is the entity group. |
| stream port | port streamu | The mapping section shows «port N». |
| channel mapping | mapování kanálů | Map is «namapovat», unmap «zrušit mapování», not mapped «nenamapováno». AUDIO_MAP counts are «mapy». |
| control | ovládací prvek | Tab «Ovládací prvky». |
| preset | preset | Recall is «vyvolat». |
| identify | identifikovat | |
| counter | čítač | |
| locked, lost lock | synchronizace, ztráta synchronizace | Diagnostics count them as nouns: «3 synchronizace, 1 ztráta synchronizace». |
| holding over | holdover | Kept in English, as timing practice says it: «Režim holdover», «v režimu holdover». |
| station | stanice | The IEEE 802.11 term, rather than «klient»: 1 stanice, 5 stanic. «Stanice, synchronizovaná» in the entity list. |
| access point | přístupový bod | |
| beacon | beacon | In «rámce beacon»: «Mode B, z rámců beacon». |
| signal | signál | The inspector label, in dBm. |
| interrupted | přerušení | «Bez přerušení», «2 přerušení». |
| timestamp | časová značka | |
| advertise, advertised | ohlašovat, ohlášeno | Advertising is «ohlašování». |
| interface | rozhraní | |
| hardware clock | hardwarové hodiny | |
| virtual (interface) | virtuální | |

## Choices a native speaker should check

- «stream» for stream, rather than «datový tok» or «proud».
- «přepínač» for bridge rather than «switch», which many network engineers say.
- «kontrolér» for controller rather than «řadič».
- «Protokol» for the Log panel, as Windows names logs; in a network tool it may read as "protocol".
- «odchylka» for offset rather than «offset».
- «přiřazeno» for a bound input.
- «mediální hodiny» for media clock, and «synchronizace» counted as lock events in the diagnostics.
- Peer delay kept in English.
- «holdover» kept in English for holding over.
- Wireless locked written as «Synchronizováno», the same word as the map's gPTP «Sync».
- «Chyba serva» for the station's servo error.
