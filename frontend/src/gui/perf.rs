use egui::Context;

use super::PerfState;

pub const MAX_FPS_HISTORY: usize = 10;

pub fn show(ctx: &Context, perf: &mut PerfState) {
    egui::TopBottomPanel::top("perf").show(ctx, |ui| {
        let (current, average, min, max) = {
            if perf.fps_history.len() > 0 {
                if perf.fps_history.len() > MAX_FPS_HISTORY {
                    perf.fps_history.pop_front();
                }

                let newest = *perf.fps_history.back().unwrap();
                perf.max_fps = perf.max_fps.max(newest);
                perf.min_fps = perf.min_fps.min(newest);
                
                let average: usize = perf.fps_history.iter().sum::<usize>() / perf.fps_history.len();
                
                (
                    format!("{newest}"),
                    format!("{average}"),
                    format!("{}", perf.min_fps),
                    format!("{}", perf.max_fps)
                )
            } else {
                (
                    "N/A".to_owned(),
                    "N/A".to_owned(),
                    "N/A".to_owned(),
                    "N/A".to_owned()
                )
            }
        };

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("FPS: {current}"));
            ui.label(format!("Avg. FPS: {average}"));
            ui.label(format!("Min: {min}"));
            ui.label(format!("Max: {max}"));
        });
    });
}