# Ukrainian (uk) terms

How `i18n/uk/triib.ftl` renders the glossary's roles and terms of art.

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, not declined: «Потоки Talker», «від Talker». |
| listener | Listener | Latin script, not declined: «немає готового Listener». |
| grandmaster | Grandmaster | Latin script, not declined: «від Grandmaster». Never «майстер». |
| entity | сутність | Not «пристрій»: an entity can be software. 1 сутність, 2 сутності, 5 сутностей. |
| entity model | модель сутності | |
| controller | контролер | |
| stream | потік | Also «аудіопотік». |
| stream input, stream output | вхід потоку, вихід потоку | Plural «входи потоків», «виходи потоків». |
| connection, connect, bind | підключення, підключити, прив'язати | "Bound" is «прив'язано»; disconnect is «відключити». |
| media clock | медіасинхронізація | CRF streams are «потоки медіасинхронізації». |
| clock domain | домен синхронізації | Audio clocks use «синхронізація»; PTP clocks use «годинник» («ідентифікатор годинника»). |
| clock source | джерело синхронізації | |
| sampling rate | частота дискретизації | |
| bridge | комутатор | «AVB-комутатор» where AVB is named. Chosen over «міст». |
| reservation | резервування | |
| egress | вихідний трафік | Egress port is «вихідний порт». |
| link | лінк | Avoids «канал» (audio channel) and «з'єднання» (close to connection). |
| peer delay | Peer delay | Kept in English, as in PTP practice. Link delay is «затримка лінку». |
| offset | зсув | |
| hop | перехід | «Переходів від Grandmaster». |
| unicast, multicast | unicast, multicast | Kept in English, as network engineers write them. |
| fan-out | (phrase) | «До N Listener на потік, далі multicast». Fan-in (MSRP code 15) is «вхідні порти (fan-in)». |
| descriptor | дескриптор | |
| cluster | кластер | Not «група», which is the entity group. |
| stream port | порт потоку | The mapping section shows «порт N». |
| channel mapping | призначення каналів | Map is «призначити», unmap «скасувати призначення», not mapped «не призначено». AUDIO_MAP counts are «карти». |
| control | регулятор | Tab «Регулятори». |
| preset | пресет | Recall is «викликати». |
| identify | ідентифікувати | |
| counter | лічильник | |
| locked, lost lock | захоплення синхронізації, втрата синхронізації | Diagnostics count them as nouns: «3 захоплення синхронізації». Media lock is «захоплення медіасинхронізації». |
| holding over | режим утримання | The synchronization term for holdover; «утримання» in the entity list's short cells. |
| station | станція | The IEEE 802.11 term (STA), rather than «клієнт»: 1 станція, 2 станції, 5 станцій. |
| access point | точка доступу | |
| beacon | Beacon | Latin script, as the frame type: «кадри Beacon». |
| signal | сигнал | The inspector label, in dBm. |
| interrupted | переривання | «Без переривань», «2 переривання». |
| timestamp | мітка часу | |
| advertise, advertised | оголошувати, оголошено | Advertising is «оголошення». |
| interface | інтерфейс | |
| hardware clock | апаратний годинник | |
| virtual (interface) | віртуальний | |

## Choices a native speaker should check

- «сутність» for entity, and «медіасинхронізація» for media clock, with the clock family built on «синхронізація».
- «лінк» for link: what network engineers say, though informal in print.
- «комутатор» for bridge rather than «міст».
- «захоплення синхронізації» for lock, and the diagnostics written as noun counts.
- «регулятор» for control, which also covers mute.
- Peer delay, unicast and multicast kept in English.
- The percent sign without a space («80%»), following CLDR; Ukrainian print often writes «80 %».
- «режим утримання» for holding over, rather than the English «holdover».
- «Похибка підстроювання» for the station's servo error, avoiding «регулятор» (control).
- «кругова затримка» for round-trip time.
