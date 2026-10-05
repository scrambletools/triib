"""Read-only ADP and ACMP queries, for golden captures: ENTITY_DISCOVER,
GET_TX_STATE, GET_RX_STATE (full and truncated PDUs, valid and unknown
unique IDs) and GET_TX_CONNECTION. Nothing here changes a connection.

Uses the frame helpers of the atdecc_controller.py tool.

usage: sudo python3 queries.py <tools-dir> <interface> <entity-id>...
"""
import sys
import time

if len(sys.argv) < 4:
    sys.exit(__doc__)
sys.path.insert(0, sys.argv[1])
import atdecc_controller as tool

INTERFACE = sys.argv[2]
ENTITIES = [bytes.fromhex(entity.removeprefix("0x")) for entity in sys.argv[3:]]
GET_TX_CONNECTION_COMMAND = 12

mac = tool.get_mac_address(INTERFACE)
controller = tool.mac_to_entity_id(mac)
sock = tool.open_raw_socket(INTERFACE)
zero = b"\x00" * 8
sequence = 0x4100


def send(payload):
    tool.send_frame(sock, INTERFACE, tool.ACMP_MULTICAST, mac, payload)
    time.sleep(0.25)


def acmp(message_type, talker, listener, talker_uid=0, listener_uid=0,
         connection_count=0, truncated=False):
    global sequence
    sequence += 1
    payload = tool.build_acmp_message(message_type, controller, talker, listener,
                                      talker_uid, listener_uid, connection_count,
                                      sequence)
    if truncated:
        # The 2013 and Milan form: control_data_length 44, 56 octets.
        header = bytearray(payload[:4])
        header[2] = (header[2] & 0xf8) | (44 >> 8)
        header[3] = 44 & 0xff
        payload = bytes(header) + payload[4:56]
    send(payload)


discover = tool.encode_atdecc_header(tool.AVTP_SUBTYPE_ADP, tool.ADP_MSG_ENTITY_DISCOVER,
                                     0, 0, 0, 56) + zero + b"\x00" * 56
send(discover)
time.sleep(1.0)
for entity in ENTITIES:
    acmp(tool.ACMP_MSG_GET_TX_STATE_COMMAND, entity, zero)
    acmp(tool.ACMP_MSG_GET_RX_STATE_COMMAND, zero, entity)
    acmp(tool.ACMP_MSG_GET_RX_STATE_COMMAND, zero, entity, truncated=True)
    acmp(tool.ACMP_MSG_GET_TX_STATE_COMMAND, entity, zero, truncated=True)
acmp(tool.ACMP_MSG_GET_RX_STATE_COMMAND, zero, ENTITIES[0], listener_uid=99, truncated=True)
acmp(tool.ACMP_MSG_GET_TX_STATE_COMMAND, ENTITIES[0], zero, talker_uid=99, truncated=True)
acmp(GET_TX_CONNECTION_COMMAND, ENTITIES[0], zero, truncated=True)
time.sleep(1.0)
print("sent, last sequence", hex(sequence))
