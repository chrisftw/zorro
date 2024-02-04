extern crate png;
extern crate base64;
#[macro_use]
extern crate lazy_static;

const CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
lazy_static! {
    static ref CHARMAP: HashMap<char, u8> = {
        let mut cm = HashMap::with_capacity(64);
        CHARS.char_indices().for_each(|(i,c)| {cm.insert(c, i as u8);});
        cm
    };
}
lazy_static! {
    static ref REVCHARMAP: HashMap<u8, char> = {
        let mut cm = HashMap::with_capacity(64);
        CHARS.char_indices().for_each(|(i,c)| {cm.insert(i as u8, c);});
        cm
    };
}

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::BufWriter;

pub fn encode(raw: &str, target_path: &str, mode: &str, depth: u8, src_path: &str) {
    let u8_values = encode_to_u8s(raw);
    let src_pixels:Vec<u8>;
    let mut w:u32 = 0;
    let mut h:u32 = 0;
    if !src_path.is_empty() {
        src_pixels = read_png(src_path);
        let info = read_png_info(src_path);
        w = info.width;
        h = info.height;
    } else {
        src_pixels = vec![];
    }
    let pixel_data = pixelize(u8_values, mode, depth, &src_pixels);
    write_png(pixel_data, target_path, w, h);
}

pub fn encode_from_file(filepath: &str, target_path: &str, mode: &str, depth: u8, src_path: &str) {
    let contents = fs::read_to_string(filepath)
        .expect("Something went wrong reading the file");
    encode(&contents, target_path, mode, depth, src_path);
}

pub fn decode(img_path: &str) -> String {
    let pparts = read_png(img_path);
    decode_pixels(&pparts)
}

pub fn decode_pixels(pparts: &[u8]) -> String {
    let u8_vals = depixelize(pparts);
    decode_from_u8s(u8_vals)
}

pub fn decode_to_file(in_path: &str, out_path: &str) {
    let decoded_data = decode(in_path);
    fs::write(out_path, decoded_data).expect("Unable to write file");
}

pub fn decode_file_data(in_data: &[u8]) -> String {
    let pparts = read_png_data(in_data);
    decode_pixels(&pparts)
}

fn write_png(mut pixel_parts: Vec<u8>, target_path: &str, mut w:u32, mut h:u32) {
    if w == 0 || h == 0 {
        let size:u32 = pixel_parts.len() as u32;
        let side:u32 = ((size as f64)/3.0).sqrt().ceil() as u32;
        if w == 0 {
            w = side;
        }
        if h == 0 {
            let area:u32 = side * side;
            let blank_parts:u32 = (area*3) - size;
            let blank_lines:u32 = blank_parts / (3 * side);
            let vec_size: usize = (blank_parts % (side*3)) as usize;
            let mut blanks = vec![0; vec_size];

            pixel_parts.append(&mut blanks);
            h = side - blank_lines;
        }
    }
    let path = Path::new(target_path);
    let file = File::create(path).unwrap();
    let writer = &mut BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, w, h);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut filewriter = encoder.write_header().unwrap();
    filewriter.write_image_data(&pixel_parts).unwrap(); // save
}

fn read_png(path: &str) -> Vec<u8> {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    //decode_png(decoder);
    let (info, mut reader) = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    // Inspect more details of the last read frame.
    buf
}

fn read_png_info(path: &str) -> png::OutputInfo {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    let (info, _) = decoder.read_info().unwrap();
    info
}

fn read_png_data(data: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(data);
    let (info, mut reader) = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; info.buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    buf
}

fn write_meta_pixel(mode: &str, depth: u8) -> Vec<u8> {
    let mut pixel_data:Vec<u8> = Vec::new();
    match mode {
        "static" => pixel_data.push(1), // mode static (1)
        "hidden" => pixel_data.push(2), // mode static (2)
        _ => println!("Unknown Mode: {:?}", mode)
    }
    pixel_data.push(depth%8); // bit-depth 8
    pixel_data.push(1); // version 1
    pixel_data
}

