/**
 * Represents an error that occurs when an RPC call is aborted.
 *
 * @class
 * @extends {Error}
 */
export declare class RPCAbortError extends Error {
    id: number;
    name: string;
    data: string | ArrayLike<number>;
    /**
    * Creates an instance of RPCAbortError.
    *
    * @param {number} id - The unique identifier of the RPC call that was aborted.
    * @param {string} name - The name of the RPC event.
    * @param {string | ArrayLike<number>} data - The data associated with the RPC call.
    * @param {string} [reason] - An optional reason for why the call was aborted.
    */
    constructor(id: number, name: string, data: string | ArrayLike<number>, reason?: string);
}
export declare class SocketIo {
    #private;
    /**
     * The WebSocket instance used for communication.
     */
    ws: WebSocket;
    /**
    * Creates a new SocketIo instance.
    * @param {string | URL} url - The URL to connect to via WebSocket.
    */
    constructor(url: string | URL);
    /**
     * Retrieves the current connection status.
     * An object containing arrays of pending and active events ids.
     */
    status(): {
        pending: string[];
        events: string[];
    };
    /**
    * Removes a registered event.
    * @param {string} name - The name of the event to remove.
    * returns `true` if the event was successfully removed
    */
    removeEvent(name: string): boolean;
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
    on(name: string): AsyncGenerator<Uint8Array, Uint8Array | undefined, unknown>;
    /**
     * Returns a promise that resolves when the connection is successfully established.
     */
    connect(): Promise<unknown>;
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
    call(name: string, data: string | ArrayLike<number>, opt?: {
        signal?: AbortSignal;
    }): Promise<Uint8Array>;
    /**
    * Sends a notification message to the server without waiting for a response.
    * @param {string} name - The event name to send.
    * @param {string | ArrayLike<number>} data - The data to send.
    * @example
    * socket.notify('update', 'new data');
    */
    notify(name: string, data: string | ArrayLike<number>): void;
}
