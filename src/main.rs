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
    #[structopt(long = "merge")]
    merge: Option<Option<bool>>,
}

#[derive(Debug)]
enum ImageType {
    Png,
    Jpg,
    Gif,
    Unknown,
}

#[derive(Debug)]
struct ImageData<'a> {
    image_type: ImageType,
    data: &'a [u8],
}

fn find_images(data: &[u8]) -> Vec<ImageData> {
    let mut images = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if i + 8 <= data.len() && &data[i..i + 8] == b"\x89PNG\r\n\x1a\n" {
            if let Some((end, _)) = data[i..]
                .windows(8)
                .enumerate()
                .find(|(_, window)| *window == b"IEND\xaeB`\x82")
            {
                let end = i + end + 8;
                if end <= data.len() {
                    images.push(ImageData {
                        image_type: ImageType::Png,
                        data: &data[i..end],
                    });
                    i = end;
                    continue;
                }
            }
        } else if i + 2 <= data.len() && &data[i..i + 2] == b"\xFF\xD8" {
            if let Some((end, _)) = data[i..]
                .windows(2)
                .enumerate()
                .find(|(_, window)| *window == b"\xFF\xD9")
            {
                let end = i + end + 2;
                if end <= data.len() {
                    images.push(ImageData {
                        image_type: ImageType::Jpg,
                        data: &data[i..end],
                    });
                    i = end;
                    continue;
                }
            }
        } else if i + 6 <= data.len()
            && (&data[i..i + 6] == b"GIF89a" || &data[i..i + 6] == b"GIF87a")
        {
            if let Some((end, _)) = data[i..].windows(1).enumerate().find(|(idx, _)| {
                let idx = i + idx;
                data[idx..].len() >= 2 && &data[idx..idx + 2] == b"\x00\x3B"
            }) {
                let end = i + end + 2;
                if end <= data.len() {
                    images.push(ImageData {
                        image_type: ImageType::Gif,
                        data: &data[i..end],
                    });
                    i = end;
                    continue;
                }
            }
        } else {
            // 找到不属于 PNG、JPG 或 GIF 的二进制数据段
            let start = i;
            // 找到下一段已知格式的开始或文件结尾
            let end = (i..data.len())
                .find(|&j| {
                    j + 8 <= data.len()
                        && (&data[j..j + 8] == b"\x89PNG\r\n\x1a\n"
                            || j + 2 <= data.len() && &data[j..j + 2] == b"\xFF\xD8"
                            || j + 6 <= data.len()
                                && (&data[j..j + 6] == b"GIF89a" || &data[j..j + 6] == b"GIF87a"))
                })
                .unwrap_or(data.len());
            images.push(ImageData {
                image_type: ImageType::Unknown,
                data: &data[start..end],
            });
            i = end;
        }
    }
    images
}

fn save_images(images: &Vec<ImageData>, output_path: &PathBuf) {
    info!("images: {} 个", images.len());
    let num_digits = std::cmp::max(images.len().to_string().len(), 3);

    for (i, img_data) in images.iter().enumerate() {
        let t = &img_data.image_type;
        let ext = match t {
            ImageType::Png => "png",
            ImageType::Jpg => "jpg",
            ImageType::Gif => "gif",
            ImageType::Unknown => "bin",
        };
        let img_name = format!("image_{:0width$}.{}", i + 1, ext, width = num_digits);
        let output_image_path = output_path.join(&img_name);

        let file = fs::File::create(&output_image_path);
        match file {
            Ok(mut f) => {
                match f.write_all(img_data.data) {
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
    }
}

fn get_bool_opt(arg: Option<Option<bool>>, default_val: bool) -> bool {
    if let Some(opt) = arg {
        if let Some(ret) = opt {
            ret
        } else {
            true
        }
    } else {
        default_val
    }
}

fn separating_image(input_path: &PathBuf, output_path: &PathBuf) -> usize {
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

    let images = find_images(&buffer);
    count += images.len();

    save_images(&images, output_path);

    count
}

fn merge_images(input_path: &PathBuf, output_path: &PathBuf) -> std::io::Result<()> {
    // 读取目录中的所有文件
    let mut entries: Vec<_> = fs::read_dir(input_path)?
        .filter_map(|entry| entry.ok())
        .collect();

    // 按文件名排序
    entries.sort_by_key(|entry| entry.path());

    // 打开输出文件
    let mut output_file = fs::File::create(output_path)
        .unwrap_or_else(|e| panic!("创建文件 {} 失败, {:?}", output_path.display(), e));

    // 依次读取每个文件并写入到输出文件中
    for entry in entries {
        let path = entry.path();
        if path.is_file() {
            info!("正在处理文件: {:?}", path);
            let mut file = fs::File::open(&path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .unwrap_or_else(|e| panic!("读取文件 {} 失败, {:?}", path.display(), e));
            output_file
                .write_all(&buffer)
                .unwrap_or_else(|e| panic!("写入文件 {} 失败, {:?}", path.display(), e));
        }
    }

    info!("合并完成，输出文件路径: {:?}", output_path);
    Ok(())
}

fn _main() {
    utils::init_logger();
    let opt: Opt = if cfg!(debug_assertions) {
        Opt {
            // cmd: "./test/image.bin".to_string(),
            cmd: "./target/debug/output".to_string(),
            output: None,
            merge: Some(Some(true)),
        }
    } else {
        Opt::from_args()
    };
    let current_path = &*config::CURRENT_PATH;
    let input_path = PathBuf::from(&opt.cmd);

    let merge = get_bool_opt(opt.merge, false);
    if merge {
        let output_path = PathBuf::from(opt.output.unwrap_or(String::from(
            current_path.join("output.bin").to_str().unwrap(),
        )));
        merge_images(&input_path, &output_path).unwrap_or_else(|e| panic!("合并失败: {:?}", e));
        info!("合并完成! {}", output_path.display());
    } else {
        let output_path = PathBuf::from(
            opt.output
                .unwrap_or(String::from(current_path.join("output").to_str().unwrap())),
        );
        let count = separating_image(&input_path, &output_path);
        info!("输出文件夹: {}", output_path.display());
        info!("全部完成! 总数: {}", count);
    }
}

fn main() {
    if let Err(err) = std::panic::catch_unwind(_main) {
        error!("{:?}", err);
    }
    #[cfg(not(debug_assertions))]
    utils::pause();
}
