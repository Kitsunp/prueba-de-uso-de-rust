//! Timeline panel for the editor workbench.
//!
//! Displays and allows editing of animation keyframes.

use eframe::egui;
use visual_novel_engine::Timeline;

/// Timeline panel widget.
pub struct TimelinePanel<'a> {
    timeline: &'a mut Timeline,
    current_time: &'a mut u32,
    is_playing: &'a mut bool,
}

impl<'a> TimelinePanel<'a> {
    pub fn new(
        timeline: &'a mut Timeline,
        current_time: &'a mut u32,
        is_playing: &'a mut bool,
    ) -> Self {
        Self {
            timeline,
            current_time,
            is_playing,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("⏱ Timeline");
        ui.separator();

        // Playback controls
        ui.horizontal(|ui| {
            // Play/Pause button
            let play_text = if *self.is_playing { "⏸" } else { "▶" };
            if ui.button(play_text).clicked() {
                *self.is_playing = !*self.is_playing;
            }

            // Stop button
            if ui.button("⏹").clicked() {
                *self.is_playing = false;
                *self.current_time = 0;
                self.timeline.seek(0);
            }

            // Rewind
            if ui.button("⏮").clicked() {
                *self.current_time = 0;
                self.timeline.seek(0);
            }

            ui.separator();

            // Time display
            let seconds = *self.current_time as f32 / self.timeline.ticks_per_second as f32;
            ui.label(format!(
                "Time: {:.2}s ({} ticks)",
                seconds, *self.current_time
            ));

            // Duration
            let duration = self.timeline.duration();
            let duration_secs = duration as f32 / self.timeline.ticks_per_second as f32;
            ui.label(format!("Duration: {:.2}s", duration_secs));
        });

        ui.separator();

        // Time slider
        let duration = self.timeline.duration().max(1);
        let mut time_float = *self.current_time as f32;
        ui.horizontal(|ui| {
            ui.label("Scrub:");
            if ui
                .add(egui::Slider::new(&mut time_float, 0.0..=duration as f32).show_value(false))
                .changed()
            {
                *self.current_time = time_float as u32;
                self.timeline.seek(*self.current_time);
            }
        });

        ui.separator();

        // Track list
        ui.label(format!("Tracks: {}", self.timeline.track_count()));

        egui::ScrollArea::vertical()
            .max_height(80.0)
            .show(ui, |ui| {
                for (idx, track) in self.timeline.tracks().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Track {}: Entity {:?} - {:?}",
                            idx,
                            track.target.raw(),
                            track.property
                        ));
                        ui.label(format!("({} keyframes)", track.len()));
                    });
                }

                if self.timeline.track_count() == 0 {
                    ui.label("No tracks. Add keyframes to create animations.");
                }
            });
    }
}
