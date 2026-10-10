# Arabic (ar) terms

How `i18n/ar/triib.ftl` renders the glossary's roles and terms of art.
Talker, Listener and Grandmaster stay in Latin script, capitalized, never
take the Arabic article, and do not inflect (تدفقات Talker, حجز Talker,
حتى 5 Listener). One and two are written as words, the noun with واحد
and the dual (كيان واحد, كيانان); from 3 the number shows, with the
plural for 3 to 10, the accusative singular for 11 to 99 and the singular
for 0 and 100 and up (5 كيانات, 21 كيانًا, 100 كيان). Media resets keep
the digit in every form, as a word form reads oddly there. Quotations use « ». A talker-to-listener
arrow is ← so that it points the right way in right-to-left text.
Separators: list "، ", thousands ",", decimal ".", percent "80%" (the
marks Arabic uses alongside ASCII digits).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized, no article: مخارج Talker, بانتظار Talker. |
| listener | Listener | Latin script, capitalized: مداخل Listener. |
| grandmaster | Grandmaster | Latin script. Never a word built on سيد or رئيسي. |
| entity | كيان، كيانات | Not جهاز, which is kept for devices on the network map. |
| entity model | نموذج الكيان | |
| controller | وحدة تحكم | Kept apart from control (عنصر تحكم). |
| stream | تدفق، تدفقات | Flowing is يتدفق. |
| stream input, stream output | مدخل تدفق، مخرج تدفق | Headings مداخل التدفق، مخارج التدفق; matrix corners مخارج Talker، مداخل Listener. |
| connection, connect, bind | اتصال، توصيل، ربط | Connected متصل; disconnect فصل. Bound is مربوط, kept apart from متصل (connected and receiving). |
| media clock | ساعة الوسائط | Media lock is قفل الوسائط. |
| clock domain | نطاق الساعة | Domain is نطاق throughout (نطاق gPTP). |
| clock source | مصدر الساعة | |
| sampling rate | معدل أخذ العينات | |
| bridge | جسر | A switch the English calls a switch (an AVB Lite fallback reason) is محوّل. |
| reservation | حجز | فشل حجز Talker. |
| egress | الخروج | Egress port is منفذ الخروج. |
| link | وصلة | الوصلة تعمل، الوصلة مقطوعة. |
| peer delay | تأخير النظير | Link delay is تأخير الوصلة; latency is زمن الانتقال. |
| offset | الإزاحة | |
| hop | قفزة، قفزات | |
| unicast, multicast | بث أحادي، بث متعدد | |
| fan-out | (a phrase) | حتى 4 Listener لكل تدفق، ثم بث متعدد. |
| descriptor | واصف، واصفات | |
| cluster | عنقود، عناقيد | Not مجموعة, which is the entity group. |
| stream port | منفذ التدفق | Port is منفذ throughout. |
| channel mapping | تعيين القنوات | Map تعيين, unmap إلغاء التعيين, not mapped غير معيَّن. |
| control | عنصر تحكم، عناصر التحكم | |
| preset | إعداد مسبق، إعدادات مسبقة | Recall is استدعاء. |
| identify | تعريف | |
| counter | عدّاد، عدادات | |
| locked, lost lock | تم القفل، فُقد القفل | Not locked لم يتم القفل. |
| holding over | في وضع الاحتفاظ | The clock keeping time on its own: محطة، في وضع الاحتفاظ. |
| station | محطة، محطات | The IEEE 802.11 term; feminine: محطة واحدة، محطتان، 5 محطات، 21 محطةً. |
| access point | نقطة وصول | |
| beacon | المنارة | Beacon frames are إطارات المنارة. |
| signal | الإشارة | The label is قوة الإشارة (signal strength). |
| interrupted | انقطع | |
| timestamp | طابع زمني، طوابع زمنية | |
| advertise, advertised | الإعلان، مُعلَن | Announce itself is يعلن عن نفسه. An MSRP declaration is تصريح, kept apart. |
| interface | واجهة، واجهات | |
| hardware clock | ساعة عتادية | |
| virtual (interface) | افتراضية | |

## Choices a native speaker should check

- Dual forms of compounds such as مخرجا تدفق and مدخلا تدفق (two stream
  outputs or inputs), and a zero count written 0 كيان rather than لا كيانات.
- تعريف for Identify. تحديد الهوية is clearer but long for a button.
- الفاحص for the Inspector panel. المفتش and المراقب are alternatives.
- الخروج for the Egress column, and جسر rather than محوّل AVB for bridge.
- ID stays in Latin (ID الكيان, ID التدفق) because the tests keep it as
  written; معرّف الكيان would be the native term.
- "," and "." as separators beside ASCII digits, rather than ٬ and ٫.
- ساعة عتادية for hardware clock, and زمن الانتقال for latency.
- وضع الاحتفاظ for holdover (الاستبقاء is another rendering), إطارات
  المنارة for beacons, رشقات for FTM bursts and المؤازر for the servo.
