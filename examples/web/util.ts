export const bufferToBase64 = (buffer: Uint8Array) => {
    const array = Array(buffer.length)
        .fill("")
        .map((_, i) => String.fromCharCode(buffer[i]))
        .join("");
    const base64 = btoa(array);
    return base64;
};

export const base64ToBuffer = (base64: string) => {
    const data = window.atob(base64);
    const length = data.length;
    const buffer = new Uint8Array(length);
    for (let i = 0; i < length; i++) {
        buffer[i] = data.charCodeAt(i);
    }
    return buffer;
};
