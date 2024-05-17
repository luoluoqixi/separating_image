mod config;
mod utils;

use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use log::{error, info};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "separating_image")]
struct Opt {
    #[structopt()]
    cmd: String,
    #[structopt(long = "output")]
    output: Option<String>,
    #[structopt(long = "keep_raw_bin")]
    keep_raw_binary: Option<Option<bool>>,
}

fn find_png_images(data: &[u8]) -> Vec<&[u8]> {
    let mut images = Vec::new();
    let mut i = 0;
    while i + 8 <= data.len() {
        // PNG 文件头标志
        if &data[i..i + 8] == b"\x89PNG\r\n\x1a\n" {
            if let Some((end, _)) = data[i..]
                .windows(8)
                .enumerate()
                .find(|(_, window)| *window == b"IEND\xaeB`\x82")
            {
                let end = i + end + 8;
                if end <= data.len() {
                    images.push(&data[i..end]);
                    i = end;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
    images
}

fn find_jpg_images(data: &[u8]) -> Vec<&[u8]> {
    let mut images = Vec::new();
    let mut i = 0;
    while i + 2 <= data.len() {
        if &data[i..i + 2] == b"\xFF\xD8" {
            if let Some((end, _)) = data[i..]
                .windows(2)
                .enumerate()
                .find(|(_, window)| *window == b"\xFF\xD9")
            {
                let end = i + end + 2;
                if end <= data.len() {
                    images.push(&data[i..end]);
                    i = end;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
    images
}

fn find_gif_images(data: &[u8]) -> Vec<&[u8]> {
    let mut images = Vec::new();
    let mut i = 0;
    while i + 6 <= data.len() {
        if &data[i..i + 6] == b"GIF89a" || &data[i..i + 6] == b"GIF87a" {
            if let Some((end, _)) = data[i..].windows(1).enumerate().find(|(idx, _)| {
                let idx = i + idx;
                data[idx..].len() >= 2 && &data[idx..idx + 2] == b"\x00\x3B"
            }) {
                let end = i + end + 2;
                if end <= data.len() {
                    images.push(&data[i..end]);
                    i = end;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
    images
}

fn save_images(prefix: &str, images: &Vec<&[u8]>, output_path: &PathBuf, keep_raw_binary: bool) {
    info!("{}: {} 个", prefix, images.len());
    for (i, img_data) in images.iter().enumerate() {
        let img_name = format!("{}_image_{}.{}", prefix, i, prefix);
        let output_image_path = output_path.join(&img_name);
        if keep_raw_binary {
            let file = fs::File::create(&output_image_path);
            match file {
                Ok(mut f) => {
                    match f.write_all(img_data) {
                        Ok(_) => {
                            info!("{}", format!("{}", &img_name));
                        }
                        Err(e) => {
                            error!("{}", format!("写入图片 {} 失败: {:?}", &img_name, e));
                        }
                    };
                }
                Err(e) => {
                    error!("{}", format!("创建图片 {} 失败: {:?}", &img_name, e));
                }
            }
        } else {
            match image::load_from_memory(img_data) {
                Ok(img) => {
                    match img.save(&output_image_path) {
                        Ok(_) => {
                            info!("{}", format!("{}", &img_name));
                        }
                        Err(e) => {
                            error!("{}", format!("保存图片 {} 失败: {:?}", &img_name, e));
                        }
                    }
                    img.save(&output_image_path).expect("保存图片失败");
                }
                Err(e) => {
                    error!("{}", format!("加载图片 {} 失败: {:?}", &img_name, e));
                }
            }
        }
    }
}

fn get_bool_opt(arg: Option<Option<bool>>) -> bool {
    if let Some(opt) = arg {
        if let Some(ret) = opt {
            ret
        } else {
            true
        }
    } else {
        false
    }
}

fn separating_image(input_path: &PathBuf, output_path: &PathBuf, keep_raw_binary: bool) -> usize {
    if !output_path.exists() {
        fs::create_dir_all(output_path)
            .expect(format!("创建输出文件夹失败: {}", output_path.display()).as_str());
    }
    let mut file = fs::File::open(input_path)
        .expect(format!("打开文件失败: {}", input_path.display()).as_str());
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .expect(format!("读取文件失败: {}", input_path.display()).as_str());

    let mut count: usize = 0;

    let images = find_png_images(&buffer);
    count += images.len();
    save_images("png", &images, output_path, keep_raw_binary);

    let images = find_jpg_images(&buffer);
    count += images.len();
    save_images("jpg", &images, output_path, keep_raw_binary);

    let images = find_gif_images(&buffer);
    count += images.len();
    save_images("gif", &images, output_path, keep_raw_binary);

    count
}

fn _main() {
    utils::init_logger();
    let opt: Opt = if cfg!(debug_assertions) {
        Opt {
            cmd: "./test/image.bin".to_string(),
            output: None,
            keep_raw_binary: Some(Some(true)),
        }
    } else {
        Opt::from_args()
    };
    let current_path = &*config::CURRENT_PATH;
    let input_path = PathBuf::from(&opt.cmd);
    let output_path = PathBuf::from(
        opt.output
            .unwrap_or(String::from(current_path.join("output").to_str().unwrap())),
    );
    let keep_raw_binary = get_bool_opt(opt.keep_raw_binary);
    let count = separating_image(&input_path, &output_path, keep_raw_binary);
    info!("输出文件夹: {}", output_path.display());
    info!("全部完成! 总数: {}", count);
}

fn main() {
    if let Err(err) = std::panic::catch_unwind(_main) {
        error!("{:?}", err);
    }
    #[cfg(not(debug_assertions))]
    utils::pause();
}
