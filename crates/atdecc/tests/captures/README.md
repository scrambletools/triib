# Golden captures

Frames from real devices, for `tests/golden.rs`. Each `.pcap` (classic pcap,
not pcapng) has a `.tsv` beside it with tshark's dissection of every frame,
which the test treats as an independent reading of the same octets.

## bench-adp-acmp

Captured from a host port of an AVB switch while `queries.py` sent only
read-only messages: an ENTITY_DISCOVER, then GET_TX_STATE and GET_RX_STATE
to each entity in both the 2021 form (control_data_length 84) and the
2013 and Milan form (44), commands with unknown unique IDs, and a
GET_TX_CONNECTION. Nothing in it changes a connection.

On the network:

- two Milan endpoints on ESP32 (one wired, one reached over Wi-Fi through a
  bridge), whose entity IDs are their MAC followed by `0000`;
- a macOS AVB entity, which answers in the short form;
- a bridge sending MAAP announcements, which share the AVTP ethertype.

The full form commands from `queries.py` declare control_data_length 84 but
carry 8 octets less, a real sender's mistake the decoder has to survive.

## bench-enumeration

The same network while `triib-cli describe <interface>` read every
entity's descriptors: an ENTITY_DISCOVER, GET_MILAN_INFO to each entity
(the macOS entity does not answer it, so it is retried once), 77
READ_DESCRIPTOR exchanges, then the streams' state: GET_RX_STATE for each
stream input and GET_STREAM_INFO for every stream. The controller sent
nothing that changes an entity, and no stream was bound. `tests/enumeration.rs` replays the entities' side through a
controller, which must send the same commands byte for byte.

## bench-network

The same network while `triib-cli network <interface>` read every entity
and then asked each AVB interface for GET_AVB_INFO, GET_AS_PATH and
GET_COUNTERS, for `tests/network.rs`. The switch is the grandmaster of
both wired entities:

- the macOS entity answers the first two and refuses GET_COUNTERS with
  NOT_IMPLEMENTED; its GET_AVB_INFO flags are all clear and its traffic
  classes are the SR class IDs 6 and 5;
- the wired ESP32 endpoint answers all three, its GET_COUNTERS response
  claiming 8 octets more than it carries (`docs/esp_avb.md`, finding 13);
- the Wi-Fi endpoint's AVB_INTERFACE descriptor did not read on this run,
  so it was not asked.

Its `.tsv` was exported with `-E occurrence=a -E aggregator=,`, so fields
that repeat, such as the MSRP mappings, list every value.

To make another:

```
dumpcap -i <interface> -f 'ether proto 0x22f0' -a duration:14 -F pcap -w capture.pcap &
sudo python3 queries.py <tools-dir> <interface> <entity-id>...
tshark -r capture.pcap -T fields -E header=y -E separator=/t -E occurrence=f \
  -e frame.number -e ieee1722.subtype -e ieee17221.message_type ... > capture.tsv
```

The field list is the header line of `bench-adp-acmp.tsv`.
