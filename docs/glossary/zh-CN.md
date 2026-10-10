# Simplified Chinese (zh-CN) terms

How `i18n/zh-CN/triib.ftl` renders the glossary's roles and terms of art.
Latin terms (AVB, gPTP, Grandmaster, units) are set off from Chinese text by
a space, and sentences use full-width punctuation (，。：；“”（）).

| English | Rendering | Note |
|---|---|---|
| talker | 发送端 | As the glossary sets out. Matrix header 发送端输出; columns 发送端流. |
| listener | 接收端 | Matrix header 接收端输入. |
| grandmaster | Grandmaster | Kept in Latin script, never 主时钟. "Hops from grandmaster" is 距 Grandmaster 跳数. |
| entity | 实体 | As in Chinese ATDECC/AVDECC writing. Not 设备: an entity can be software. |
| entity model | 实体模型 | |
| controller | 控制器 | Also the role name in an entity's roles. |
| stream | 流 | Classifier 个: { $count } 个流. Audio stream 音频流, media clock stream 媒体时钟流. |
| stream input, stream output | 流输入, 流输出 | |
| connection, connect, bind | 连接, 连接, 绑定 | "Bound" is 已绑定; disconnect is 断开连接. |
| media clock | 媒体时钟 | "Media locked" in diagnostics is 媒体锁定. |
| clock domain | 时钟域 | |
| clock source | 时钟源 | An unnamed one is 时钟源 { $index }. |
| sampling rate | 采样率 | |
| bridge | 交换机 | What live-sound and installation engineers say (AVB 交换机). 网桥 was the alternative; "AVB bridge" is AVB 交换机. |
| reservation | 预留 | As in 流预留协议 (SRP); "reservation failed" is 预留失败. |
| egress | 出口 | Egress port 出口端口; the egress column 出口流量. |
| link | 链路 | Link up / down: 链路连通 / 链路断开; link drops 链路断开次数. Interface "up" is 已连通. |
| peer delay | 对等延迟 | The PTP Pdelay mechanism. |
| offset | 偏移 | "{ offset } from { grandmaster }" is 相对 … 偏移 …. |
| hop | 跳 | { $count } 跳; "hops from grandmaster" 距 Grandmaster 跳数. |
| unicast, multicast | 单播, 组播 | 组播 as in mainland networking documentation (多播 is the textbook alternative). |
| fan-out | 扇出 | Not used as a word in the interface; the AVB Lite line is the phrase 逐一发送给 … 个接收端，之后改用组播. Fan-in (MSRP code 15) is 扇入. |
| descriptor | 描述符 | |
| cluster | 簇 | As for Zigbee and file-system clusters. Not 组, which is the entity group. |
| stream port | 流端口 | Shown in channel mappings as 端口 { $number }. |
| channel mapping | 通道映射 | Map / unmap: 映射 / 取消映射; not mapped 未映射. |
| control | 控制项 | Not 控件 (a UI widget). An unnamed one is 控制项 { $index }. |
| preset | 预设 | Recall is 调用, as on mixing desks (调用场景). |
| identify | 识别 | "Could not identify" is phrased 无法让 … 执行识别, so it does not read as "could not recognize". |
| counter | 计数器 | |
| locked, lost lock | 锁定, 失锁 | [0] variants: 未锁定, 未失锁. |
| interrupted | 中断 | |
| timestamp | 时间戳 | |
| advertise, advertised | 通告, 已通告 | Discovery is 发现 (正在发现). |
| interface | 接口 | Network interface 网络接口. |
| hardware clock | 硬件时钟 | |
| virtual (interface) | 虚拟 | 虚拟接口. |

## Choices a native speaker should check

- **交换机 for bridge.** Chosen because AVB practitioners in China say AVB
  交换机; TSN and standards writing says 网桥. It is used everywhere,
  including MSRP failure 2 (交换机资源不足).
- **簇 for cluster.** A technical grouping word that does not clash with 组
  (entity group). 音频簇 or keeping "cluster" are the alternatives.
- **List separator `，`.** The same separator joins roles (发送端 2，接收端 2),
  status parts (enp6s0，已连通，硬件时钟 ptp0) and diagnostics clauses
  (锁定 3 次，失锁 1 次). `、` reads better for pure noun lists but worse
  for the clauses.
- **识别 for identify.** 定位 (locate) is what some device managers use for
  "blink to find".
- **pull 系数 for a sampling rate's pull.** Only shown for reserved pull
  codes; the standard's word "pull" is kept with 系数 (factor).
- **控制项 for control** and **出口流量 for the egress column**.
