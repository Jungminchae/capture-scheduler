use crate::config::Config;
use chrono::{Local, Timelike};
use image::GenericImageView;
use image::codecs::jpeg::JpegEncoder;
use std::fs::File;
use xcap::Monitor;

pub fn check_and_capture(config: &Config) {
    let now = Local::now();
    let current_time_key = (now.hour(), now.minute(), now.second());

    for schedule in &config.schedule_times {
        if (schedule.hour, schedule.minute, schedule.second) == current_time_key {
            perform_capture(config);
            std::thread::sleep(std::time::Duration::from_millis(1001));
            break;
        }
    }
}

fn perform_capture(config: &Config) {
    if let Some(rect) = config.capture_area {
        let physical_x = (rect.x as f32 * rect.scale_factor) as u32;
        let physical_y = (rect.y as f32 * rect.scale_factor) as u32;
        let physical_width = (rect.width as f32 * rect.scale_factor) as u32;
        let physical_height = (rect.height as f32 * rect.scale_factor) as u32;

        if let Ok(monitors) = Monitor::all() {
            if let Some(primary_monitor) = monitors
                .into_iter()
                .find(|m| m.is_primary().unwrap_or(false))
            {
                if let Ok(image) = primary_monitor.capture_image() {
                    let cropped_image = image
                        .view(
                            physical_x as u32,
                            physical_y as u32,
                            physical_width,
                            physical_height,
                        )
                        .to_image();

                    let now = Local::now();
                    let timestamp = if now.hour() >= 17 {
                        now.format("%Y%m%d_%H시%M분%S초").to_string()
                    } else {
                        now.format("%Y%m%d_%H시%M분").to_string()
                    };
                    let filename = match &config.base_filename {
                        Some(name) if !name.is_empty() => format!("{}_{}.jpg", name, timestamp),
                        _ => format!("{}_온라인_출석체크.jpg", timestamp),
                    };
                    let path = config.save_directory.join(filename);

                    let file_out = match File::create(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            eprintln!("Failed to create file {:?}: {}", path, e);
                            return;
                        }
                    };
                    let mut encoder = JpegEncoder::new_with_quality(file_out, 90);
                    if encoder.encode_image(&cropped_image).is_ok() {
                        println!("Captured and saved image to {:?}", path);
                    } else {
                        eprintln!("Failed to encode and save image to {:?}", path);
                    }
                }
            }
        }
    }
}
