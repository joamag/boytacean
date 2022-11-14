export const bufferToBase64 = (buffer: Uint8Array) => {
    const data = Array(buffer.length)
        .fill(null)
        .map((_, i) => String.fromCharCode(buffer[i]))
        .join("");
    const base64 = btoa(data);
    return base64;
};

export const base64ToBuffer = (base64: string) => {
    const data = atob(base64);
    const array = Array(data.length)
        .fill(null)
        .map((_, i) => data.charCodeAt(i));
    const buffer = new Uint8Array(array);
    return buffer;
};
