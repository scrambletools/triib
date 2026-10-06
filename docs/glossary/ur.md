# Urdu (ur) terms

How `i18n/ur/triib.ftl` renders the glossary's roles and terms of art.
Urdu technical writing carries most networking and audio terms as English
words in Urdu script (اسٹریم, برج, لنک), as engineers say them, and
triib's text follows that. Talker, Listener and Grandmaster stay in Latin
script, capitalized, and do not inflect (Talker کا ریزرویشن, 5 Listener).
English loans take -ز or -س in the plural (اسٹریمز, پری سیٹس). Sentences
end with ۔. Quotations use straight quotes. A talker-to-listener arrow is
← so that it points the right way in right-to-left text. Separators: list
"، ", thousands ",", decimal ".", percent "80%".

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized: Talker آؤٹ پٹس. |
| listener | Listener | Latin script, capitalized: Listener ان پٹس. |
| grandmaster | Grandmaster | Latin script. Never a word built on آقا or ماسٹر. |
| entity | اینٹیٹی، اینٹیٹیز | Feminine. Device is ڈیوائس, kept for the network map. |
| entity model | اینٹیٹی ماڈل | |
| controller | کنٹرولر | Kept apart from control (کنٹرول). |
| stream | اسٹریم، اسٹریمز | Feminine. Flowing is رواں. |
| stream input, stream output | اسٹریم ان پٹ، اسٹریم آؤٹ پٹ | Masculine. |
| connection, connect, bind | کنکشن، منسلک کرنا، بائنڈ شدہ | Connected منسلک; disconnect منقطع کرنا. Bound is بائنڈ شدہ, kept apart from منسلک (connected and receiving). |
| media clock | میڈیا کلاک | Media lock is میڈیا لاک. |
| clock domain | کلاک ڈومین | Domain is ڈومین throughout (gPTP ڈومین). |
| clock source | کلاک سورس | |
| sampling rate | سیمپلنگ ریٹ | |
| bridge | برج | A switch the English calls a switch is سوئچ. |
| reservation | ریزرویشن | |
| egress | ایگریس | Egress port is ایگریس پورٹ. |
| link | لنک | لنک اپ، لنک ڈاؤن. |
| peer delay | پیئر ڈیلے | Link delay is لنک ڈیلے; latency is لیٹنسی. |
| offset | آفسیٹ | |
| hop | ہاپ، ہاپس | |
| unicast, multicast | یونی کاسٹ، ملٹی کاسٹ | |
| fan-out | (a phrase) | فی اسٹریم 4 Listener تک، پھر ملٹی کاسٹ. |
| descriptor | ڈسکرپٹر، ڈسکرپٹرز | |
| cluster | کلسٹر، کلسٹرز | Not گروپ, which is the entity group. |
| stream port | اسٹریم پورٹ | Port is پورٹ throughout. |
| channel mapping | چینل میپنگ | Map میپ کرنا, unmap ان میپ کرنا, not mapped میپ نہیں. |
| control | کنٹرول، کنٹرولز | |
| preset | پری سیٹ، پری سیٹس | Recall is ری کال. |
| identify | شناخت کریں | |
| counter | کاؤنٹر، کاؤنٹرز | |
| locked, lost lock | لاک ہوا، لاک ٹوٹا | |
| interrupted | تعطل | Kept apart from منقطع (disconnect). |
| timestamp | ٹائم اسٹیمپ، ٹائم اسٹیمپس | |
| advertise, advertised | اعلان، اعلان شدہ | An MSRP declaration is ڈیکلریشن, kept apart. |
| interface | انٹرفیس، انٹرفیسز | |
| hardware clock | ہارڈویئر کلاک | |
| virtual (interface) | ورچوئل | |

## Choices a native speaker should check

- اینٹیٹی for entity, rather than ہستی or وجود.
- How far the transliterated register goes (پیئر ڈیلے, ایگریس, کلسٹر)
  against Urdu words such as اخراج for egress.
- بائنڈ شدہ for bound.
- Genders given to loanwords: اسٹریم feminine, ان پٹ and آؤٹ پٹ masculine.
- کھنچاؤ for a sampling rate's pull.
