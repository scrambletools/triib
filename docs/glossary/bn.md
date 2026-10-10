# Bengali (bn) terms

How `i18n/bn/triib.ftl` renders the glossary's roles and terms of art.
Terms Bengali-speaking audio and network engineers say in English stay in
Latin script, joined to Bengali case endings with a hyphen (stream-এর,
interface-এ, gPTP tree-তে). They take the English plural where the sense
is plural (সব streams), but stay singular after a number with the
classifier টি, as Bengali grammar has it (3টি stream). Ordinary interface
words are Bengali. Sentences end with the danda (।).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized, as the glossary sets. Talker outputs, Talker-এর reservation. |
| listener | Listener | Latin script, capitalized. |
| grandmaster | Grandmaster | Latin script. Never a word built on মাস্টার or প্রভু. |
| entity | entity, entities | Genitive entity-র. Not ডিভাইস, which is kept for devices on the map. |
| entity model | entity model | |
| controller | controller | |
| stream | stream, streams | |
| stream input, stream output | stream input, stream output | Matrix headers Talker outputs, Listener inputs. |
| connection, connect, bind | সংযোগ, সংযোগ করা, আবদ্ধ | Connected is সংযুক্ত; disconnect is সংযোগ বিচ্ছিন্ন করা. "Bound" is আবদ্ধ, kept apart from সংযুক্ত (connected and receiving). |
| media clock | media clock | Media lock: media lock হয়েছে. |
| clock domain | clock domain | |
| clock source | clock source | An unnamed one is Source 2. |
| sampling rate | sampling rate | |
| bridge | bridge | "switch" stays where the English says switch (an AVB Lite fallback reason). |
| reservation | reservation | Talker-এর reservation ব্যর্থ. |
| egress | egress | Egress port. |
| link | link | link বন্ধ, Link চালু. |
| peer delay | peer delay | Link delay likewise stays English. |
| offset | offset | |
| hop | hop, hops | Grandmaster থেকে 3 hops দূরে. |
| unicast, multicast | unicast, multicast | |
| fan-out | fan-out | lite-fanout is a phrase: প্রতি stream 4টি Listener পর্যন্ত, তারপর multicast. MSRP's fan-in also stays. |
| descriptor | descriptor | Descriptor types: 5 ধরনের descriptor. |
| cluster | cluster | Not গ্রুপ, which is the entity group. |
| stream port | stream port | Shown as port 0. |
| channel mapping | channel mapping | Verbs map করা, unmap করা. |
| control | control | |
| preset | preset | Recalling one: Recall, recall করা. |
| identify | Identify, identify করা | শনাক্ত করা would read as "detect". |
| counter | counter | |
| locked, lost lock | lock হয়েছে, lock হারিয়েছে | |
| holding over | holdover | Latin script, as telecom and PTP engineers say it: Holdover-এ আছে, Station, holdover-এ. |
| station | station, stations | Latin script, as Wi-Fi engineers say it; singular after the classifier (3টি station). |
| access point | access point | Latin script. Genitive access point-এর. |
| beacon | beacon | Latin script: Mode B, beacon থেকে. |
| signal | সিগন্যাল | The Bengali spelling phones and software use for signal strength. |
| interrupted | ব্যাহত | |
| timestamp | timestamp | |
| advertise, advertised | advertise করা, Advertise করা হয়েছে | Also for an entity announcing itself (নিজেকে advertise করা). |
| interface | interface | নেটওয়ার্ক interface. |
| hardware clock | hardware clock | |
| virtual (interface) | virtual | Virtual interfaces. |

## Choices a native speaker should check

- Latin script for the terms of art (stream, entity, clock domain) instead of Bengali spellings (স্ট্রিম, এনটিটি). Many inspector labels end up wholly English.
- আবদ্ধ for "bound". Engineers may simply say bind (bind করা).
- Identify and Recall as Latin buttons beside Bengali ones (নাম বদলান, মুছুন).
- The classifier with ASCII digits (3টি stream, 1,204টি frame), and singular nouns after it.
- পার হচ্ছে for streams passing through a bridge.
- রিসোর্স for MSRP and MMRP resources, rather than সম্পদ.
- ডায়াগনস্টিকস for Diagnostics, পরিদর্শক for Inspector, প্রত্যয়িত for Milan certification.
