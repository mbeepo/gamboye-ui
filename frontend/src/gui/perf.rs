use std::{collections::VecDeque, time::{Duration, Instant}};

use egui::Context;

use crate::{comms::EmuMsgIn, state::PerfState};

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

pub fn record_frame(state: &mut super::TopState) {
    state.perf.frames += 1;
    
    let now = Instant::now();

    let Some(last_second) = state.perf.last_second else {
        state.perf.last_second = Some(now);
        return;
    };

    let duration = now.duration_since(last_second).as_millis();
    dbg!(duration);

    if duration >= 1000 {
        state.perf.last_second = Some(now);
        state.perf.fps_history.push_back(state.perf.frames);
        state.perf.frames = 0;
    } else if state.perf.frames >= crate::gui::MAX_FRAMERATE {
        if let Some(ref emu_channel) = state.emu.sender {
            let awaken = last_second + Duration::from_millis(1000);
            state.emu.wait_until = Some(awaken);
            emu_channel.send(EmuMsgIn::Pause).unwrap();

            let awaken_sender = emu_channel.clone();
            tokio::spawn(async move {
                tokio::time::sleep_until(awaken.into()).await;
                awaken_sender.send(EmuMsgIn::Resume).unwrap();
            });
        }
    }
}