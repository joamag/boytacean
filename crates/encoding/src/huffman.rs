use boytacean_common::error::Error;
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    io::{Cursor, Read},
    mem::size_of,
};

#[derive(Debug, Eq, PartialEq)]
struct Node {
    frequency: u32,
    character: Option<u8>,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.frequency.cmp(&self.frequency)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn encode_huffman(data: &[u8]) -> Result<Vec<u8>, Error> {
    let frequency_map = build_frequency(data);
    let tree = build_tree(&frequency_map)
        .ok_or(Error::CustomError(String::from("Failed to build tree")))?;

    let mut codes = vec![Vec::new(); 256];
    build_codes(&tree, Vec::new(), &mut codes);

    let encoded_tree = encode_tree(&tree);
    let encoded_data = encode_data(data, &codes);
    let tree_length = encoded_tree.len() as u32;
    let data_length = data.len() as u64;

    let mut result = Vec::new();
    result.extend(tree_length.to_be_bytes());
    result.extend(encoded_tree);
    result.extend(data_length.to_be_bytes());
    result.extend(encoded_data);

    Ok(result)
}

pub fn decode_huffman(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut reader = Cursor::new(data);

    let mut buffer = [0x00; size_of::<u32>()];
    reader
        .read_exact(&mut buffer)
        .map_err(|_| Error::InvalidData)?;
    let tree_length = u32::from_be_bytes(buffer);

    let mut buffer = vec![0; tree_length as usize];
    reader
        .read_exact(&mut buffer)
        .map_err(|_| Error::InvalidData)?;
    let tree = decode_tree(&mut buffer.as_slice());

    let mut buffer = [0x00; size_of::<u64>()];
    reader
        .read_exact(&mut buffer)
        .map_err(|_| Error::InvalidData)?;
    let data_length = u64::from_be_bytes(buffer);

    let mut buffer =
        vec![0; data.len() - size_of::<u32>() - tree_length as usize - size_of::<u64>()];
    reader
        .read_exact(&mut buffer)
        .map_err(|_| Error::InvalidData)?;

    let result = decode_data(&buffer, &tree, data_length);

    Ok(result)
}

fn build_frequency(data: &[u8]) -> [u32; 256] {
    let mut frequency_map = [0_u32; 256];
    for &byte in data {
        frequency_map[byte as usize] += 1;
    }
    frequency_map
}

fn build_tree(frequency_map: &[u32; 256]) -> Option<Box<Node>> {
    let mut heap: BinaryHeap<Box<Node>> = BinaryHeap::new();

    for (byte, &frequency) in frequency_map.iter().enumerate() {
        if frequency == 0 {
            continue;
        }
        heap.push(Box::new(Node {
            frequency,
            character: Some(byte as u8),
            left: None,
            right: None,
        }));
    }

    while heap.len() > 1 {
        let left = heap.pop().unwrap();
        let right = heap.pop().unwrap();

        let merged = Box::new(Node {
            frequency: left.frequency + right.frequency,
            character: None,
            left: Some(left),
            right: Some(right),
        });

        heap.push(merged);
    }

    heap.pop()
}

fn build_codes(node: &Node, prefix: Vec<u8>, codes: &mut [Vec<u8>]) {
    if let Some(character) = node.character {
        codes[character as usize] = prefix;
    } else {
        if let Some(ref left) = node.left {
            let mut left_prefix = prefix.clone();
            left_prefix.push(0);
            build_codes(left, left_prefix, codes);
        }
        if let Some(ref right) = node.right {
            let mut right_prefix = prefix;
            right_prefix.push(1);
            build_codes(right, right_prefix, codes);
        }
    }
}

fn encode_data(data: &[u8], codes: &[Vec<u8>]) -> Vec<u8> {
    let mut bit_buffer = Vec::new();
    let mut current_byte = 0u8;
    let mut bit_count = 0;

    for &byte in data {
        let code = &codes[byte as usize];
        for &bit in code {
            current_byte <<= 1;
            if bit == 1 {
                current_byte |= 1;
            }
            bit_count += 1;

            if bit_count == 8 {
                bit_buffer.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }
    }

    if bit_count > 0 {
        current_byte <<= 8 - bit_count;
        bit_buffer.push(current_byte);
    }

    bit_buffer
}

fn decode_data(encoded: &[u8], root: &Node, data_length: u64) -> Vec<u8> {
    let mut decoded = Vec::new();
    let mut current_node = root;
    let mut bit_index = 0;

    for &byte in encoded {
        if decoded.len() as u64 == data_length {
            break;
        }

        for bit_offset in (0..8).rev() {
            let bit = (byte >> bit_offset) & 1;
            current_node = if bit == 0 {
                current_node.left.as_deref().unwrap()
            } else {
                current_node.right.as_deref().unwrap()
            };

            if let Some(character) = current_node.character {
                decoded.push(character);
                current_node = root;
            }

            if decoded.len() as u64 == data_length {
                break;
            }

            bit_index += 1;
            if bit_index == encoded.len() * 8 {
                break;
            }
        }
    }

    decoded
}

fn encode_tree(node: &Node) -> Vec<u8> {
    let mut result = Vec::new();
    if let Some(character) = node.character {
        result.push(1);
        result.push(character);
    } else {
        result.push(0);
        if let Some(ref left) = node.left {
            result.extend(encode_tree(left));
        }
        if let Some(ref right) = node.right {
            result.extend(encode_tree(right));
        }
    }
    result
}

fn decode_tree(data: &mut &[u8]) -> Box<Node> {
    let mut node = Box::new(Node {
        frequency: 0,
        character: None,
        left: None,
        right: None,
    });

    if data[0] == 1 {
        node.character = Some(data[1]);
        *data = &data[2..];
    } else {
        *data = &data[1..];
        node.left = Some(decode_tree(data));
        node.right = Some(decode_tree(data));
    }
    node
}

#[cfg(test)]
mod tests {
    use super::{decode_huffman, encode_huffman};

    #[test]
    fn test_huffman_encoding() {
        let data = b"this is an example for huffman encoding, huffman encoding, huffman encoding";
        let encoded = encode_huffman(data).unwrap();
        let decoded = decode_huffman(&encoded).unwrap();
        assert_eq!(data.to_vec(), decoded);
        assert_eq!(encoded.len(), 109);
        assert_eq!(decoded.len(), 75);
    }
}
