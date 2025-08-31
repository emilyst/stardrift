//! Automated screenshot functionality
//!
//! This module provides CLI-driven automated screenshot capture for
//! UI testing and validation workflows, with immediate capture that
//! preserves UI visibility.

use crate::prelude::*;
use bevy::app::AppExit;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use chrono::Local;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScreenshotTimingMode {
    Time,
    Frames,
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenshotDelay {
    pub value: f32,
    pub mode: ScreenshotTimingMode,
}

#[derive(Debug)]
enum AutoScreenshotState {
    WaitingForInitial,
    WaitingForInterval,
    Complete,
}

#[derive(Resource)]
pub struct AutomatedScreenshotSchedule {
    pub mode: ScreenshotTimingMode,
    pub initial_delay: Option<ScreenshotDelay>,
    pub interval: Option<ScreenshotDelay>,
    pub remaining_count: usize,
    pub exit_after_completion: bool,
    timer: Timer,
    frame_counter: u32,
    state: AutoScreenshotState,
    frames_since_last_screenshot: u32,
}

impl AutomatedScreenshotSchedule {
    pub fn new(
        initial_delay: Option<f32>,
        interval: Option<f32>,
        count: usize,
        use_frames: bool,
        exit_after: bool,
    ) -> Option<Self> {
        if initial_delay.is_none() && interval.is_none() {
            return None;
        }

        let mode = if use_frames {
            ScreenshotTimingMode::Frames
        } else {
            ScreenshotTimingMode::Time
        };

        let initial_delay_converted = initial_delay.map(|v| ScreenshotDelay { value: v, mode });
        let interval_converted = interval.map(|v| ScreenshotDelay { value: v, mode });

        // Set up timer for time-based mode
        let timer = if !use_frames {
            let duration = initial_delay
                .or(interval)
                .map(|v| Duration::from_secs_f32(v))
                .unwrap_or(Duration::from_secs(0));
            Timer::new(duration, TimerMode::Once)
        } else {
            Timer::default()
        };

        Some(Self {
            mode,
            initial_delay: initial_delay_converted,
            interval: interval_converted,
            remaining_count: count,
            exit_after_completion: exit_after,
            timer,
            frame_counter: 0,
            state: if initial_delay.is_some() {
                AutoScreenshotState::WaitingForInitial
            } else {
                AutoScreenshotState::WaitingForInterval
            },
            frames_since_last_screenshot: 0,
        })
    }

    pub fn tick(&mut self, delta: f32) -> bool {
        match self.mode {
            ScreenshotTimingMode::Time => {
                self.timer.tick(Duration::from_secs_f32(delta));
            }
            ScreenshotTimingMode::Frames => {
                self.frame_counter += 1;
            }
        }

        match self.state {
            AutoScreenshotState::WaitingForInitial => {
                if let Some(delay) = self.initial_delay {
                    if self.should_trigger(delay) {
                        self.reset_timer();
                        self.remaining_count = self.remaining_count.saturating_sub(1);
                        self.frames_since_last_screenshot = 0;

                        if self.remaining_count > 0 && self.interval.is_some() {
                            self.state = AutoScreenshotState::WaitingForInterval;
                            // Set up interval timer
                            if let (ScreenshotTimingMode::Time, Some(interval)) =
                                (self.mode, self.interval)
                            {
                                self.timer = Timer::new(
                                    Duration::from_secs_f32(interval.value),
                                    TimerMode::Once,
                                );
                            }
                        } else {
                            self.state = AutoScreenshotState::Complete;
                        }
                        return true;
                    }
                }
            }
            AutoScreenshotState::WaitingForInterval => {
                if let Some(interval) = self.interval {
                    if self.should_trigger(interval) {
                        self.reset_timer();
                        self.remaining_count = self.remaining_count.saturating_sub(1);
                        self.frames_since_last_screenshot = 0;

                        if self.remaining_count == 0 {
                            self.state = AutoScreenshotState::Complete;
                        } else if self.mode == ScreenshotTimingMode::Time {
                            // Reset timer for next interval
                            self.timer = Timer::new(
                                Duration::from_secs_f32(interval.value),
                                TimerMode::Once,
                            );
                        }
                        return true;
                    }
                }
            }
            AutoScreenshotState::Complete => {}
        }

        false
    }

    fn should_trigger(&self, delay: ScreenshotDelay) -> bool {
        match delay.mode {
            ScreenshotTimingMode::Time => self.timer.finished(),
            ScreenshotTimingMode::Frames => self.frame_counter >= delay.value as u32,
        }
    }

    fn reset_timer(&mut self) {
        match self.mode {
            ScreenshotTimingMode::Time => {
                self.timer.reset();
            }
            ScreenshotTimingMode::Frames => {
                self.frame_counter = 0;
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, AutoScreenshotState::Complete)
    }
}

#[derive(Resource)]
pub struct AutomatedScreenshotNaming {
    pub base_directory: PathBuf,
    pub base_name: String,
    pub use_timestamp: bool,
    pub use_sequential: bool,
    pub current_sequence: usize,
    pub created_files: Vec<PathBuf>,
    pub list_paths: bool,
}

impl AutomatedScreenshotNaming {
    pub fn new(
        directory: Option<String>,
        name: Option<String>,
        no_timestamp: bool,
        sequential: bool,
        list_paths: bool,
        config: &SimulationConfig,
    ) -> Self {
        // Priority: CLI args > config file > defaults
        let base_directory = directory
            .as_ref()
            .or(config.screenshots.directory.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let base_name = name.unwrap_or_else(|| config.screenshots.filename_prefix.clone());

        let use_timestamp = !no_timestamp && !sequential && config.screenshots.include_timestamp;

        Self {
            base_directory,
            base_name,
            use_timestamp,
            use_sequential: sequential,
            current_sequence: 1,
            created_files: Vec::new(),
            list_paths,
        }
    }

    pub fn generate_path(&mut self) -> PathBuf {
        // Ensure directory exists
        if !self.base_directory.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.base_directory) {
                error!("Failed to create screenshot directory: {}", e);
                // Fallback to current directory
                self.base_directory = PathBuf::from(".");
            }
        }

        let filename = if self.use_sequential {
            let name = format!("{}_{:04}.png", self.base_name, self.current_sequence);
            self.current_sequence += 1;
            name
        } else if self.use_timestamp {
            let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
            format!("{}_{}.png", self.base_name, timestamp)
        } else {
            // Single static name
            format!("{}.png", self.base_name)
        };

        let full_path = self.base_directory.join(filename);
        self.created_files.push(full_path.clone());

        if self.list_paths {
            // Output to stdout for easy capture by automation tools
            println!("SCREENSHOT_PATH: {}", full_path.display());
        }

        full_path
    }
}

pub fn process_automated_screenshots(
    mut commands: Commands,
    mut schedule: ResMut<AutomatedScreenshotSchedule>,
    mut naming: Option<ResMut<AutomatedScreenshotNaming>>,
    time: Res<Time>,
    mut app_exit_events: EventWriter<AppExit>,
    config: Res<SimulationConfig>,
) {
    let delta = match schedule.mode {
        ScreenshotTimingMode::Time => time.delta().as_secs_f32(),
        ScreenshotTimingMode::Frames => 1.0, // Count frames
    };

    // Track frames since last screenshot for exit timing
    schedule.frames_since_last_screenshot += 1;

    if schedule.tick(delta) {
        // Generate path for screenshot
        let path_string = if let Some(ref mut naming) = naming {
            let path = naming.generate_path();
            path.to_string_lossy().to_string()
        } else {
            // Fallback to default naming
            let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
            let filename = format!("{}_{}.png", config.screenshots.filename_prefix, timestamp);

            if let Some(ref dir) = config.screenshots.directory {
                PathBuf::from(dir)
                    .join(filename)
                    .to_string_lossy()
                    .to_string()
            } else {
                filename
            }
        };

        info!(
            "Taking automated screenshot ({} remaining): {}",
            schedule.remaining_count, path_string
        );

        // Take screenshot immediately without hiding UI
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path_string));
    }

    if schedule.is_complete() && schedule.exit_after_completion {
        // Wait sufficient frames after the last screenshot to ensure it saves
        // Testing shows 3 frames is the minimum, but we use 5 for safety margin
        // to account for system load variations and ensure reliable saves
        if schedule.frames_since_last_screenshot > 5 {
            info!("All automated screenshots completed, exiting...");
            app_exit_events.write(AppExit::Success);
        }
    }
}
