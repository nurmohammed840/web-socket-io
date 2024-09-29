/**
 * Represents an error that occurs when an RPC call is aborted.
 *
 * @class
 * @extends {Error}
 */
export class RPCAbortError extends Error {
    id;
    name;
    data;
    /**
    * Creates an instance of RPCAbortError.
    *
    * @param {number} id - The unique identifier of the RPC call that was aborted.
    * @param {string} name - The name of the RPC event.
    * @param {string | ArrayLike<number>} data - The data associated with the RPC call.
    * @param {string} [reason] - An optional reason for why the call was aborted.
    */
    constructor(id, name, data, reason) {
        super(reason);
        this.id = id;
        this.name = name;
        this.data = data;
    }
}
export class SocketIo {
    /**
     * The WebSocket instance used for communication.
     */
    ws;
    #next_id = 1;
    #rpc = {};
    #event = {};
    /**
    * Creates a new SocketIo instance.
    * @param {string | URL} url - The URL to connect to via WebSocket.
    */
    constructor(url) {
        this.ws = new WebSocket(url, "websocket.io-rpc-v0.1");
        this.ws.binaryType = "arraybuffer";
        this.ws.onmessage = (ev) => {
            const data = new Uint8Array(ev.data);
            const frame_type = data[0];
            // Notify
            if (frame_type == 1) {
                const event_name_len = data[1];
                const event_name = new TextDecoder().decode(data.slice(2, event_name_len + 2));
                const payload = data.slice(2 + event_name_len);
                this.#event[event_name]?.enqueue(payload);
            }
            // Response
            else if (frame_type == 4) {
                const rpc_id = new DataView(data.buffer).getUint32(1, false);
                const payload = data.slice(5);
                this.#rpc[rpc_id]?.(payload);
                delete this.#rpc[rpc_id];
            }
        };
    }
    /**
     * Retrieves the current connection status.
     * An object containing arrays of pending and active events ids.
     */
    status() {
        return {
            pending: Object.keys(this.#rpc),
            events: Object.keys(this.#event)
        };
    }
    /**
    * Removes a registered event.
    * @param {string} name - The name of the event to remove.
    * returns `true` if the event was successfully removed
    */
    removeEvent(name) {
        return delete this.#event[name];
    }
    /**
     * Listens for the specified event and yields received data asynchronously.
     * @param {string} name - The name of the event to listen for.
     * @example
     * (async () => {
     *   for await (const data of socket.on('message')) {
     *     console.log(new TextDecoder().decode(data)); // Process incoming data
     *   }
     * })();
     */
    async *on(name) {
        const stream = new ReadableStream({
            start: c => {
                this.#event[name] ??= c;
            }
        });
        const reader = stream.getReader();
        while (true) {
            const { done, value } = await reader.read();
            if (done)
                return value;
            yield value;
        }
    }
    /**
     * Returns a promise that resolves when the connection is successfully established.
     */
    async connect() {
        if (this.ws.readyState == this.ws.OPEN) {
            return;
        }
        if (this.ws.readyState == this.ws.CONNECTING) {
            return await new Promise((resolve, reject) => {
                this.ws.onopen = ev => resolve(ev);
                this.ws.onclose = ev => reject(ev);
                this.ws.onerror = ev => reject(ev);
            });
        }
    }
    /**
     * Sends a message to the server and waits for a response.
     *
     * @param {string} name - The event name to send.
     * @param {string | ArrayLike<number>} data - The data to send.
     * @param {{ signal?: AbortSignal }} [opt] - Optional configuration, including an abort signal.
     *
     * @example
     * const res = await socket.call('greet', 'hello');
     * console.log(new TextDecoder().decode(res)); // Server's response
     */
    async call(name, data, opt) {
        const event_name = encodeEventName(name);
        const id = this.#next_id++;
        const rpc_id = new Uint8Array(4);
        new DataView(rpc_id.buffer).setUint32(0, id, false);
        const { promise, resolve, reject } = Promise.withResolvers();
        if (opt?.signal) {
            opt.signal.onabort = () => {
                this.ws.send(concatBytes([
                    [3], // frame type (1 byte)
                    rpc_id
                ]));
                reject(new RPCAbortError(id, name, data, opt.signal?.reason));
                delete this.#rpc[id];
            };
        }
        this.#rpc[id] = resolve;
        this.ws.send(concatBytes([
            [2], // frame type (1 byte)
            rpc_id,
            [event_name.length], // method name length (1 byte)
            event_name, // method name (utf8 bytes)
            typeof data == "string" ? new TextEncoder().encode(data) : data
        ]));
        const response = await promise;
        if (opt?.signal) {
            opt.signal.onabort = (_) => { };
        }
        return response;
    }
    /**
    * Sends a notification message to the server without waiting for a response.
    * @param {string} name - The event name to send.
    * @param {string | ArrayLike<number>} data - The data to send.
    * @example
    * socket.notify('update', 'new data');
    */
    notify(name, data) {
        const event_name = encodeEventName(name);
        this.ws.send(concatBytes([
            [
                1, // frame type (1 byte)
                event_name.length // method name length (1 byte)
            ],
            event_name, // method name (utf8 bytes)
            typeof data == "string" ? new TextEncoder().encode(data) : data
        ]));
    }
}
function encodeEventName(ev) {
    const event = new TextEncoder().encode(ev);
    if (event.byteLength > 255) {
        throw new Error(`event name too big: '${ev}'`);
    }
    return event;
}
function concatBytes(chunks) {
    let size = 0, offset = 0;
    for (const chunk of chunks)
        size += chunk.length;
    const bytes = new Uint8Array(size);
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
    }
    return bytes;
}
