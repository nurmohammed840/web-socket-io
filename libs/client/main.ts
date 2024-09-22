

export class SocketIo<I, O> {
    ws!: WebSocket
    constructor(url: string | URL) {
        this.ws = new WebSocket(url);
    }

    send<K extends keyof I>(ev: K, value: I[K]) { }

    notify<K extends keyof I>(ev: K, value: I[K]) { }

    async *on<K extends keyof O>(ev: K): AsyncGenerator<O[K]> {
        // yield "adwwa"
    }
}

interface Events {
    name: "Data"
}

interface Output {
    name: "Data"
}

let socket = new SocketIo<Events, Output>("");
socket.send("name", "Data");

for await (const ev of socket.on("name")) { }