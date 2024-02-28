use egui::Context;

use super::PerfState;

pub const MAX_FPS_HISTORY: usize = 10;

pub fn show(ctx: &Context, perf_state: &mut PerfState) {
    egui::TopBottomPanel::top("perf").show(ctx, |ui| {
        let (current, average, min, max) = {
            if perf_state.fps_history.len() > 0 {
                if perf_state.fps_history.len() > MAX_FPS_HISTORY {
                    perf_state.fps_history.pop_front();
                }

                let newest = *perf_state.fps_history.back().unwrap();
                perf_state.max_fps = perf_state.max_fps.max(newest);
                perf_state.min_fps = perf_state.min_fps.min(newest);
                
                let average: usize = perf_state.fps_history.iter().sum::<usize>() / perf_state.fps_history.len();
                
                (
                    format!("{newest}"),
                    format!("{average}"),
                    format!("{}", perf_state.min_fps),
                    format!("{}", perf_state.max_fps)
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