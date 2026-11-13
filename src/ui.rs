use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use tokio::sync::{Mutex, mpsc};
use unicode_width::UnicodeWidthStr;

use crate::process_manager::{LifecycleOptions, ProcessManager, restart_label, runtime_label};
use crate::types::ProcessError;

const MAX_LOG_LINES: usize = 1000;
const MAX_COMMAND_HISTORY: usize = 100;

// Fallback palette for processes without a manager colour, in ratatui's names
// (magenta excluded - reserved for gaffa).
const PROCESS_COLORS: [Color; 7] = [
    Color::Cyan,
    Color::Yellow,
    Color::Blue,
    Color::Green,
    Color::LightCyan,
    Color::LightYellow,
    Color::LightBlue,
];

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub process: String,
    pub content: String,
    pub is_error: bool,
    /// gaffa's own message rather than a line of process output.
    pub system: bool,
}

pub enum UICommand {
    ExecuteCommand(String),
    Quit,
}

pub struct AppState {
    pub logs: Arc<Mutex<VecDeque<LogEntry>>>,
    pub tx: Arc<Mutex<Option<mpsc::UnboundedSender<LogEntry>>>>,
    pub status_tx: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<String>>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(VecDeque::new())),
            tx: Arc::new(Mutex::new(None)), // No channel initially
            status_tx: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn add_log(&self, process: String, content: String, is_error: bool) {
        self.push(LogEntry {
            timestamp: Instant::now(),
            process,
            content,
            is_error,
            system: false,
        })
        .await;
    }

    pub async fn add_system_log(&self, content: String) {
        self.push(LogEntry {
            timestamp: Instant::now(),
            process: "gaffa".to_string(),
            content,
            is_error: false,
            system: true,
        })
        .await;
    }

    async fn push(&self, entry: LogEntry) {
        let tx = self.tx.lock().await;
        if let Some(tx) = tx.as_ref() {
            let _ = tx.send(entry);
        } else {
            // No consumer yet - buffer directly
            drop(tx);
            let mut logs = self.logs.lock().await;
            logs.push_back(entry);
        }
    }

    pub async fn set_process_status(&self, status_lines: Vec<String>) {
        let status_tx = self.status_tx.lock().await;
        if let Some(tx) = status_tx.as_ref() {
            let _ = tx.send(status_lines);
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

struct UIState {
    logs: VecDeque<LogEntry>,
    input: String,
    cursor_position: usize, // Byte index into `input`, always on a char boundary
    command_history: Vec<String>,
    history_index: Option<usize>,
    process_status: Vec<String>,
    should_quit: bool,
    show_status: bool,
    log_scroll_offset: usize,               // For scrolling through logs
    last_log_count: usize,                  // To detect new logs
    process_colors: HashMap<String, Color>, // Map process names to colors
}

impl UIState {
    fn new() -> Self {
        Self {
            logs: VecDeque::new(),
            input: String::new(),
            cursor_position: 0,
            command_history: Vec::new(),
            history_index: None,
            process_status: Vec::new(),
            should_quit: false,
            show_status: false, // Hidden by default
            log_scroll_offset: 0,
            last_log_count: 0,
            process_colors: HashMap::new(),
        }
    }

    fn get_process_color(
        &mut self,
        process_name: &str,
        manager_color: Option<crate::paint::Color>,
    ) -> Color {
        if process_name == "gaffa" {
            return Color::Magenta;
        }

        // Use the color from ProcessManager if available
        if let Some(color) = manager_color {
            // crossterm's `DarkCyan` is ratatui's `Cyan`, its `Cyan` ratatui's `LightCyan`.
            let ratatui_color = match color {
                crate::paint::Color::DarkCyan => Color::Cyan,
                crate::paint::Color::DarkYellow => Color::Yellow,
                crate::paint::Color::DarkBlue => Color::Blue,
                crate::paint::Color::DarkGreen => Color::Green,
                crate::paint::Color::Cyan => Color::LightCyan,
                crate::paint::Color::Yellow => Color::LightYellow,
                crate::paint::Color::Blue => Color::LightBlue,
                _ => Color::White,
            };
            self.process_colors
                .insert(process_name.to_string(), ratatui_color);
            return ratatui_color;
        }

        // Fallback to sequential assignment if no manager color
        let next_color_index = self.process_colors.len() % PROCESS_COLORS.len();
        *self
            .process_colors
            .entry(process_name.to_string())
            .or_insert(PROCESS_COLORS[next_color_index])
    }

    fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    fn insert_str(&mut self, text: &str) {
        self.input.insert_str(self.cursor_position, text);
        self.cursor_position += text.len();
    }

    /// Remove the character before the cursor; true when something changed.
    fn backspace(&mut self) -> bool {
        if !self.move_left() {
            return false;
        }
        self.input.remove(self.cursor_position);
        true
    }

    /// Remove the character under the cursor; true when something changed.
    fn delete(&mut self) -> bool {
        if self.cursor_position >= self.input.len() {
            return false;
        }
        self.input.remove(self.cursor_position);
        true
    }

    fn move_left(&mut self) -> bool {
        match self.input[..self.cursor_position]
            .char_indices()
            .next_back()
        {
            Some((index, _)) => {
                self.cursor_position = index;
                true
            }
            None => false,
        }
    }

    fn move_right(&mut self) -> bool {
        match self.input[self.cursor_position..].chars().next() {
            Some(c) => {
                self.cursor_position += c.len_utf8();
                true
            }
            None => false,
        }
    }

    fn move_home(&mut self) {
        self.cursor_position = 0;
    }

    fn move_end(&mut self) {
        self.cursor_position = self.input.len();
    }

    /// The screen column of the cursor inside the input box; CJK and emoji take two.
    fn cursor_column(&self) -> usize {
        self.input[..self.cursor_position].width()
    }

    /// Recall the previous command; true when the input changed.
    fn history_up(&mut self) -> bool {
        let index = match self.history_index {
            None if !self.command_history.is_empty() => self.command_history.len() - 1,
            Some(index) if index > 0 => index - 1,
            _ => return false,
        };
        self.history_index = Some(index);
        self.input = self.command_history[index].clone();
        self.move_end();
        true
    }

    /// Recall the next command, or clear the input past the newest; true when the input changed.
    fn history_down(&mut self) -> bool {
        match self.history_index {
            Some(index) if index + 1 < self.command_history.len() => {
                self.history_index = Some(index + 1);
                self.input = self.command_history[index + 1].clone();
            }
            Some(_) => {
                self.history_index = None;
                self.input.clear();
            }
            None => return false,
        }
        self.move_end();
        true
    }
}

#[allow(clippy::too_many_lines)]
pub async fn run_terminal_ui(
    manager: Arc<ProcessManager>,
    state: Arc<AppState>,
    processes_to_start: Vec<String>,
    procfile_path: &str,
) -> Result<(), ProcessError> {
    // Create channels
    let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UICommand>();
    let (log_tx, log_rx) = mpsc::unbounded_channel::<LogEntry>();
    let (status_tx, status_rx) = mpsc::unbounded_channel::<Vec<String>>();
    let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel::<()>();

    // Update the app state's channels
    {
        let mut tx = state.tx.lock().await;
        *tx = Some(log_tx.clone());

        let mut stx = state.status_tx.lock().await;
        *stx = Some(status_tx.clone());
    }

    let manager_for_status = Arc::clone(&manager);
    let status_tx_clone = status_tx.clone();
    let status_handle = tokio::spawn(async move {
        // Update status more frequently for real-time feel
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let runtime = manager_for_status.runtime.lock().await;
            let mut status_lines = vec![];

            for (name, info) in runtime.processes.iter() {
                status_lines.push(format!(
                    "{:>12} | {:>12} | {:>22} | {:>12}",
                    name,
                    format!("{:?}", info.status),
                    restart_label(info.restart_count),
                    runtime_label(info)
                ));
            }

            let _ = status_tx_clone.send(status_lines);
        }
    });

    // Clone for various tasks
    let manager_for_commands = Arc::clone(&manager);
    let state_for_commands = Arc::clone(&state);

    let log_file = manager.config.lock().await.log_file.clone();
    let log_handle = tokio::spawn(collect_logs(log_rx, Arc::clone(&state.logs), log_file));

    // Spawn the UI in a separate task that can block
    let logs_clone = Arc::clone(&state.logs);
    let manager_for_ui = Arc::clone(&manager);
    let ui_handle = tokio::task::spawn_blocking(move || {
        run_ui_loop(ui_tx, logs_clone, status_rx, shutdown_rx, manager_for_ui)
    });

    // Give the UI a moment to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Now log the startup banner and start processes
    let state_for_startup = Arc::clone(&state);
    let manager_for_startup = Arc::clone(&manager);
    let procfile_path_owned = procfile_path.to_string();
    tokio::spawn(async move {
        state_for_startup
            .add_system_log(format!(
                "Loading {} processes from {}",
                processes_to_start.len(),
                procfile_path_owned
            ))
            .await;

        // Start the specified processes with UI state
        for process_name in &processes_to_start {
            match manager_for_startup
                .start_process_with_opts(
                    process_name,
                    &LifecycleOptions::with_ui(state_for_startup.clone()),
                )
                .await
            {
                Ok(()) => {
                    state_for_startup
                        .add_system_log(format!("Starting process '{process_name}'"))
                        .await;
                }
                Err(e) => {
                    state_for_startup
                        .add_system_log(format!("Failed to start '{process_name}': {e}"))
                        .await;
                }
            }
        }
    });

    // Handle commands in the async context
    let command_handle = tokio::spawn(async move {
        while let Some(cmd) = ui_rx.recv().await {
            match cmd {
                UICommand::ExecuteCommand(input) => {
                    match manager_for_commands
                        .handle_command_with_opts(
                            &input,
                            &LifecycleOptions::with_ui(state_for_commands.clone()),
                        )
                        .await
                    {
                        Ok(()) => {}
                        Err(e) => {
                            state_for_commands
                                .add_system_log(format!("Error: {e}"))
                                .await;
                        }
                    }
                }
                UICommand::Quit => {
                    // stop_all_with_opts will log the shutdown message
                    manager_for_commands
                        .stop_all_with_opts(&LifecycleOptions::with_ui(state_for_commands.clone()))
                        .await;

                    // Fix any processes that were terminated but status wasn't updated
                    manager_for_commands.fix_orphaned_process_status().await;

                    // Wait for all processes to actually stop
                    let timeout = std::time::Instant::now() + std::time::Duration::from_secs(10);
                    loop {
                        let all_stopped =
                            manager_for_commands.all_started_processes_stopped().await;

                        if all_stopped {
                            state_for_commands
                                .add_system_log("All processes stopped. Exiting...".to_string())
                                .await;
                            // Give UI time to show the message
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                            // Send one final message to signal complete shutdown
                            state_for_commands
                                .add_system_log("Shutdown complete.".to_string())
                                .await;
                            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                            // Signal UI to exit
                            let _ = shutdown_tx.send(());
                            break; // Exit immediately after showing messages
                        }

                        if std::time::Instant::now() > timeout {
                            state_for_commands
                                .add_system_log(
                                    "Timeout waiting for processes to stop. Forcing exit..."
                                        .to_string(),
                                )
                                .await;
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            // Signal UI to exit even on timeout
                            let _ = shutdown_tx.send(());
                            break;
                        }

                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    break;
                }
            }
        }
    });

    let ui_result = wait_for_ui(ui_handle, command_handle, tokio::signal::ctrl_c()).await;

    // Clean up
    status_handle.abort();
    log_handle.abort();

    // Cleanup terminal immediately
    cleanup_terminal();

    // Ensure output is flushed and terminal is ready for normal output
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    // Add a delay to ensure terminal state is fully restored
    std::thread::sleep(std::time::Duration::from_millis(200));

    ui_result
}

/// Wait for the UI, the command handler or an interrupt, whichever ends first.
/// A UI error is the caller's to report; a UI panic goes on unwinding.
async fn wait_for_ui(
    ui_handle: tokio::task::JoinHandle<Result<(), ProcessError>>,
    command_handle: tokio::task::JoinHandle<()>,
    interrupt: impl std::future::Future<Output = std::io::Result<()>>,
) -> Result<(), ProcessError> {
    tokio::select! {
        joined = ui_handle => joined.unwrap_or_else(|e| std::panic::resume_unwind(e.into_panic())),
        _ = command_handle => Ok(()),
        // No stop_all here: the UI stops the processes through UICommand::Quit
        _ = interrupt => Ok(()),
    }
}

/// Move log entries into the buffer the UI loop reads, and gaffa's own messages
/// into the log file — the manager writes process output there itself.
async fn collect_logs(
    mut log_rx: mpsc::UnboundedReceiver<LogEntry>,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    log_file: Option<Arc<Mutex<std::fs::File>>>,
) {
    use std::io::Write;

    while let Some(entry) = log_rx.recv().await {
        if let Some(log_file) = log_file.as_ref().filter(|_| entry.system) {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let clean = crate::output::strip_ansi_escapes(&entry.content);
            let log_line = format!("[{timestamp}] [{}] {clean}\n", entry.process);
            let mut file = log_file.lock().await;
            let _ = file.write_all(log_line.as_bytes());
            let _ = file.flush();
        }

        let mut logs = logs.lock().await;
        logs.push_back(entry);
        if logs.len() > MAX_LOG_LINES {
            logs.pop_front();
        }
    }
}

/// Restore the terminal: cursor, raw mode, alternate screen, mouse, colours.
fn cleanup_terminal() {
    use std::io::{Write, stderr, stdout};

    // First, ensure we show the cursor
    let _ = execute!(stdout(), crossterm::cursor::Show);

    // Disable raw mode - this is critical for restoring terminal
    let _ = disable_raw_mode();

    // Leave alternate screen and disable mouse capture
    let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture,);

    // Reset all text attributes to default
    let _ = execute!(
        stdout(),
        crossterm::style::ResetColor,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::FromCursorDown),
    );

    // Force flush both stdout and stderr
    let _ = stdout().flush();
    let _ = stderr().flush();

    // Give the terminal time to process all the commands
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Platform-specific terminal restoration
    #[cfg(unix)]
    {
        // On Unix, we might need to reset terminal attributes
        use std::process::Command;
        let _ = Command::new("stty").arg("sane").status();
    }

    #[cfg(windows)]
    {
        // On Windows, reset console mode
        use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
        use winapi::um::processenv::GetStdHandle;
        use winapi::um::winbase::{STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};
        use winapi::um::wincon::{
            ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT, ENABLE_PROCESSED_OUTPUT,
            ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WRAP_AT_EOL_OUTPUT,
        };

        unsafe {
            // Reset input handle
            let input_handle = GetStdHandle(STD_INPUT_HANDLE);
            if input_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let input_mode = ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT;
                SetConsoleMode(input_handle, input_mode);
            }

            // Reset output handle
            let output_handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if output_handle != winapi::um::handleapi::INVALID_HANDLE_VALUE {
                let mut current_mode: u32 = 0;
                if GetConsoleMode(output_handle, &mut current_mode) != 0 {
                    let output_mode = ENABLE_PROCESSED_OUTPUT
                        | ENABLE_WRAP_AT_EOL_OUTPUT
                        | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                    SetConsoleMode(output_handle, output_mode);
                }
            }
        }
    }
}

