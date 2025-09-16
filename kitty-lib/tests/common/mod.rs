#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

#[cfg(test)]
use image::{ImageBuffer, Rgba};
#[cfg(test)]
use image_compare::rgba_hybrid_compare;

/// Test harness for launching and managing Kitty processes during integration testing.
///
/// This harness provides a controlled environment for testing Kitty terminal interactions,
/// including launching Kitty with specific configuration, executing remote commands,
/// screenshot capture and comparison, and ensuring proper cleanup.
pub struct KittyTestHarness {
    /// The running Kitty process
    process: Child,
    /// Path to the Unix socket for remote control
    socket_path: PathBuf,
    /// Path to the test configuration file
    #[allow(dead_code)]
    config_path: PathBuf,
    /// Temporary directory to keep socket directory alive
    _socket_temp: TempDir,
    /// Directory for storing test screenshots
    screenshots_dir: PathBuf,
    /// Kitty window ID for screenshot capture
    window_id: Option<u32>,
}

impl KittyTestHarness {
    /// Try launching Kitty with captured stdio for debugging
    async fn try_launch_with_captured_stdio(
        config_path: &Path,
        socket_path: &Path,
        debug_mode: bool,
    ) -> Result<Child, Box<dyn std::error::Error>> {
        if debug_mode {
            println!("[DEBUG] Launching Kitty with captured stdio for debugging");
        }

        let mut cmd = Command::new("kitty");
        cmd.arg("--config")
            .arg(config_path)
            .arg("--listen-on")
            .arg(format!("unix:{}", socket_path.display()));

        if debug_mode {
            // Capture output for debugging
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        cmd.stdin(Stdio::null());

        let process = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Kitty process: {}", e))?;

        if debug_mode {
            println!("[DEBUG] Kitty process spawned with PID: {:?}", process.id());
        }

        Ok(process)
    }

    /// Try launching Kitty with --single-instance flag
    async fn try_launch_single_instance(
        config_path: &Path,
        socket_path: &Path,
        debug_mode: bool,
    ) -> Result<Child, Box<dyn std::error::Error>> {
        if debug_mode {
            println!("[DEBUG] Launching Kitty with --single-instance flag");
        }

        let process = Command::new("kitty")
            .arg("--config")
            .arg(config_path)
            .arg("--listen-on")
            .arg(format!("unix:{}", socket_path.display()))
            .arg("--single-instance")
            .stdin(Stdio::null())
            .stdout(if debug_mode {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stderr(if debug_mode {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to spawn Kitty process with --single-instance: {}",
                    e
                )
            })?;

        if debug_mode {
            println!(
                "[DEBUG] Kitty --single-instance process spawned with PID: {:?}",
                process.id()
            );
        }

        Ok(process)
    }

    /// Try launching Kitty with default approach (null stdio)
    async fn try_launch_default(
        config_path: &Path,
        socket_path: &Path,
        debug_mode: bool,
    ) -> Result<Child, Box<dyn std::error::Error>> {
        if debug_mode {
            println!("[DEBUG] Launching Kitty with default approach (null stdio)");
        }

        let process = Command::new("kitty")
            .arg("--config")
            .arg(config_path)
            .arg("--listen-on")
            .arg(format!("unix:{}", socket_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn Kitty process with default approach: {}", e))?;

        if debug_mode {
            println!(
                "[DEBUG] Kitty default process spawned with PID: {:?}",
                process.id()
            );
        }

        Ok(process)
    }
    /// Launch a new Kitty instance with test configuration.
    ///
    /// This method:
    /// 1. Creates a temporary Unix socket for remote control
    /// 2. Launches Kitty with the test configuration
    /// 3. Waits for the process to be ready
    /// 4. Extracts the window ID for screenshot capture
    /// 5. Sets up screenshot directory
    ///
    /// # Returns
    ///
    /// A `KittyTestHarness` instance that can be used to interact with the launched Kitty process.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Kitty executable is not found
    /// - Failed to create temporary socket
    /// - Failed to launch the process
    /// - Process doesn't become ready within timeout
    pub async fn launch() -> Result<Self, Box<dyn std::error::Error>> {
        Self::launch_with_test_name("default").await
    }

    /// Launch a new Kitty instance with test configuration and specific test name for screenshots.
    ///
    /// This method:
    /// 1. Creates a temporary Unix socket for remote control
    /// 2. Launches Kitty with the test configuration
    /// 3. Waits for the process to be ready
    /// 4. Extracts the window ID for screenshot capture
    /// 5. Sets up screenshot directory for the specific test
    ///
    /// # Arguments
    ///
    /// * `test_name` - Name of the test, used for organizing screenshots
    ///
    /// # Returns
    ///
    /// A `KittyTestHarness` instance that can be used to interact with the launched Kitty process.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Kitty executable is not found
    /// - Failed to create temporary socket
    /// - Failed to launch the process
    /// - Process doesn't become ready within timeout
    pub async fn launch_with_test_name(
        test_name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Check for debug mode
        let debug_mode = std::env::var("KSM_TEST_DEBUG").unwrap_or_default() == "1";

        if debug_mode {
            println!("[DEBUG] KSM_TEST_DEBUG enabled - verbose logging active");
        }

        // Create a temporary directory for the socket - let Kitty create the actual socket file
        let socket_temp_dir = tempfile::TempDir::new()?;
        let socket_path = socket_temp_dir.path().join("kitty_test_socket");

        // Set up screenshots directory
        let screenshots_dir = setup_screenshot_directory(test_name)?;

        // Get the config path - look for it in tests/fixtures relative to workspace root
        let config_path = find_test_config()?;

        if debug_mode {
            println!(
                "[DEBUG] Launching Kitty with:\n  config: {:?}\n  socket: {:?}\n  screenshots: {:?}",
                config_path, socket_path, screenshots_dir
            );
        }

        log::info!(
            "Launching Kitty with config: {:?}, socket: {:?}, screenshots: {:?}",
            config_path,
            socket_path,
            screenshots_dir
        );

        // Verify config file exists
        if !config_path.exists() {
            return Err(format!("Config file not found: {:?}", config_path).into());
        }

        // Verify socket directory is writable
        if let Some(socket_dir) = socket_path.parent() {
            if !socket_dir.exists() {
                return Err(format!("Socket directory does not exist: {:?}", socket_dir).into());
            }
        } else {
            return Err("Socket path has no parent directory".into());
        }

        // Check if Kitty is available and get version
        let kitty_version_output = tokio::process::Command::new("kitty")
            .arg("--version")
            .output()
            .await
            .map_err(|e| {
                format!(
                    "Failed to check Kitty version. Make sure 'kitty' is in PATH. Error: {}",
                    e
                )
            })?;

        if !kitty_version_output.status.success() {
            return Err("Kitty --version command failed".into());
        }

        let kitty_version = String::from_utf8_lossy(&kitty_version_output.stdout);
        if debug_mode {
            println!("[DEBUG] Kitty version: {}", kitty_version.trim());
        }

        // Try multiple launch strategies
        let mut launch_errors = Vec::new();

        // Strategy 1: Launch with captured stdio for debugging
        if debug_mode {
            println!("[DEBUG] Attempting Strategy 1: Launch with captured stdio");
        }

        let launch_result =
            Self::try_launch_with_captured_stdio(&config_path, &socket_path, debug_mode).await;

        let process = match launch_result {
            Ok(process) => process,
            Err(e) => {
                launch_errors.push(format!("Strategy 1 failed: {}", e));

                if debug_mode {
                    println!("[DEBUG] Strategy 1 failed, trying Strategy 2: Single instance mode");
                }

                // Strategy 2: Try with --single-instance flag
                match Self::try_launch_single_instance(&config_path, &socket_path, debug_mode).await
                {
                    Ok(process) => process,
                    Err(e) => {
                        launch_errors.push(format!("Strategy 2 failed: {}", e));

                        if debug_mode {
                            println!(
                                "[DEBUG] Strategy 2 failed, trying Strategy 3: Default launch"
                            );
                        }

                        // Strategy 3: Fall back to original approach with null stdio
                        match Self::try_launch_default(&config_path, &socket_path, debug_mode).await
                        {
                            Ok(process) => process,
                            Err(e) => {
                                launch_errors.push(format!("Strategy 3 failed: {}", e));

                                let all_errors = launch_errors.join("; ");
                                return Err(format!(
                                    "All launch strategies failed. Errors: {}",
                                    all_errors
                                )
                                .into());
                            }
                        }
                    }
                }
            }
        };

        let mut harness = Self {
            process,
            socket_path,
            config_path,
            _socket_temp: socket_temp_dir,
            screenshots_dir,
            window_id: None,
        };

        // Wait for Kitty to be ready (socket to be available)
        harness.wait_for_ready(debug_mode).await?;

        // Extract window ID for screenshot capture
        harness.extract_window_id().await?;

        if debug_mode {
            println!(
                "[DEBUG] Kitty test harness launched successfully with window ID: {:?}",
                harness.window_id
            );
        }

        log::info!(
            "Kitty test harness launched successfully with window ID: {:?}",
            harness.window_id
        );
        Ok(harness)
    }

    /// Execute a remote command in the Kitty instance.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The Kitty remote control command to execute (without the `kitty @` prefix)
    ///
    /// # Returns
    ///
    /// The output of the command as a String.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or times out.
    pub async fn execute_command(&self, cmd: &str) -> Result<String, Box<dyn std::error::Error>> {
        log::debug!("Executing command: {}", cmd);

        let output = timeout(
            Duration::from_secs(10),
            tokio::process::Command::new("kitty")
                .arg("@")
                .arg("--to")
                .arg(format!("unix:{}", self.socket_path.display()))
                .args(cmd.split_whitespace())
                .output(),
        )
        .await??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Command failed: {} - {}", cmd, stderr).into());
        }

        let result = String::from_utf8(output.stdout)?;
        log::debug!("Command output: {}", result.trim());
        Ok(result)
    }

    /// Execute a query command that might return no results (handles "No matching tabs" gracefully)
    pub async fn execute_query_command(
        &self,
        cmd: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        log::debug!("Executing query command: {}", cmd);

        let output = timeout(
            Duration::from_secs(10),
            tokio::process::Command::new("kitty")
                .arg("@")
                .arg("--to")
                .arg(format!("unix:{}", self.socket_path.display()))
                .args(cmd.split_whitespace())
                .output(),
        )
        .await??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Handle "No matching tabs" as an empty result instead of an error
            if stderr.contains("No matching tabs") {
                log::debug!("Query returned no matching tabs - returning empty JSON array");
                return Ok("[]".to_string());
            }
            return Err(format!("Query command failed: {} - {}", cmd, stderr).into());
        }

        let result = String::from_utf8(output.stdout)?;
        log::debug!("Query command output: {}", result.trim());
        Ok(result)
    }

    /// Clean up the Kitty process and associated resources.
    ///
    /// This method gracefully terminates the Kitty process and ensures
    /// all resources are properly cleaned up.
    ///
    /// # Errors
    ///
    /// Returns an error if the process cannot be terminated cleanly.
    pub async fn cleanup(mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Cleaning up Kitty test harness");

        // Try to quit gracefully first
        if let Err(e) = self.execute_command("quit").await {
            log::warn!("Failed to quit gracefully: {}", e);
        } else {
            // Wait a moment for graceful shutdown
            sleep(Duration::from_millis(500)).await;
        }

        // Check if process is still running and force kill if necessary
        match self.process.try_wait()? {
            Some(status) => {
                log::info!("Kitty process exited with status: {}", status);
            }
            None => {
                log::warn!("Kitty process still running, force killing");
                self.process.kill()?;
                self.process.wait()?;
            }
        }

        log::info!("Kitty test harness cleanup completed");
        Ok(())
    }

    /// Get the socket path being used for remote control.
    #[allow(dead_code)]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Get the config path being used.
    #[allow(dead_code)]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get the screenshots directory.
    #[allow(dead_code)]
    pub fn screenshots_dir(&self) -> &Path {
        &self.screenshots_dir
    }

    /// Capture a screenshot of the Kitty window.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the screenshot file (without extension)
    ///
    /// # Returns
    ///
    /// Path to the captured screenshot file.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Window ID is not available
    /// - Screenshot capture fails
    /// - File system errors occur
    pub async fn capture_screenshot(
        &self,
        name: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let window_id = self
            .window_id
            .ok_or("Window ID not available for screenshot capture")?;

        let screenshot_path = self.screenshots_dir.join(format!("{}.png", name));

        log::debug!(
            "Capturing screenshot for window {} to {:?}",
            window_id,
            screenshot_path
        );

        // Use macOS screencapture command to capture the specific window
        let output = tokio::process::Command::new("screencapture")
            .arg("-l") // Capture specific window by ID
            .arg(window_id.to_string())
            .arg(&screenshot_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Screenshot capture failed: {}", stderr).into());
        }

        // Verify the file was created
        if !screenshot_path.exists() {
            return Err("Screenshot file was not created".into());
        }

        log::info!("Screenshot captured successfully: {:?}", screenshot_path);
        Ok(screenshot_path)
    }

    /// Extract the window ID from Kitty ls output for screenshot capture.
    async fn extract_window_id(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Extracting window ID for screenshot capture");

        let output = self.execute_command("ls").await?;
        let parsed_json: serde_json::Value = serde_json::from_str(&output)
            .map_err(|e| format!("Failed to parse ls output as JSON: {}", e))?;

        // Navigate the JSON structure to find the first window ID
        if let Some(os_windows) = parsed_json.as_array() {
            for os_window in os_windows {
                if let Some(os_window_id) = os_window.get("id").and_then(|id| id.as_u64()) {
                    self.window_id = Some(os_window_id as u32);
                    log::info!("Extracted window ID: {}", os_window_id);
                    return Ok(());
                }
            }
        }

        Err("Could not extract window ID from Kitty ls output".into())
    }

    /// Wait for the Kitty process to be ready for commands.
    async fn wait_for_ready(&self, debug_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
        // Give Kitty more time to start up initially
        if debug_mode {
            println!("[DEBUG] Waiting 1 second for Kitty initial startup...");
        }
        sleep(Duration::from_millis(1000)).await;

        let max_attempts = 50;
        let delay = Duration::from_millis(200);

        // Check if socket file exists (it won't exist initially - Kitty creates it)
        if debug_mode {
            println!(
                "[DEBUG] Checking socket file existence: {:?}",
                self.socket_path
            );
            println!("[DEBUG] Socket exists: {}", self.socket_path.exists());
            if self.socket_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&self.socket_path) {
                    println!("[DEBUG] Socket metadata: {:?}", metadata);
                }
            } else {
                println!("[DEBUG] Socket doesn't exist yet - waiting for Kitty to create it");
            }
        }

        for attempt in 1..=max_attempts {
            if debug_mode && (attempt <= 5 || attempt % 10 == 0) {
                println!(
                    "[DEBUG] Checking if Kitty is ready (attempt {}/{})",
                    attempt, max_attempts
                );
            }

            log::debug!(
                "Checking if Kitty is ready (attempt {}/{})",
                attempt,
                max_attempts
            );

            // Try to execute a simple command to check if Kitty is ready
            let test_command = tokio::process::Command::new("kitty")
                .arg("@")
                .arg("--to")
                .arg(format!("unix:{}", self.socket_path.display()))
                .arg("ls")
                .output();

            match timeout(Duration::from_secs(3), test_command).await {
                Ok(Ok(output)) if output.status.success() => {
                    if debug_mode {
                        println!("[DEBUG] Kitty is ready after {} attempts", attempt);
                        println!(
                            "[DEBUG] Test command output: {}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                    }
                    log::info!("Kitty is ready after {} attempts", attempt);
                    return Ok(());
                }
                Ok(Ok(output)) => {
                    if debug_mode && attempt <= 5 {
                        println!("[DEBUG] Command failed with status: {}", output.status);
                        println!(
                            "[DEBUG] Stderr: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        println!(
                            "[DEBUG] Stdout: {}",
                            String::from_utf8_lossy(&output.stdout)
                        );
                    }
                    log::debug!("Command failed, Kitty not ready yet");
                }
                Ok(Err(e)) => {
                    if debug_mode && attempt <= 5 {
                        println!("[DEBUG] Failed to execute test command: {}", e);
                    }
                    log::debug!("Failed to execute test command: {}", e);
                }
                Err(_) => {
                    if debug_mode && attempt <= 5 {
                        println!("[DEBUG] Command timed out after 3 seconds");
                    }
                    log::debug!("Command timed out, Kitty not ready yet");
                }
            }

            // Check socket file status
            if debug_mode && attempt <= 5 {
                println!(
                    "[DEBUG] Socket exists after attempt {}: {}",
                    attempt,
                    self.socket_path.exists()
                );
            }

            sleep(delay).await;
        }

        // Final diagnostics before failing
        if debug_mode {
            println!("[DEBUG] Final diagnostics:");
            println!("[DEBUG] Socket path: {:?}", self.socket_path);
            println!("[DEBUG] Socket exists: {}", self.socket_path.exists());

            // Try to list socket directory
            if let Some(socket_dir) = self.socket_path.parent() {
                println!("[DEBUG] Socket directory: {:?}", socket_dir);
                match std::fs::read_dir(socket_dir) {
                    Ok(entries) => {
                        println!("[DEBUG] Socket directory contents:");
                        for entry in entries.flatten() {
                            println!("[DEBUG]   - {:?}", entry.path());
                        }
                    }
                    Err(e) => println!("[DEBUG] Failed to read socket directory: {}", e),
                }
            }

            // Check if Kitty process is still running
            println!("[DEBUG] Checking if Kitty process is still running...");
            // Note: We can't easily check this without making the field mutable
        }

        Err(format!(
            "Kitty failed to become ready after {} attempts. Socket path: {:?}, Socket exists: {}",
            max_attempts,
            self.socket_path,
            self.socket_path.exists()
        )
        .into())
    }
}

impl Drop for KittyTestHarness {
    /// Ensure cleanup happens even if the user forgets to call cleanup() or if there's a panic.
    fn drop(&mut self) {
        log::debug!("KittyTestHarness dropping, attempting emergency cleanup");

        // Try to kill the process if it's still running
        if let Err(e) = self.process.try_wait() {
            log::warn!("Failed to check process status during drop: {}", e);
        } else {
            match self.process.try_wait() {
                Ok(Some(_)) => {
                    // Process already exited
                    log::debug!("Process already exited during drop");
                }
                Ok(None) => {
                    // Process still running, kill it
                    log::warn!("Force killing Kitty process during drop");
                    if let Err(e) = self.process.kill() {
                        log::error!("Failed to kill process during drop: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("Error checking process status during drop: {}", e);
                }
            }
        }
    }
}

/// Find the test configuration file.
///
/// Looks for `tests/fixtures/kitty.test.conf` relative to the workspace root.
pub fn find_test_config() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Start from current directory and work up to find workspace root
    let mut current_dir = std::env::current_dir()?;

    loop {
        let config_path = current_dir
            .join("tests")
            .join("fixtures")
            .join("kitty.test.conf");
        if config_path.exists() {
            return Ok(config_path);
        }

        // Also check in kitty-lib subdirectory (in case we're running from workspace root)
        let alt_config_path = current_dir
            .join("kitty-lib")
            .join("tests")
            .join("fixtures")
            .join("kitty.test.conf");
        if alt_config_path.exists() {
            return Ok(alt_config_path);
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break,
        }
    }

    Err("Could not find tests/fixtures/kitty.test.conf in workspace".into())
}

/// Set up the screenshot directory for a test.
///
/// Creates a directory structure like `tests/screenshots/test_name/` for organizing
/// screenshots by test case.
///
/// # Arguments
///
/// * `test_name` - Name of the test case
///
/// # Returns
///
/// Path to the created screenshot directory.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn setup_screenshot_directory(test_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Find workspace root by looking for tests directory
    let mut current_dir = std::env::current_dir()?;

    loop {
        let tests_dir = current_dir.join("tests");
        if tests_dir.exists() {
            let screenshots_dir = tests_dir.join("screenshots").join(test_name);
            std::fs::create_dir_all(&screenshots_dir)?;
            return Ok(screenshots_dir);
        }

        // Also check in kitty-lib subdirectory
        let alt_tests_dir = current_dir.join("kitty-lib").join("tests");
        if alt_tests_dir.exists() {
            let screenshots_dir = alt_tests_dir.join("screenshots").join(test_name);
            std::fs::create_dir_all(&screenshots_dir)?;
            return Ok(screenshots_dir);
        }

        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break,
        }
    }

    Err("Could not find tests directory in workspace".into())
}

/// Compare two screenshot images and return a similarity score.
///
/// Uses the `image-compare` crate with the hybrid RGBA comparison algorithm.
/// The similarity score ranges from 0.0 (completely different) to 1.0 (identical).
///
/// # Arguments
///
/// * `actual_path` - Path to the actual screenshot
/// * `expected_path` - Path to the expected (baseline) screenshot
///
/// # Returns
///
/// A similarity score between 0.0 and 1.0, where higher values indicate more similarity.
///
/// # Errors
///
/// Returns an error if:
/// - Either image file cannot be read
/// - Images have different dimensions
/// - Image comparison fails
#[cfg(test)]
pub fn compare_screenshots(
    actual_path: &Path,
    expected_path: &Path,
) -> Result<f64, Box<dyn std::error::Error>> {
    log::debug!(
        "Comparing screenshots: {:?} vs {:?}",
        actual_path,
        expected_path
    );

    // Load both images
    let actual_img = image::open(actual_path)
        .map_err(|e| format!("Failed to open actual image at {:?}: {}", actual_path, e))?;
    let expected_img = image::open(expected_path).map_err(|e| {
        format!(
            "Failed to open expected image at {:?}: {}",
            expected_path, e
        )
    })?;

    // Convert to RGBA format
    let actual_rgba = actual_img.to_rgba8();
    let expected_rgba = expected_img.to_rgba8();

    // Check dimensions match
    if actual_rgba.dimensions() != expected_rgba.dimensions() {
        return Err(format!(
            "Image dimensions don't match: actual {:?} vs expected {:?}",
            actual_rgba.dimensions(),
            expected_rgba.dimensions()
        )
        .into());
    }

    // Perform comparison
    let result = rgba_hybrid_compare(&expected_rgba, &actual_rgba)
        .map_err(|e| format!("Image comparison failed: {}", e))?;

    let similarity = result.score;
    log::debug!("Screenshot comparison score: {:.4}", similarity);

    // Generate diff image if similarity is low
    if similarity < 0.95 {
        if let Err(e) = generate_diff_image(&actual_rgba, &expected_rgba, actual_path) {
            log::warn!("Failed to generate diff image: {}", e);
        }
    }

    Ok(similarity)
}

/// Generate a diff image showing differences between actual and expected screenshots.
///
/// The diff image is saved next to the actual image with a `.diff.png` extension.
///
/// # Arguments
///
/// * `actual` - The actual screenshot as RGBA image buffer
/// * `expected` - The expected screenshot as RGBA image buffer
/// * `actual_path` - Path where the actual screenshot is stored
///
/// # Returns
///
/// Returns Ok(()) if the diff image is generated successfully.
///
/// # Errors
///
/// Returns an error if the diff image cannot be saved.
#[cfg(test)]
fn generate_diff_image(
    actual: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    expected: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    actual_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let (width, height) = actual.dimensions();
    let mut diff_img = ImageBuffer::new(width, height);

    // Create a diff by highlighting differences in red
    for (x, y, pixel) in diff_img.enumerate_pixels_mut() {
        let actual_pixel = actual.get_pixel(x, y);
        let expected_pixel = expected.get_pixel(x, y);

        if actual_pixel == expected_pixel {
            // Same pixel - keep original
            *pixel = *actual_pixel;
        } else {
            // Different pixel - highlight in red
            *pixel = Rgba([255, 0, 0, 255]);
        }
    }

    // Save diff image
    let diff_path = actual_path.with_extension("diff.png");
    diff_img
        .save(&diff_path)
        .map_err(|e| format!("Failed to save diff image to {:?}: {}", diff_path, e))?;

    log::info!("Generated diff image: {:?}", diff_path);
    Ok(())
}

/// Assert that two screenshots are similar within a threshold.
///
/// This is a convenience function for tests that compares two screenshots
/// and panics with a descriptive message if they are not similar enough.
///
/// # Arguments
///
/// * `actual_path` - Path to the actual screenshot
/// * `expected_path` - Path to the expected screenshot
/// * `threshold` - Minimum similarity score (default: 0.95)
/// * `message` - Custom assertion message
///
/// # Panics
///
/// Panics if the similarity score is below the threshold.
#[cfg(test)]
#[allow(dead_code)]
pub fn assert_screenshots_similar(
    actual_path: &Path,
    expected_path: &Path,
    threshold: f64,
    message: &str,
) {
    match compare_screenshots(actual_path, expected_path) {
        Ok(similarity) => {
            assert!(
                similarity >= threshold,
                "{}: Screenshot similarity {:.4} is below threshold {:.4}. Actual: {:?}, Expected: {:?}",
                message,
                similarity,
                threshold,
                actual_path,
                expected_path
            );
        }
        Err(e) => {
            panic!("{}: Screenshot comparison failed: {}", message, e);
        }
    }
}

/// Test environment setup and management for integration tests.
///
/// This module provides utilities for creating temporary test projects,
/// initializing git repositories, setting up configuration files, and
/// managing test environments that can be cleaned up after tests.
pub struct TestEnvironment {
    /// Base directory for all test projects (typically /tmp/ksm-test-env-<uuid>)
    pub base_dir: TempDir,
    /// Paths to created project directories
    pub project_dirs: Vec<PathBuf>,
    /// Paths to created git repositories
    pub git_repos: Vec<PathBuf>,
    /// Path to the test configuration file
    pub config_file: Option<PathBuf>,
    /// Environment ID for debugging
    pub env_id: String,
}

impl TestEnvironment {
    /// Create a new test environment with the specified projects.
    ///
    /// # Arguments
    ///
    /// * `project_names` - Names of projects to create (e.g., ["project-a", "project-b"])
    /// * `with_git` - Whether to initialize git repositories in the projects
    ///
    /// # Returns
    ///
    /// A `TestEnvironment` instance with created projects and repositories.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create temporary directory
    /// - Failed to create project directories
    /// - Failed to initialize git repositories
    pub async fn new(
        project_names: &[&str],
        with_git: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let env_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        log::info!("Creating test environment: {}", env_id);

        // Create base temporary directory
        let base_dir = TempDir::new().map_err(|e| {
            format!(
                "Failed to create temporary directory for test environment: {}",
                e
            )
        })?;

        log::debug!("Test environment base directory: {:?}", base_dir.path());

        let mut test_env = Self {
            base_dir,
            project_dirs: Vec::new(),
            git_repos: Vec::new(),
            config_file: None,
            env_id,
        };

        // Create project directories
        for &project_name in project_names {
            test_env
                .create_project_directory(project_name, with_git)
                .await?;
        }

        log::info!(
            "Test environment {} created with {} projects",
            test_env.env_id,
            test_env.project_dirs.len()
        );

        Ok(test_env)
    }

