# Russian (ru) terms

How `i18n/ru/triib.ftl` renders the glossary's roles and terms of art.

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, not declined: «Потоки Talker», «от Talker». |
| listener | Listener | Latin script, not declined: «нет готового Listener». |
| grandmaster | Grandmaster | Latin script, not declined: «от Grandmaster». Never «мастер». |
| entity | сущность | Not «устройство»: an entity can be software. 1 сущность, 2 сущности, 5 сущностей. |
| entity model | модель сущности | |
| controller | контроллер | |
| stream | поток | Also «аудиопоток». |
| stream input, stream output | вход потока, выход потока | Plural «входы потоков», «выходы потоков». |
| connection, connect, bind | подключение, подключить, привязать | "Bound" is «привязано»; disconnect is «отключить». |
| media clock | медиасинхронизация | CRF streams are «потоки медиасинхронизации». |
| clock domain | домен синхронизации | Audio clocks use «синхронизация»; PTP clocks use «часы» («идентификатор часов»). |
| clock source | источник синхронизации | As in Russian audio-interface manuals. |
| sampling rate | частота дискретизации | |
| bridge | коммутатор | «AVB-коммутатор» where AVB is named. Chosen over «мост», which practitioners rarely say. |
| reservation | резервирование | |
| egress | исходящий трафик | Egress port is «исходящий порт». |
| link | линк | Avoids «канал» (audio channel) and «соединение» (close to connection). |
| peer delay | Peer delay | Kept in English, as in PTP practice. Link delay is «задержка линка». |
| offset | смещение | |
| hop | переход | «Переходов от Grandmaster». |
| unicast, multicast | unicast, multicast | Kept in English, as network engineers write them. |
| fan-out | (phrase) | «До N Listener на поток, затем multicast». Fan-in (MSRP code 15) is «входящие порты (fan-in)». |
| descriptor | дескриптор | |
| cluster | кластер | Not «группа», which is the entity group. |
| stream port | порт потока | The mapping section shows «порт N». |
| channel mapping | назначение каналов | Map is «назначить», unmap «снять назначение», not mapped «не назначено». AUDIO_MAP counts are «карты». |
| control | регулятор | Tab «Регуляторы». |
| preset | пресет | Recall is «вызвать». |
| identify | идентифицировать | |
| counter | счётчик | |
| locked, lost lock | захват синхронизации, потеря синхронизации | Diagnostics count them as nouns: «3 захвата синхронизации». Media lock is «захват медиасинхронизации». |
| interrupted | прерывание | «Без прерываний», «2 прерывания». |
| timestamp | метка времени | |
| advertise, advertised | объявлять, объявлено | Advertising is «объявление». |
| interface | интерфейс | |
| hardware clock | аппаратные часы | |
| virtual (interface) | виртуальный | |

## Choices a native speaker should check

- «сущность» for entity, rather than «объект» from the GOST translations of OSI.
- «медиасинхронизация» for media clock, and the clock family built on «синхронизация» rather than «тактирование» or «клок».
- «линк» for link: what network engineers say, though informal in print.
- «коммутатор» for bridge rather than «мост».
- «регулятор» for control, which also covers mute; «элемент управления» is exact but too long for a tab.
- «захват синхронизации» for lock, and the diagnostics written as noun counts.
- Peer delay, unicast and multicast kept in English.
- «Тревога» for alarm.
