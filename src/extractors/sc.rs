use crate::{error::Error, utils::Reader};
use colored::Colorize;
use conv::ValueInto;
use image::{imageops, GenericImage, GenericImageView, GrayImage, ImageBuffer, Pixel, RgbaImage};
use imageproc::{
    definitions::{Clamp, Image},
    drawing::draw_polygon_mut,
    geometric_transformations::{warp_into, Interpolation, Projection},
    point::Point as Point2D,
};
use rayon::prelude::*;
use std::{
    cmp::Ordering,
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};

/// Struct to represent a sheet item.
#[derive(Debug)]
struct SheetItem {
    x: u32,
    y: u32,
    divider: u32,
}

impl SheetItem {
    fn new(x: u32, y: u32, divider: u32) -> Self {
        Self { x, y, divider }
    }
}

/// Struct to represent a 2-dimensional point.
#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Struct to represent a region on a sheet.
#[derive(Debug, Default)]
struct Region {
    sheet_id: u32,
    num_points: u32,
    rotation: u32,
    mirroring: u32,
    shape_points: Vec<Point>,
    sheet_points: Vec<Point>,
    sprite_width: u32,
    sprite_height: u32,
    region_zero_x: u32,
    region_zero_y: u32,
    top: i32,
    left: i32,
    bottom: i32,
    right: i32,
}

/// Struct to represent a sprite item.
#[derive(Debug)]
struct SpriteItem {
    id: u32,
    total_regions: u32,
    regions: Vec<Region>,
}

impl SpriteItem {
    fn new(id: u32, total_regions: u32, regions: Vec<Region>) -> Self {
        Self {
            id,
            total_regions,
            regions,
        }
    }
}

/// Struct to hold data about a sprite.
#[derive(Debug)]
struct SpriteGlobal {
    sprite_width: u32,
    sprite_height: u32,
    global_zero_x: u32,
    global_zero_y: u32,
}

impl SpriteGlobal {
    fn new(sprite_width: u32, sprite_height: u32, global_zero_x: u32, global_zero_y: u32) -> Self {
        Self {
            sprite_width,
            sprite_height,
            global_zero_x,
            global_zero_y,
        }
    }
}

/// Enum to represent different types of region rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Rotation {
    Same,
    Less,
    More,
}

impl Rotation {
    fn is_same(&self) -> bool {
        matches!(*self, Self::Same)
    }
}

