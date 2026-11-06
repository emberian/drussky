// `a` is supplied by the Rust transport. Error bodies only reach its exact
// public-message allowlist; arbitrary upstream text is never displayed.
const controller = new AbortController();
const timer = setTimeout(() => controller.abort(), a.timeout);
let errorStatus;
try {
    const options = {
        method: a.body === null ? 'GET' : 'POST', redirect: 'error',
        credentials: 'omit', signal: controller.signal,
        headers: {'Authorization': 'Bearer ' + a.token}
    };
    if (a.body !== null) {
        options.headers['Content-Type'] = 'application/json';
        options.body = a.body;
    }
    const response = await fetch(a.url, options);
    if (!response.ok) errorStatus = response.status;
    if (!response.body) return errorStatus ? {status: errorStatus} : {error: 'empty'};
    const limit = errorStatus ? a.error_limit : a.limit;
    const reader = response.body.getReader();
    let size = 0, chunks = [];
    while (true) {
        const part = await reader.read();
        if (part.done) break;
        size += part.value.byteLength;
        if (size > limit) {
            await reader.cancel();
            return errorStatus ? {status: errorStatus} : {error: 'large'};
        }
        chunks.push(part.value);
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
    const text = new TextDecoder().decode(bytes);
    return errorStatus ? {status: errorStatus, detail: text} : {body: text};
} catch (error) {
    return errorStatus ? {status: errorStatus} :
        {error: error.name === 'AbortError' ? 'timeout' : 'connection'};
} finally {
    clearTimeout(timer);
}
