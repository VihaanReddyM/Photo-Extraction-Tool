//! Device Monitor Module
//!
//! Provides hot-plug detection and device state tracking for iOS devices (iPhone/iPad).
//! This module monitors for device connections/disconnections and tracks
//! device states (locked, trusted, etc.). No iTunes or additional drivers required.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use log::{debug, info, trace};

use crate::device::{DeviceContentTrait, DeviceInfo, DeviceManagerTrait};
use crate::ui::events::{DeviceEvent, UiEvent};

// =============================================================================
// Device State
// =============================================================================

/// Current state of a connected device
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    /// Device is connected and accessible
    Connected,
    /// Device is locked and needs to be unlocked
    Locked,
    /// Device needs to be trusted (user hasn't tapped "Trust")
    NeedsTrust,
    /// Device is disconnected
    Disconnected,
    /// Device state is unknown or being determined
    Unknown,
}

/// Detailed information about a monitored device
#[derive(Debug, Clone)]
pub struct MonitoredDevice {
    /// Basic device information
    pub info: DeviceInfo,
    /// Current state
    pub state: DeviceState,
    /// When the device was first seen in this session
    pub first_seen: Instant,
    /// When the device was last seen/accessed
    pub last_seen: Instant,
    /// Whether this device was known from a previous session
    pub previously_known: bool,
    /// Number of times we've failed to access this device
    pub access_failures: u32,
    /// Last error message, if any
    pub last_error: Option<String>,
}

impl MonitoredDevice {
    /// Create a new monitored device
    pub fn new(info: DeviceInfo, previously_known: bool) -> Self {
        let now = Instant::now();
        Self {
            info,
            state: DeviceState::Unknown,
            first_seen: now,
            last_seen: now,
            previously_known,
            access_failures: 0,
            last_error: None,
        }
    }

    /// Update the last seen time
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Record an access failure
    pub fn record_failure(&mut self, error: String) {
        self.access_failures += 1;
        self.last_error = Some(error);
    }

    /// Reset failure count (e.g., after successful access)
    pub fn reset_failures(&mut self) {
        self.access_failures = 0;
        self.last_error = None;
    }

    /// Check if device appears to be unresponsive
    pub fn is_unresponsive(&self) -> bool {
        self.access_failures >= 3
    }

    /// Get time since device was last seen
    pub fn time_since_last_seen(&self) -> Duration {
        self.last_seen.elapsed()
    }
}

// =============================================================================
// Monitor Configuration
// =============================================================================

/// Configuration for the device monitor
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// How often to poll for device changes (in milliseconds)
    pub poll_interval_ms: u64,
    /// How long before a device is considered disconnected if not seen
    pub disconnect_timeout_ms: u64,
    /// Maximum consecutive failures before marking device as unresponsive
    pub max_failures: u32,
    /// Whether to only monitor Apple devices
    pub apple_only: bool,
    /// Whether to automatically try to determine device state
    pub probe_device_state: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 2000,       // Poll every 2 seconds
            disconnect_timeout_ms: 10000, // 10 seconds without seeing = disconnected
            max_failures: 3,
            apple_only: true,
            probe_device_state: true,
        }
    }
}

impl MonitorConfig {
    /// Create a config with faster polling (for responsive UIs)
    pub fn fast() -> Self {
        Self {
            poll_interval_ms: 500,
            disconnect_timeout_ms: 5000,
            ..Default::default()
        }
    }

    /// Create a config with slower polling (for background monitoring)
    pub fn slow() -> Self {
        Self {
            poll_interval_ms: 5000,
            disconnect_timeout_ms: 15000,
            ..Default::default()
        }
    }

    /// Set poll interval
    pub fn with_poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }

    /// Set disconnect timeout
    pub fn with_disconnect_timeout(mut self, ms: u64) -> Self {
        self.disconnect_timeout_ms = ms;
        self
    }

    /// Set whether to monitor all devices or just Apple devices
    pub fn apple_only(mut self, apple_only: bool) -> Self {
        self.apple_only = apple_only;
        self
    }
}

// =============================================================================
// Device Monitor
// =============================================================================

/// Thread-safe device monitor that tracks connected devices
///
/// This monitor runs in a background thread and periodically polls for
/// device changes. It emits events when devices are connected, disconnected,
/// or change state.
pub struct DeviceMonitor {
    /// Configuration
    config: MonitorConfig,
    /// Currently tracked devices
    devices: Arc<RwLock<HashMap<String, MonitoredDevice>>>,
    /// Known device IDs from previous sessions
    known_device_ids: Arc<RwLock<Vec<String>>>,
    /// Shutdown flag
    shutdown_flag: Arc<AtomicBool>,
    /// Event sender
    event_tx: Sender<UiEvent>,
    /// Event receiver (for UI)
    event_rx: std::sync::Mutex<Receiver<UiEvent>>,
    /// Monitor thread handle
    thread_handle: std::sync::Mutex<Option<JoinHandle<()>>>,
    /// Whether monitor is running
    is_running: AtomicBool,
}

