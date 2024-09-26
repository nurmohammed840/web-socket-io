"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SocketIo = void 0;
class SocketIo {
    ws;
    #id = 1;
    #req = {};
    #event = {};
    constructor(url) {
        this.ws = new WebSocket(url);
        this.ws.binaryType = "arraybuffer";
        this.ws.onmessage = (ev) => {
            let data = new Uint8Array(ev.data);
            let frame_type = data[0];
            // responce
            if (frame_type == 1) {
                let call_id = new DataView(data.buffer).getUint32(1, false);
                let payload = data.slice(5);
                this.#req[call_id]?.(payload);
                delete this.#req[call_id];
            }
            // emit
            else if (frame_type == 2) {
                let event_name_len = data[1];
                let event_name = new TextDecoder().decode(data.slice(2, event_name_len + 2));
                let payload = data.slice(2 + event_name_len);
                this.#event[event_name]?.enqueue(payload);
            }
        };
    }
    status() {
        return {
            pending: Object.keys(this.#req),
            events: Object.keys(this.#event)
        };
    }
    removeEvent(name) {
        return delete this.#event[name];
    }
    async *on(name) {
        let self = this;
        let stream = new ReadableStream({
            start(c) {
                self.#event[name] ??= c;
            }
        });
        let reader = stream.getReader();
        while (true) {
            const { done, value } = await reader.read();
            if (done)
                return value;
            yield value;
        }
    }
    async connect() {
        if (this.ws.readyState == this.ws.OPEN) {
            return;
        }
        if (this.ws.readyState == this.ws.CONNECTING) {
            return new Promise((resolve, reject) => {
                this.ws.onopen = ev => resolve(ev);
                this.ws.onclose = ev => reject(ev);
                this.ws.onerror = ev => reject(ev);
            });
        }
    }
    call(ev, value, opt) {
        let event = encodeEventName(ev);
        let id = this.#id++;
        let call_id = new Uint8Array(4);
        new DataView(call_id.buffer).setUint32(0, id, false);
        let { promise, resolve, reject } = Promise.withResolvers();
        if (opt?.signal) {
            opt.signal.onabort = (_ev) => {
                this.ws.send(concatBytes([
                    [3], // frame type (1 byte)
                    call_id
                ]));
                reject("cancelation");
                delete this.#req[id];
            };
        }
        this.#req[id] = resolve;
        this.ws.send(concatBytes([
            [
                1, // frame type (1 byte)
                event.byteLength // method name length (1 byte)
            ],
            event, // method name (utf8 bytes)
            call_id,
            typeof value == "string" ? new TextEncoder().encode(value) : value
        ]));
        return promise;
    }
    notify(ev, value) {
        let event = encodeEventName(ev);
        this.ws.send(concatBytes([
            [
                2, // frame type (1 byte)
                event.byteLength // method name length (1 byte)
            ],
            event, // method name (utf8 bytes)
            typeof value == "string" ? new TextEncoder().encode(value) : value
        ]));
    }
}
exports.SocketIo = SocketIo;
function encodeEventName(ev) {
    let event = new TextEncoder().encode(ev);
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
if (!Promise.withResolvers) {
    //@ts-ignore
    Promise.withResolvers = function () {
        let resolve, reject;
        const promise = new Promise((res, rej) => {
            resolve = res;
            reject = rej;
        });
        return { promise, resolve, reject };
    };
}
// --------------------------------------------------------
