# Traditional Chinese (zh-TW) terms

How `i18n/zh-TW/triib.ftl` renders the glossary's roles and terms of art.
It uses Taiwan's vocabulary rather than a character conversion of zh-CN:
串流, 網路, 介面, 取樣率, 頻寬, 位址, 訊框, 韌體, 音訊, 裝置, 交換器. Latin
terms are set off by a space; sentences use full-width punctuation and
「」 quotation marks.

| English | Rendering | Note |
|---|---|---|
| talker | 發送端 | As the glossary sets out. Matrix header 發送端輸出; columns 發送端串流. |
| listener | 接收端 | Matrix header 接收端輸入. |
| grandmaster | Grandmaster | Kept in Latin script, never 主時鐘. |
| entity | 實體 | Not 裝置: an entity can be software. |
| entity model | 實體模型 | |
| controller | 控制器 | |
| stream | 串流 | Classifier 個. Audio stream 音訊串流, media clock stream 媒體時鐘串流. |
| stream input, stream output | 串流輸入, 串流輸出 | |
| connection, connect, bind | 連線, 連線, 綁定 | "Bound" is 已綁定; disconnect is 中斷連線. 連線 is kept apart from 連結 (link). |
| media clock | 媒體時鐘 | |
| clock domain | 時鐘域 | |
| clock source | 時鐘源 | |
| sampling rate | 取樣率 | |
| bridge | 交換器 | Taiwan's word for switch; "AVB bridge" is AVB 交換器. Bridge port 交換器埠. |
| reservation | 保留 | "Reservation failed" is 保留失敗. |
| egress | 出口 | Egress port 出口埠; the egress column 出口流量. |
| link | 連結 | Link up / down: 連結正常 / 連結中斷; link delay 連結延遲. |
| peer delay | 對等延遲 | |
| offset | 偏移 | |
| hop | 躍點 | { $count } 個躍點; "hops from grandmaster" 距 Grandmaster 躍點數. |
| unicast, multicast | 單播, 多播 | |
| fan-out | 扇出 | Not used as a word in the interface; the AVB Lite line is the phrase 逐一傳送給 … 個接收端，之後改用多播. Fan-in is 扇入. |
| descriptor | 描述元 | Taiwan's term (as in 安全性描述元), not 描述符. |
| cluster | 叢集 | Taiwan's term for a cluster. Not 群組, which is the entity group. |
| stream port | 串流埠 | Shown in channel mappings as 埠 { $number }. |
| channel mapping | 通道對應 | Map / unmap: 對應 / 取消對應. |
| control | 控制項 | |
| preset | 預設集 | Not 預設, which in Taiwan means "default" (系統預設). Recall is 套用. |
| identify | 識別 | "Could not identify" is phrased 無法讓 … 執行識別. |
| counter | 計數器 | |
| locked, lost lock | 鎖定, 失鎖 | [0] variants: 未鎖定, 未失鎖. |
| holding over | 保持 | The telecom clock state. Shown as 保持中; the alarm says 處於保持狀態. |
| station | 站台 | The IEEE 802.11 term (STA). { $count } 個站台. |
| access point | 存取點 | Taiwan's term, as in 無線存取點. |
| beacon | 信標 | Mode B is Mode B，基於信標. |
| signal | 訊號強度 | The label for the RSSI in dBm. Wi-Fi channel is 頻道, kept apart from 通道 (audio channel); an FTM burst is 叢發. |
| interrupted | 中斷 | |
| timestamp | 時間戳記 | |
| advertise, advertised | 通告, 已通告 | Discovery is 探索 (探索中), as in 網路探索. |
| interface | 介面 | Network interface 網路介面. |
| hardware clock | 硬體時鐘 | |
| virtual (interface) | 虛擬 | 虛擬介面. |

## Choices a native speaker should check

- **保留 for reservation.** Taiwan networking texts use both 保留 and 預留
  (資源保留協定 / 資源預留協定). 保留 is used everywhere, including the
  network map's failure lines.
- **套用 for recall (presets).** Natural in Taiwan software (套用預設集); a
  mixing-desk user may expect 叫出 or 呼叫.
- **通道對應 for channel mapping.** 對應 follows Taiwan software usage;
  對映 or 映射 are the alternatives.
- **躍點 for hop** (Microsoft Taiwan's term) where engineers may simply say
  跳.
- **記錄 for the log panel** and **檢閱器 for the inspector** (Apple Taiwan
  usage); 日誌 and 檢查器 are the alternatives.
- **List separator `，`**, for the same reason as in zh-CN: it joins clauses
  as well as nouns.
- **站台 for station** and **叢發 for an FTM burst.** 站台 follows the
  802.11 term (STA); router pages say 用戶端. 叢發 is Taiwan's word for
  burst, where mainland writing has 突發.