fn run_ui_loop(
    tx: mpsc::UnboundedSender<UICommand>,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    status_rx: mpsc::UnboundedReceiver<Vec<String>>,
    shutdown_rx: mpsc::UnboundedReceiver<()>,
    manager: Arc<ProcessManager>,
) -> Result<(), ProcessError> {
    let original_hook = install_cleanup_panic_hook();

    // Setup terminal
    let setup_result = (|| -> Result<(), ProcessError> {
        enable_raw_mode().map_err(ProcessError::InputRead)?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen).map_err(ProcessError::InputRead)?;
        execute!(stdout, EnableMouseCapture).map_err(ProcessError::InputRead)?;
        Ok(())
    })();

    if let Err(e) = setup_result {
        cleanup_terminal();
        restore_panic_hook(original_hook);
        return Err(e);
    }

    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            cleanup_terminal();
            restore_panic_hook(original_hook);
            return Err(ProcessError::InputRead(e));
        }
    };

    let mut ui_state = UIState::new();
    let res = run_app(
        &mut terminal,
        &mut ui_state,
        tx,
        logs.clone(),
        status_rx,
        shutdown_rx,
        manager,
    );

    // Don't show summary here - it will be shown after terminal is restored

    // Always restore terminal, even on error
    cleanup_terminal();
    restore_panic_hook(original_hook);

    res
}

