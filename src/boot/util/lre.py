import sys

STRIP_EXTRA = False


def lre_encode(data):
    encoded = bytearray()
    length = len(data)
    index = 0

    while index < length:
        run_length = 1
        while (
            index + run_length < length
            and data[index] == data[index + run_length]
            and run_length < 255
        ):
            run_length += 1

        if run_length > 1:
            encoded.append(run_length)
            encoded.append(data[index])
            index += run_length
        else:
            encoded.append(1)
            encoded.append(data[index])
            index += 1

    encoded.append(0)
    return bytes(encoded)


def lre_decode(encoded_data):
    decoded = bytearray()
    length = len(encoded_data)
    index = 0

    while index < length:
        run_length = encoded_data[index]
        run_length = ord(run_length)
        if run_length == 0:
            break
        decoded.extend(encoded_data[index + 1] * run_length)
        index += 2

    return bytes(decoded)


if __name__ == "__main__":
    if not len(sys.argv) > 3:
        print("Usage: lre.py <mode> <infile> <outfile>")
        sys.exit(1)
    mode, infile, outfile = sys.argv[1:]

    if mode in ("encode", "compress"):
        operation = lre_encode
    elif mode in ("decode", "decompress"):
        operation = lre_decode
    else:
        raise ValueError("Invalid mode")

    with open(infile, "rb") as file:
        data = file.read()
        if STRIP_EXTRA and mode in ("encode", "compress"):
            data = data.rstrip(b"\x00")

    with open(outfile, "wb") as file:
        file.write(operation(data))
