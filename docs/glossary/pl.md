# Polish (pl) terms

How `i18n/pl/triib.ftl` renders the glossary's roles and terms of art.

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lowercase in a sentence, inflected as an inanimate noun: talkera, talkerem; plural talkery, talkerów. |
| listener | listener | As talker: listenera, listenerem; plural listenery, listenerów. |
| grandmaster | grandmaster | Inflected: grandmastera, grandmasterem. Never «nadrzędny» or «master». |
| entity | encja | Not «urządzenie»: an entity can be software. 1 encja, 2 encje, 5 encji. |
| entity model | model encji | |
| controller | kontroler | |
| stream | strumień | «Strumień audio», «strumień zegara mediów». |
| stream input, stream output | wejście strumienia, wyjście strumienia | Plural «wejścia strumieni», «wyjścia strumieni». |
| connection, connect, bind | połączenie, połączyć (z), powiązać | "Bound" is «powiązano»; disconnect is «rozłączyć». States use the impersonal forms: Połączono, Powiązano. |
| media clock | zegar mediów | Media lock is «synchronizacja z zegarem mediów». |
| clock domain | domena zegara | |
| clock source | źródło zegara | As in Polish audio-interface manuals. |
| sampling rate | częstotliwość próbkowania | |
| bridge | przełącznik | «Przełącznik AVB» where AVB is named. Chosen over «most». |
| reservation | rezerwacja | |
| egress | ruch wychodzący | Egress port is «port wyjściowy». |
| link | łącze | |
| peer delay | peer delay | Kept in English, as in PTP practice. Link delay is «opóźnienie łącza». |
| offset | przesunięcie | |
| hop | przeskok | «Przeskoki od grandmastera». |
| unicast, multicast | unicast, multicast | As Polish networking writes them. |
| fan-out | (phrase) | «Do N listenerów na strumień, potem multicast». Fan-in (MSRP code 15) is «porty wejściowe (fan-in)». |
| descriptor | deskryptor | |
| cluster | klaster | Not «grupa», which is the entity group. |
| stream port | port strumienia | The mapping section shows «port N». |
| channel mapping | mapowanie kanałów | Map is «zmapować», unmap «usunąć mapowanie», not mapped «nie zmapowano». AUDIO_MAP counts are «mapy». |
| control | regulator | Tab «Regulatory». |
| preset | preset | Recall is «przywołaj». |
| identify | identyfikuj | Button; the verb in sentences is «zidentyfikować». |
| counter | licznik | |
| locked, lost lock | synchronizacja, utrata synchronizacji | Diagnostics count them as nouns: «3 synchronizacje, 1 utrata synchronizacji». |
| interrupted | przerwa | «Bez przerw», «2 przerwy». |
| timestamp | znacznik czasu | |
| advertise, advertised | ogłaszać, ogłoszono | Advertising is «ogłaszanie». |
| interface | interfejs | |
| hardware clock | zegar sprzętowy | |
| virtual (interface) | wirtualny | |

## Choices a native speaker should check

- «encja» for entity, rather than «jednostka» (which also reads as "unit") or «podmiot».
- Talker and listener inflected as inanimate nouns, with plurals «talkery», «listenery»; some readers may expect animate forms.
- «przełącznik» for bridge rather than «most» or «switch».
- «regulator» for control, which also covers mute; «element sterujący» is exact but long.
- «zegar mediów» for media clock, and «synchronizacja» counted as lock events in the diagnostics.
- Peer delay and «Firmware» kept in English.
- The percent sign without a space («80%»), following CLDR.
