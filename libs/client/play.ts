// let frame = new Uint8Array();
// let view = new DataView(frame.buffer);
// view.setUint8(0, 42)
// console.log(frame);


// const c = new AbortController();

// setTimeout(() => {
//     c.abort("hello")
// }, 2000);

// c.signal.onabort = (ev) => {

// };

function concatBuffers(chunks: Uint8Array[]) {
    if (chunks.length == 1) {
        return chunks[0];
    }
    let size = 0;
    for (const chunk of chunks) {
        size += chunk.byteLength;
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }
    return bytes;
}

console.log(
    concatBuffers([
        new Uint8Array([1]),
        new Uint8Array([256]),
    ])
);