    /// Create a project directory with optional git initialization.
    ///
    /// # Arguments
    ///
    /// * `project_name` - Name of the project directory
    /// * `with_git` - Whether to initialize a git repository
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation or git initialization fails.
    pub async fn create_project_directory(
        &mut self,
        project_name: &str,
        with_git: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_path = self.base_dir.path().join(project_name);

        log::debug!("Creating project directory: {:?}", project_path);
        std::fs::create_dir_all(&project_path).map_err(|e| {
            format!(
                "Failed to create project directory {:?}: {}",
                project_path, e
            )
        })?;

        // Create some basic project structure
        self.create_project_files(&project_path, project_name)
            .await?;

        self.project_dirs.push(project_path.clone());

        if with_git {
            self.initialize_git_repository(&project_path).await?;
            self.git_repos.push(project_path);
        }

        Ok(())
    }

    /// Create basic project files to simulate a real project structure.
    ///
    /// # Arguments
    ///
    /// * `project_path` - Path to the project directory
    /// * `project_name` - Name of the project for file content
    ///
    /// # Errors
    ///
    /// Returns an error if file creation fails.
    async fn create_project_files(
        &self,
        project_path: &Path,
        project_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a README file
        let readme_path = project_path.join("README.md");
        let readme_content = format!(
            "# {}\n\nThis is a test project created for KSM integration testing.\n\nProject ID: {}\n",
            project_name, self.env_id
        );
        std::fs::write(&readme_path, readme_content)
            .map_err(|e| format!("Failed to create README.md: {}", e))?;

        // Create a src directory with a main file
        let src_dir = project_path.join("src");
        std::fs::create_dir_all(&src_dir)?;

        let main_file = src_dir.join("main.rs");
        let main_content = format!(
            "// Test project: {}\n// Environment: {}\n\nfn main() {{\n    println!(\"Hello from {}!\");\n}}\n",
            project_name, self.env_id, project_name
        );
        std::fs::write(&main_file, main_content)?;

        // Create a Cargo.toml for Rust projects
        let cargo_toml = project_path.join("Cargo.toml");
        let cargo_content = format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
            project_name.replace('-', "_")
        );
        std::fs::write(&cargo_toml, cargo_content)?;

