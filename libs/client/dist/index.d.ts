export declare class SocketIo {
    #private;
    ws: WebSocket;
    constructor(url: string | URL);
    status(): {
        pending: string[];
        events: string[];
    };
    removeEvent(name: string): boolean;
    on(name: string): AsyncGenerator<any, any, unknown>;
    connect(): Promise<any>;
    call<T>(ev: string, value: string | ArrayLike<number>, opt?: {
        signal?: AbortSignal;
    }): Promise<T>;
    notify(ev: string, value: string | ArrayLike<number>): void;
}
interface PromiseWithResolvers<T> extends Promise<T> {
    resolve(value: T | PromiseLike<T>): void;
    reject(reason?: any): void;
    promise: Promise<T>;
}
declare global {
    interface PromiseConstructor {
        withResolvers<T>(): PromiseWithResolvers<T>;
    }
}
export {};