fn pixelize(data: Vec<u8>, mode: &str, depth: u8, src_pixels: &[u8]) -> Vec<u8> {
    let mut pixel_data = write_meta_pixel(mode, depth);
    if mode == "static" {
        for i in (0..data.len()).step_by(4) {
            pixel_data.push((data[i] << 2) + ((data[i+1] & 48) >> 4));
            pixel_data.push(((data[i+1] & 15) << 4) + ((data[i+2] & 60) >> 2));
            pixel_data.push((data[i+2] << 6) + data[i+3]);
        }
    } else if mode == "hidden" {
        // merge first src_pixel into meta pixel
        for i in 0..3 {
            pixel_data[i] += src_pixels[i] & 248;
        }
        let blank_pixels:i32 = (src_pixels.len() as i32) - (((6/depth) as i32) * (data.len() as i32)) - 3; // minus 3 for meta pixel
        if blank_pixels < 0 {
            panic!("source image is too small too hold this data.");
        }
        if depth == 1 {
            // not implemented
        } else if depth == 2 {
            for i in (0..data.len()).step_by(1) {
                pixel_data.push((src_pixels[i*3 + 3] & 252) + ((data[i] & 48) >> 4));
                pixel_data.push((src_pixels[(i*3) + 4] & 252) + ((data[i] & 12) >> 2));
                pixel_data.push((src_pixels[(i*3) + 5] & 252) + (data[i] & 3));
            }
        } else if depth == 3 {
            // not implemented
        } else if depth == 4 {
            // not implemented
        } else if depth == 6 {
            for i in (0..data.len()).step_by(3) {
                pixel_data.push((src_pixels[i] & 192) + (data[i] & 63));
                pixel_data.push((src_pixels[i + 1] & 192) + (data[i+1] & 63));
                pixel_data.push((src_pixels[i + 2] & 192) + (data[i+2] & 63));
            }
        }
        // rest of source needs 0's on it.
        let src_len = src_pixels.len();
        let last_pixel = src_len - (blank_pixels as usize);
        //println!("leftovers should be divisible by 3: {:?}", last_pixel);
        for pixel in src_pixels.iter().take(src_len).skip(last_pixel) {
            pixel_data.push(pixel & 252);
        }
    }
    pixel_data
}

fn depixelize(pixels: &[u8]) -> Vec<u8> {
    let _mode:u8 = pixels[0] & 7;
    let depth:u8 = pixels[1] & 7;
    let _version:u8 = pixels[2] & 7;
    //println!("MODE: {:?}, DEPTH: {:?}, VERSION: {:?}", _mode, depth, _version );
    let mut u8_vals:Vec<u8> = Vec::new();
    if depth == 0 { // depth 8
        for i in (3..pixels.len()).step_by(3) {
            u8_vals.push(pixels[i] >> 2);
            u8_vals.push(((pixels[i] & 3) << 4) + (pixels[i+1] >> 4));
            u8_vals.push(((pixels[i+1] & 15) << 2) + (pixels[i+2] >> 6));
            u8_vals.push(pixels[i+2] & 63);
        }
    } else if depth == 1 {
        let mut mask:u8 = 0;
        let mut tmp:u8 = 0;
        for i in (3..pixels.len()).step_by(3) {
            if mask == 0 {
                tmp = ((pixels[i] & 1) << 6) + ((pixels[i+1] & 1) << 5) + ((pixels[i+2] & 1) << 4);
                mask = 56;
            } else {
                u8_vals.push(tmp + ((pixels[i] & 1) << 3) + ((pixels[i+1] & 1) << 2) + (pixels[i+2] & 1));
                tmp = 0;
                mask = 0;
            }
        }
        if mask != 0 {
            u8_vals.push(tmp);
        }
    } else if depth == 2 {
        for i in (3..pixels.len()).step_by(3) {
            u8_vals.push(((pixels[i] & 3) << 4) + ((pixels[i+1] & 3) << 2) + (pixels[i+2] & 3));
        }
    } else if depth == 3 {
        let mut mask:u8 = 0;
        let mut tmp:u8 = 0;
        for i in (3..pixels.len()).step_by(3) {
            if mask == 0 {
                u8_vals.push(((pixels[i] & 7) << 3) + (pixels[i+1] & 7));
                tmp = (pixels[i+2] & 7) << 3;
                mask = 56;
            } else {
                u8_vals.push(tmp + (pixels[i] & 7));
                u8_vals.push(((pixels[i+1] & 7) << 3) + (pixels[i+2] & 7));
                tmp = 0;
                mask = 0;
            }
        }
        if mask != 0 {
            u8_vals.push(tmp);
        }
    } else if depth == 4 {
        for i in (3..pixels.len()).step_by(3) {
            u8_vals.push(((pixels[i] & 15) << 2) + ((pixels[i+1] & 12) >> 4));
            u8_vals.push(((pixels[i+1] & 3) << 4) + (pixels[i+2] & 15));
        }
    // five is a bit of a pain this way.
    } else if depth == 6 {
        for i in (3..pixels.len()).step_by(3) {
            u8_vals.push(pixels[i] & 63);
            u8_vals.push(pixels[i+1] & 63);
            u8_vals.push(pixels[i+2] & 63);
        }
    }
    u8_vals
}