        log::debug!("Created project files for {}", project_name);
        Ok(())
    }

    /// Initialize a git repository in the specified directory.
    ///
    /// Creates a git repository with an initial commit to simulate
    /// a real VCS-managed project.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path where to initialize the git repository
    ///
    /// # Errors
    ///
    /// Returns an error if git commands fail.
    pub async fn initialize_git_repository(
        &self,
        repo_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("Initializing git repository at: {:?}", repo_path);

        // Initialize git repository
        let init_output = tokio::process::Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .await?;

        if !init_output.status.success() {
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            return Err(format!("Git init failed: {}", stderr).into());
        }

        // Configure git user for this repository
        tokio::process::Command::new("git")
            .args(["config", "user.name", "KSM Test"])
            .current_dir(repo_path)
            .output()
            .await?;

        tokio::process::Command::new("git")
            .args(["config", "user.email", "test@ksm.example.com"])
            .current_dir(repo_path)
            .output()
            .await?;

        // Add all files to git
        let add_output = tokio::process::Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(repo_path)
            .output()
            .await?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(format!("Git add failed: {}", stderr).into());
        }

        // Create initial commit
        let commit_message = format!("Initial commit for test project (env: {})", self.env_id);
        let commit_output = tokio::process::Command::new("git")
            .args(["commit", "-m", &commit_message])
            .current_dir(repo_path)
            .output()
            .await?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            return Err(format!("Git commit failed: {}", stderr).into());
        }

        // Create a few more commits to simulate development history
        self.create_development_history(repo_path).await?;

        log::info!("Git repository initialized at: {:?}", repo_path);
        Ok(())
    }

    /// Create some development history in the git repository.
    ///
    /// Adds a few commits to simulate a realistic project history.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the git repository
    ///
    /// # Errors
    ///
    /// Returns an error if git operations fail.
    async fn create_development_history(
        &self,
        repo_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Add a .gitignore file
        let gitignore_path = repo_path.join(".gitignore");
        let gitignore_content = "target/\n*.log\n*.tmp\n.DS_Store\n";
        std::fs::write(&gitignore_path, gitignore_content)?;

        tokio::process::Command::new("git")
            .args(["add", ".gitignore"])
            .current_dir(repo_path)
            .output()
            .await?;

        tokio::process::Command::new("git")
            .args(["commit", "-m", "Add .gitignore"])
            .current_dir(repo_path)
            .output()
            .await?;

        // Modify the main file
        let main_file = repo_path.join("src").join("main.rs");
        let mut content = std::fs::read_to_string(&main_file)?;
        content.push_str("\n// Updated for testing\n");
        std::fs::write(&main_file, content)?;

        tokio::process::Command::new("git")
            .args(["add", "src/main.rs"])
            .current_dir(repo_path)
            .output()
            .await?;

        tokio::process::Command::new("git")
            .args(["commit", "-m", "Update main.rs"])
            .current_dir(repo_path)
            .output()
            .await?;

        log::debug!("Created development history for repository");
        Ok(())
    }

    /// Create a KSM test configuration file for this test environment.
    ///
    /// # Arguments
    ///
    /// * `additional_config` - Optional additional configuration to merge
    ///
    /// # Returns
    ///
    /// Path to the created configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration file creation fails.
    pub async fn create_ksm_config(
        &mut self,
        additional_config: Option<&str>,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_path = self.base_dir.path().join("ksm_test_config.toml");

        log::debug!("Creating KSM test config: {:?}", config_path);

        // Build the configuration content
        let mut config_content =
            format!("# KSM Test Configuration - Environment: {}\n", self.env_id);
        config_content.push_str("# Generated by test environment setup\n\n");
        config_content.push_str("[global]\nversion = \"1.0\"\n\n");

        // Add search directories
        config_content.push_str("[search]\n");
        config_content.push_str(&format!(
            "dirs = [\"{}/\"]\n",
            self.base_dir.path().display()
        ));
        config_content.push_str("vcs = []\n\n");

        // Add project configurations
        config_content.push_str("[projects]\n");
        for (index, project_dir) in self.project_dirs.iter().enumerate() {
            if let Some(_project_name) = project_dir.file_name().and_then(|n| n.to_str()) {
                config_content.push_str(&format!(
                    "test_project_{} = \"{}\"\n",
                    index + 1,
                    project_dir.display()
                ));
            }
        }
        config_content.push('\n');

        // Add key mappings
        config_content.push_str("[keys]\n");
        for (index, _) in self.project_dirs.iter().enumerate() {
            config_content.push_str(&format!(
                "P{} = \"test_project_{}\"\n",
                index + 1,
                index + 1
            ));
        }
        config_content.push('\n');

        // Add session configuration
        config_content.push_str("[session]\n\n");
        config_content.push_str("[session.navigation]\nwrap_tabs = true\n\n");
        config_content.push_str("[session.unnamed_session]\n");
        config_content.push_str("treat_as_session = false\n");
        config_content.push_str("enable_navigation = true\n");

        // Add any additional configuration
        if let Some(additional) = additional_config {
            config_content.push_str("\n# Additional configuration\n");
            config_content.push_str(additional);
        }

        std::fs::write(&config_path, config_content)
            .map_err(|e| format!("Failed to create KSM config file: {}", e))?;

        self.config_file = Some(config_path.clone());
        log::info!("KSM test configuration created: {:?}", config_path);
        Ok(config_path)
    }

    /// Get the path to a specific project directory.
    ///
    /// # Arguments
    ///
    /// * `project_name` - Name of the project to find
    ///
    /// # Returns
    ///
    /// Optional path to the project directory if found.
    pub fn get_project_path(&self, project_name: &str) -> Option<&PathBuf> {
        self.project_dirs.iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == project_name)
                .unwrap_or(false)
        })
    }

    /// Get all project paths.
    pub fn get_all_project_paths(&self) -> &[PathBuf] {
        &self.project_dirs
    }

    /// Get all git repository paths.
    pub fn get_all_git_repos(&self) -> &[PathBuf] {
        &self.git_repos
    }

    /// Get the base directory path.
    pub fn get_base_dir(&self) -> &Path {
        self.base_dir.path()
    }

    /// Get the environment ID.
    pub fn get_env_id(&self) -> &str {
        &self.env_id
    }

    /// Get the path to the KSM configuration file if created.
    pub fn get_config_file(&self) -> Option<&PathBuf> {
        self.config_file.as_ref()
    }

    /// Create helper scripts for manual test verification.
    ///
    /// Creates shell scripts that can be used to manually verify
    /// the test environment setup.
    ///
    /// # Returns
    ///
    /// Path to the directory containing helper scripts.
    ///
    /// # Errors
    ///
    /// Returns an error if script creation fails.
    pub async fn create_helper_scripts(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let scripts_dir = self.base_dir.path().join("scripts");
        std::fs::create_dir_all(&scripts_dir)?;

        // Create verification script
        let verify_script_path = scripts_dir.join("verify_environment.sh");
        let verify_script_content = format!(
            r##"#!/bin/bash
# Test Environment Verification Script
# Environment ID: {env_id}
# Generated by KSM test environment setup

echo "=== KSM Test Environment Verification ==="
echo "Environment ID: {env_id}"
echo "Base directory: {base_dir}"
echo

echo "=== Project Directories ==="
{project_checks}

echo "=== Git Repositories ==="
{git_checks}

echo "=== Configuration File ==="
{config_check}

echo "=== Environment Variables for Testing ==="
echo "export KSM_TEST_ENV_ID={env_id}"
echo "export KSM_TEST_BASE_DIR={base_dir}"
{env_vars}

echo
echo "=== Manual Test Commands ==="
echo "# List all projects:"
echo "ls -la {base_dir}"
echo
echo "# Check git status in each repository:"
{git_status_commands}
echo
echo "=== Verification Complete ==="
"##,
            env_id = self.env_id,
            base_dir = self.base_dir.path().display(),
            project_checks = self.project_dirs
                .iter()
                .map(|path| format!(
                    "echo \"  - {}\" && test -d \"{}\" && echo \"    ✓ EXISTS\" || echo \"    ✗ MISSING\"",
                    path.file_name().unwrap().to_str().unwrap(),
                    path.display()
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            git_checks = self.git_repos
                .iter()
                .map(|path| format!(
                    "echo \"  - {}\" && test -d \"{}/.git\" && echo \"    ✓ GIT REPO\" || echo \"    ✗ NOT A GIT REPO\"",
                    path.file_name().unwrap().to_str().unwrap(),
                    path.display()
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            config_check = if let Some(config) = &self.config_file {
                format!(
                    "test -f \"{}\" && echo \"  ✓ CONFIG EXISTS: {}\" || echo \"  ✗ CONFIG MISSING\"",
                    config.display(),
                    config.display()
                )
            } else {
                "echo \"  - No configuration file created\"".to_string()
            },
            env_vars = self.project_dirs
                .iter()
                .enumerate()
                .map(|(i, path)| format!(
                    "echo \"export KSM_TEST_PROJECT_{}={}\"",
                    i + 1,
                    path.display()
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            git_status_commands = self.git_repos
                .iter()
                .map(|path| format!("echo \"cd {} && git status\"", path.display()))
                .collect::<Vec<_>>()
                .join("\n"),
        );

        std::fs::write(&verify_script_path, verify_script_content)?;

        // Make the script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&verify_script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&verify_script_path, perms)?;
        }

        // Create cleanup script
        let cleanup_script_path = scripts_dir.join("cleanup_environment.sh");
        let cleanup_script_content = format!(
            r##"#!/bin/bash
# Test Environment Cleanup Script
# Environment ID: {env_id}
# WARNING: This will delete all test data!

echo "=== KSM Test Environment Cleanup ==="
echo "Environment ID: {env_id}"
echo "Base directory: {base_dir}"
echo
echo "WARNING: This will permanently delete all test data!"
read -p "Are you sure? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]
then
    echo "Cleaning up test environment..."
    rm -rf "{base_dir}"
    echo "✓ Test environment cleaned up"
else
    echo "Cleanup cancelled"
fi
"##,
            env_id = self.env_id,
            base_dir = self.base_dir.path().display()
        );

        std::fs::write(&cleanup_script_path, cleanup_script_content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&cleanup_script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&cleanup_script_path, perms)?;
        }

        log::info!("Helper scripts created in: {:?}", scripts_dir);
        Ok(scripts_dir)
    }

    /// Validate the test environment setup.
    ///
    /// Performs comprehensive checks to ensure all components
    /// of the test environment are properly set up.
    ///
    /// # Returns
    ///
    /// A summary of validation results.
    ///
    /// # Errors
    ///
    /// Returns an error if critical validation checks fail.
    pub async fn validate(&self) -> Result<TestEnvironmentValidation, Box<dyn std::error::Error>> {
        let mut validation = TestEnvironmentValidation {
            env_id: self.env_id.clone(),
            base_dir_exists: self.base_dir.path().exists(),
            project_dirs_valid: Vec::new(),
            git_repos_valid: Vec::new(),
            config_file_valid: false,
            total_projects: self.project_dirs.len(),
            total_git_repos: self.git_repos.len(),
            validation_errors: Vec::new(),
        };

        // Validate project directories
        for project_dir in &self.project_dirs {
            let is_valid = project_dir.exists()
                && project_dir.join("README.md").exists()
                && project_dir.join("Cargo.toml").exists()
                && project_dir.join("src").join("main.rs").exists();

            validation.project_dirs_valid.push(is_valid);

            if !is_valid {
                validation.validation_errors.push(format!(
                    "Project directory validation failed: {:?}",
                    project_dir
                ));
            }
        }

        // Validate git repositories
        for git_repo in &self.git_repos {
            let git_dir = git_repo.join(".git");
            let is_valid = git_dir.exists() && git_dir.is_dir();

            validation.git_repos_valid.push(is_valid);

            if !is_valid {
                validation
                    .validation_errors
                    .push(format!("Git repository validation failed: {:?}", git_repo));
            } else {
                // Additional git validation - check for commits
                match tokio::process::Command::new("git")
                    .args(["log", "--oneline", "-n", "1"])
                    .current_dir(git_repo)
                    .output()
                    .await
                {
                    Ok(output) if output.status.success() => {
                        // Git repo has commits
                    }
                    _ => {
                        validation
                            .validation_errors
                            .push(format!("Git repository has no commits: {:?}", git_repo));
                    }
                }
            }
        }

        // Validate configuration file
        if let Some(config_file) = &self.config_file {
            validation.config_file_valid = config_file.exists();
            if !validation.config_file_valid {
                validation
                    .validation_errors
                    .push(format!("Configuration file missing: {:?}", config_file));
            }
        }

        log::info!(
            "Test environment validation completed: {} errors",
            validation.validation_errors.len()
        );
        Ok(validation)
    }
}

/// Validation results for a test environment.
#[derive(Debug)]
pub struct TestEnvironmentValidation {
    pub env_id: String,
    pub base_dir_exists: bool,
    pub project_dirs_valid: Vec<bool>,
    pub git_repos_valid: Vec<bool>,
    pub config_file_valid: bool,
    pub total_projects: usize,
    pub total_git_repos: usize,
    pub validation_errors: Vec<String>,
}

impl TestEnvironmentValidation {
    /// Check if all validations passed.
    pub fn is_valid(&self) -> bool {
        self.validation_errors.is_empty()
            && self.base_dir_exists
            && self.project_dirs_valid.iter().all(|&v| v)
            && self.git_repos_valid.iter().all(|&v| v)
    }

    /// Get a summary of validation results.
    pub fn summary(&self) -> String {
        format!(
            "Test Environment Validation Summary:\n\
             Environment ID: {}\n\
             Base Directory: {}\n\
             Projects: {}/{} valid\n\
             Git Repos: {}/{} valid\n\
             Config File: {}\n\
             Errors: {}\n\
             Overall: {}",
            self.env_id,
            if self.base_dir_exists { "✓" } else { "✗" },
            self.project_dirs_valid.iter().filter(|&&v| v).count(),
            self.total_projects,
            self.git_repos_valid.iter().filter(|&&v| v).count(),
            self.total_git_repos,
            if self.config_file_valid { "✓" } else { "✗" },
            self.validation_errors.len(),
            if self.is_valid() {
                "✓ VALID"
            } else {
                "✗ INVALID"
            }
        )
    }
}
