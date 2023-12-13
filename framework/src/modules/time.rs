use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use smallvec::{smallvec, SmallVec};

const CACHED_DELTA_TIMES_COUNT: usize = 20;

#[derive(Debug)]
pub struct Time {
    frame_count: usize,
    last_frame: Instant,
    delta_time: Duration,
    total_time: Duration,
    start_time: Instant,
    delta_times: VecDeque<Duration>,
    stats: TimeStats,
}

#[derive(Debug, Default)]
pub struct TimeStats {
    fps: Stats,
    delta_ms: Stats,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub max: f64,
    pub min: f64,
    pub avg: f64,
    pub std: f64,
}

impl Default for Time {
    fn default() -> Self {
        let mut delta_times = VecDeque::new();
        delta_times.push_back(Duration::from_millis(10));
        Time {
            start_time: Instant::now(),
            total_time: Duration::ZERO,
            frame_count: 0,
            last_frame: Instant::now() - Duration::from_millis(10),
            delta_time: Duration::from_millis(10),
            delta_times,
            stats: TimeStats::default(),
        }
    }
}

impl Time {
    pub fn delta_secs(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    pub fn delta_secs_f64(&self) -> f64 {
        self.delta_time.as_secs_f64()
    }

    pub fn total_secs(&self) -> f32 {
        self.total_time.as_secs_f32()
    }

    pub fn total_secs_f64(&self) -> f64 {
        self.total_time.as_secs_f64()
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    pub fn update(&mut self) {
        self.total_time = Instant::now() - self.start_time;
        let this_frame = Instant::now();
        if self.delta_times.len() >= CACHED_DELTA_TIMES_COUNT {
            self.delta_times.pop_back();
        }
        self.delta_time = this_frame.duration_since(self.last_frame);
        self.delta_times.push_front(self.delta_time);
        self.last_frame = this_frame;
        self.frame_count += 1;
        self.stats.recalculate(&self.delta_times);
    }

    pub fn egui_time_stats(&mut self, mut egui_ctx: egui::Context) {
        egui::Window::new("Time Stats").show(&mut egui_ctx, |ui| {
            ui.label(format!(
                "{} fps / {:.1} ms",
                self.stats.fps.avg as i64, self.stats.delta_ms.avg,
            ));
            if ui.button("Log Time Stats").clicked() {
                dbg!(&self);
            }
        });
    }
}

impl TimeStats {
    fn recalculate(&mut self, delta_times: &VecDeque<Duration>) {
        assert!(!delta_times.is_empty());
        assert!(delta_times.len() <= CACHED_DELTA_TIMES_COUNT);

        let mut delta_ms: SmallVec<[f64; CACHED_DELTA_TIMES_COUNT]> = smallvec![];
        let mut fps: SmallVec<[f64; CACHED_DELTA_TIMES_COUNT]> = smallvec![];
        for d in delta_times {
            let secs = d.as_secs_f64();
            delta_ms.push(secs * 1000.0);
            fps.push(1.0 / secs);
        }

        self.delta_ms = Stats::new(&delta_ms);
        self.fps = Stats::new(&fps);
    }
}

impl Stats {
    fn new(nums: &[f64]) -> Self {
        let mut max: f64 = f64::NAN;
        let mut min: f64 = f64::NAN;
        let mut sum: f64 = 0.0;
        let mut sqsum: f64 = 0.0;
        for e in nums {
            sum += *e;
            sqsum += *e * *e;
            if !(*e < max) {
                max = *e;
            }

            if !(*e > min) {
                min = *e;
            }
        }
        let len = nums.len() as f64;
        let avg = sum / len;
        let var = (sqsum / len) - ((sum / len) * (sum / len));
        let std = var.sqrt();
        Stats { max, min, avg, std }
    }
}
