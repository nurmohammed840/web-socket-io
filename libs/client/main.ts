
export class SocketIo {
    ws!: WebSocket;
    #id = 1;
    #req: Record<number, (value: any) => void> = {}

    constructor(url: string | URL) {
        this.ws = new WebSocket(url);
        // this.ws.onmessage = (ev) => {
        //     ev.data;
        // }
    }

    async connect(): Promise<void> {
        if (this.ws.readyState == this.ws.OPEN) {
            return
        }
        if (this.ws.readyState == this.ws.CONNECTING) {
            return new Promise((resolve, reject) => {
                this.ws.onopen = _ev => resolve()
                this.ws.onclose = ev => reject(ev)
                this.ws.onerror = ev => reject(ev)
            });
        }
    }

    call<T>(ev: string, value: string | ArrayLike<number>, opt?: { signal?: AbortSignal }) {
        let event = encodeEventName(ev);
        let id = this.#id++;
        let call_id = new Uint8Array(4);
        new DataView(call_id.buffer).setUint32(0, id, false);

        let { promise, resolve, reject } = Promise.withResolvers<T>();

        if (opt?.signal) {
            opt.signal.onabort = (_ev) => {
                this.ws.send(concatBytes([
                    [3],  // frame type (1 byte)
                    call_id
                ]));
                reject("cancelation");
                delete this.#req[id];
            }
        }

        this.#req[id] = resolve;
        this.ws.send(concatBytes([
            [
                1,                  // frame type (1 byte)
                event.byteLength    // method name length (1 byte)
            ],
            event,                  // method name (utf8 bytes)
            call_id,
            typeof value == "string" ? new TextEncoder().encode(value) : value
        ]));

        return promise
    }

    notify(ev: string, value: string | ArrayLike<number>) {
        let event = encodeEventName(ev);
        this.ws.send(concatBytes([
            [
                2,                  // frame type (1 byte)
                event.byteLength    // method name length (1 byte)
            ],
            event,                  // method name (utf8 bytes)
            typeof value == "string" ? new TextEncoder().encode(value) : value
        ]));
    }

    // async *on<K extends keyof O>(ev: K): AsyncGenerator<O[K]> {
    //     // yield "adwwa"
    // }
}

function encodeEventName(ev: string) {
    let event = new TextEncoder().encode(ev);
    if (event.byteLength > 255) {
        throw new Error(`event name too big: '${ev}'`)
    }
    return event
}

function concatBytes(chunks: ArrayLike<number>[]) {
    let size = 0, offset = 0;

    for (const chunk of chunks) size += chunk.length;
    const bytes = new Uint8Array(size);

    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.length;
    }
    return bytes;
}


// ------------------------- polyfill -------------------------------

interface PromiseWithResolvers<T> extends Promise<T> {
    resolve(value: T | PromiseLike<T>): void;
    reject(reason?: any): void;
    promise: Promise<T>
}

declare global {
    interface PromiseConstructor {
        withResolvers<T>(): PromiseWithResolvers<T>;
    }
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