/// Processes extracted `.sc` file data.
///
/// This function does NOT process files with `.sc` extension. It process
/// the file generated after using QuickBMS on the `.sc` file. The png images
/// extracted from `_tex.sc` file corresponding to this `.sc` file must be
/// present in `png_dir`.
///
/// A single `.sc` file contains data for multiple sprites. All of the
/// sprites are extracted and saved by this process in the `out_dir`.
///
/// `parallelize` tells if the directory files are processed parallelly. It is
/// simply used to control the stdout output. Within this function, sprites are
/// always processed parallelly to increase efficiency.
///
/// ## Errors
///
/// If the png images are not present, [`Error::Other`] is returned.
///
/// If decompression is unsuccessful, [`Error::DecompressionError`] is returned.
///
/// If `out_dir` does not exist or if reading images from `png_dir` is not
/// possible, [`Error::IoError`] is returned.
///
/// [`Error::DecompressionError`]: ./error/enum.Error.html#variant.DecompressionError
/// [`Error::Other`]: ./error/enum.Error.html#variant.Other
/// [`Error::IoError`]: ./error/enum.Error.html#variant.IoError
pub fn process_sc(
    data: &[u8],
    file_name: &str,
    out_dir: &Path,
    png_dir: &Path,
    parallelize: bool,
) -> Result<(), Error> {
    if !parallelize {
        println!("\nProcessing `{}` image(s)...", file_name.green().bold());
    }

    let mut stream = Reader::new(Cursor::new(data));
    let mut offset_shape = 0;
    let mut offset_sheet = 0;

    let shape_count = stream.read_uint16();
    let _total_animations = stream.read_uint16();
    let total_textures = stream.read_uint16();
    let _text_field_count = stream.read_uint16();
    let mut use_low_res = false;
    let _matrix_count = stream.read_uint16();
    let _color_transformation_count = stream.read_uint16();

    let mut sheet_data = Vec::new();
    for _ in 0..total_textures {
        sheet_data.push(SheetItem::new(0, 0, 1));
    }

    let mut sprite_data = Vec::new();
    for _ in 0..shape_count {
        sprite_data.push(SpriteItem::new(0, 0, Vec::new()));
    }

    let sheet_image = Arc::new(Mutex::new(Vec::new()));

    for x in 0..total_textures as usize {
        let png_path = png_dir.join(format!("{}_tex{}.png", file_name, "_".repeat(x)));
        if png_path.exists() {
            let opened_image = match image::open(&png_path) {
                Ok(i) => i,
                Err(_) => {
                    return Err(Error::IoError(format!(
                        "{} {}",
                        "Unable to open image".red(),
                        png_path.to_str().unwrap().red()
                    )))
                }
            };

            sheet_image.lock().unwrap().push(opened_image);
        } else {
            return Err(Error::from(
                format!(
                    "Expected extracted png image `{}` for file",
                    png_path.to_str().unwrap(),
                )
                .red()
                .to_string(),
            ));
        }
    }

    // Read 500 bytes
    stream.read(5);

    let export_count = stream.read_uint16();

    for _ in 0..export_count {
        stream.read_uint16();
    }

    for _ in 0..export_count {
        let length = stream.read_byte() as usize;
        stream.read_string(length);
    }

    while stream.len() > 0 {
        let data_block_tag = hex::encode(stream.read(1));
        let data_block_size = stream.read_uint32();

        if data_block_tag == "01" || data_block_tag == "18" {
            let _pixel_type = stream.read_byte();

            sheet_data[offset_sheet].x = stream.read_uint16().into();
            sheet_data[offset_sheet].y = stream.read_uint16().into();

            let image = &sheet_image.lock().unwrap()[offset_sheet];

            if image.width() != sheet_data[offset_sheet].x
                && image.height() != sheet_data[offset_sheet].y
            {
                use_low_res = true;
            }
            offset_sheet += 1;
        } else if data_block_tag == "1e" || data_block_tag == "1a" {
            continue;
        } else if data_block_tag == "12" {
            // A polygon.
            let i = if use_low_res { 2 } else { 1 };

            sprite_data[offset_shape].id = stream.read_uint16().into();
            sprite_data[offset_shape].total_regions = stream.read_uint16().into();
            stream.read_uint16();

            let mut regions = Vec::new();
            for _ in 0..sprite_data[offset_shape].total_regions {
                regions.push(Region {
                    top: -32767,
                    left: 32767,
                    bottom: 32767,
                    right: -32767,
                    ..Default::default()
                });
            }
            sprite_data[offset_shape].regions = regions;

            for y in 0..sprite_data[offset_shape].total_regions as usize {
                let data_block_tag_16 = hex::encode(stream.read(1));

                if data_block_tag_16 == "16" {
                    let _data_block_size_16 = stream.read_uint32();
                    sprite_data[offset_shape].regions[y].sheet_id = stream.read_byte().into();
                    sprite_data[offset_shape].regions[y].num_points = stream.read_byte().into();

                    let mut shape_points = Vec::new();
                    let mut sheet_points = Vec::new();
                    for _ in 0..sprite_data[offset_shape].regions[y].num_points {
                        shape_points.push(Point::new(0, 0));
                        sheet_points.push(Point::new(0, 0));
                    }

                    sprite_data[offset_shape].regions[y].shape_points = shape_points;
                    sprite_data[offset_shape].regions[y].sheet_points = sheet_points;

                    for z in 0..sprite_data[offset_shape].regions[y].num_points as usize {
                        sprite_data[offset_shape].regions[y].shape_points[z].x =
                            stream.read_int32();
                        sprite_data[offset_shape].regions[y].shape_points[z].y =
                            stream.read_int32();
                    }
                    for z in 0..sprite_data[offset_shape].regions[y].num_points as usize {
                        sprite_data[offset_shape].regions[y].sheet_points[z].x =
                            ((stream.read_uint16() as f32
                                * sheet_data[sprite_data[offset_shape].regions[y].sheet_id as usize]
                                    .x as f32
                                / 65535.0)
                                .round()
                                / (i as f32)) as i32;

                        sprite_data[offset_shape].regions[y].sheet_points[z].y =
                            ((stream.read_uint16() as f32
                                * sheet_data[sprite_data[offset_shape].regions[y].sheet_id as usize]
                                    .y as f32
                                / 65535.0)
                                .round()
                                / (i as f32)) as i32;
                    }
                }
            }

            stream.read(5);
            offset_shape += 1;
            continue;
        } else if data_block_tag == "08" {
            // A matrix.
            let mut points = Vec::new();
            for _ in 0..6 {
                points.push(stream.read_int32());
            }
            continue;
        } else if data_block_tag == "0c" {
            // An animation.
            let _clip_id = stream.read_uint16();
            let _clip_fps = stream.read_byte();
            let _clip_frame_count = stream.read_uint16();

            let cnt_1 = stream.read_int32();

            for _ in 0..cnt_1 {
                stream.read_uint16();
                stream.read_uint16();
                stream.read_uint16();
            }

            let cnt_2 = stream.read_int16();
            for _ in 0..cnt_2 {
                stream.read_int16();
            }

            for _ in 0..cnt_2 {
                stream.read_byte();
            }

            for _ in 0..cnt_2 {
                let string_length = stream.read_byte() as usize;
                if string_length < 255 {
                    stream.read_string(string_length);
                }
            }
        } else {
            stream.read(data_block_size as usize);
        }
    }

    write_shape(
        &mut sprite_data,
        &mut sheet_data,
        shape_count,
        sheet_image,
        &file_name,
        &out_dir,
    )
}

