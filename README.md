# web-socket-io

It provides a robust framework for real-time communication over
[WebSocket](https://en.wikipedia.org/wiki/WebSocket), inspired by
[Socket.IO](https://socket.io/). It simplifies the process of sending and
receiving messages while offering built-in support for cancellation and timeout
functionalities.

[![Crates.io][crates-badge]][crates-url]
[![Documentation](https://docs.rs/web-socket-io/badge.svg)](https://docs.rs/web-socket-io)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

[crates-badge]: https://img.shields.io/crates/v/web-socket-io.svg
[crates-url]: https://crates.io/crates/web-socket-io

## Features

- **Request/Response**: clients to send requests and receive responses from the
  server.
- **Cancellation**: mechanisms to cancel ongoing operations on requests.
- **Bi-directional Notifications**: allowing both clients and servers to notify
  each other of events instantly. similar to [Socket.IO](https://socket.io/)

### Learn More

- [Tutorial](https://nurmohammed840.github.io/web-socket-io/Tutorial.html) -
  Step-by-step guide to get you started.
- [Protocol Design](https://nurmohammed840.github.io/web-socket-io/Protocol.html) -
  Overview of the protocol used for communication.

### License

This project is licensed under the MIT License.
