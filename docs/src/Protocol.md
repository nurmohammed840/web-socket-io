# Protocol

This protocol draws inspiration from the
[JSON-RPC](https://www.jsonrpc.org/specification) specification, encoded in a
binary format.

Currently, the browser initiates communication with the server via
[WebSocket](https://en.wikipedia.org/wiki/WebSocket) using the specified
subprotocol `"websocket.io-rpc-v0.1"`.

## Frame

Each frame begins with an opcode (`u8`), indicating the frame type.

| Op Code (u8) | Frame Type | Description                                                                    |
| :----------: | :--------: | ------------------------------------------------------------------------------ |
|      1       |   Notify   | Sent by the client or server to indicate an event with no `Response` expected. |
|      2       |  Request   | Sent only by the client to initiate an RPC call and expect a `Response`.       |
|      3       |   Reset    | Sent only by the client to cancel an ongoing RPC call.                         |
|      4       |  Response  | Sent only by the server to return the result of a `Request`.                   |

### Notify Frame

A `Notify` is a `Request` frame without an `id` field. A `Request` frame that is
a `Notify` signifies the Client's lack of interest in the corresponding
`Response` frame, and as such no `Response` frame needs to be returned to the
client. The Server MUST NOT reply to a `Notify`.

|  Notify Frame   |   Type   |
| :-------------: | :------: |
|     Op Code     | 1 (`u8`) |
| Event Name Size |   `u8`   |
|   Event Name    |   UTF8   |
|     Payload     | `&[u8]`  |

- **Event Name Size**: The length of the event name, represented as `u8`. So
  maximum event name (utf8 encoded) length is 255 bytes.
- **Event Name**: A String (UTF8 encoded) containing the name of the method to
  be invoked.
- **Payload**: Application encoded data in bytes.

### Request Frame

A rpc call is represented by sending a Call Frame to a Server.

|   Call Frame    |   Type   |
| :-------------: | :------: |
|     Op Code     | 2 (`u8`) |
|       ID        |  `u32`   |
| Event Name Size |   `u8`   |
|   Event Name    |   UTF8   |
|     Payload     | `&[u8]`  |

- **ID**: A unique identifier for the RPC call, encoded in big-endian byte
  order.

The remaining fields are encoded in the same manner as `Notify` frame.

### Reset Frame

The `Reset` Frame is used to terminate the processing of an ongoing RPC call,
such as in the case of a timeout. If the server has already processed the RPC
call, the `Reset` frame has no effect.

| Reset Frame |   Type   |
| :---------: | :------: |
|   Op Code   | 3 (`u8`) |
|     ID      |  `u32`   |

- **ID**: The unique identifier (encoded in big endian byte order) of the RPC
  call to cancel.

### Response Frame

When a rpc call is made, the Server MUST reply with a `Response`, except for in
the case of `Notify`.

| Response Frame |   Type   |
| :------------: | :------: |
|    Op Code     | 4 (`u8`) |
|       ID       |  `u32`   |
|    Payload     | `&[u8]`  |

- **ID**: A unique identifier (`u32`), encoded in big endian byte order. It MUST
  be the same as the value of the `id` field in the `Request` frame.
- **Payload**: Application encoded data in bytes.