/// Writes shapes from the data on images.
fn write_shape(
    sprite_data: &mut Vec<SpriteItem>,
    sheet_data: &mut Vec<SheetItem>,
    shape_count: u16,
    sheet_image: Arc<Mutex<Vec<image::DynamicImage>>>,
    file_name: &str,
    out_dir: &Path,
) -> Result<(), Error> {
    let mut max_left = 0;
    let mut max_right = 0;
    let mut max_above = 0;
    let mut max_below = 0;

    let mut sprite_global = SpriteGlobal::new(0, 0, 0, 0);

    for sprite_item in sprite_data.iter_mut().take(shape_count as usize) {
        for y in 0..sprite_item.total_regions as usize {
            let mut region_min_x = 32676;
            let mut region_max_x = -32676;
            let mut region_min_y = 32676;
            let mut region_max_y = -32676;

            let mut temp_x;
            let mut temp_y;

            for z in 0..sprite_item.regions[y].num_points as usize {
                temp_x = sprite_item.regions[y].shape_points[z].x;
                temp_y = sprite_item.regions[y].shape_points[z].y;

                sprite_item.regions[y].top = if temp_y > sprite_item.regions[y].top {
                    temp_y
                } else {
                    sprite_item.regions[y].top
                };
                sprite_item.regions[y].left = if temp_x < sprite_item.regions[y].left {
                    temp_x
                } else {
                    sprite_item.regions[y].left
                };
                sprite_item.regions[y].bottom = if temp_y < sprite_item.regions[y].bottom {
                    temp_y
                } else {
                    sprite_item.regions[y].bottom
                };
                sprite_item.regions[y].right = if temp_x > sprite_item.regions[y].right {
                    temp_x
                } else {
                    sprite_item.regions[y].right
                };

                temp_x = sprite_item.regions[y].sheet_points[z].x;
                temp_y = sprite_item.regions[y].sheet_points[z].y;

                region_min_x = if temp_x < region_min_x {
                    temp_x
                } else {
                    region_min_x
                };
                region_max_x = if temp_x > region_max_x {
                    temp_x
                } else {
                    region_max_x
                };
                region_min_y = if temp_y < region_min_y {
                    temp_y
                } else {
                    region_min_y
                };
                region_max_y = if temp_y > region_max_y {
                    temp_y
                } else {
                    region_max_y
                };
            }

            region_rotation(&mut sprite_item.regions[y]);

            if sprite_item.regions[y].rotation == 90 || sprite_item.regions[y].rotation == 270 {
                sprite_item.regions[y].sprite_width = (region_max_y - region_min_y) as u32;
                sprite_item.regions[y].sprite_height = (region_max_x - region_min_x) as u32;
            } else {
                sprite_item.regions[y].sprite_width = (region_max_x - region_min_x) as u32;
                sprite_item.regions[y].sprite_height = (region_max_y - region_min_y) as u32;
            }

            temp_x = sprite_item.regions[y].sprite_width as i32;
            temp_y = sprite_item.regions[y].sprite_height as i32;

            // Determine origin pixel (0. 0)
            sprite_item.regions[y].region_zero_x = (sprite_item.regions[y].left as f64
                * (temp_x as f64)
                / (sprite_item.regions[y].right - sprite_item.regions[y].left) as f64)
                .abs()
                .round() as u32;

            sprite_item.regions[y].region_zero_y = (sprite_item.regions[y].bottom as f64
                * (temp_y as f64)
                / (sprite_item.regions[y].top - sprite_item.regions[y].bottom) as f64)
                .abs()
                .round() as u32;

            // Sprite image dimensions.
            // Max sprite size is determined from the zero points.
            // The higher the 0, more pixels to the left/top are required.
            // The higher the difference between the 0 and the image width/height,
            // more pixels to the right/bottom are required.
            max_left = if sprite_item.regions[y].region_zero_x > max_left {
                sprite_item.regions[y].region_zero_x
            } else {
                max_left
            };
            max_above = if sprite_item.regions[y].region_zero_y > max_above {
                sprite_item.regions[y].region_zero_y
            } else {
                max_above
            };

            temp_x = (sprite_item.regions[y].sprite_width as i32
                - sprite_item.regions[y].region_zero_x as i32) as i32;
            temp_y = (sprite_item.regions[y].sprite_height as i32
                - sprite_item.regions[y].region_zero_y as i32) as i32;

            max_right = if temp_x > max_right {
                temp_x
            } else {
                max_right
            };
            max_below = if temp_y > max_below {
                temp_y
            } else {
                max_below
            };
        }
    }

    // File sprite size takes into account the mask's line, so we add 2 to each dimension.
    sprite_global.sprite_width = max_left + max_right as u32 + 2;
    sprite_global.sprite_height = max_above + max_below as u32 + 2;
    sprite_global.global_zero_x = max_left;
    sprite_global.global_zero_y = max_above;

    // Number of digits in the number.
    let max_range = (shape_count as f64).log10().round() as usize + 1;

    (0..shape_count as usize).into_par_iter().try_for_each(|x| {
        let out_image = Arc::new(Mutex::new(RgbaImage::new(
            sprite_global.sprite_width,
            sprite_global.sprite_height,
        )));

        (0..sprite_data[x].total_regions as usize)
            .into_par_iter()
            .for_each(|y| {
                let mut polygon = Vec::new();
                for z in 0..sprite_data[x].regions[y].num_points as usize {
                    polygon.push(Point2D::new(
                        sprite_data[x].regions[y].sheet_points[z].x,
                        sprite_data[x].regions[y].sheet_points[z].y,
                    ));
                }

                if polygon[0] == polygon[polygon.len() - 1] {
                    return;
                }

                let sheet_id = sprite_data[x].regions[y].sheet_id as usize;

                let mut im_mask = GrayImage::new(sheet_data[sheet_id].x, sheet_data[sheet_id].y);
                draw_polygon_mut(&mut im_mask, polygon.as_slice(), image::Luma([255]));

                let bounds = get_bbox(&im_mask);

                let (temp_x, temp_y) = (bounds.2 - bounds.0, bounds.3 - bounds.1);
                im_mask =
                    imageops::crop(&mut im_mask, bounds.0, bounds.1, temp_x, temp_y).to_image();

                let mut temp_region = RgbaImage::new(temp_x, temp_y);
                let copy_img =
                    sheet_image.lock().unwrap()[sheet_id].crop(bounds.0, bounds.1, temp_x, temp_y);

                // Overlay image content (`copy_img`) on `temp_region`, with `im_mask` as the mask.
                masked_overlay(&mut temp_region, &copy_img, 0, 0, &im_mask);

                // Mirror image if required.
                if sprite_data[x].regions[y].mirroring == 1 {
                    imageops::flip_horizontal_in_place(&mut temp_region);
                }

                // Rotate image as appropriate.
                // Rotation is skipped if the angle is `0` or `360` to avoid
                // unnecessary processing.
                let angle = sprite_data[x].regions[y].rotation;
                let rotated_image = if angle != 0 && angle != 360 {
                    rotate_uncropped(
                        &temp_region,
                        (angle as f32).to_radians(),
                        Interpolation::Nearest,
                        image::Rgba([0, 0, 0, 0]),
                    )
                } else {
                    temp_region
                };

                let paste_left =
                    sprite_global.global_zero_x - sprite_data[x].regions[y].region_zero_x;
                let paste_top =
                    sprite_global.global_zero_y - sprite_data[x].regions[y].region_zero_y;

                if out_image
                    .lock()
                    .unwrap()
                    .copy_from(&rotated_image, paste_left, paste_top)
                    .is_err()
                {
                    println!(
                        "{}",
                        "There was an error processing a portion of the image.".red()
                    );
                }
            });

        let save_path = out_dir.join(format!("{}_sprite_{:0>2$}.png", file_name, x, max_range));

        if out_image.lock().unwrap().save(save_path).is_err() {
            return Err(Error::IoError(format!("{}", "Unable to save image.".red())));
        }

        Ok(())
    })
}

