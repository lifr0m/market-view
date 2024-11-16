## Websockets Cheat Sheet

### Send

- `client.feed` + `client.flush` (manually feed items and then manually flush)
- `client.send` (add one item and then flush)
- `client.send_all` (feed from stream and then flush)

### Receive

- `client.next` (receive next item)

### Close

- `client.close` (close the stream)
