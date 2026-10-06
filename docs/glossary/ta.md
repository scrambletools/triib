# Tamil (ta) terms

How `i18n/ta/triib.ftl` renders the glossary's roles and terms of art.
Terms Tamil-speaking audio and network engineers say in English stay in
Latin script, written in their English singular or plural, joined to Tamil
case endings with a hyphen (stream-ஐ, entity-இன், interface-இல்,
Grandmaster-இலிருந்து). Ordinary interface words are Tamil, as Tamil
software writes them (அமைப்புகள், மூடு, சாதனம், கணினி, கருவிப்பட்டி).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized, as the glossary sets. Talker outputs, Talker-இன் reservation. |
| listener | Listener | Latin script, capitalized. Plural Listeners (lite-fanout). |
| grandmaster | Grandmaster | Latin script. Never a word built on மாஸ்டர் or தலைமை. |
| entity | entity, entities | Not சாதனம், which is kept for devices on the map. |
| entity model | entity model | |
| controller | controller | |
| stream | stream, streams | |
| stream input, stream output | stream input, stream output | Matrix headers Talker outputs, Listener inputs. |
| connection, connect, bind | இணைப்பு, இணை, பிணைக்கப்பட்டது | Connected is இணைக்கப்பட்டது; disconnect is துண்டி. "Bound" is பிணைக்கப்பட்டது, kept apart from இணைக்கப்பட்டது (connected and receiving). |
| media clock | media clock | Media lock: media lock ஆனது. |
| clock domain | clock domain | |
| clock source | clock source | An unnamed one is Source 2. |
| sampling rate | sampling rate | |
| bridge | bridge | "switch" stays where the English says switch (an AVB Lite fallback reason). |
| reservation | reservation | Talker-இன் reservation தோல்வி. |
| egress | egress | Egress port. |
| link | link | link இல்லை, Link இணைவு, Link துண்டிப்பு. |
| peer delay | peer delay | Link delay likewise stays English. |
| offset | offset | |
| hop | hop, hops | Grandmaster-இலிருந்து 3 hops. |
| unicast, multicast | unicast, multicast | |
| fan-out | fan-out | lite-fanout is a phrase: ஒரு stream-க்கு 4 Listeners வரை, பிறகு multicast. MSRP's fan-in also stays. |
| descriptor | descriptor | Descriptor types: 5 descriptor வகைகள். |
| cluster | cluster | Not குழு, which is the entity group. |
| stream port | stream port | Shown as port 0. |
| channel mapping | channel mapping | Verbs map செய், unmap செய். |
| control | control | |
| preset | preset | Recalling one: Recall, recall செய். |
| identify | Identify, identify செய் | அடையாளம் காண் would read as "recognize". |
| counter | counter | |
| locked, lost lock | lock ஆனது, lock இழந்தது | |
| interrupted | தடைபட்டது | |
| timestamp | timestamp | |
| advertise, advertised | advertise செய், Advertise செய்யப்பட்டது | Also for an entity announcing itself (தன்னை advertise செய்). |
| interface | interface | நெட்வொர்க் interface. |
| hardware clock | hardware clock | |
| virtual (interface) | virtual | Virtual interfaces. |

## Choices a native speaker should check

- Latin script for the terms of art (stream, entity, clock domain) where Tamil software tends toward Tamil words or Tamil-script spellings. Many inspector labels end up wholly English.
- பிணைக்கப்பட்டது for "bound". Engineers may simply say bind (bind செய்யப்பட்டது).
- Identify and Recall as Latin buttons beside Tamil ones (பெயர்மாற்று, நீக்கு).
- The hyphen before case endings on Latin words and on values (entity-ஐ, { $interface }-இல்), and -ஐ throughout rather than -யை after vowel-final words.
- ஆய்வி for Inspector, கண்டறிதல் for Diagnostics, பதிவு for Log, ஒத்திசைந்தது for Synced.
- கடந்து செல்பவை for streams passing through a bridge.
- Bare imperatives (மூடு, சேமி, கிளிக் செய்) as Tamil desktop software uses them, rather than polite -உங்கள் forms.
