WebSocket API
==============================================================================

For live updates of OGN positions the `ogn-web-gateway` provides a WebSocket
API described in this document.

By default the server will send no data over the websocket unless the client
has subscribed to either a list of APRS sender IDs or a geographic bounding
box (e.g. for map viewers).


APRS Sender ID Subscriptions
------------------------------------------------------------------------------

Clients can subscribe to APRS sender IDs by sending e.g.:

```
+id|FLRDD87AC
```

or to unsubscribe from a previously subscribed ID:

```
-id|FLRDD87AC
```


APRS Bounding Box Subscription
------------------------------------------------------------------------------

Clients start with an empty bounding box by default. There is only a single
bounding box subscription per client and it can be changed like this:

```
bbox|-12.521|25.171|28.704|61.963
```

The order of the angles is: west, south, east, north.


OGN Position Records
------------------------------------------------------------------------------

Messages received from the websocket at `/api/live` are prefixed with a `$`
character followed by the message type. Since OGN position records are most
common those are explicitly **not** prefixed and don't carry the message type
to save bytes.

The fields in an OGN position records are separated by the `|` character and
are sent in the following order:

- APRS sender ID (e.g. `FLRDD87AC`)
- Unix timestamp (in seconds)
- WGS84 longitude (in degrees)
- WGS84 latitude (in degrees)
- Course (in degrees from North)
- Altitude (in meters)

Example:

```
FLRC04EFE|1531605102|-75.117233|45.493900|16|743
```