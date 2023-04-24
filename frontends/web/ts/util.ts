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

export const bufferToImageData = (buffer: Uint8Array, width: number) => {
    const clampedBuffer = new Uint8ClampedArray(buffer.length);

    for (let index = 0; index < clampedBuffer.length; index += 4) {
        clampedBuffer[index + 0] = buffer[index];
        clampedBuffer[index + 1] = buffer[index + 1];
        clampedBuffer[index + 2] = buffer[index + 2];
        clampedBuffer[index + 3] = buffer[index + 3];
    }

    return new ImageData(clampedBuffer, width);
};

export const bufferToDataUrl = (buffer: Uint8Array, width: number) => {
    const imageData = bufferToImageData(buffer, width);

    const canvas = document.createElement("canvas");
    canvas.width = imageData.width;
    canvas.height = imageData.height;

    const context = canvas.getContext("2d");
    context?.putImageData(imageData, 0, 0);

    const dataUrl = canvas.toDataURL();
    return dataUrl;
};
