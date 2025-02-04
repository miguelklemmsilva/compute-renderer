use std::time::{Duration, Instant};
use sysinfo::{get_current_pid, System};

pub struct PerformanceData {
    pub avg_fps: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub fps_5_percent_low: f64,
    pub fps_1_percent_low: f64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

pub struct PerformanceCollector {
    frame_times: Vec<f64>,
    cpu_usages: Vec<f32>,
    memory_usages: Vec<u64>,
    system: System,
    current_pid: sysinfo::Pid,
    start_time: Instant,
    last_frame_time: Instant,
    benchmark_duration: Duration,
    scene_name: String,
    scene_index: usize,
    has_started: bool,
}

impl PerformanceCollector {
    pub fn new(scene_name: String, scene_index: usize, benchmark_duration: Duration) -> Self {
        Self {
            frame_times: Vec::new(),
            cpu_usages: Vec::new(),
            memory_usages: Vec::new(),
            system: System::new_all(),
            current_pid: get_current_pid().expect("Failed to get current PID"),
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            benchmark_duration,
            scene_name,
            scene_index,
            has_started: false,
        }
    }

    pub fn update(&mut self) -> bool {
        if !self.has_started {
            // Reset timers on first frame
            self.start_time = Instant::now();
            self.last_frame_time = Instant::now();
            self.has_started = true;
            return false;
        }

        let frame_time = self.last_frame_time.elapsed().as_secs_f64();
        self.frame_times.push(frame_time);
        self.last_frame_time = Instant::now();

        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        let cpu_usage = self.system.global_cpu_usage();
        self.cpu_usages.push(cpu_usage);

        if let Some(process) = self.system.process(self.current_pid) {
            let memory_usage = process.memory();
            self.memory_usages.push(memory_usage);
        } else {
            self.memory_usages.push(0);
        }

        // Return true if benchmark is complete
        self.start_time.elapsed() >= self.benchmark_duration
    }

    pub fn finalise(&self) -> PerformanceData {
        let data = self.calculate_metrics();
        self.print_results(&data);
        data
    }

    fn calculate_metrics(&self) -> PerformanceData {
        let avg_frame_time = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
        let avg_fps = 1.0 / avg_frame_time;

        let min_frame_time = *self
            .frame_times
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max_frame_time = *self
            .frame_times
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        let min_fps = 1.0 / max_frame_time;
        let max_fps = 1.0 / min_frame_time;

        let avg_cpu_usage = self.cpu_usages.iter().sum::<f32>() / self.cpu_usages.len() as f32;
        let avg_memory_usage =
            self.memory_usages.iter().sum::<u64>() / self.memory_usages.len() as u64;

        // Calculate percentile FPS
        let mut sorted_frame_times = self.frame_times.clone();
        sorted_frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let total_frames = sorted_frame_times.len();

        let percentile_5_index = (total_frames as f64 * 0.05).ceil() as usize;
        let percentile_1_index = (total_frames as f64 * 0.01).ceil() as usize;

        let fps_5_percent_low = 1.0
            / (sorted_frame_times
                .iter()
                .skip(total_frames - percentile_5_index)
                .sum::<f64>()
                / percentile_5_index as f64);

        let fps_1_percent_low = 1.0
            / (sorted_frame_times
                .iter()
                .skip(total_frames - percentile_1_index)
                .sum::<f64>()
                / percentile_1_index as f64);

        PerformanceData {
            avg_fps,
            min_fps,
            max_fps,
            fps_5_percent_low,
            fps_1_percent_low,
            cpu_usage: avg_cpu_usage,
            memory_usage: avg_memory_usage,
        }
    }

    fn print_results(&self, data: &PerformanceData) {
        println!(
            "Performance Data for Scene {}: {}",
            self.scene_index + 1,
            self.scene_name
        );
        println!("Average FPS: {:.2}", data.avg_fps);
        println!("Min FPS: {:.2}", data.min_fps);
        println!("Max FPS: {:.2}", data.max_fps);
        println!("5% Low FPS: {:.2}", data.fps_5_percent_low);
        println!("1% Low FPS: {:.2}", data.fps_1_percent_low);
        println!("Average CPU Usage: {:.2}%", data.cpu_usage);
        println!(
            "Average Memory Usage: {:.2} MB",
            data.memory_usage as f64 / (1024.0 * 1024.0)
        );
        println!("----------------------------------------");
    }
}