/// Returns bounding box of the image.
///
/// The bounding box discards transparent pixels.
fn get_bbox<I>(image: &I) -> (u32, u32, u32, u32)
where
    I: GenericImageView<Pixel = image::Luma<u8>>,
{
    let mut bounds = (-1, -1, -1, -1);
    let mut additions = (1, 1, 1, 1);
    for pixel in image.pixels() {
        let pix_x = pixel.0 as i32;
        let pix_y = pixel.1 as i32;
        let color = (pixel.2).0;

        // Coloured pixel.
        if color[0] != 0 {
            if bounds.0 > pix_x || bounds.0 < 0 {
                if bounds.0 < pix_x && bounds.0 < 0 {
                    additions.0 = 0;
                }
                bounds.0 = pix_x;
            }
            if bounds.2 < pix_x || bounds.2 < 0 {
                if bounds.2 > pix_x && bounds.2 < 0 {
                    additions.2 = 0;
                }
                bounds.2 = pix_x;
            }
            if bounds.1 > pix_y || bounds.1 < 0 {
                if bounds.1 < pix_y && bounds.1 < 0 {
                    additions.1 = 0;
                }
                bounds.1 = pix_y;
            }
            if bounds.3 < pix_y || bounds.3 < 0 {
                if bounds.3 > pix_y && bounds.3 < 0 {
                    additions.3 = 0;
                }
                bounds.3 = pix_y;
            }
        }
    }

    (
        bounds.0 as u32 + additions.0,
        bounds.1 as u32 + additions.1,
        bounds.2 as u32 + additions.2,
        bounds.3 as u32 + additions.3,
    )
}

