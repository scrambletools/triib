# Thai (th) terms

How `i18n/th/triib.ftl` renders the glossary's roles and terms of art.
Talker, Listener and Grandmaster stay in Latin script, capitalized. The
text uses no full stops, separates sentences with a space, quotes with
“ ”, and sets thousands apart with a comma (1,204,331).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Latin script: "สตรีม Talker", "เอาต์พุต Talker", "Talker แต่ละตัว". |
| listener | Listener | Latin script: "อินพุต Listener". |
| grandmaster | Grandmaster | Latin script: "ฮอปจาก Grandmaster". Never a word built on มาสเตอร์ or แม่ข่าย. |
| entity | เอนทิตี | The transliteration Thai technical writing uses; not อุปกรณ์, which is device. |
| entity model | โมเดลเอนทิตี | |
| controller | คอนโทรลเลอร์ | |
| stream | สตรีม | |
| stream input, stream output | อินพุตสตรีม, เอาต์พุตสตรีม | |
| connection, connect, bind | การเชื่อมต่อ, เชื่อมต่อ; ผูก | Disconnect ตัดการเชื่อมต่อ; a bound input is ผูกแล้ว. |
| media clock | มีเดียคล็อก | As in เวิร์ดคล็อก. |
| clock domain | โดเมนคล็อก | |
| clock source | แหล่งคล็อก | |
| sampling rate | อัตราสุ่มตัวอย่าง | |
| bridge | สวิตช์ | What Thai engineers say; "AVB bridge" is สวิตช์ AVB. Bridge port พอร์ตสวิตช์. |
| reservation | การจอง | "การจองล้มเหลว". |
| egress | ขาออก | Egress port พอร์ตขาออก; the column is ขาออก. |
| link | ลิงก์ | ลิงก์ทำงาน, ลิงก์ขาด; drops ลิงก์หลุด. |
| peer delay | peer delay | Kept in Latin script. |
| offset | ออฟเซ็ต | |
| hop | ฮอป | |
| unicast, multicast | ยูนิคาสต์, มัลติคาสต์ | |
| fan-out | fan-out | Not shown as a word; "fan-in" is kept in Latin script in the MSRP failure. |
| descriptor | ดีสคริปเตอร์ | |
| cluster | คลัสเตอร์ | |
| stream port | พอร์ตสตรีม | The mapping view shows พอร์ต N. |
| channel mapping | การแมปช่องสัญญาณ | แมป, ยกเลิกการแมป. |
| control | ตัวควบคุม | |
| preset | พรีเซ็ต | เรียกคืน for recall. |
| identify | ระบุตัว | |
| counter | ตัวนับ | |
| locked, lost lock | ล็อก, หลุดล็อก | [0] variants: ไม่เคยล็อก, ไม่เคยหลุดล็อก. |
| holding over | holdover | Kept in Latin script, as telecom engineers write it: อยู่ใน holdover, "สถานี อยู่ใน holdover". |
| station | สถานี | The IEEE 802.11 term in Thai writing; ไคลเอนต์ is what router pages say. |
| access point | แอคเซสพอยต์ | Transliterated, as Thai IT writing spells it. |
| beacon | บีคอน | "Mode B, จากบีคอน". |
| signal | สัญญาณ | As in ความแรงสัญญาณ. |
| interrupted | หยุดชะงัก | |
| timestamp | ไทม์สแตมป์ | |
| advertise, advertised | ประกาศ, ประกาศแล้ว | The ADP section is การประกาศ. |
| interface | อินเทอร์เฟซ | |
| hardware clock | คล็อกฮาร์ดแวร์ | |
| virtual (interface) | เสมือน | |

Other recurring words: frame เฟรม, device อุปกรณ์, network เครือข่าย,
path เส้นทาง, tree ทรี, latency and delay ความหน่วง, Log
บันทึกเหตุการณ์ (บันทึก alone is Save).

## Choices a native speaker should check

- อัตราสุ่มตัวอย่าง for sampling rate, rather than แซมปลิงเรต or the English.
- ผูกแล้ว for a bound input, and ระบุตัว for identify.
- ทรี for the gPTP tree and the clock tree, rather than ต้นไม้.
- ขาออก for egress, rather than keeping "egress".
- คล็อก throughout for clocks, rather than นาฬิกา.
- `common-list-separator` is ", ": CLDR lists Thai with spaces, but the
  parts joined (such as "enp6s0, ทำงาน, คล็อกฮาร์ดแวร์ ptp0") contain
  spaces themselves.
