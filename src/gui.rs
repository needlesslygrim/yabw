use std::path::PathBuf;
use eframe::egui;
use crate::{MediaType, Resolution, Download};
use std::path::Path;
use rfd::FileDialog;


pub struct App;

impl App {
    pub fn build() -> Result<(), Box<dyn std::error::Error>> {
        let options = eframe::NativeOptions::default();

        let mut url = String::new();
        let mut media_type = MediaType::Audio;
        let mut download_dir = String::new();

        eframe::run_simple_native("YABW", options, move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("A simple wrapper for yt-dlp");
                ui.horizontal(|ui| {
                    let url_label = ui.label("Enter the URL of the video to download: ");
                    ui.text_edit_singleline(&mut url).labelled_by(url_label.id);
                });
                egui::ComboBox::from_label("Pick a media type: ")
                    .selected_text(match media_type {
                        MediaType::Audio => "Audio",
                        MediaType::Video(_) => "Video",
                    })
                    .show_ui(ui, |ui| {
                        ui.style_mut().wrap = Some(false);
                        ui.set_min_width(60.0);
                        ui.selectable_value(&mut media_type, crate::MediaType::Audio, "Audio");
                        ui.selectable_value(
                            &mut media_type,
                            MediaType::Video(crate::Resolution::P144),
                            "Video",
                        );
                    });
                if let MediaType::Video(mut resolution) = media_type {
                    egui::ComboBox::from_label("Pick a resolution:")
                        .selected_text(match resolution {
                            Resolution::P144 => "144p",
                            Resolution::P240 => "240p",
                            Resolution::P480 => "480p",
                            Resolution::P720 => "720p",
                            Resolution::P1080 => "1080p",
                            Resolution::P1440 => "1440p",
                            Resolution::P2160 => "2160p",
                        })
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            ui.selectable_value(&mut resolution, Resolution::P144, "144p");
                            ui.selectable_value(&mut resolution, Resolution::P240, "240p");
                            ui.selectable_value(&mut resolution, Resolution::P480, "480p");
                            ui.selectable_value(&mut resolution, Resolution::P720, "720p");
                            ui.selectable_value(&mut resolution, Resolution::P1080, "1080p");
                            ui.selectable_value(&mut resolution, Resolution::P1440, "1440p");
                            ui.selectable_value(&mut resolution, Resolution::P2160, "2160p");
                        });
                    media_type = MediaType::Video(resolution);
                };
                ui.horizontal(|ui| {
                    let url_label = ui.label("Choose a directory to download to:");
                    ui.text_edit_singleline(&mut download_dir).labelled_by(url_label.id);
                    if ui.button("Pick").clicked() {
                        if let Some(dir) = rfd::FileDialog::new()
                            .set_directory("/home/needlesslygrim/Downloads")
                            .pick_folder() {
                          download_dir =  dir.into_os_string().into_string().expect("Something proper utf8");
                        }
                    };

                });

                if ui.button("Download").clicked() {
                    println!(
                        "{:#?}",
                        Download {
                            url: url.clone(),
                            media_type,
                            filepath: Default::default(),
                            download_dir: &Path::new(&download_dir)
                        }
                    );
                }});
        })?;
        Ok(())
    }

}

