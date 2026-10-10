# Hindi (hi) terms

How `i18n/hi/triib.ftl` renders the glossary's roles and terms of art.
Terms Indian audio and network engineers say in English stay in Latin
script, written in their English singular or plural, with Hindi grammar
around them (Talker का reservation, streams छिपी हैं). Ordinary interface
words are Hindi, or the loanword Hindi software already spells in Devanagari
(सेटिंग्स, नेटवर्क, कनेक्ट). Sentences end with the danda (।).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized, as the glossary sets. Talker outputs, Talker का reservation. |
| listener | Listener | Latin script, capitalized. Plural Listeners (lite-fanout). |
| grandmaster | Grandmaster | Latin script. Never a word built on मास्टर or स्वामी. |
| entity | entity, entities | Feminine in agreement (entities पढ़ी जाने के बाद). Not डिवाइस, which is kept for devices on the map. |
| entity model | entity model | |
| controller | controller | |
| stream | stream, streams | Feminine in agreement (stream छिपी है), as स्ट्रीम is in Hindi. |
| stream input, stream output | stream input, stream output | Matrix headers Talker outputs, Listener inputs. |
| connection, connect, bind | कनेक्शन, कनेक्ट करना, बँधा | Disconnect is डिसकनेक्ट करना. "Bound" is बँधा (बँधी for streams), kept apart from कनेक्ट है (connected and receiving). |
| media clock | media clock | Media lock: media lock हुआ. |
| clock domain | clock domain | |
| clock source | clock source | An unnamed one is Source 2. |
| sampling rate | sampling rate | |
| bridge | bridge | "switch" stays where the English says switch (an AVB Lite fallback reason). |
| reservation | reservation | Talker का reservation विफल. |
| egress | egress | Egress port. |
| link | link | link बंद, Link चालू. |
| peer delay | peer delay | Link delay likewise stays English. |
| offset | offset | |
| hop | hop, hops | Grandmaster से 3 hops. |
| unicast, multicast | unicast, multicast | |
| fan-out | fan-out | lite-fanout is a phrase: हर stream 4 Listeners तक, फिर multicast. MSRP's fan-in also stays. |
| descriptor | descriptor | Descriptor types: 5 प्रकार के descriptors. |
| cluster | cluster | Not समूह, which is the entity group. |
| stream port | stream port | Shown as port 0. |
| channel mapping | channel mapping | Verbs map करना, unmap करना. |
| control | control | |
| preset | preset | Recalling one: Recall, recall करना. |
| identify | Identify, identify करना | पहचानना would read as "recognize". |
| counter | counter | |
| locked, lost lock | lock हुआ, lock टूटा | |
| holding over | holdover | Latin script, as telecom and PTP engineers say it: Holdover में, Station, holdover में. |
| station | station, stations | Latin script, as Wi-Fi engineers say it. Access point, 3 stations. |
| access point | access point | Latin script. The map label and the role are Access point. |
| beacon | beacon, beacons | Latin script: Mode B, beacons से. |
| signal | सिग्नल | The Devanagari spelling Hindi phones and software use for signal strength. |
| interrupted | बाधित | |
| timestamp | timestamp | |
| advertise, advertised | advertise करना, Advertise हुई | Also for an entity announcing itself (खुद को advertise करना). |
| interface | interface | नेटवर्क interface. |
| hardware clock | hardware clock | |
| virtual (interface) | virtual | Virtual interfaces. |

## Choices a native speaker should check

- Latin script for the terms of art (stream, entity, clock domain) instead of the Devanagari spellings Hindi software often uses (स्ट्रीम, एंटिटी). Many inspector labels end up wholly English.
- बँधा / बँधी for "bound". Engineers may simply say bind (bind है).
- Identify and Recall as Latin buttons beside Hindi ones (नाम बदलें, हटाएँ).
- Feminine agreement for stream and entity, masculine for interface, input, reservation and link.
- आर-पार for streams passing through a bridge ("4 आर-पार").
- Ordinary computing words in Devanagari: नेटवर्क, फ़र्मवेयर, कैश, सिंक, लॉग, डिवाइस; निरीक्षक for Inspector and निदान for Diagnostics.
- निर्माता for Vendor; Hindi IT writing often says वेंडर.
