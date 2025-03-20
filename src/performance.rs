use std::time::{Duration, Instant};
use sysinfo::{get_current_pid, System};
// Define structures to hold performance metrics for benchmarking the rendering process.
pub struct PerformanceData {
    pub avg_fps: f64,
    pub min_fps: f64,
    pub max_fps: f64,
    pub fps_5_percent_low: f64,
    pub fps_1_percent_low: f64,
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

// PerformanceData holds key benchmarking metrics such as average, minimum, and maximum FPS, as well as CPU and memory usage.
pub struct PerformanceCollector {
    set_in_period: f32, // seconds
    frame_times: Vec<f64>,
    cpu_usages: Vec<f32>,
    memory_usages: Vec<u64>,
    system: System,
    current_pid: sysinfo::Pid,
    start_time: Instant,
    pub last_frame_time: Instant,
    benchmark_duration: Duration,
    scene_name: String,
    scene_index: usize,
    has_started: bool,
    has_printed: bool,
}

// PerformanceCollector gathers runtime performance metrics over a set duration for a given scene, enabling analysis of rendering performance.
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
            has_printed: false,
            set_in_period: 2.0,
        }
    }

    pub fn update(&mut self) -> bool {
        // Periodically update collected metrics: records frame times and system performance after an initial stabilization period.
        if !self.has_started {
            self.start_time = std::time::Instant::now();
            self.last_frame_time = std::time::Instant::now();
            self.has_started = true;
            return false;
        }

        // Skip the first few frames to allow the application to stabilise.
        if self.start_time.elapsed() < Duration::from_secs_f32(self.set_in_period) {
            return false;
        }

        // Measure elapsed time
        let frame_time = self.last_frame_time.elapsed().as_secs_f64();

        self.frame_times.push(frame_time);

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

        // Return true if the benchmark has run for the desired duration, indicating it's time to finalise the metrics.
        self.start_time.elapsed()
            >= self
                .benchmark_duration
                .saturating_add(Duration::from_secs_f32(self.set_in_period))
    }

    pub fn finalise(&mut self) -> PerformanceData {
        if self.has_printed {
            return self.calculate_metrics();
        }
        let data = self.calculate_metrics();
        self.print_results(&data);
        self.has_printed = true;
        data
    }

    fn calculate_metrics(&self) -> PerformanceData {
        // Analyse the collected frame times and system usage data to compute performance metrics.
        // This includes calculating average FPS and determining performance consistency through percentiles.
        if self.frame_times.is_empty() {
            return PerformanceData {
                avg_fps: 0.0,
                min_fps: 0.0,
                max_fps: 0.0,
                fps_5_percent_low: 0.0,
                fps_1_percent_low: 0.0,
                cpu_usage: 0.0,
                memory_usage: 0,
            };
        }

        // Calculate the average frame time, then derive the average FPS as its reciprocal.
        let avg_frame_time = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
        let avg_fps = 1.0 / avg_frame_time;

        // Sort the collected frame times to facilitate extraction of the fastest and slowest segments.
        let mut sorted_frame_times = self.frame_times.clone();
        sorted_frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let total_frames = sorted_frame_times.len();

        // Compute the average of the fastest 5% of frame times to approximate the maximum achievable FPS.
        let fastest_count = ((total_frames as f64) * 0.05).ceil() as usize;
        let fastest_avg =
            sorted_frame_times.iter().take(fastest_count).sum::<f64>() / fastest_count as f64;
        let max_fps = 1.0 / fastest_avg;

        // Compute the average of the slowest 5% of frame times to estimate the minimum FPS during performance dips.
        let slowest_count = ((total_frames as f64) * 0.05).ceil() as usize;
        let slowest_avg = sorted_frame_times
            .iter()
            .rev()
            .take(slowest_count)
            .sum::<f64>()
            / slowest_count as f64;
        let min_fps = 1.0 / slowest_avg;

        let avg_cpu_usage = self.cpu_usages.iter().sum::<f32>() / self.cpu_usages.len() as f32;
        let avg_memory_usage: u64 =
            self.memory_usages.iter().sum::<u64>() / self.memory_usages.len() as u64;

        // Calculate lower FPS percentiles (5% and 1%) to gauge worst-case performance scenarios.
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
        // Print the computed performance metrics to the console for easy review and analysis.
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