fn region_rotation(region: &mut Region) {
    let mut sum_sheet = 0;
    let mut sum_shape = 0;

    for z in 0..region.num_points as usize {
        sum_sheet += (region.sheet_points[(z + 1) % (region.num_points as usize)].x
            - region.sheet_points[z].x) as i64
            * (region.sheet_points[(z + 1) % (region.num_points as usize)].y
                + region.sheet_points[z].y) as i64;

        sum_shape += (region.shape_points[(z + 1) % (region.num_points as usize)].x
            - region.shape_points[z].x) as i64
            * (region.shape_points[(z + 1) % (region.num_points as usize)].y
                + region.shape_points[z].y) as i64;
    }

    let sheet_orientation = if sum_sheet < 0 { -1 } else { 1 };
    let shape_orientation = if sum_shape < 0 { -1 } else { 1 };

    region.mirroring = if shape_orientation == sheet_orientation {
        0
    } else {
        1
    };

    if region.mirroring == 1 {
        for x in 0..region.num_points as usize {
            region.shape_points[x].x *= -1;
        }
    }

    // Define region rotation.
    // px, qx mean "where in x is point 1, according to point 0"
    // py, qy mean "where in y is point 1, according to point 0"
    // Possible values are "m"ore, "l"ess and "s"ame.
    let px = match region.sheet_points[1].x.cmp(&region.sheet_points[0].x) {
        Ordering::Greater => Rotation::More,
        Ordering::Less => Rotation::Less,
        Ordering::Equal => Rotation::Same,
    };

    // More -> Less here.
    let py = match region.sheet_points[1].y.cmp(&region.sheet_points[0].y) {
        Ordering::Greater => Rotation::Less,
        Ordering::Less => Rotation::More,
        Ordering::Equal => Rotation::Same,
    };

    let qx = match region.shape_points[1].x.cmp(&region.shape_points[0].x) {
        Ordering::Greater => Rotation::More,
        Ordering::Less => Rotation::Less,
        Ordering::Equal => Rotation::Same,
    };

    let qy = match region.shape_points[1].y.cmp(&region.shape_points[0].y) {
        Ordering::Greater => Rotation::More,
        Ordering::Less => Rotation::Less,
        Ordering::Equal => Rotation::Same,
    };

    // Now, define rotation.
    // Short of listing all 32 outcomes (like mm-mm, mm-ml, mm-ms, etc), this
    // monstrous if blocks seems a better way to do this.
    let mut rotation = if px == qx && py == qy {
        0
    } else if px.is_same() {
        if px == qy {
            if py == qx {
                90
            } else {
                270
            }
        } else {
            180
        }
    } else if py.is_same() {
        if py == qx {
            if px == qy {
                270
            } else {
                90
            }
        } else {
            180
        }
    } else if px != qx && py != qy {
        180
    } else if px == py {
        if px != qx {
            270
        } else if py != qy {
            90
        } else {
            0
        }
    } else if px != py {
        if px != qx {
            90
        } else if py != qy {
            270
        } else {
            0
        }
    } else {
        0
    };

    if sheet_orientation == -1 && (rotation == 90 || rotation == 270) {
        rotation = (rotation + 180) % 360
    }

    region.rotation = rotation;
}

