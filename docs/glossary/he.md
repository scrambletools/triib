# Hebrew (he) terms

How `i18n/he/triib.ftl` renders the glossary's roles and terms of art.
Talker, Listener and Grandmaster stay in Latin script, capitalized. Hebrew
prefixes join Latin words and values with a hyphen (ה-Talker, ב-enp6s0,
מה-Grandmaster). A count of one is written as a word (ישות אחת); other
counts show the number (2 ישויות), and "twice" uses the dual פעמיים. Buttons are nouns (שמירה, סגירה); instructions use the
plural imperative (לחצו, בחרו). Quotations use straight quotes. A
talker-to-listener arrow is ← so that it points the right way in
right-to-left text. Separators: list ", ", thousands ",", decimal ".",
percent "80%".

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script, capitalized: ה-Talker, זרמי Talker. |
| listener | Listener | Latin script, capitalized: כניסות Listener. |
| grandmaster | Grandmaster | Latin script. Never a word built on אדון or ראשי. |
| entity | ישות, ישויות | Feminine. Device is התקן, kept for the network map. |
| entity model | מודל ישות | |
| controller | בקר | Kept apart from control (פקד). |
| stream | זרם, זרמים | Flowing is זורם. |
| stream input, stream output | כניסת זרם, יציאת זרם | |
| connection, connect, bind | חיבור, לחבר, משויך | Connected מחובר; disconnect ניתוק. Bound is משויך, kept apart from מחובר (connected and receiving). |
| media clock | שעון מדיה | Media lock is נעילת מדיה. |
| clock domain | תחום שעון | Domain is תחום throughout (תחום gPTP). |
| clock source | מקור שעון | |
| sampling rate | קצב דגימה | |
| bridge | גשר | A switch the English calls a switch is מתג. |
| reservation | שריון | As in שריון רוחב פס. |
| egress | תעבורה יוצאת | Egress port is פורט היציאה. |
| link | קישור | קישור פעיל, קישור מנותק. |
| peer delay | השהיית עמית | Link delay and latency are השהיה too. |
| offset | היסט | |
| hop | קפיצה, קפיצות | |
| unicast, multicast | יוניקאסט, מולטיקאסט | |
| fan-out | (a phrase) | עד 4 Listener לכל זרם, ואז מולטיקאסט. |
| descriptor | מתאר, מתארים | |
| cluster | אשכול, אשכולות | Not קבוצה, which is the entity group. |
| stream port | פורט זרם | Port is פורט throughout. |
| channel mapping | מיפוי ערוצים | Map למפות, unmap ביטול מיפוי, not mapped לא ממופה. |
| control | פקד, פקדים | |
| preset | פריסט, פריסטים | Recall is טעינה. |
| identify | זיהוי | |
| counter | מונה, מונים | |
| locked, lost lock | ננעל, איבד נעילה | |
| holding over | holdover | Latin script with a hyphenated prefix: ב-holdover, תחנה, ב-holdover. |
| station | תחנה, תחנות | Feminine: תחנה, נעולה; תחנה אחת, 3 תחנות. |
| access point | נקודת גישה | |
| beacon | beacon | Latin script: ממסגרות beacon. |
| signal | אות | The label is עוצמת אות (signal strength). |
| interrupted | נקטע | |
| timestamp | חותמת זמן, חותמות זמן | |
| advertise, advertised | הכרזה, מוכרז | Announce itself is להכריז על עצמה. An MSRP declaration is הצהרה, kept apart. |
| interface | ממשק, ממשקים | |
| hardware clock | שעון חומרה | |
| virtual (interface) | וירטואלי | |

## Choices a native speaker should check

- משויך for bound, and שריון for reservation (rather than הזמנה or
  הקצאה).
- יוניקאסט and מולטיקאסט in Hebrew letters, rather than Latin or
  שידור יחיד and שידור קבוצתי.
- מפקח for the Inspector panel.
- Counts of two keep the digit (2 ישויות) rather than שתי ישויות.
- ID stays in Latin (ID ישות, ה-ID של הזרם) because the tests keep it as
  written; מזהה ישות would be the native term.
- עצירה for Pause in the log, to keep השהיה for delay and latency.
- ב-holdover in Latin script; במצב החזקה would be the Hebrew phrase. An
  FTM burst is פרץ.
