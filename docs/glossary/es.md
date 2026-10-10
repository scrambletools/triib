# Spanish (es) term choices

How `i18n/es/triib.ftl` renders the terms in `docs/GLOSSARY.md`. The text
addresses the user as *tú*, uses « » for quotation marks, a no-break space
before `%` and a no-break space as the thousands separator, as the RAE
recommends.

| English | Rendering | Note |
|---|---|---|
| talker | talker (m.), talkers | Loanword, lower case mid-sentence: «Flujos de talker», «esperando al talker». |
| listener | listener (m.), listeners | Loanword, as for talker. |
| grandmaster | grandmaster (m.) | Kept in Latin script; never «maestro». |
| entity | entidad | Not «dispositivo», which is kept for *device* in the network view. |
| entity model | modelo de entidad | |
| controller | controlador | |
| stream | flujo | The word Spanish AVB and Dante manuals use. |
| stream input, stream output | entrada de flujo, salida de flujo | Matrix: «Salidas de talker», «Entradas de listener». |
| connection, connect, bind | conexión, conectar; vincular | A bound input is «vinculada» (la entrada); «conectar» stays for the act of connecting. |
| media clock | reloj de medios | Also in «VLAN de medios», «reinicio de medios». |
| clock domain | dominio de reloj | |
| clock source | fuente de reloj | |
| sampling rate | frecuencia de muestreo | |
| bridge | switch (m.), switches | Practitioners say switch; «puente» reads as a lay word. «switch AVB» where the standard says AVB bridge. |
| reservation | reserva | |
| egress | salida | «puerto de salida»; the column and the alarm say «tráfico de salida» so it does not read as a stream output. |
| link | enlace | «Enlace activo», «enlace caído», «Caídas del enlace». |
| peer delay | peer delay | Kept, as Spanish PTP writing does. |
| offset | offset (m.) | Kept, as PTP tools show it. |
| hop | salto | |
| unicast, multicast | unicast, multicast | Kept. |
| fan-out | fan-out | Not shown as a word; the inspector says «Hasta N listeners por flujo, luego multicast». |
| descriptor | descriptor | |
| cluster | clúster, clústeres | RAE spelling. |
| stream port | puerto de flujo | |
| channel mapping | mapeo de canales | Verbs: «mapear», «quitar el mapeo». |
| control | control | |
| preset | preset (m.), presets | The word desks and their manuals use; «Recuperar» for recall. |
| identify | identificar | |
| counter | contador | |
| locked, lost lock | enganchado, perder el enganche | PLL lock: «se enganchó», «perdió el enganche». |
| holding over | en holdover | Kept in English, as Spanish telecom and PTP writing does. |
| station | estación (f.) | The IEEE 802.11 term: «Estación, enganchada». |
| access point | punto de acceso | |
| beacon | baliza (f.) | «Mode B, a partir de las balizas». |
| signal | señal | The inspector label, in dBm. |
| interrupted | interrumpido | |
| timestamp | marca de tiempo | |
| advertise, advertised | anunciar, anunciado | The ADP section is «Anuncio». |
| interface | interfaz (f.) | «activa», «inalámbrica» agree with it. |
| hardware clock | reloj de hardware | |
| virtual (interface) | virtual | |

Other recurring words: frame «trama», reported «informado», heard
«detectado» (in the map) and «Recibida» (in the log), Settings «Ajustes»,
log «Registro».

## Choices a native speaker should check

- «flujo» for *stream*: many engineers say «stream» aloud; the written
  manuals lean to «flujo».
- «reloj de medios» for *media clock*: an alternative is to keep
  «media clock».
- «enganche» for *lock*: some engineers say «lock» or «sincronizado».
- «offset» and «peer delay» kept in English rather than «desfase» and
  «retardo entre pares».
- «switch» for *bridge*, including «Puerto del switch» and «Switches por
  debajo».
- «Frecuencia de muestreo» may be long for the inspector's labels.
- The no-break space as thousands separator (RAE) rather than the point
  Spain also uses or the comma used in Mexico.
- «en holdover» for *holding over*, rather than «en retención».
- «baliza» for *beacon*; many Wi-Fi engineers say «beacon».
- «Inalámbrico» for the Wireless column; «Wi-Fi» is the alternative.
- «Sin sincronización» for the time mode «No time».