impl DeviceMonitor {
    /// Create a new device monitor with default configuration
    pub fn new() -> Self {
        Self::with_config(MonitorConfig::default())
    }

    /// Create a new device monitor with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel();

        Self {
            config,
            devices: Arc::new(RwLock::new(HashMap::new())),
            known_device_ids: Arc::new(RwLock::new(Vec::new())),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            event_tx,
            event_rx: std::sync::Mutex::new(event_rx),
            thread_handle: std::sync::Mutex::new(None),
            is_running: AtomicBool::new(false),
        }
    }

    /// Add known device IDs (from previous sessions/config)
    pub fn add_known_devices(&self, device_ids: Vec<String>) {
        let mut known = self.known_device_ids.write().unwrap();
        for id in device_ids {
            if !known.contains(&id) {
                known.push(id);
            }
        }
    }

    /// Start the device monitor with the given device manager
    pub fn start<M>(&self, device_manager: Arc<M>) -> Result<(), String>
    where
        M: DeviceManagerTrait + Send + Sync + 'static,
    {
        if self.is_running.load(Ordering::SeqCst) {
            return Err("Monitor is already running".to_string());
        }

        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.is_running.store(true, Ordering::SeqCst);

        let config = self.config.clone();
        let devices = Arc::clone(&self.devices);
        let known_device_ids = Arc::clone(&self.known_device_ids);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let event_tx = self.event_tx.clone();

        let handle = thread::spawn(move || {
            Self::monitor_loop(
                device_manager,
                config,
                devices,
                known_device_ids,
                shutdown_flag,
                event_tx,
            );
        });

        *self.thread_handle.lock().unwrap() = Some(handle);
        info!("Device monitor started");
        Ok(())
    }

    /// Stop the device monitor
    pub fn stop(&self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        info!("Stopping device monitor...");
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.is_running.store(false, Ordering::SeqCst);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            let _ = handle.join();
        }

        info!("Device monitor stopped");
    }

    /// Check if monitor is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Get list of currently connected devices
    pub fn connected_devices(&self) -> Vec<MonitoredDevice> {
        self.devices
            .read()
            .unwrap()
            .values()
            .filter(|d| d.state != DeviceState::Disconnected)
            .cloned()
            .collect()
    }

    /// Get a specific device by ID
    pub fn get_device(&self, device_id: &str) -> Option<MonitoredDevice> {
        self.devices.read().unwrap().get(device_id).cloned()
    }

    /// Get count of connected devices
    pub fn device_count(&self) -> usize {
        self.devices
            .read()
            .unwrap()
            .values()
            .filter(|d| d.state != DeviceState::Disconnected)
            .count()
    }

    /// Try to receive the next event (non-blocking)
    pub fn try_recv_event(&self) -> Option<UiEvent> {
        match self.event_rx.lock().unwrap().try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => None,
        }
    }

    /// Receive event with timeout
    pub fn recv_event_timeout(&self, timeout: Duration) -> Option<UiEvent> {
        self.event_rx.lock().unwrap().recv_timeout(timeout).ok()
    }

    /// Drain all pending events
    pub fn drain_events(&self) -> Vec<UiEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv_event() {
            events.push(event);
        }
        events
    }

    /// Force a refresh of device list (for manual refresh button)
    pub fn force_refresh(&self) {
        // Clear the devices map to force re-detection
        // The monitor loop will pick up changes on next iteration
        // We don't actually clear - we just mark all as needing refresh
        let mut devices = self.devices.write().unwrap();
        for device in devices.values_mut() {
            device.state = DeviceState::Unknown;
        }
    }

    /// The main monitor loop that runs in the background thread
    fn monitor_loop<M>(
        device_manager: Arc<M>,
        config: MonitorConfig,
        devices: Arc<RwLock<HashMap<String, MonitoredDevice>>>,
        known_device_ids: Arc<RwLock<Vec<String>>>,
        shutdown_flag: Arc<AtomicBool>,
        event_tx: Sender<UiEvent>,
    ) where
        M: DeviceManagerTrait + Send + Sync + 'static,
    {
        let poll_interval = Duration::from_millis(config.poll_interval_ms);
        let disconnect_timeout = Duration::from_millis(config.disconnect_timeout_ms);

        loop {
            // Check for shutdown
            if shutdown_flag.load(Ordering::SeqCst) {
                debug!("Monitor shutdown requested");
                break;
            }

            // Enumerate devices
            let current_devices = if config.apple_only {
                device_manager.enumerate_apple_devices()
            } else {
                device_manager.enumerate_all_devices()
            };

            match current_devices {
                Ok(device_list) => {
                    Self::process_device_list(
                        &device_list,
                        &devices,
                        &known_device_ids,
                        &event_tx,
                        disconnect_timeout,
                        &config,
                    );
                }
                Err(e) => {
                    trace!("Error enumerating devices: {}", e);
                    // Don't spam errors, just continue polling
                }
            }

            // Sleep before next poll
            thread::sleep(poll_interval);
        }
    }

    /// Process the list of devices and emit appropriate events
    fn process_device_list(
        device_list: &[DeviceInfo],
        devices: &Arc<RwLock<HashMap<String, MonitoredDevice>>>,
        known_device_ids: &Arc<RwLock<Vec<String>>>,
        event_tx: &Sender<UiEvent>,
        disconnect_timeout: Duration,
        _config: &MonitorConfig,
    ) {
        let mut devices_guard = devices.write().unwrap();
        let known = known_device_ids.read().unwrap();

        // Track which devices we've seen this iteration
        let mut seen_ids: Vec<String> = Vec::new();

        // Process each device in the current list
        for device_info in device_list {
            let device_id = &device_info.device_id;
            seen_ids.push(device_id.clone());

            if let Some(existing) = devices_guard.get_mut(device_id) {
                // Device already known - update it
                existing.touch();
                existing.info = device_info.clone();

                // If it was disconnected, it's reconnected now
                if existing.state == DeviceState::Disconnected {
                    existing.state = DeviceState::Connected;
                    existing.reset_failures();

                    let _ = event_tx.send(UiEvent::Device(DeviceEvent::Connected {
                        device: device_info.clone(),
                        previously_known: existing.previously_known,
                    }));

                    info!("Device reconnected: {}", device_info.friendly_name);
                }
            } else {
                // New device
                let previously_known = known.contains(device_id);
                let mut new_device = MonitoredDevice::new(device_info.clone(), previously_known);
                new_device.state = DeviceState::Connected;

                devices_guard.insert(device_id.clone(), new_device);

                let _ = event_tx.send(UiEvent::Device(DeviceEvent::Connected {
                    device: device_info.clone(),
                    previously_known,
                }));

                info!(
                    "New device connected: {} (previously known: {})",
                    device_info.friendly_name, previously_known
                );
            }
        }

        // Check for disconnected devices
        for (device_id, device) in devices_guard.iter_mut() {
            if !seen_ids.contains(device_id) && device.state != DeviceState::Disconnected {
                // Device not seen - check if it's been long enough to consider disconnected
                if device.time_since_last_seen() > disconnect_timeout {
                    device.state = DeviceState::Disconnected;

                    let _ = event_tx.send(UiEvent::Device(DeviceEvent::Disconnected {
                        device_id: device_id.clone(),
                        device_name: Some(device.info.friendly_name.clone()),
                    }));

                    info!("Device disconnected: {}", device.info.friendly_name);
                }
            }
        }
    }
}

