# Vietnamese (vi) terms

How `i18n/vi/triib.ftl` renders the glossary's roles and terms of art.
Talker, Listener and Grandmaster stay as written, capitalized, as other
networking loanwords do. Counts set thousands apart with a dot (1.204.331).

| English | Rendering | Note |
|---|---|---|
| talker | Talker | Kept as written; matrix header Đầu ra Talker, column Luồng Talker. |
| listener | Listener | Matrix header Đầu vào Listener. |
| grandmaster | Grandmaster | Never đồng hồ chủ. "Hops from grandmaster" is Số bước nhảy từ Grandmaster. |
| entity | thực thể | Not thiết bị: an entity can be software. |
| entity model | mô hình thực thể | |
| controller | bộ điều khiển | |
| stream | luồng | Audio stream luồng âm thanh, media clock stream luồng đồng hồ media. |
| stream input, stream output | đầu vào luồng, đầu ra luồng | |
| connection, connect, bind | kết nối, kết nối, gán | "Bound" is đã gán; disconnect is ngắt kết nối. |
| media clock | đồng hồ media | "Media locked" is khóa media. |
| clock domain | miền đồng hồ | |
| clock source | nguồn đồng hồ | |
| sampling rate | tần số lấy mẫu | |
| bridge | switch | What Vietnamese network engineers say; "AVB bridge" is switch AVB. The label is Switch. |
| reservation | đặt trước | "Reservation failed" is đặt trước thất bại. |
| egress | ra | Egress port cổng ra; the egress column Lưu lượng ra. |
| link | liên kết | Link up / down: Liên kết hoạt động / Mất liên kết. |
| peer delay | peer delay | Kept in English, as PTP's Pdelay messages are named. |
| offset | độ lệch | |
| hop | bước nhảy | |
| unicast, multicast | unicast, multicast | Kept, as practitioners say them. |
| fan-out | (phrase) | The AVB Lite line reads gửi lần lượt tới tối đa … Listener, sau đó chuyển sang multicast. Fan-in (MSRP code 15) stays fan-in. |
| descriptor | bộ mô tả | |
| cluster | cụm | Not nhóm, which is the entity group. |
| stream port | cổng luồng | Shown in channel mappings as cổng { $number }. |
| channel mapping | ánh xạ kênh | Map / unmap: ánh xạ / bỏ ánh xạ. |
| control | điều khiển | With a classifier in sentences: mục điều khiển. |
| preset | preset | Kept, as on mixing desks. Recall is gọi lại. |
| identify | nhận dạng | "Could not identify" is phrased as sending the identify command. |
| counter | bộ đếm | |
| locked, lost lock | khóa, mất khóa | [0] variants: chưa khóa, chưa mất khóa. |
| interrupted | bị gián đoạn | |
| timestamp | dấu thời gian | |
| advertise, advertised | quảng bá, đã quảng bá | Discovery is phát hiện. |
| interface | giao diện (mạng) | Appearance in Settings is Diện mạo, to keep giao diện for interfaces. |
| hardware clock | đồng hồ phần cứng | |
| virtual (interface) | ảo | giao diện ảo. |

## Choices a native speaker should check

- **đặt trước for reservation.** Natural for bandwidth booking; dành riêng
  or dự trữ are the alternatives.
- **gán for bind.** Reads as "assigned"; liên kết is avoided because it is
  the word for link.
- **switch for bridge** and **đồng hồ media for media clock**: loanwords
  practitioners use, over cầu nối and the literal đồng hồ phương tiện.
- **preset and peer delay kept in English**, where cài đặt sẵn and độ trễ
  ngang hàng are the Vietnamese alternatives.
- **Kết hợp for (ADP) association**, chosen to keep liên kết for link.
- Shares such as 1.7% come from the code with a decimal point, not
  Vietnamese's decimal comma; only the percent sign's place is set here.
