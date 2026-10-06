# Indonesian (id) terms

How `i18n/id/triib.ftl` renders the glossary's roles and terms of art. The
text follows the register of Indonesian audio and network engineers, who
keep many English terms. Counts set thousands apart with a dot
(1.204.331).

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lower case mid-sentence: "Stream talker", "menunggu talker". |
| listener | listener | Loanword, as for talker: "Input listener". |
| grandmaster | grandmaster | Loanword, lower case mid-sentence: "hop dari grandmaster". Never "jam induk". |
| entity | entitas | Not "perangkat", which is kept for device in the network view. |
| entity model | model entitas | |
| controller | controller | The word network engineers use; "pengontrol" is the alternative. |
| stream | stream | Kept, as Indonesian AV engineers write it; "aliran" reads as a lay word. |
| stream input, stream output | input stream, output stream | Matrix: "Output talker", "Input listener". |
| connection, connect, bind | koneksi, menghubungkan; terikat | "Terhubung", "memutus" for disconnect; a bound input is "Terikat". |
| media clock | media clock | Kept: "jam" reads as a wall clock. |
| clock domain | domain clock | |
| clock source | sumber clock | |
| sampling rate | sampling rate | Kept, as audio engineers say it. |
| bridge | switch | Practitioners say switch; "switch AVB" where the standard says AVB bridge. "Port switch". |
| reservation | reservasi | |
| egress | egress | "port egress"; the column is "Egress". |
| link | link | "Link aktif", "Link mati", "Link terputus" for drops. "tautan" is a web link. |
| peer delay | peer delay | Kept. |
| offset | offset | Kept. |
| hop | hop | "{ $count } hop dari grandmaster". |
| unicast, multicast | unicast, multicast | Kept. |
| fan-out | fan-out | Not shown as a word; the inspector says "Hingga N listener per stream, lalu multicast". "fan-in" is kept in the MSRP failure. |
| descriptor | deskriptor | |
| cluster | klaster | KBBI spelling. |
| stream port | port stream | The mapping view shows "port N". |
| channel mapping | pemetaan kanal | Map "peta", "memetakan", "melepas pemetaan". |
| control | kontrol | |
| preset | preset | "Panggil" for recall. |
| identify | identifikasi | |
| counter | penghitung | |
| locked, lost lock | terkunci, lepas kunci | As for signal lock: "terkunci sekali", "lepas kunci 3 kali". |
| interrupted | terputus | |
| timestamp | timestamp | Kept; "stempel waktu" is the formal alternative. |
| advertise, advertised | mengumumkan, diumumkan | The ADP section is "Pengumuman". |
| interface | antarmuka | |
| hardware clock | clock hardware | |
| virtual (interface) | virtual | |

Other recurring words: frame "frame", device "perangkat", network
"jaringan", path "jalur", tree "pohon", latency "latensi", Settings
"Pengaturan", file folder "folder".

## Choices a native speaker should check

- "clock" kept in English (media clock, domain clock, sumber clock, clock
  hardware) rather than "jam", which Malay uses.
- "stream", "sampling rate" and "timestamp" kept in English rather than
  "aliran", "laju sampel" and "stempel waktu".
- "terkunci" and "lepas kunci" for PLL lock; some engineers may simply say
  "lock".
- "switch" for bridge, including "Port switch" and "Switch di bawah".
- "controller" kept rather than "pengontrol" or "pengendali".
- Shares such as 1.7% come from the code with a decimal point, not the
  Indonesian decimal comma; only the percent sign's place is set here.