impl Default for DeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DeviceMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

// =============================================================================
// Device State Checker
// =============================================================================

/// Utility for checking and probing device states
pub struct DeviceStateChecker;

impl DeviceStateChecker {
    /// Try to determine the state of a device by attempting to access it
    pub fn probe_device_state<M>(device_manager: &M, device_id: &str) -> DeviceState
    where
        M: DeviceManagerTrait,
    {
        match device_manager.open_device(device_id) {
            Ok(content) => {
                // Try to enumerate root objects
                match content.enumerate_objects() {
                    Ok(objects) => {
                        if objects.is_empty() {
                            // Device accessible but no content - might be locked
                            DeviceState::Locked
                        } else {
                            DeviceState::Connected
                        }
                    }
                    Err(e) => {
                        let error_str = e.to_string().to_lowercase();
                        if error_str.contains("access denied")
                            || error_str.contains("trust")
                            || error_str.contains("0x80070005")
                        {
                            DeviceState::NeedsTrust
                        } else if error_str.contains("locked") {
                            DeviceState::Locked
                        } else {
                            DeviceState::Unknown
                        }
                    }
                }
            }
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("access denied") || error_str.contains("trust") {
                    DeviceState::NeedsTrust
                } else if error_str.contains("not found") || error_str.contains("disconnected") {
                    DeviceState::Disconnected
                } else {
                    DeviceState::Unknown
                }
            }
        }
    }

    /// Get a user-friendly message for a device state
    pub fn state_message(state: &DeviceState) -> &'static str {
        match state {
            DeviceState::Connected => "Connected and ready",
            DeviceState::Locked => "Device is locked. Please unlock your iOS device.",
            DeviceState::NeedsTrust => {
                "Please tap 'Trust' on your device when prompted to allow access."
            }
            DeviceState::Disconnected => "Device is disconnected",
            DeviceState::Unknown => "Checking device status...",
        }
    }

    /// Get an icon/emoji suggestion for the state (for UIs that support it)
    pub fn state_icon(state: &DeviceState) -> &'static str {
        match state {
            DeviceState::Connected => "✅",
            DeviceState::Locked => "🔒",
            DeviceState::NeedsTrust => "⚠️",
            DeviceState::Disconnected => "❌",
            DeviceState::Unknown => "❓",
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert_eq!(config.poll_interval_ms, 2000);
        assert_eq!(config.disconnect_timeout_ms, 10000);
        assert!(config.apple_only);
    }

    #[test]
    fn test_monitor_config_fast() {
        let config = MonitorConfig::fast();
        assert_eq!(config.poll_interval_ms, 500);
        assert_eq!(config.disconnect_timeout_ms, 5000);
    }

    #[test]
    fn test_monitor_config_slow() {
        let config = MonitorConfig::slow();
        assert_eq!(config.poll_interval_ms, 5000);
        assert_eq!(config.disconnect_timeout_ms, 15000);
    }

    #[test]
    fn test_monitor_config_builder() {
        let config = MonitorConfig::default()
            .with_poll_interval(1000)
            .with_disconnect_timeout(5000)
            .apple_only(false);

        assert_eq!(config.poll_interval_ms, 1000);
        assert_eq!(config.disconnect_timeout_ms, 5000);
        assert!(!config.apple_only);
    }

    #[test]
    fn test_monitored_device_creation() {
        let info = DeviceInfo::new("test-id", "Test iPhone", "Apple Inc.", "iPhone 15");
        let device = MonitoredDevice::new(info.clone(), false);

        assert_eq!(device.info.device_id, "test-id");
        assert_eq!(device.state, DeviceState::Unknown);
        assert!(!device.previously_known);
        assert_eq!(device.access_failures, 0);
        assert!(device.last_error.is_none());
    }

    #[test]
    fn test_monitored_device_failure_tracking() {
        let info = DeviceInfo::new("test-id", "Test iPhone", "Apple Inc.", "iPhone 15");
        let mut device = MonitoredDevice::new(info, false);

        assert!(!device.is_unresponsive());

        device.record_failure("Error 1".to_string());
        assert!(!device.is_unresponsive());

        device.record_failure("Error 2".to_string());
        assert!(!device.is_unresponsive());

        device.record_failure("Error 3".to_string());
        assert!(device.is_unresponsive());
        assert_eq!(device.access_failures, 3);
        assert_eq!(device.last_error, Some("Error 3".to_string()));

        device.reset_failures();
        assert!(!device.is_unresponsive());
        assert_eq!(device.access_failures, 0);
        assert!(device.last_error.is_none());
    }

    #[test]
    fn test_device_state_messages() {
        assert_eq!(
            DeviceStateChecker::state_message(&DeviceState::Connected),
            "Connected and ready"
        );
        assert!(DeviceStateChecker::state_message(&DeviceState::Locked).contains("unlock"));
        assert!(DeviceStateChecker::state_message(&DeviceState::NeedsTrust).contains("Trust"));
    }

    #[test]
    fn test_device_state_icons() {
        assert_eq!(
            DeviceStateChecker::state_icon(&DeviceState::Connected),
            "✅"
        );
        assert_eq!(DeviceStateChecker::state_icon(&DeviceState::Locked), "🔒");
        assert_eq!(
            DeviceStateChecker::state_icon(&DeviceState::NeedsTrust),
            "⚠️"
        );
        assert_eq!(
            DeviceStateChecker::state_icon(&DeviceState::Disconnected),
            "❌"
        );
        assert_eq!(DeviceStateChecker::state_icon(&DeviceState::Unknown), "❓");
    }

    #[test]
    fn test_device_monitor_creation() {
        let monitor = DeviceMonitor::new();
        assert!(!monitor.is_running());
        assert_eq!(monitor.device_count(), 0);
    }

    #[test]
    fn test_add_known_devices() {
        let monitor = DeviceMonitor::new();
        monitor.add_known_devices(vec!["device1".to_string(), "device2".to_string()]);

        let known = monitor.known_device_ids.read().unwrap();
        assert_eq!(known.len(), 2);
        assert!(known.contains(&"device1".to_string()));
        assert!(known.contains(&"device2".to_string()));
    }

    #[test]
    fn test_add_known_devices_dedup() {
        let monitor = DeviceMonitor::new();
        monitor.add_known_devices(vec!["device1".to_string(), "device1".to_string()]);

        let known = monitor.known_device_ids.read().unwrap();
        assert_eq!(known.len(), 1);
    }
}
