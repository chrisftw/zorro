extern crate png;
extern crate base64;
#[macro_use]
extern crate lazy_static;

const CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
lazy_static! {
    static ref CHARMAP: HashMap<char, u8> = {
        let mut cm = HashMap::with_capacity(64);
        CHARS.char_indices().for_each(|(i,c)| {cm.insert(c, i as u8); ()});
        cm
    };
}
lazy_static! {
    static ref REVCHARMAP: HashMap<u8, char> = {
        let mut cm = HashMap::with_capacity(64);
        CHARS.char_indices().for_each(|(i,c)| {cm.insert(i as u8, c); ()});
        cm
    };
}

use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::BufWriter;

pub fn encode(raw: &str, target_path: &str, mode: &str) {
    let u8_values = encode_to_u8s(raw);
    let pixel_data = pixelize(u8_values, mode, 8);
    write_png(pixel_data, target_path);
}

pub fn encode_from_file(filepath: &str, target_path: &str, mode: &str) {
    let contents = fs::read_to_string(filepath)
        .expect("Something went wrong reading the file");
    encode(&contents, target_path, mode);
}

pub fn decode(img_path: &str) -> String {
    let pparts = read_png(img_path);
    return decode_pixels(&pparts);
}

pub fn decode_pixels(pparts: &[u8]) -> String {
    let u8_vals = depixelize(pparts);
    return decode_from_u8s(u8_vals);
}

pub fn decode_to_file(in_path: &str, out_path: &str) {
    let decoded_data = decode(in_path);
    fs::write(out_path, decoded_data).expect("Unable to write file");
}

pub fn decode_file_data(in_data: &[u8]) -> String {
    let pparts = read_png_data(in_data);
    return decode_pixels(&pparts);
}

fn write_png(mut pixel_parts: Vec<u8>, target_path: &str) {
    let size:u32 = pixel_parts.len() as u32;
    let side:u32 = ((size as f64)/3.0).sqrt().ceil() as u32;
    let area:u32 = side * side;
    let blank_parts:u32 = (area*3) - size;
    let blank_lines:u32 = blank_parts / (3 * side);
    let vec_size: usize = (blank_parts % (side*3)) as usize;
    let mut blanks = vec![0; vec_size];

    pixel_parts.append(&mut blanks);
    let path = Path::new(target_path);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, side, side-blank_lines);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&pixel_parts).unwrap(); // save
}

fn read_png(path: &str) -> Vec<u8> {
    let decoder = png::Decoder::new(File::open(path).unwrap());
    //decode_png(decoder);
    let (info, mut reader) = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; info.buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    reader.next_frame(&mut buf).unwrap();
    // Inspect more details of the last read frame.
    //let in_animation = reader.info().frame_control.is_some();
    return buf;
}

fn read_png_data(data: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(data);
    //decode_png(decoder);
    let (info, mut reader) = decoder.read_info().unwrap();
    // Allocate the output buffer.
    let mut buf = vec![0; info.buffer_size()];
    // Read the next frame. An APNG might contain multiple frames.
    reader.next_frame(&mut buf).unwrap();
    // Inspect more details of the last read frame.
    //let in_animation = reader.info().frame_control.is_some();
    return buf;
}

// fn decode_png(decoder: png::Decoder) -> Vec<u8> {
//    let (info, mut reader) = decoder.read_info().unwrap();
//     // Allocate the output buffer.
//     let mut buf = vec![0; info.buffer_size()];
//     // Read the next frame. An APNG might contain multiple frames.
//     reader.next_frame(&mut buf).unwrap();
//     // Inspect more details of the last read frame.
//     //let in_animation = reader.info().frame_control.is_some();
//     return buf;
// }

fn write_meta_pixel(mode: &str, depth: u8) -> Vec<u8> {
    let mut pixel_data:Vec<u8> = Vec::new();
    match mode {
        "static" => pixel_data.push(1), // mode static (1)
        "hidden" => pixel_data.push(2), // mode static (2)
        _ => println!("Unknown Mode: {:?}", mode)
    }
    pixel_data.push(depth%8); // bit-depth 8
    pixel_data.push(1); // version 1
    return pixel_data;
}

fn pixelize(data: Vec<u8>, mode: &str, depth: u8) -> Vec<u8> {
    let mut pixel_data = write_meta_pixel(mode, depth);
    if mode == "static" {
        for i in (0..data.len()).step_by(4) {
            pixel_data.push((data[i] << 2) + ((data[i+1] & 48) >> 4));
            pixel_data.push(((data[i+1] & 15) << 4) + ((data[i+2] & 60) >> 2));
            pixel_data.push((data[i+2] << 6) + data[i+3]);
        }
    }
    return pixel_data;
}

fn depixelize(pixels: &[u8]) -> Vec<u8> {
    let mode:u8 = pixels[0] & 7;
    let depth:u8 = ((pixels[1] & 7)+1)%9;
    let version:u8 = pixels[2] & 7;
    println!("MODE: {:?}, DEPTH: {:?}, VERSION: {:?}", mode, depth, version );
    let mut u8_vals:Vec<u8> = Vec::new();
    if mode == 1 {
        for i in (3..pixels.len()).step_by(3) {
            u8_vals.push(pixels[i] >> 2);
            u8_vals.push(((pixels[i] & 3) << 4) + (pixels[i+1] >> 4));
            u8_vals.push(((pixels[i+1] & 15) << 2) + (pixels[i+2] >> 6));
            u8_vals.push(pixels[i+2] & 63);
        }
    }
    return u8_vals;
}

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
    let blank_pads = v.len() % 4;
    if blank_pads != 0 {
        for _ in 0..(4-blank_pads) {
            v.push(0);
        }
    }
    return v;
}

pub fn decode_from_u8s(mut data: Vec<u8>) -> String {
    // pull off padding count and subtract 1
    let mut pads:u8 = data.pop().unwrap_or(0);
    while pads == 0 {
        pads = data.pop().unwrap_or(0);
    }
    // restringify ???  needed?
    let mut encoded_str = String::from("");
    for n in &data {  
        encoded_str.push(REVCHARMAP[n]);
    };
    // add back the =s
    for _ in 0..(pads-1) {
        encoded_str.push('=');
    }
    let decoded_str_u8s =  base64::decode(encoded_str).unwrap();
    let decoded_str = String::from_utf8(decoded_str_u8s.clone()).unwrap();
    return decoded_str;
}

use std::io::Read;

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
        encode(raw_data, "./examples/days.png", "static");
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
        encode_from_file("./examples/colors.json", "./examples/colors.png", "static");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn it_decodes_to_file() {
        decode_to_file("./examples/colors1.png", "./examples/colors1.json");
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn it_encodes_big_file() {
        encode_from_file("./examples/big_file.json", "./examples/big_file.png", "static");
        assert_eq!(2+2, 4);
    }

    #[test]
    fn it_decodes_from_png_data() {
        let raw_text_data = "{\"days\": [\"Su\", \"Mo\", \"Tu\", \"We\", \"Th\", \"Fr\", \"Sa\"]}";
        let raw_data = get_file_as_byte_vec("./examples/days1.png");
        let found_data = decode_file_data(&raw_data);
        assert_eq!(raw_text_data, found_data);
    }
}
