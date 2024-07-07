#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <unistd.h>
#include <assert.h>

static void opts(uint8_t byte, uint8_t *options)
{
    *(options++) = byte | ((byte << 1) & 0xff);
    *(options++) = byte & (byte << 1);
    *(options++) = byte | ((byte >> 1) & 0xff);
    *(options++) = byte & (byte >> 1);
}

void write_all(FILE *stream, const void *buf, size_t count)
{
    size_t written = 0;
    while (written < count)
    {
        size_t result = fwrite((const char *)buf + written, 1, count - written, stream);
        if (result <= 0)
        {
            perror("fwrite");
            exit(EXIT_FAILURE);
        }
        written += result;
    }
}

int main(int argc, char *argv[])
{
    if (argc < 3)
    {
        fprintf(stderr, "Usage: %s <input_file> <output_file>\n", argv[0]);
        return EXIT_FAILURE;
    }

    FILE *input = fopen(argv[1], "rb");
    if (!input)
    {
        perror("fopen input");
        return EXIT_FAILURE;
    }

    FILE *output = fopen(argv[2], "wb");
    if (!output)
    {
        perror("fopen output");
        fclose(input);
        return EXIT_FAILURE;
    }

    static uint8_t source[0x4000];
    size_t size = fread(source, 1, sizeof(source), input);
    fclose(input);
    unsigned pos = 0;
    assert(size <= 0x4000);

    // Remove trailing zeroes, as the GB memory is zeroed
    // during the Boot ROM initialization
    while (size && source[size - 1] == 0)
    {
        size--;
    }

    uint8_t literals[8];
    size_t literals_size = 0;
    unsigned bits = 0;
    unsigned control = 0;
    unsigned prev[2] = {-1, -1}; // Unsigned to allow "not set" values

    while (true)
    {
        uint8_t byte = 0;
        if (pos == size)
        {
            if (bits == 0)
                break;
        }
        else
        {
            byte = source[pos++];
        }

        if (byte == prev[0] || byte == prev[1])
        {
            bits += 2;
            control <<= 1;
            control |= 1;
            control <<= 1;
            if (byte == prev[1])
            {
                control |= 1;
            }
        }
        else
        {
            bits += 2;
            control <<= 2;
            uint8_t options[4];
            opts(prev[1], options);
            bool found = false;
            for (unsigned i = 0; i < 4; i++)
            {
                if (options[i] == byte)
                {
                    // 01 = modify
                    control |= 1;

                    bits += 2;
                    control <<= 2;
                    control |= i;
                    found = true;
                    break;
                }
            }
            if (!found)
            {
                literals[literals_size++] = byte;
            }
        }

        prev[0] = prev[1];
        prev[1] = byte;
        if (bits >= 8)
        {
            uint8_t outctl = control >> (bits - 8);
            assert(outctl != 1); // 1 is reserved as the end byte
            write_all(output, &outctl, 1);
            write_all(output, literals, literals_size);
            bits -= 8;
            control &= (1 << bits) - 1;
            literals_size = 0;
        }
    }
    uint8_t end_byte = 1;
    write_all(output, &end_byte, 1);
    fclose(output);

    return 0;
}