type PanicHook = Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Sync + Send>;

/// Put a terminal cleanup in front of the current panic hook, and hand back that hook.
fn install_cleanup_panic_hook() -> Arc<PanicHook> {
    let original: Arc<PanicHook> = Arc::new(std::panic::take_hook());
    let chained = Arc::clone(&original);
    std::panic::set_hook(Box::new(move |info| {
        cleanup_terminal();
        chained(info);
    }));
    original
}

/// Reinstate the panic hook that was in place before `install_cleanup_panic_hook`.
fn restore_panic_hook(original: Arc<PanicHook>) {
    std::panic::set_hook(Box::new(move |info| original(info)));
}

#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    ui_state: &mut UIState,
    tx: mpsc::UnboundedSender<UICommand>,
    logs: Arc<Mutex<VecDeque<LogEntry>>>,
    mut status_rx: mpsc::UnboundedReceiver<Vec<String>>,
    mut shutdown_rx: mpsc::UnboundedReceiver<()>,
    manager: Arc<ProcessManager>,
) -> Result<(), ProcessError> {
    let mut needs_redraw = true;
    let mut last_redraw = std::time::Instant::now();

    loop {
        // Process all available events without blocking
        while event::poll(std::time::Duration::from_millis(0)).map_err(ProcessError::InputRead)? {
            match event::read().map_err(ProcessError::InputRead)? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            ui_state.should_quit = true;
                            let _ = tx.send(UICommand::Quit);
                            // Don't return immediately - let the command handler finish
                        }
                        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Paste from clipboard
                            if let Ok(text) = cli_clipboard::get_contents() {
                                ui_state.insert_str(&text);
                                needs_redraw = true;
                            }
                        }
                        KeyCode::Char('t') if ui_state.input.is_empty() => {
                            // Toggle status window when 't' is pressed with empty input
                            ui_state.show_status = !ui_state.show_status;
                            needs_redraw = true;
                        }
                        KeyCode::Char(c) => {
                            ui_state.insert_char(c);
                            needs_redraw = true;
                        }
                        KeyCode::Backspace => needs_redraw |= ui_state.backspace(),
                        KeyCode::Delete => needs_redraw |= ui_state.delete(),
                        KeyCode::Enter => {
                            let input = ui_state.input.trim().to_string();
                            if !input.is_empty() {
                                ui_state.command_history.push(input.clone());
                                // Limit command history size
                                if ui_state.command_history.len() > MAX_COMMAND_HISTORY {
                                    ui_state.command_history.remove(0);
                                }
                                ui_state.history_index = None;

                                if input == "q" || input == "quit" {
                                    ui_state.should_quit = true;
                                    let _ = tx.send(UICommand::Quit);
                                    // Don't return immediately - let the UI show teardown messages
                                } else {
                                    let _ = tx.send(UICommand::ExecuteCommand(input));
                                }

                                ui_state.input.clear();
                                ui_state.cursor_position = 0;
                                needs_redraw = true;
                            }
                        }
                        KeyCode::Left => needs_redraw |= ui_state.move_left(),
                        KeyCode::Right => needs_redraw |= ui_state.move_right(),
                        KeyCode::Home if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            ui_state.move_home();
                            needs_redraw = true;
                        }
                        KeyCode::End if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            ui_state.move_end();
                            needs_redraw = true;
                        }
                        KeyCode::Up => needs_redraw |= ui_state.history_up(),
                        KeyCode::Down => needs_redraw |= ui_state.history_down(),
                        KeyCode::PageUp => {
                            // Scroll up by one page
                            let page_size = 10;
                            ui_state.log_scroll_offset =
                                ui_state.log_scroll_offset.saturating_add(page_size);
                            // Cap at maximum
                            let max_scroll = ui_state.logs.len().saturating_sub(5);
                            if ui_state.log_scroll_offset > max_scroll {
                                ui_state.log_scroll_offset = max_scroll;
                            }
                            needs_redraw = true;
                        }
                        KeyCode::PageDown => {
                            // Scroll down by one page
                            let page_size = 10;
                            ui_state.log_scroll_offset =
                                ui_state.log_scroll_offset.saturating_sub(page_size);
                            needs_redraw = true;
                        }
                        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Jump to beginning of logs
                            let max_scroll = ui_state.logs.len().saturating_sub(5);
                            ui_state.log_scroll_offset = max_scroll;
                            needs_redraw = true;
                        }
                        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Jump to end of logs (latest)
                            ui_state.log_scroll_offset = 0;
                            needs_redraw = true;
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            ui_state.log_scroll_offset =
                                ui_state.log_scroll_offset.saturating_add(8);
                            let max_scroll = ui_state.logs.len().saturating_sub(5);
                            if ui_state.log_scroll_offset > max_scroll {
                                ui_state.log_scroll_offset = max_scroll;
                            }
                            needs_redraw = true;
                        }
                        MouseEventKind::ScrollDown => {
                            ui_state.log_scroll_offset =
                                ui_state.log_scroll_offset.saturating_sub(8);
                            needs_redraw = true;
                        }
                        _ => {
                            // Ignore other mouse events to allow text selection
                        }
                    }
                }
                _ => {}
            }
        }

        let mut should_redraw = false;

        // Update UI state with latest logs
        if let Ok(current_logs) = logs.try_lock() {
            let new_count = current_logs.len();

            // Only update if there are changes
            if new_count != ui_state.last_log_count {
                // More efficient: only clone new logs instead of all logs
                let logs_to_add = new_count.saturating_sub(ui_state.last_log_count);
                if logs_to_add > 0 && ui_state.logs.is_empty() {
                    // Initial population
                    ui_state.logs.extend(current_logs.iter().cloned());
                } else if logs_to_add > 0 {
                    // Add only new logs
                    ui_state
                        .logs
                        .extend(current_logs.iter().skip(ui_state.last_log_count).cloned());
                    // Limit log buffer size to prevent unbounded growth
                    const MAX_LOGS: usize = 10000;
                    while ui_state.logs.len() > MAX_LOGS {
                        ui_state.logs.pop_front();
                    }
                }
                should_redraw = true;

                // Sync colors from ProcessManager
                if let Some(manager_colors) = manager.try_get_colors() {
                    let processes: Vec<(String, Option<crate::paint::Color>)> = ui_state
                        .logs
                        .iter()
                        .map(|log| {
                            (
                                log.process.clone(),
                                manager_colors.get(&log.process).copied(),
                            )
                        })
                        .collect();

                    for (process, color) in processes {
                        ui_state.get_process_color(&process, color);
                    }
                }

                // Handle scroll offset when new logs arrive
                if new_count > ui_state.last_log_count {
                    let new_logs = new_count - ui_state.last_log_count;

                    if ui_state.log_scroll_offset == 0 {
                        // User is at the bottom, keep following
                    } else if ui_state.log_scroll_offset < 5 {
                        // User scrolled just a bit, reset to follow
                        ui_state.log_scroll_offset = 0;
                    } else {
                        // User has scrolled up significantly, maintain their position
                        // by increasing the offset by the number of new logs
                        ui_state.log_scroll_offset += new_logs;
                    }
                }

                ui_state.last_log_count = new_count;
            }
        }

        // Check for status updates - drain all to get latest
        let mut latest_status = None;
        while let Ok(status_lines) = status_rx.try_recv() {
            latest_status = Some(status_lines);
        }
        if let Some(status) = latest_status {
            ui_state.process_status = status;
            should_redraw = true;
        }

        // Check for shutdown signal
        if let Ok(()) = shutdown_rx.try_recv() {
            break Ok(());
        }

        // If quitting, just continue showing UI until command handler finishes
        if ui_state.should_quit {
            // The command handler is taking care of waiting for processes to stop
            // Add a small delay to avoid busy-waiting and let other tasks run
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        // Force redraw for time updates when status is visible
        if ui_state.show_status && last_redraw.elapsed() > std::time::Duration::from_millis(100) {
            should_redraw = true;
        }

        // Only redraw when necessary
        if needs_redraw || should_redraw {
            terminal
                .draw(|f| ui(f, ui_state))
                .map_err(ProcessError::InputRead)?;
            needs_redraw = false;
            last_redraw = std::time::Instant::now();
        }

        // Add a small sleep to prevent CPU spinning
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn ui(f: &mut Frame, ui_state: &mut UIState) {
    let chunks = if ui_state.show_status {
        // Calculate dynamic height for status panel based on number of processes
        // Add 2 for borders + 2 for header and separator
        let status_height =
            u16::try_from((ui_state.process_status.len() + 4).min(17)).unwrap_or(17); // Cap at 17 lines

        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),               // Process output area
                Constraint::Length(status_height), // Process status area (dynamic)
                Constraint::Length(3),             // Input area
                Constraint::Length(1),             // Help bar
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),   // Process output area (expanded)
                Constraint::Length(3), // Input area
                Constraint::Length(1), // Help bar
            ])
            .split(f.area())
    };

    // Process output area
    render_logs(f, chunks[0], ui_state);

    // Process status area (only if visible)
    if ui_state.show_status {
        render_process_status(f, chunks[1], ui_state);
        render_input_with_help(f, chunks[2], ui_state);
        render_help_bar(f, chunks[3]);
    } else {
        render_input_with_help(f, chunks[1], ui_state);
        render_help_bar(f, chunks[2]);
    }
}