/// Returns a new image, which contains the whole original image rotated by `theta` radians
/// and centered inside the returned image. The pixels outside the original image will be `default`.
/// The dimensions of the returned image can differ from the original image.
///
/// Source: https://github.com/image-rs/imageproc/blob/9eeacac54719c89c1a7f43763786381ce6a8f557/src/geometric_transformations.rs#L323
fn rotate_uncropped<P>(
    image: &Image<P>,
    theta: f32,
    interpolation: Interpolation,
    default: P,
) -> Image<P>
where
    P: Pixel + Send + Sync + 'static,
    <P as Pixel>::Subpixel: Send + Sync,
    <P as Pixel>::Subpixel: ValueInto<f32> + Clamp<f32>,
{
    let (width, height) = (image.width() as f32, image.height() as f32);
    let (mut new_width, new_height) = (
        (width * theta.cos().abs() + height * theta.sin().abs()) as u32,
        (height * theta.cos().abs() + width * theta.sin().abs()) as u32,
    );

    let (cx, cy) = (width / 2f32, height / 2f32);
    let (new_cx, new_cy) = ((new_width / 2) as f32, (new_height / 2) as f32);

    if new_width == 0 {
        new_width = new_height;
    }
    let mut new_image = ImageBuffer::from_pixel(new_width, new_height, default);
    let projection = Projection::translate(new_cx, new_cy)
        * Projection::rotate(-theta)
        * Projection::translate(-cx, -cy);

    warp_into(image, &projection, interpolation, default, &mut new_image);

    new_image
}

/// Overlay an image at a given coordinate (x, y) if the point is not transparent on the mask.
/// The mask must have the same dimensions as `bottom`.
fn masked_overlay<I, J, K>(bottom: &mut I, top: &J, x: u32, y: u32, mask: &K)
where
    I: GenericImage,
    J: GenericImageView<Pixel = I::Pixel>,
    K: GenericImageView<Pixel = image::Luma<u8>>,
{
    let bottom_dims = bottom.dimensions();
    let top_dims = top.dimensions();

    // Crop our top image if we're going out of bounds
    let (range_width, range_height) = imageops::overlay_bounds(bottom_dims, top_dims, x, y);

    for top_y in 0..range_height {
        for top_x in 0..range_width {
            if mask.get_pixel(x + top_x, y + top_y)[0] == 0 {
                continue;
            }

            bottom
                .get_pixel_mut(x + top_x, y + top_y)
                .blend(&top.get_pixel(top_x, top_y));
        }
    }
}
