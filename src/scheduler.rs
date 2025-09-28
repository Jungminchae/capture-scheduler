use crate::config::Config;
use chrono::{Local, Timelike};
use image::GenericImageView;
use xcap::Monitor;

pub fn check_and_capture(config: &Config) {
    let now = Local::now();
    let current_time_key = (now.hour(), now.minute());

    for schedule in &config.schedule_times {
        if (schedule.hour, schedule.minute) == current_time_key {
            perform_capture(config);
            std::thread::sleep(std::time::Duration::from_secs(61));
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
                    let timestamp = now.format("%Y-%m-%d_%H-%M-%S").to_string();
                    let filename = match &config.base_filename {
                        Some(name) if !name.is_empty() => format!("{}_{}.png", name, timestamp),
                        _ => format!("capture_{}.png", timestamp),
                    };
                    let path = config.save_directory.join(filename);

                    if cropped_image.save(&path).is_ok() {
                        println!("Screenshot saved to {:?}", path);
                    } else {
                        eprintln!("Failed to save screenshot.");
                    }
                }
            }
        }
    }
}
