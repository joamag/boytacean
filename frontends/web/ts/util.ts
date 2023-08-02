/**
 * Encodes a linear buffer into a base64 string.
 *
 * @param buffer The linear buffer to encode.
 * @returns The resulting base64 string.
 */
export const bufferToBase64 = (buffer: Uint8Array) => {
    const data = Array(buffer.length)
        .fill(null)
        .map((_, i) => String.fromCharCode(buffer[i]))
        .join("");
    const base64 = btoa(data);
    return base64;
};

/**
 * Converts a base64 string into a linear buffer.
 *
 * @param base64 The base64 string to decode.
 * @returns The resulting linear buffer.
 */
export const base64ToBuffer = (base64: string) => {
    const data = atob(base64);
    const array = Array(data.length)
        .fill(null)
        .map((_, i) => data.charCodeAt(i));
    const buffer = new Uint8Array(array);
    return buffer;
};

/**
 * Converts a linear buffer into an image data object
 * ready to be used with a canvas context.
 *
 * @param buffer The linear buffer to convert.
 * @param width The width of the image in the buffer.
 * @returns The resulting image data object.
 */
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

/**
 * Converts the provided buffer containing an image data into
 * a data URL ready to be used in an <img> tag.
 *
 * @param buffer The buffer containing the image data.
 * @param width The width of the image in the buffer.
 * @returns The resulting data URL.
 */
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