fn render_logs(f: &mut Frame, area: Rect, ui_state: &mut UIState) {
    let visible_height = area.height.saturating_sub(2) as usize; // -2 for borders
    let total_logs = ui_state.logs.len();

    // Calculate start index based on scroll offset
    let start_idx = if total_logs > visible_height {
        // When scrolling, offset from the end
        (total_logs - visible_height).saturating_sub(ui_state.log_scroll_offset)
    } else {
        0
    };

    // Pre-compute colors for all unique process names to avoid borrow issues
    let unique_processes: Vec<String> = ui_state
        .logs
        .iter()
        .map(|entry| entry.process.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for process in unique_processes {
        ui_state.get_process_color(&process, None);
    }

    let process_colors = ui_state.process_colors.clone();

    let log_items: Vec<ListItem> = ui_state
        .logs
        .iter()
        .skip(start_idx)
        .take(visible_height)
        .map(|entry| {
            use ratatui::text::{Line, Span};

            let process_color = if entry.process == "gaffa" {
                Color::Magenta
            } else {
                process_colors
                    .get(&entry.process)
                    .copied()
                    .unwrap_or(Color::Green)
            };

            // Create a line with colored process name and appropriately colored content
            let line = if entry.process == "gaffa" {
                // For gaffa messages, color the entire line magenta
                Line::from(vec![Span::styled(
                    format!("{:>12} | {}", entry.process, entry.content),
                    Style::default().fg(Color::Magenta),
                )])
            } else {
                // For process output, only color the process name
                Line::from(vec![
                    Span::styled(
                        format!("{:>12}", entry.process),
                        Style::default().fg(process_color),
                    ),
                    Span::raw(" | "),
                    Span::raw(&entry.content),
                ])
            };

            ListItem::new(line)
        })
        .collect();

    // Create title with scroll indicator
    let title = if ui_state.log_scroll_offset > 0 {
        // Calculate which lines are being shown based on actual display
        let actual_start = start_idx + 1; // Convert to 1-based for display
        let actual_end = (start_idx + visible_height).min(total_logs);
        format!("Output (showing {actual_start}-{actual_end} of {total_logs} lines) [Scrolled]")
    } else {
        format!("Output ({total_logs} lines)")
    };

    let logs_list = List::new(log_items).block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(logs_list, area);
}

fn render_process_status(f: &mut Frame, area: Rect, ui_state: &UIState) {
    use ratatui::text::Span;

    let mut lines: Vec<Line> = vec![];

    // Add process status lines
    for line in &ui_state.process_status {
        // Parse the status line to colorize it
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let name = parts[0].trim();
            let status = parts[1].trim();
            let restarts = parts[2].trim();
            let uptime = parts[3].trim();

            // Choose color based on status
            let status_color = if status.contains("Running") {
                Color::Green
            } else if status.contains("Stopped") {
                Color::Yellow
            } else if status.contains("Restarting") {
                Color::Blue
            } else {
                Color::White
            };

            // Get the actual process color from the map
            let name_color = ui_state
                .process_colors
                .get(name)
                .copied()
                .unwrap_or(Color::Cyan);

            lines.push(Line::from(vec![
                Span::styled(format!("{name:>12}"), Style::default().fg(name_color)),
                Span::raw(" | "),
                Span::styled(format!("{status:>12}"), Style::default().fg(status_color)),
                Span::raw(" | "),
                Span::styled(format!("{restarts:>22}"), Style::default().fg(Color::White)),
                Span::raw(" | "),
                Span::styled(format!("{uptime:>12}"), Style::default().fg(Color::Magenta)),
            ]));
        } else {
            lines.push(Line::from(line.as_str()));
        }
    }

    let status = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Status--ps---|----status----|--------restarts--------|---runtime----"),
    );

    f.render_widget(status, area);
}

