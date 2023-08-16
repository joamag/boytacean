/**
 * Converts an array of bytes into an image data object
 * ready to be used with a canvas context.
 *
 * @param buffer The array of bytes to convert.
 * @param width The width of the image in the buffer.
 * @returns The resulting image data object.
 */
export const bufferToImageData = (
    buffer: Uint8Array,
    width: number
): ImageData => {
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
 * Converts the provided array of bytes containing an image
 * data into a data URL ready to be used in an <img> tag.
 *
 * @param buffer The array of bytes containing the image data.
 * @param width The width of the image in the buffer.
 * @param format The format of the image as a MIME string.
 * @returns The resulting data URL.
 */
export const bufferToDataUrl = (
    buffer: Uint8Array,
    width: number,
    format = "image/png"
): string => {
    const imageData = bufferToImageData(buffer, width);

    const canvas = document.createElement("canvas");
    canvas.width = imageData.width;
    canvas.height = imageData.height;

    const context = canvas.getContext("2d");
    context?.putImageData(imageData, 0, 0);

    const dataUrl = canvas.toDataURL(format);
    return dataUrl;
};
