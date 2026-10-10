# Persian (fa) terms

How `i18n/fa/triib.ftl` renders the glossary's roles and terms of art.
Talker, Listener and Grandmaster stay in Latin script, capitalized, joined
by ezafe like any noun (رزرو Talker, جریان‌های Talker). Persian keeps a
noun singular after a number, so the one and other forms read the same
(1 موجودیت, 5 موجودیت). Plural suffixes and verb prefixes use the
zero-width non-joiner (جریان‌ها, می‌شود). Quotations use « ». A
talker-to-listener arrow is ← so that it points the right way in
right-to-left text. Separators: list "، ", thousands "٬", decimal "٫",
percent "80٪", the Persian marks, beside the ASCII digits triib shows.

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized: خروجی‌های Talker. |
| listener | Listener | Latin script, capitalized: ورودی‌های Listener. |
| grandmaster | Grandmaster | Latin script. Never a word built on ارباب or اصلی. |
| entity | موجودیت | The term standards and data modeling use. Device is دستگاه, kept for the network map. |
| entity model | مدل موجودیت | |
| controller | کنترلر | Kept apart from control (کنترل). |
| stream | جریان | Flowing is جاری. |
| stream input, stream output | ورودی جریان، خروجی جریان | |
| connection, connect, bind | اتصال، وصل کردن، مقید | Connected متصل; disconnect قطع اتصال. Bound is مقید (as in متغیر مقید), kept apart from متصل. |
| media clock | ساعت رسانه | Media lock is قفل رسانه. |
| clock domain | دامنه ساعت | Domain is دامنه throughout (دامنه gPTP). |
| clock source | منبع ساعت | |
| sampling rate | نرخ نمونه‌برداری | |
| bridge | پل | A switch the English calls a switch is سوئیچ. |
| reservation | رزرو | |
| egress | ترافیک خروجی | Egress port is پورت خروجی. |
| link | لینک | لینک برقرار، لینک قطع. |
| peer delay | تأخیر همتا | Link delay and latency are تأخیر too. |
| offset | آفست | |
| hop | گام | |
| unicast, multicast | تک‌پخشی، چندپخشی | |
| fan-out | (a phrase) | تا 4 Listener برای هر جریان، سپس چندپخشی. |
| descriptor | توصیفگر | |
| cluster | خوشه | Not گروه, which is the entity group. |
| stream port | پورت جریان | Port is پورت throughout. |
| channel mapping | نگاشت کانال | Map نگاشت, unmap حذف نگاشت, not mapped نگاشت‌نشده. |
| control | کنترل | |
| preset | پیش‌تنظیم | Recall is فراخوانی. |
| identify | شناسایی | |
| counter | شمارنده | |
| locked, lost lock | قفل برقرار شده، قفل از دست رفته | |
| interrupted | وقفه | Kept apart from قطع (link down, disconnect). |
| timestamp | مهر زمانی | |
| advertise, advertised | اعلام، اعلام‌شده | An MSRP declaration is اعلان, kept apart. |
| interface | واسط | |
| hardware clock | ساعت سخت‌افزاری | |
| virtual (interface) | مجازی | |

## Choices a native speaker should check

- جریان for stream; استریم is what many engineers say aloud.
- پل for bridge; بریج or سوئیچ AVB may read more naturally.
- مقید for bound, and واسط rather than اینترفیس for interface.
- The Persian separators ٬ ٫ ٪ beside ASCII digits, rather than , . %.
- آلارم for alarm, to keep it apart from هشدار (warnings in the log).
- ID stays in Latin (ID موجودیت) because the tests keep it as written;
  شناسه موجودیت would be the native term.