#[allow(clippy::same_item_push)]
pub fn encode_to_u8s(raw: &str) -> Vec<u8> {
    let encoded_str = base64::encode(raw);
    let mut v : Vec<u8> = Vec::new();
    // pull off the ='s first and count them.
    let mut pad_count:u8 = 1;
    encoded_str.char_indices().for_each(|(_,c)| {
        if c == '=' {
            pad_count += 1;
        } else {
            v.push(CHARMAP[&c]);
        }
    });
    v.push(pad_count); // Now add pad_count
    let blank_pads = v.len() % 3;
    if blank_pads != 0 {
        for _ in 0..(3-blank_pads) {
            v.push(0);
        }
    }
    v
}

pub fn decode_from_u8s(mut data: Vec<u8>) -> String {
    // pull off padding count and subtract 1
    let mut pads:u8 = data.pop().unwrap_or(0);
    while pads == 0 {
        pads = data.pop().unwrap_or(0);
    }
    // restringify ???  needed?
    let mut encoded_str = String::from("");
    //println!("ENCODED: {:?}", data);
    for n in data {
        encoded_str.push(REVCHARMAP[&n]);
    };
    // add back the =s
    for _ in 0..(pads-1) {
        encoded_str.push('=');
    }
    let decoded_str_u8s =  base64::decode(encoded_str).unwrap();
    String::from_utf8(decoded_str_u8s).unwrap()
}

use std::io::Read;

#[allow(dead_code)]
fn get_file_as_byte_vec(filename: &str) -> Vec<u8> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_encodes_to_u8s() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        println!("called `zorro::encode()`");
        let resp = encode_to_u8s(raw_data);
        assert_eq!(vec![30, 50, 9, 36, 24, 23, 37, 51, 8, 35, 40, 32, 22, 50, 9, 19, 29, 18, 8, 44, 8, 2, 9, 13, 27, 50, 8, 44, 8, 2, 9, 20, 29, 18, 8, 44, 8, 2, 9, 23, 25, 18, 8, 44, 8, 2, 9, 20, 26, 2, 8, 44, 8, 2, 9, 6, 28, 34, 8, 44, 8, 2, 9, 19, 24, 18, 9, 29, 31, 16, 3, 0], resp);
    }

    #[test]
    fn it_encodes_to_png() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        println!("called `zorro::encode()`");
        encode(raw_data, "./examples/days.png", "static", 8, "");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn it_decodes_from_png() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        let found_data = decode("./examples/days1.png");
        assert_eq!(raw_data, found_data);
    }

    #[test]
    fn it_encodes_from_file() {
        encode_from_file("./examples/colors.json", "./examples/colors.png", "static", 8, "");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn it_decodes_to_file() {
        decode_to_file("./examples/colors1.png", "./examples/colors1.json");
        assert_eq!(2 + 2, 4);
    }

    // #[test]
    // fn it_encodes_big_file() {
    //     encode_from_file("./examples/big_file.json", "./examples/big_file.png", "static", 8, "");
    //     assert_eq!(2+2, 4);
    // }

    #[test]
    fn it_decodes_from_png_data() {
        let raw_text_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        let raw_data = get_file_as_byte_vec("./examples/days1.png");
        let found_data = decode_file_data(&raw_data);
        assert_eq!(raw_text_data, found_data);
    }

    #[test]
    fn it_encodes_in_hidden_mode_depth_2() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        encode(raw_data, "./examples/basn2c08-2.png", "hidden", 2, "./examples/basn2c08.png");
        assert_eq!(2+2, 4);
    }

    #[test]
    fn it_decodes_from_hidden_mode_depth_2() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        let found_data = decode("./examples/basn2c08-2dc.png");
        assert_eq!(raw_data, found_data);
    }

    #[test]
    fn it_encodes_in_hidden_mode_depth_6() {
        let raw_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        encode(raw_data, "./examples/basn2c08-6.png", "hidden", 6, "./examples/basn2c08.png");
        assert_eq!(2+2, 4);
    }
}
