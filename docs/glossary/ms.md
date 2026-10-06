# Malay (ms) terms

How `i18n/ms/triib.ftl` renders the glossary's roles and terms of art. It
follows Malaysian technical usage (fail, rangkaian, perkakasan, tetapan),
kept distinct from Indonesian where the two differ. Counts set thousands
apart with a comma (1,204,331).

| English | Rendering | Note |
|---|---|---|
| talker | talker | Loanword, lower case mid-sentence: "Strim talker", "menunggu talker". |
| listener | listener | Loanword, as for talker: "Input listener". |
| grandmaster | grandmaster | Loanword, lower case mid-sentence. Never "jam induk". |
| entity | entiti | Not "peranti", which is kept for device in the network view. |
| entity model | model entiti | |
| controller | pengawal | As in "pengawal domain"; the role list says "pengawal". |
| stream | strim | Malay spelling, as in "penstriman". |
| stream input, stream output | input strim, output strim | Matrix: "Output talker", "Input listener". |
| connection, connect, bind | sambungan, menyambung; terikat | "Disambungkan", "memutuskan sambungan"; a bound input is "Terikat". |
| media clock | jam media | |
| clock domain | domain jam | |
| clock source | sumber jam | |
| sampling rate | kadar pensampelan | |
| bridge | suis | "suis rangkaian" is the Malay term for a network switch; "suis AVB" where the standard says AVB bridge. |
| reservation | tempahan | |
| egress | egress | "port egress"; the column is "Egress". |
| link | pautan | "Pautan aktif", "Pautan terputus", "Pautan putus" for drops. |
| peer delay | peer delay | Kept. |
| offset | ofset | Malay spelling. |
| hop | hop | |
| unicast, multicast | unicast, multicast | Kept; "unisiar" and "multisiar" are rarely used. |
| fan-out | fan-out | Not shown as a word; the inspector says "Sehingga N listener bagi setiap strim, kemudian multicast". "fan-in" is kept in the MSRP failure. |
| descriptor | deskriptor | |
| cluster | kluster | |
| stream port | port strim | The mapping view shows "port N". |
| channel mapping | pemetaan saluran | "memetakan", "menyahpetakan". |
| control | kawalan | |
| preset | praset | "Panggil semula" for recall. |
| identify | kenal pasti | |
| counter | pembilang | |
| locked, lost lock | terkunci, terlepas kunci | |
| interrupted | terganggu | |
| timestamp | cap masa | |
| advertise, advertised | mengumumkan, diumumkan | The ADP section is "Pengumuman". |
| interface | antara muka | |
| hardware clock | jam perkakasan | |
| virtual (interface) | maya | |

Other recurring words: frame "bingkai", device "peranti", network
"rangkaian", path "laluan", tree "pepohon", latency "kependaman",
bandwidth "lebar jalur", error "ralat", column "lajur", Settings
"Tetapan".

## Choices a native speaker should check

- "jam" for clock (jam media, domain jam, sumber jam); Malaysian audio
  engineers may prefer to keep "clock", as the Indonesian file does.
- "strim" and "praset" rather than the English "stream" and "preset".
- "bingkai" for frame and "kependaman" for latency, where engineers often
  say the English words.
- "suis" for bridge, including "Port suis" and "Suis di bawah".
- "pengawal" for controller.
- "Perkaitan" for the ADP association, and "deskriptor" rather than
  "pemerihal".
