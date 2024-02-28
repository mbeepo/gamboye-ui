use std::sync::Arc;

use egui::{mutex::Mutex, Context, Ui, ViewportBuilder, ViewportId};

use super::PerfState;


pub fn show(perf_state: Mutex<PerfState>) -> impl for<'a> Fn(&'a mut Ui) {
    |ui: &mut Ui| {
        let (current, average, min, max) = {
            let mut perf_state = perf_state.lock();
            
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

        ui.label(format!("FPS: {current}"));
        ui.label(format!("Avg. FPS: {average}"));
        ui.label(format!("Min: {min}"));
        ui.label(format!("Max: {max}"));
    }
}