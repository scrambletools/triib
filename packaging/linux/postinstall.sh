#!/bin/sh
# Lets triib and triib-cli send and receive raw Ethernet without root.
for program in /usr/bin/triib /usr/bin/triib-cli; do
    if ! setcap cap_net_raw+ep "$program"; then
        echo "triib: could not grant $program CAP_NET_RAW; run: sudo setcap cap_net_raw+ep $program" >&2
    fi
done
exit 0
