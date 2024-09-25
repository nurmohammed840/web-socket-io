import { SocketIo } from "../../client/main.ts";

const socket = new SocketIo("ws://localhost:3000/basic");
await socket.connect();

socket.notify("hello", "world");