fn render_input_with_help(f: &mut Frame, area: Rect, ui_state: &UIState) {
    let title = "Command";

    let input = Paragraph::new(ui_state.input.as_str())
        .style(Style::default())
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);

    // Show cursor at the correct position
    f.set_cursor_position((
        area.x + u16::try_from(ui_state.cursor_column()).unwrap_or(0) + 1,
        area.y + 1,
    ));
}

fn render_help_bar(f: &mut Frame, area: Rect) {
    let help_text = " q: quit | t: toggle status | start <name> | s <name>/all: stop | r <name>: restart | PgUp/PgDn/Mouse: scroll ";
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(help, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_color_assignment() {
        let mut ui_state = UIState::new();

        // gaffa always gets magenta
        assert_eq!(ui_state.get_process_color("gaffa", None), Color::Magenta);

        // Processes take the palette in order
        let color1 = ui_state.get_process_color("web", None);
        assert_eq!(color1, Color::Cyan);
        assert_eq!(ui_state.get_process_color("worker", None), Color::Yellow);
        assert_eq!(ui_state.get_process_color("scheduler", None), Color::Blue);
        assert_eq!(ui_state.get_process_color("database", None), Color::Green);

        // Same process should get same color
        assert_eq!(ui_state.get_process_color("web", None), color1);
    }

    fn typed(text: &str) -> UIState {
        let mut ui_state = UIState::new();
        for c in text.chars() {
            ui_state.insert_char(c);
        }
        ui_state
    }

    #[test]
    fn test_typing_multibyte_characters_keeps_them_in_order() {
        let mut ui_state = typed("äöü€");
        ui_state.insert_char('x');
        ui_state.insert_str("ß!");

        assert_eq!(ui_state.input, "äöü€xß!");
        assert_eq!(ui_state.cursor_position, ui_state.input.len());
    }

    #[test]
    fn test_left_and_right_step_over_whole_multibyte_characters() {
        let mut ui_state = typed("a€b");

        assert!(ui_state.move_left());
        assert!(ui_state.move_left());
        ui_state.insert_char('x');
        assert_eq!(ui_state.input, "ax€b");

        assert!(ui_state.move_right());
        ui_state.insert_char('y');
        assert_eq!(ui_state.input, "ax€yb");

        ui_state.move_home();
        assert!(!ui_state.move_left());
        ui_state.move_end();
        assert!(!ui_state.move_right());
    }

    #[test]
    fn test_backspace_and_delete_remove_whole_multibyte_characters() {
        let mut ui_state = typed("aä€");

        assert!(ui_state.backspace());
        assert_eq!(ui_state.input, "aä");

        ui_state.move_home();
        assert!(ui_state.move_right());
        assert!(ui_state.delete());
        assert_eq!(ui_state.input, "a");
        assert!(!ui_state.delete());
    }

    #[test]
    fn test_history_recall_puts_the_cursor_at_the_end() {
        let mut ui_state = UIState::new();
        ui_state.command_history = vec!["restart wörker".to_string(), "s web".to_string()];

        assert!(ui_state.history_up());
        assert!(ui_state.history_up());
        ui_state.insert_char('!');
        assert_eq!(ui_state.input, "restart wörker!");

        assert!(ui_state.history_down());
        ui_state.insert_char('?');
        assert_eq!(ui_state.input, "s web?");
    }

    #[test]
    fn test_history_recall_from_a_longer_input_does_not_leave_the_cursor_past_the_end() {
        let mut ui_state = typed("a long half-typed command");
        ui_state.command_history = vec!["s".to_string()];

        assert!(ui_state.history_up());
        ui_state.insert_char('x');
        assert_eq!(ui_state.input, "sx");
    }

    #[tokio::test]
    async fn test_the_log_file_takes_gaffas_messages_and_leaves_process_output_to_the_manager() {
        let path =
            std::env::temp_dir().join(format!("gaffa_test_ui_log_{}.log", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();

        let state = AppState::new();
        let (tx, rx) = mpsc::unbounded_channel();
        *state.tx.lock().await = Some(tx);
        state
            .add_system_log("Starting process 'web'".to_string())
            .await;
        state
            .add_log("web".to_string(), "listening".to_string(), false)
            .await;
        state
            .add_log(
                "gaffa".to_string(),
                "a process named gaffa".to_string(),
                false,
            )
            .await;
        *state.tx.lock().await = None;

        let logs = Arc::new(Mutex::new(VecDeque::new()));
        collect_logs(rx, Arc::clone(&logs), Some(Arc::new(Mutex::new(file)))).await;

        let written = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(written.lines().count(), 1, "got: {written}");
        assert!(
            written.contains("[gaffa] Starting process 'web'"),
            "got: {written}"
        );
        assert_eq!(logs.lock().await.len(), 3);
    }

    #[tokio::test]
    async fn test_a_failed_ui_task_is_the_result_while_commands_and_interrupt_still_wait() {
        let ui_handle = tokio::task::spawn_blocking(|| {
            Err(ProcessError::InputRead(std::io::Error::other(
                "no terminal",
            )))
        });
        let command_handle = tokio::spawn(std::future::pending::<()>());

        let result = wait_for_ui(ui_handle, command_handle, std::future::pending()).await;

        assert!(
            matches!(result, Err(ProcessError::InputRead(ref e)) if e.to_string() == "no terminal"),
            "got: {result:?}"
        );
    }

    #[test]
    fn test_restoring_the_panic_hook_reinstates_the_one_before() {
        use std::sync::atomic::{AtomicBool, Ordering};

        static BEFORE_RAN: AtomicBool = AtomicBool::new(false);
        std::panic::set_hook(Box::new(|_| BEFORE_RAN.store(true, Ordering::SeqCst)));

        let original = install_cleanup_panic_hook();
        restore_panic_hook(original);
        let _ = std::panic::catch_unwind(|| panic!("probe"));
        drop(std::panic::take_hook());

        assert!(BEFORE_RAN.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cursor_column_counts_characters_not_bytes() {
        let mut ui_state = typed("äöü€");
        assert_eq!(ui_state.cursor_column(), 4);
        ui_state.move_left();
        assert_eq!(ui_state.cursor_column(), 3);
    }

    #[test]
    fn test_cursor_column_counts_wide_characters_as_two_columns() {
        let mut ui_state = typed("a中🚀");
        assert_eq!(ui_state.cursor_column(), 5);
        ui_state.move_left();
        assert_eq!(ui_state.cursor_column(), 3);
    }
}
